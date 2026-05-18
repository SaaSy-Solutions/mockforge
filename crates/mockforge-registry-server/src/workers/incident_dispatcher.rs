//! Notification-channel dispatcher (cloud-enablement task #3 / Phase 2).
//!
//! Polls `incidents` for open rows that haven't been dispatched yet,
//! evaluates each against the org's `routing_rules`, and fans out to
//! every matched `notification_channels` row. Per-channel results land
//! in `incident_events` as `notification_sent`; the per-incident
//! "we're done with this one" marker is `notification_dispatched`.
//!
//! Real outbound for `webhook`, `slack`, and `email` channels. The
//! `email` path uses the shared `crate::email::EmailService`, which
//! also powers verification/reset/security alerts; it auto-picks the
//! configured provider (Postmark / Brevo / SMTP) from env. When no
//! provider is configured the attempt is recorded as `skipped` with a
//! reason that surfaces the missing config — the dispatcher does not
//! silently pretend to send. `pagerduty` is still skipped pending the
//! Events-API mapping (#552).
//!
//! Reliability:
//! - 5s tick cadence so a real outage shows up in seconds, not minutes.
//! - Per-call 10s HTTP timeout so a slow channel can't stall the whole
//!   queue.
//! - The `notification_dispatched` marker is written even on per-channel
//!   failure — partial failure is logged, not retried, because most of
//!   the failures are 4xx (bad webhook URL) and a tight retry would
//!   spam the operator's webhook receiver. Retry handling is a separate
//!   concern.

use std::time::Duration;

use mockforge_registry_core::models::{Incident, NotificationChannel, RoutingRule};
use sqlx::PgPool;
use tracing::{debug, error, info, warn};

const TICK_INTERVAL: Duration = Duration::from_secs(5);
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
const BATCH_LIMIT: i64 = 50;

pub fn start_incident_dispatcher_worker(pool: PgPool) {
    let client = match reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .user_agent("mockforge-registry/1.0 (incident-dispatcher)")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "incident dispatcher: failed to build HTTP client; worker disabled");
            return;
        }
    };

    info!(
        "incident dispatcher worker started — ticking every {}s, batch={}",
        TICK_INTERVAL.as_secs(),
        BATCH_LIMIT
    );
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(TICK_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // Skip the immediate first tick so workers settle on boot.
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(e) = run_tick(&pool, &client).await {
                error!(error = %e, "incident dispatcher tick failed");
            }
        }
    });
}

/// One polling tick. Returns the number of incidents dispatched this
/// tick (for tests + observability).
pub async fn run_tick(pool: &PgPool, client: &reqwest::Client) -> sqlx::Result<u32> {
    let pending = Incident::list_pending_dispatch(pool, BATCH_LIMIT).await?;
    if pending.is_empty() {
        return Ok(0);
    }

    debug!(count = pending.len(), "incident dispatcher: processing batch");

    let mut dispatched = 0u32;
    for incident in pending {
        match dispatch_one(pool, client, &incident).await {
            Ok(_) => dispatched += 1,
            Err(e) => {
                error!(
                    incident_id = %incident.id,
                    error = %e,
                    "incident dispatch failed; will retry next tick",
                );
                // Don't mark_dispatched on hard error — let the next tick
                // try again. The error here is a DB-side problem, not a
                // per-channel HTTP failure (those are recorded inline).
            }
        }
    }
    Ok(dispatched)
}

