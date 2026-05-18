//! Real-time Fly.io runtime-log subscription over NATS (#556).
//!
//! Fly publishes every container log line to its internal NATS proxy on
//! subject `logs.<app>.<region>.<machine_id>`. Subscribing gives us
//! sub-second delivery, where REST polling in [`crate::fly_logs`] was
//! capped at the loop's tick cadence (currently 2s) and added latency
//! for the round-trip plus Fly's own buffering on the GET endpoint.
//!
//! ## Reachability constraint
//!
//! The NATS proxy is reachable at `[fdaa::3]:4223` only from inside a
//! Fly app's 6PN network (or via WireGuard). The registry server is a
//! Fly app — that's where production traffic to this code lives — so
//! the connection succeeds. Self-hosted and local-dev installs have no
//! route to that address and must fall back to the REST poller; that
//! fallback is the caller's responsibility (see the SSE handler in
//! `handlers::hosted_mocks::build_runtime_logs_stream`).
//!
//! ## Auth
//!
//! Org slug as username, read-only PAT as password. The token is the
//! same `FLYIO_API_TOKEN` env var already used by [`crate::fly_logs`];
//! `FLY_ORG_SLUG` is the new env var this module introduces.
//!
//! ## Stability caveat
//!
//! Fly documents the NATS proxy as "internal transport without a
//! versioned API or stability guarantee." That's why the fallback to
//! REST polling is wired unconditionally — if the proxy moves or
//! changes shape, the runtime-logs UI degrades to polling rather than
//! breaking.

use async_nats::{Client, ConnectOptions, Subscriber};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::time::Duration;
use tracing::{debug, info};

use crate::fly_logs::LogEntry;

const DEFAULT_NATS_URL: &str = "nats://[fdaa::3]:4223";
const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 1500;

/// Connection settings derived from env. `None` means we don't have
/// enough config to even attempt a NATS connection (no org slug or no
/// API token) — callers should fall back to polling without warning.
#[derive(Debug, Clone)]
pub struct FlyNatsConfig {
    pub url: String,
    pub org_slug: String,
    pub token: String,
    pub connect_timeout: Duration,
}

