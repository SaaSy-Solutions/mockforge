//! Fly.io runtime log client for hosted-mock deployments.
//!
//! `GET /api/v1/hosted-mocks/{id}/logs` historically returned rows from the
//! local `deployment_logs` table — deployment lifecycle events (created,
//! deploying, deploy complete, errors), capped at 100. That endpoint stays as
//! "events" for the UI's Events tab.
//!
//! This module adds the missing piece: actual runtime logs from the Fly
//! machine running each hosted mock, surfaced via two new endpoints:
//!
//! - `GET /api/v1/hosted-mocks/{id}/runtime-logs` — REST pull, last N entries.
//! - `GET /api/v1/hosted-mocks/{id}/runtime-logs/stream` — SSE that polls Fly
//!   every couple of seconds and streams new entries to the browser.
//!
//! Configuration via environment variables (all optional — if `FLYIO_API_TOKEN`
//! is unset the endpoints return an empty list and the SSE stream emits a
//! "not configured" event then closes):
//!
//! - `FLYIO_API_TOKEN` — bearer token. Same one the orchestrator uses.
//! - `FLY_LOGS_URL` — base URL of the Fly logs REST API. Defaults to
//!   `https://api.fly.io/api/v1`.
//! - `FLY_LOGS_TIMEOUT_MS` — per-request timeout (default 5000).
//! - `FLY_LOGS_DEFAULT_LIMIT` — REST default page size (default 200).

use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::Duration;
use tracing::debug;

/// One log line emitted by a Fly machine running a hosted mock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

/// Pull-based Fly logs client. Real-time NATS subscription is a follow-up
/// (#232 Phase 6 looks at structured shipping from the container instead);
/// polling buys us a usable runtime-logs view today.
#[derive(Clone)]
pub struct FlyLogsClient {
    base_url: String,
    token: String,
    default_limit: u32,
    http: Client,
}

impl FlyLogsClient {
    /// Build a client from environment variables. Returns `None` when the Fly
    /// API token isn't set — handlers degrade to empty / "not configured".
    pub fn from_env() -> Option<Self> {
        let token = std::env::var("FLYIO_API_TOKEN").ok()?;
        let base_url = std::env::var("FLY_LOGS_URL")
            .unwrap_or_else(|_| "https://api.fly.io/api/v1".to_string());
        let timeout_ms = std::env::var("FLY_LOGS_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5000);
        let default_limit = std::env::var("FLY_LOGS_DEFAULT_LIMIT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(200);

        let http = Client::builder().timeout(Duration::from_millis(timeout_ms)).build().ok()?;

        Some(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
            default_limit,
            http,
        })
    }

    /// Fetch recent log entries for a Fly app.
    ///
    /// `since` filters to entries strictly newer than the given timestamp —
    /// used by the SSE poll loop to avoid re-emitting lines.
    /// `limit` overrides the env-configured default.
    pub async fn fetch_recent(
        &self,
        app_name: &str,
        since: Option<DateTime<Utc>>,
        limit: Option<u32>,
    ) -> Result<Vec<LogEntry>, FlyLogsError> {
        let limit = limit.unwrap_or(self.default_limit);
        let url = format!("{}/apps/{}/logs", self.base_url, app_name);
        let mut req = self.http.get(&url).bearer_auth(&self.token).query(&[("limit", limit)]);
        if let Some(ts) = since {
            // Fly accepts an RFC3339 `since` parameter on its logs endpoint.
            req = req.query(&[("since", ts.to_rfc3339())]);
        }

        debug!(app = %app_name, "Fetching Fly runtime logs");
        let resp = req.send().await.map_err(FlyLogsError::Request)?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(FlyLogsError::Status {
                status: status.as_u16(),
                body,
            });
        }

        let raw = resp.text().await.map_err(FlyLogsError::Request)?;
        Ok(parse_log_payload(&raw))
    }
}