async fn dispatch_one(
    pool: &PgPool,
    client: &reqwest::Client,
    incident: &Incident,
) -> sqlx::Result<()> {
    // 1. Find the highest-priority matching rule. If no rule matches,
    //    fall back to "all enabled channels for the org" — the user
    //    intent is "tell me about my incidents," not "swallow them
    //    silently because I haven't configured routing yet."
    let rules = RoutingRule::list_by_org(pool, incident.org_id).await?;
    let matched = rules
        .iter()
        .find(|r| r.matches(&incident.severity, &incident.source, incident.workspace_id));

    let channel_ids = match matched {
        Some(rule) => rule.channel_ids.clone(),
        None => NotificationChannel::list_by_org(pool, incident.org_id)
            .await?
            .into_iter()
            .filter(|c| c.enabled)
            .map(|c| c.id)
            .collect(),
    };

    if channel_ids.is_empty() {
        warn!(
            incident_id = %incident.id,
            org_id = %incident.org_id,
            "no notification channels configured — marking dispatched to avoid retry loop",
        );
        Incident::mark_dispatched(
            pool,
            incident.id,
            &serde_json::json!({ "channels": 0, "reason": "no_channels_configured" }),
        )
        .await?;
        return Ok(());
    }

    let mut successes = 0u32;
    let mut failures = 0u32;
    let mut skipped = 0u32;

    for channel_id in &channel_ids {
        let channel = match NotificationChannel::find_by_id(pool, *channel_id).await? {
            Some(c) if c.enabled => c,
            _ => continue, // Channel deleted / disabled since rule was authored.
        };

        let result = send_to_channel(client, &channel, incident).await;
        match result {
            ChannelResult::Sent { status_code } => {
                successes += 1;
                Incident::record_notification_attempt(
                    pool,
                    incident.id,
                    channel.id,
                    &serde_json::json!({
                        "ok": true,
                        "kind": channel.kind,
                        "status_code": status_code,
                    }),
                )
                .await?;
            }
            ChannelResult::Failed { error } => {
                failures += 1;
                Incident::record_notification_attempt(
                    pool,
                    incident.id,
                    channel.id,
                    &serde_json::json!({
                        "ok": false,
                        "kind": channel.kind,
                        "error": error,
                    }),
                )
                .await?;
            }
            ChannelResult::Skipped { reason } => {
                skipped += 1;
                Incident::record_notification_attempt(
                    pool,
                    incident.id,
                    channel.id,
                    &serde_json::json!({
                        "ok": false,
                        "kind": channel.kind,
                        "skipped": true,
                        "reason": reason,
                    }),
                )
                .await?;
            }
        }
    }

    Incident::mark_dispatched(
        pool,
        incident.id,
        &serde_json::json!({
            "channels_total": channel_ids.len(),
            "successes": successes,
            "failures": failures,
            "skipped": skipped,
            "rule_id": matched.map(|r| r.id),
        }),
    )
    .await?;

    if failures > 0 {
        warn!(
            incident_id = %incident.id,
            successes,
            failures,
            skipped,
            "incident dispatched with partial failures",
        );
    } else {
        info!(
            incident_id = %incident.id,
            successes,
            skipped,
            "incident dispatched",
        );
    }
    Ok(())
}

#[derive(Debug)]
enum ChannelResult {
    Sent { status_code: u16 },
    Failed { error: String },
    Skipped { reason: String },
}

async fn send_to_channel(
    client: &reqwest::Client,
    channel: &NotificationChannel,
    incident: &Incident,
) -> ChannelResult {
    match channel.kind.as_str() {
        "webhook" | "slack" => post_webhook_style(client, channel, incident).await,
        "email" => send_email(channel, incident).await,
        "pagerduty" => ChannelResult::Skipped {
            reason: "pagerduty channel: Events API mapping not yet wired".into(),
        },
        other => ChannelResult::Skipped {
            reason: format!("unknown channel kind: {other}"),
        },
    }
}

