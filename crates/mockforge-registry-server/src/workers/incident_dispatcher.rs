//! Notification-channel dispatcher (cloud-enablement task #3 / Phase 2).
//!
//! Polls `incidents` for open rows that haven't been dispatched yet,
//! evaluates each against the org's `routing_rules`, and fans out to
//! every matched `notification_channels` row. Per-channel results land
//! in `incident_events` as `notification_sent`; the per-incident
//! "we're done with this one" marker is `notification_dispatched`.
//!
//! Real outbound for all four channel kinds: `webhook` and `slack`
//! post a JSON body to `channel.config.url`; `email` uses the shared
//! `crate::email::EmailService` (Postmark / Brevo / SMTP, picked from
//! env); `pagerduty` posts an Events API v2 `trigger` event using the
//! channel's stored `routing_key`. The PagerDuty `dedup_key` is the
//! incident UUID so a future resolve-side dispatcher (not yet
//! implemented — the current tick only fires on incident open) can
//! close the same alert via `event_action: "resolve"`. When the email
//! provider isn't configured the attempt is recorded as `skipped`
//! with a reason that surfaces the missing config — the dispatcher
//! does not silently pretend to send.
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
        "pagerduty" => post_pagerduty(client, channel, incident).await,
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

/// PagerDuty Events API v2: POST a `trigger` event to
/// `https://events.pagerduty.com/v2/enqueue`. The routing key lives in
/// `channel.config.routing_key` (or `channel.config.integration_key` —
/// both names are common in PagerDuty docs, so we accept either). The
/// `dedup_key` is the incident UUID, so a future resolve-side
/// dispatcher can close the same alert by replaying the same event
/// with `event_action: "resolve"`. Custom event URL via
/// `MOCKFORGE_PAGERDUTY_ENQUEUE_URL` for tests.
async fn post_pagerduty(
    client: &reqwest::Client,
    channel: &NotificationChannel,
    incident: &Incident,
) -> ChannelResult {
    let routing_key = match channel
        .config
        .get("routing_key")
        .or_else(|| channel.config.get("integration_key"))
        .and_then(|v| v.as_str())
    {
        Some(k) if !k.trim().is_empty() => k.trim().to_string(),
        _ => {
            return ChannelResult::Failed {
                error: "channel.config.routing_key missing or empty \
                    (PagerDuty integration key, 32 hex chars)"
                    .into(),
            };
        }
    };

    let url = std::env::var("MOCKFORGE_PAGERDUTY_ENQUEUE_URL")
        .unwrap_or_else(|_| "https://events.pagerduty.com/v2/enqueue".to_string());

    let summary =
        format!("[{}] {}: {}", incident.severity.to_uppercase(), incident.source, incident.title);

    let body = serde_json::json!({
        "routing_key": routing_key,
        "event_action": "trigger",
        // Dedupe on the incident UUID so retries within the dispatcher
        // (or a future resolve-side replay) collapse onto one alert
        // instead of fanning out into per-tick noise.
        "dedup_key": incident.id.to_string(),
        "payload": {
            "summary": summary,
            "source": incident.source,
            "severity": pagerduty_severity(&incident.severity),
            "component": incident.source_ref,
            "custom_details": {
                "incident_id": incident.id,
                "org_id": incident.org_id,
                "workspace_id": incident.workspace_id,
                "title": incident.title,
                "description": incident.description,
                "status": incident.status,
                "created_at": incident.created_at,
            },
        },
    });

    match client.post(&url).json(&body).send().await {
        Ok(resp) => {
            let status_code = resp.status().as_u16();
            if resp.status().is_success() {
                // PagerDuty returns 202 Accepted on success with a JSON
                // envelope `{ "status": "success", "message": "...",
                // "dedup_key": "..." }`. We don't parse it — the HTTP
                // code is enough.
                ChannelResult::Sent { status_code }
            } else {
                // Read the body for the error detail when we can. PD
                // returns useful messages like "routing_key invalid"
                // here.
                let detail = resp.text().await.unwrap_or_default();
                let truncated = detail.chars().take(200).collect::<String>();
                ChannelResult::Failed {
                    error: format!("PagerDuty returned HTTP {status_code}: {truncated}"),
                }
            }
        }
        Err(e) => ChannelResult::Failed {
            error: e.to_string(),
        },
    }
}