impl FlyNatsConfig {
    pub fn from_env() -> Option<Self> {
        let org_slug = std::env::var("FLY_ORG_SLUG").ok()?;
        let token = std::env::var("FLYIO_API_TOKEN").ok()?;
        let url = std::env::var("FLY_NATS_URL").unwrap_or_else(|_| DEFAULT_NATS_URL.to_string());
        let connect_timeout_ms = std::env::var("FLY_NATS_CONNECT_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_CONNECT_TIMEOUT_MS);
        Some(Self {
            url,
            org_slug,
            token,
            connect_timeout: Duration::from_millis(connect_timeout_ms),
        })
    }
}

/// A live NATS subscription to one Fly app's logs. Owns both the
/// underlying client and the subscriber — dropping this struct closes
/// the connection and ends the subscription. The SSE handler holds it
/// for the lifetime of one streamed response.
pub struct FlyLogSubscription {
    // Field order matters for Drop: subscriber drops first, then the
    // client tears down the connection. Rust's drop order is
    // declaration order, so subscriber must come before client here.
    pub subscriber: Subscriber,
    _client: Client,
}

/// Subscribe to live runtime logs for one Fly app.
///
/// Returns a [`FlyLogSubscription`] whose `subscriber` yields raw
/// messages on `logs.<app_name>.>` (all regions, all machines). Caller
/// parses each message via [`parse_message`] and is responsible for
/// surfacing errors / disconnections.
///
/// Connection is per-subscription rather than process-wide so each SSE
/// stream gets its own short-lived connection — that keeps the failure
/// mode local (one bad subscription doesn't poison everyone) at the
/// cost of a few hundred milliseconds per stream start. If we ever
/// have hundreds of concurrent log viewers, switch to a shared
/// connection + per-subscriber subscription using a single shared
/// `async_nats::Client`.
pub async fn subscribe_app(
    config: &FlyNatsConfig,
    app_name: &str,
) -> Result<FlyLogSubscription, async_nats::Error> {
    debug!(app = %app_name, url = %config.url, "connecting to Fly NATS proxy");
    let client = ConnectOptions::new()
        .user_and_password(config.org_slug.clone(), config.token.clone())
        .connection_timeout(config.connect_timeout)
        // No reconnect retries — the SSE handler owns the lifecycle and
        // will rebuild the subscription on a fresh connection if the
        // user reopens the stream. Reconnect logic in async-nats can
        // mask "this NATS proxy disappeared, fall back to polling."
        .max_reconnects(Some(0))
        .name(format!("mockforge-registry runtime-logs ({app_name})"))
        .connect(&config.url)
        .await?;
    let subject = format!("logs.{app_name}.>");
    info!(app = %app_name, subject = %subject, "subscribed to Fly NATS log subject");
    let subscriber = client.subscribe(subject).await?;
    Ok(FlyLogSubscription {
        subscriber,
        _client: client,
    })
}

/// Parse a Fly NATS log message into the shape the runtime-logs SSE
/// stream already uses. Returns `None` for messages we can't parse —
/// quietly dropped rather than failed so an unexpected payload shape
/// degrades to "we missed one line" instead of closing the stream.
///
/// Fly's wire format historically wraps logs in a Vector-style
/// envelope; we accept several common shapes and pull the same fields
/// regardless of nesting.
pub fn parse_message(payload: &[u8]) -> Option<LogEntry> {
    let raw: FlyLogPayload = serde_json::from_slice(payload).ok()?;
    let timestamp = raw
        .timestamp
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let level = raw
        .log
        .as_ref()
        .and_then(|l| l.level.clone())
        .or(raw.level)
        .unwrap_or_else(|| "info".to_string());
    let message = raw.message?;
    let instance = raw.fly.as_ref().and_then(|f| f.app.as_ref()).and_then(|a| a.instance.clone());
    let region = raw.fly.and_then(|f| f.region);
    Some(LogEntry {
        timestamp,
        level,
        message,
        instance,
        region,
    })
}

#[derive(Debug, Deserialize)]
struct FlyLogPayload {
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    level: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    log: Option<FlyLogInner>,
    #[serde(default)]
    fly: Option<FlyEnvelope>,
}

#[derive(Debug, Deserialize)]
struct FlyLogInner {
    #[serde(default)]
    level: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FlyEnvelope {
    #[serde(default)]
    region: Option<String>,
    #[serde(default)]
    app: Option<FlyApp>,
}

#[derive(Debug, Deserialize)]
struct FlyApp {
    #[serde(default)]
    instance: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_requires_org_and_token() {
        std::env::remove_var("FLY_ORG_SLUG");
        std::env::remove_var("FLYIO_API_TOKEN");
        assert!(FlyNatsConfig::from_env().is_none());

        std::env::set_var("FLY_ORG_SLUG", "test-org");
        assert!(FlyNatsConfig::from_env().is_none(), "org without token should be None");

        std::env::set_var("FLYIO_API_TOKEN", "test-token-value");
        let cfg = FlyNatsConfig::from_env().expect("both set");
        assert_eq!(cfg.org_slug, "test-org");
        assert_eq!(cfg.token, "test-token-value");
        assert_eq!(cfg.url, DEFAULT_NATS_URL);

        std::env::set_var("FLY_NATS_URL", "nats://localhost:4222");
        let cfg = FlyNatsConfig::from_env().unwrap();
        assert_eq!(cfg.url, "nats://localhost:4222");

        std::env::remove_var("FLY_NATS_URL");
        std::env::remove_var("FLY_ORG_SLUG");
        std::env::remove_var("FLYIO_API_TOKEN");
    }

    #[test]
    fn parses_vector_envelope_form() {
        // Shape Vector/Fly typically emits — nested `log` and `fly`
        // objects with the metadata we care about.
        let raw = br#"{
            "timestamp": "2026-05-18T10:00:00Z",
            "log": { "level": "warn" },
            "message": "GET /api/users 500",
            "fly": {
                "region": "lhr",
                "app": { "instance": "machineabc123", "name": "my-app" }
            }
        }"#;
        let entry = parse_message(raw).expect("parses");
        assert_eq!(entry.message, "GET /api/users 500");
        assert_eq!(entry.level, "warn");
        assert_eq!(entry.region.as_deref(), Some("lhr"));
        assert_eq!(entry.instance.as_deref(), Some("machineabc123"));
    }

    #[test]
    fn parses_flat_form() {
        // Some Fly publishers (Machines API direct) emit a flatter
        // shape with top-level `level` instead of nested `log.level`.
        let raw = br#"{
            "timestamp": "2026-05-18T10:00:00Z",
            "level": "error",
            "message": "boom"
        }"#;
        let entry = parse_message(raw).expect("parses");
        assert_eq!(entry.level, "error");
        assert_eq!(entry.message, "boom");
        assert!(entry.region.is_none());
    }

    #[test]
    fn drops_message_without_text() {
        let raw = br#"{ "timestamp": "2026-05-18T10:00:00Z", "level": "info" }"#;
        assert!(parse_message(raw).is_none());
    }

    #[test]
    fn drops_unparsable_payload() {
        assert!(parse_message(b"not json").is_none());
    }

    #[test]
    fn defaults_missing_level_to_info() {
        let raw = br#"{ "message": "hi", "timestamp": "2026-05-18T10:00:00Z" }"#;
        let entry = parse_message(raw).unwrap();
        assert_eq!(entry.level, "info");
    }
}