/// Webhook + Slack incoming-webhook style: POST a JSON body to a URL
/// pulled from `channel.config.url`. Slack accepts the same shape via
/// its incoming-webhook endpoint when the body has a `text` field, so
/// we include both a structured payload and a flattened `text` summary.
async fn post_webhook_style(
    client: &reqwest::Client,
    channel: &NotificationChannel,
    incident: &Incident,
) -> ChannelResult {
    let url = match channel.config.get("url").and_then(|v| v.as_str()) {
        Some(u) if !u.is_empty() => u.to_string(),
        _ => {
            return ChannelResult::Failed {
                error: "channel.config.url missing or empty".into(),
            };
        }
    };

    let summary =
        format!("[{}] {}: {}", incident.severity.to_uppercase(), incident.source, incident.title);
    let body = serde_json::json!({
        "text": summary,
        "incident": {
            "id": incident.id,
            "org_id": incident.org_id,
            "workspace_id": incident.workspace_id,
            "source": incident.source,
            "source_ref": incident.source_ref,
            "severity": incident.severity,
            "status": incident.status,
            "title": incident.title,
            "description": incident.description,
            "created_at": incident.created_at,
        },
    });

    match client.post(&url).json(&body).send().await {
        Ok(resp) => {
            let status_code = resp.status().as_u16();
            if resp.status().is_success() {
                ChannelResult::Sent { status_code }
            } else {
                ChannelResult::Failed {
                    error: format!("webhook returned HTTP {status_code}"),
                }
            }
        }
        Err(e) => ChannelResult::Failed {
            error: e.to_string(),
        },
    }
}

/// Email channel. Reads recipient from `channel.config.to` (a single
/// address string). The send goes through `EmailService::from_env()`,
/// which auto-picks Postmark / Brevo / SMTP based on `EMAIL_PROVIDER`
/// — the same env-driven configuration the registry uses for
/// verification, password-reset, and security-alert mails. When no
/// provider is configured we record `skipped` with a reason rather
/// than silently "sending" into the Disabled no-op, so the operator
/// can tell their alert is not actually going out.
async fn send_email(channel: &NotificationChannel, incident: &Incident) -> ChannelResult {
    use crate::email::{EmailMessage, EmailService};

    let to = match channel.config.get("to").and_then(|v| v.as_str()) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => {
            return ChannelResult::Failed {
                error: "channel.config.to missing or empty (expected a single email address)"
                    .into(),
            };
        }
    };

    let service = match EmailService::from_env() {
        Ok(s) => s,
        Err(e) => {
            return ChannelResult::Failed {
                error: format!("email service init failed: {e}"),
            };
        }
    };

    if !service.is_configured() {
        return ChannelResult::Skipped {
            reason: "email provider not configured on registry server \
                 (set EMAIL_PROVIDER + provider-specific env vars)"
                .into(),
        };
    }

    let subject =
        format!("[{}] {}: {}", incident.severity.to_uppercase(), incident.source, incident.title);
    let description = incident.description.as_deref().unwrap_or("(no description)");

    let text_body = format!(
        "MockForge incident\n\n\
         Severity:    {sev}\n\
         Source:      {src}\n\
         Title:       {title}\n\
         Description: {desc}\n\
         \n\
         Incident ID: {id}\n\
         Org:         {org}\n\
         Created at:  {created}\n",
        sev = incident.severity,
        src = incident.source,
        title = incident.title,
        desc = description,
        id = incident.id,
        org = incident.org_id,
        created = incident.created_at,
    );

    let html_body = format!(
        "<!DOCTYPE html><html><body style=\"font-family:-apple-system,Segoe UI,Roboto,sans-serif\">\
         <h2 style=\"margin:0 0 8px 0\">[{sev_caps}] {title}</h2>\
         <p style=\"color:#555\">Source: <code>{src}</code></p>\
         <p>{desc_html}</p>\
         <hr><table style=\"font-size:13px;color:#666\">\
         <tr><td>Incident ID</td><td><code>{id}</code></td></tr>\
         <tr><td>Org</td><td><code>{org}</code></td></tr>\
         <tr><td>Created</td><td>{created}</td></tr>\
         </table></body></html>",
        sev_caps = incident.severity.to_uppercase(),
        title = html_escape(&incident.title),
        src = html_escape(&incident.source),
        desc_html = html_escape(description),
        id = incident.id,
        org = incident.org_id,
        created = incident.created_at,
    );

    let message = EmailMessage {
        to,
        subject,
        html_body,
        text_body,
    };

    match service.send(message).await {
        // 250 is the SMTP "requested mail action okay, completed" code;
        // for API providers (Postmark/Brevo) we still use 250 as the
        // "the provider accepted the send" marker so the JSON shape in
        // `incident_events` is uniform with the webhook-style channels.
        Ok(()) => ChannelResult::Sent { status_code: 250 },
        Err(e) => ChannelResult::Failed {
            error: format!("email send failed via {}: {e}", service.provider_name()),
        },
    }
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// User-facing test-fire: post a synthetic incident-shaped payload to a
/// single channel and return the result. Used by the
/// `POST /notification-channels/{id}/test-fire` route so operators can
/// validate URLs without raising a real incident.
pub async fn test_fire(channel: &NotificationChannel) -> serde_json::Value {
    let client = match reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .user_agent("mockforge-registry/1.0 (test-fire)")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return serde_json::json!({
                "ok": false,
                "kind": channel.kind,
                "error": format!("failed to build HTTP client: {e}"),
            });
        }
    };
    let synthetic = synthetic_incident(channel.org_id);
    let result = send_to_channel(&client, channel, &synthetic).await;
    match result {
        ChannelResult::Sent { status_code } => serde_json::json!({
            "ok": true,
            "kind": channel.kind,
            "status_code": status_code,
        }),
        ChannelResult::Failed { error } => serde_json::json!({
            "ok": false,
            "kind": channel.kind,
            "error": error,
        }),
        ChannelResult::Skipped { reason } => serde_json::json!({
            "ok": false,
            "kind": channel.kind,
            "skipped": true,
            "reason": reason,
        }),
    }
}