/// Map MockForge incident severities to the four PagerDuty Events API
/// v2 severities (`critical`, `error`, `warning`, `info`). Anything we
/// don't explicitly recognise downgrades to `info` — better to receive
/// a low-priority alert than to have PagerDuty reject the event for an
/// invalid severity.
fn pagerduty_severity(severity: &str) -> &'static str {
    match severity.to_ascii_lowercase().as_str() {
        "critical" | "fatal" => "critical",
        "error" | "high" => "error",
        "warning" | "warn" | "medium" => "warning",
        _ => "info",
    }
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

    fn pagerduty_channel(config: serde_json::Value) -> NotificationChannel {
        NotificationChannel {
            id: uuid::Uuid::nil(),
            org_id: uuid::Uuid::nil(),
            name: "test-pd".into(),
            kind: "pagerduty".into(),
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

    #[test]
    fn pd_severity_maps_known_buckets() {
        assert_eq!(pagerduty_severity("CRITICAL"), "critical");
        assert_eq!(pagerduty_severity("fatal"), "critical");
        assert_eq!(pagerduty_severity("error"), "error");
        assert_eq!(pagerduty_severity("HIGH"), "error");
        assert_eq!(pagerduty_severity("warning"), "warning");
        assert_eq!(pagerduty_severity("medium"), "warning");
        assert_eq!(pagerduty_severity("info"), "info");
        assert_eq!(pagerduty_severity("low"), "info");
        assert_eq!(pagerduty_severity("something-weird"), "info");
        assert_eq!(pagerduty_severity(""), "info");
    }

    #[tokio::test]
    async fn pd_missing_routing_key_is_failure() {
        let client = reqwest::Client::new();
        let channel = pagerduty_channel(serde_json::json!({}));
        let result = post_pagerduty(&client, &channel, &fake_incident()).await;
        match result {
            ChannelResult::Failed { error } => {
                assert!(error.contains("routing_key"), "unexpected error: {error}");
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn pd_accepts_integration_key_alias() {
        // PagerDuty docs use both names interchangeably. We accept
        // either; the missing-key path should NOT fire when only
        // `integration_key` is present. We verify by pointing the
        // dispatcher at a 127.0.0.1 port that nothing's listening on
        // and asserting the failure is a network error (the request
        // was actually attempted), not a missing-key validation error.
        std::env::set_var("MOCKFORGE_PAGERDUTY_ENQUEUE_URL", "http://127.0.0.1:1/v2/enqueue");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(200))
            .build()
            .unwrap();
        let channel = pagerduty_channel(serde_json::json!({ "integration_key": "abc123" }));
        let result = post_pagerduty(&client, &channel, &fake_incident()).await;
        match result {
            ChannelResult::Failed { error } => {
                // Network error (connection refused / timeout), NOT the
                // routing_key validation error.
                assert!(
                    !error.contains("routing_key"),
                    "should have accepted integration_key but got validation error: {error}"
                );
            }
            other => panic!("expected Failed (network), got {other:?}"),
        }
        std::env::remove_var("MOCKFORGE_PAGERDUTY_ENQUEUE_URL");
    }

    #[tokio::test]
    async fn pd_blank_routing_key_is_failure() {
        let client = reqwest::Client::new();
        let channel = pagerduty_channel(serde_json::json!({ "routing_key": "   " }));
        let result = post_pagerduty(&client, &channel, &fake_incident()).await;
        assert!(matches!(result, ChannelResult::Failed { .. }));
    }
}