/// Parse a Fly logs response. The API has shifted shape over the years; we try
/// two common forms and fall back to NDJSON. Anything we can't parse is
/// dropped silently — better an incomplete log view than a 500 on the
/// admin UI.
fn parse_log_payload(raw: &str) -> Vec<LogEntry> {
    // Form 1: { "data": [ { ...attributes } ] } — the JSON:API-ish wrapping.
    if let Ok(wrapped) = serde_json::from_str::<JsonApiWrapper>(raw) {
        return wrapped.into_entries();
    }
    // Form 2: bare JSON array of entries.
    if let Ok(entries) = serde_json::from_str::<Vec<RawLogLine>>(raw) {
        return entries.into_iter().filter_map(RawLogLine::into_entry).collect();
    }
    // Form 3: NDJSON — one JSON object per line.
    let mut out = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(line_struct) = serde_json::from_str::<RawLogLine>(trimmed) {
            if let Some(entry) = line_struct.into_entry() {
                out.push(entry);
            }
        }
    }
    out
}

/// Process-wide lazily-initialised client.
static GLOBAL: OnceLock<Option<FlyLogsClient>> = OnceLock::new();

pub fn global() -> Option<&'static FlyLogsClient> {
    GLOBAL.get_or_init(FlyLogsClient::from_env).as_ref()
}

#[derive(Debug, thiserror::Error)]
pub enum FlyLogsError {
    #[error("Fly logs request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Fly logs returned non-success status {status}: {body}")]
    Status { status: u16, body: String },
}

#[derive(Debug, Deserialize)]
struct JsonApiWrapper {
    data: Vec<JsonApiResource>,
}

#[derive(Debug, Deserialize)]
struct JsonApiResource {
    #[serde(default)]
    attributes: Option<RawLogLine>,
}

impl JsonApiWrapper {
    fn into_entries(self) -> Vec<LogEntry> {
        self.data
            .into_iter()
            .filter_map(|r| r.attributes.and_then(|a| a.into_entry()))
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct RawLogLine {
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    level: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default, alias = "instance_id")]
    instance: Option<String>,
    #[serde(default)]
    region: Option<String>,
}

impl RawLogLine {
    fn into_entry(self) -> Option<LogEntry> {
        let message = self.message?;
        let timestamp = self
            .timestamp
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);
        Some(LogEntry {
            timestamp,
            level: self.level.unwrap_or_else(|| "info".to_string()),
            message,
            instance: self.instance,
            region: self.region,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_api_wrapped_form() {
        let raw = r#"{
            "data": [
                {
                    "attributes": {
                        "timestamp": "2026-04-24T15:00:00Z",
                        "level": "info",
                        "message": "GET /users 200",
                        "instance_id": "abc123",
                        "region": "iad"
                    }
                }
            ]
        }"#;
        let entries = parse_log_payload(raw);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message, "GET /users 200");
        assert_eq!(entries[0].instance.as_deref(), Some("abc123"));
        assert_eq!(entries[0].region.as_deref(), Some("iad"));
    }

    #[test]
    fn parses_bare_array_form() {
        let raw = r#"[
            { "timestamp": "2026-04-24T15:00:00Z", "level": "info", "message": "hi" },
            { "timestamp": "2026-04-24T15:00:01Z", "level": "warn", "message": "ho" }
        ]"#;
        let entries = parse_log_payload(raw);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].level, "warn");
    }

    #[test]
    fn parses_ndjson_form() {
        let raw = r#"
            {"timestamp":"2026-04-24T15:00:00Z","level":"info","message":"line1"}
            {"timestamp":"2026-04-24T15:00:01Z","level":"error","message":"line2"}
        "#;
        let entries = parse_log_payload(raw);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].message, "line1");
    }

    #[test]
    fn skips_lines_without_messages() {
        let raw = r#"[ { "timestamp": "2026-04-24T15:00:00Z" } ]"#;
        let entries = parse_log_payload(raw);
        assert!(entries.is_empty());
    }

    #[test]
    fn from_env_returns_none_without_token() {
        std::env::remove_var("FLYIO_API_TOKEN");
        assert!(FlyLogsClient::from_env().is_none());
    }
}