/// Build a fake Incident the test-fire path can post. Field values are
/// recognizable so the operator's webhook receiver can render them as
/// "this is a test" rather than a real alert. Doesn't touch the DB.
fn synthetic_incident(org_id: uuid::Uuid) -> Incident {
    use chrono::Utc;
    Incident {
        id: uuid::Uuid::nil(),
        org_id,
        workspace_id: None,
        source: "mockforge.test_fire".into(),
        source_ref: None,
        dedupe_key: format!("test-fire-{}", Utc::now().timestamp()),
        severity: "low".into(),
        status: "open".into(),
        title: "MockForge test notification".into(),
        description: Some(
            "This is a test message from the notification-channel test-fire \
             endpoint. If you're seeing this, the channel is wired up correctly."
                .into(),
        ),
        postmortem_url: None,
        assigned_to: None,
        acknowledged_by: None,
        acknowledged_at: None,
        resolved_by: None,
        resolved_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    // The pure routing logic is exercised in routing_rule.rs::tests; the
    // dispatcher wires that to DB + HTTP. We don't unit-test the wiring
    // here — it'd require a Postgres + a fake webhook server, both of
    // which belong in integration tests under tests/. The matcher itself
    // already has coverage there.

    use super::*;

    fn fake_incident() -> Incident {
        synthetic_incident(uuid::Uuid::nil())
    }

    fn email_channel(config: serde_json::Value) -> NotificationChannel {
        NotificationChannel {
            id: uuid::Uuid::nil(),
            org_id: uuid::Uuid::nil(),
            name: "test-email".into(),
            kind: "email".into(),
            config,
            enabled: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn smoke_module_links() {
        // Anchor: this module must exist for the registry main.rs wiring
        // to compile. If the worker is removed, main.rs breaks first
        // — this test is just a no-op safety net for the cfg(test) build.
    }

    #[tokio::test]
    async fn email_missing_to_is_failure() {
        // No `to` field — operator misconfigured the channel. We surface
        // this as Failed (not Skipped) because skipped means "wiring is
        // fine but provider isn't ready"; this is a real config bug the
        // operator needs to fix on their side.
        let channel = email_channel(serde_json::json!({}));
        let result = send_email(&channel, &fake_incident()).await;
        match result {
            ChannelResult::Failed { error } => {
                assert!(error.contains("channel.config.to"), "unexpected error: {error}");
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn email_blank_to_is_failure() {
        let channel = email_channel(serde_json::json!({ "to": "   " }));
        let result = send_email(&channel, &fake_incident()).await;
        assert!(matches!(result, ChannelResult::Failed { .. }));
    }

    #[test]
    fn html_escape_handles_injection_chars() {
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    }
}
