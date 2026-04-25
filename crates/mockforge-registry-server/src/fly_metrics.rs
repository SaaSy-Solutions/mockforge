//! Fly Managed Prometheus client for hosted-mock runtime metrics.
//!
//! Queries Fly.io's managed Prometheus endpoint for each deployment's metrics
//! instead of reading from the local `deployment_metrics` aggregate table. This
//! closes the gap called out in #221: the table had no writer, so the metrics
//! tab in the admin UI was showing placeholder zeros.
//!
//! Configuration via environment variables (all optional — if unset, callers
//! fall back to the local Postgres counters):
//!
//! - `FLY_PROMETHEUS_URL` — base URL of the Fly Prometheus endpoint.
//!   Typically `https://api.fly.io/prometheus/<org-slug>`.
//! - `FLY_PROMETHEUS_TOKEN` — bearer token with read access.
//! - `FLY_PROMETHEUS_APP_LABEL` — Prometheus label name used to identify the
//!   Fly app. Defaults to `app`.
//! - `FLY_PROMETHEUS_TIMEOUT_MS` — per-query timeout. Defaults to 3000.
//! - `FLY_PROMETHEUS_WINDOW_DAYS` — time window for counter rollups. Defaults
//!   to 30 (calendar month parity with the existing JSON response shape).

use reqwest::Client;
use serde::Deserialize;
use std::sync::OnceLock;
use std::time::Duration;
use tracing::debug;

/// Summary of runtime metrics for a single hosted mock, shaped to match the
/// existing `MetricsResponse` contract on the HTTP API.
#[derive(Debug, Clone)]
pub struct DeploymentMetricsSnapshot {
    pub requests: i64,
    pub requests_2xx: i64,
    pub requests_4xx: i64,
    pub requests_5xx: i64,
    pub egress_bytes: i64,
    pub avg_response_time_ms: i64,
}

/// Fly Prometheus query client.
#[derive(Clone)]
pub struct FlyMetricsClient {
    base_url: String,
    token: String,
    app_label: String,
    window_days: u32,
    http: Client,
}

impl FlyMetricsClient {
    /// Build a client from environment variables. Returns `None` when Fly
    /// Prometheus isn't configured — callers fall back to the local DB.
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var("FLY_PROMETHEUS_URL").ok()?;
        let token = std::env::var("FLY_PROMETHEUS_TOKEN").ok()?;
        let app_label =
            std::env::var("FLY_PROMETHEUS_APP_LABEL").unwrap_or_else(|_| "app".to_string());
        let timeout_ms = std::env::var("FLY_PROMETHEUS_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3000);
        let window_days = std::env::var("FLY_PROMETHEUS_WINDOW_DAYS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        let http = Client::builder().timeout(Duration::from_millis(timeout_ms)).build().ok()?;

        Some(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
            app_label,
            window_days,
            http,
        })
    }

    /// Fetch all counters for a deployment by its Fly app name. On query
    /// failure, returns `Err` and the caller should fall back to the local DB.
    pub async fn snapshot_for_app(
        &self,
        app_name: &str,
    ) -> Result<DeploymentMetricsSnapshot, FlyMetricsError> {
        // PromQL uses a window function (`increase`) over the configured window.
        // We escape the app name with `quote_label_value` to keep the query safe.
        let app_filter = format!(
            r#"{label}="{value}""#,
            label = self.app_label,
            value = quote_label_value(app_name)
        );

        let window = format!("{}d", self.window_days);

        let q_total = format!(
            r#"sum(increase(mockforge_requests_total{{{filter}}}[{window}]))"#,
            filter = app_filter,
            window = window,
        );
        let q_2xx = format!(
            r#"sum(increase(mockforge_requests_total{{{filter},status=~"2.."}}[{window}]))"#,
            filter = app_filter,
            window = window,
        );
        let q_4xx = format!(
            r#"sum(increase(mockforge_requests_total{{{filter},status=~"4.."}}[{window}]))"#,
            filter = app_filter,
            window = window,
        );
        let q_5xx = format!(
            r#"sum(increase(mockforge_requests_total{{{filter},status=~"5.."}}[{window}]))"#,
            filter = app_filter,
            window = window,
        );
        // Average response time in ms: sum(duration_sum) / sum(duration_count) * 1000.
        let q_avg_ms = format!(
            r#"1000 * (sum(increase(mockforge_request_duration_seconds_sum{{{filter}}}[{window}])) / sum(increase(mockforge_request_duration_seconds_count{{{filter}}}[{window}])))"#,
            filter = app_filter,
            window = window,
        );
        // Fly exposes network egress as `fly_instance_network_sent_bytes` on
        // the machine. We scope by app label and sum.
        let q_egress = format!(
            r#"sum(increase(fly_instance_network_sent_bytes{{{filter}}}[{window}]))"#,
            filter = app_filter,
            window = window,
        );

        let (requests, r2, r4, r5, avg_ms, egress) = tokio::try_join!(
            self.query_scalar(&q_total),
            self.query_scalar(&q_2xx),
            self.query_scalar(&q_4xx),
            self.query_scalar(&q_5xx),
            self.query_scalar(&q_avg_ms),
            self.query_scalar(&q_egress),
        )?;

        Ok(DeploymentMetricsSnapshot {
            requests: requests as i64,
            requests_2xx: r2 as i64,
            requests_4xx: r4 as i64,
            requests_5xx: r5 as i64,
            egress_bytes: egress as i64,
            avg_response_time_ms: if avg_ms.is_finite() { avg_ms as i64 } else { 0 },
        })
    }

    /// Execute a PromQL instant query and return the first scalar value. Empty
    /// result sets resolve to `0.0` rather than erroring because that's what
    /// "no traffic yet" looks like for a counter window.
    async fn query_scalar(&self, promql: &str) -> Result<f64, FlyMetricsError> {
        debug!(query = %promql, "Executing PromQL instant query");
        let url = format!("{}/api/v1/query", self.base_url);
        let resp = self
            .http
            .get(&url)
            .bearer_auth(&self.token)
            .query(&[("query", promql)])
            .send()
            .await
            .map_err(FlyMetricsError::Request)?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(FlyMetricsError::Status {
                status: status.as_u16(),
                body,
            });
        }

        let parsed: PromQueryResponse = resp.json().await.map_err(FlyMetricsError::Request)?;

        match parsed.data.result.first() {
            Some(entry) => {
                let raw = entry.value.1.as_str();
                raw.parse::<f64>().map_err(|_| FlyMetricsError::InvalidScalar(raw.to_string()))
            }
            None => Ok(0.0),
        }
    }
}

/// Errors surfaced by the Fly Prometheus client. The handler treats any error
/// as "fall back to the local DB" and logs a warning.
#[derive(Debug, thiserror::Error)]
pub enum FlyMetricsError {
    #[error("PromQL request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("PromQL returned non-success status {status}: {body}")]
    Status { status: u16, body: String },
    #[error("PromQL scalar could not be parsed as f64: {0}")]
    InvalidScalar(String),
}

/// Escape a Fly app name for use inside a PromQL `label="value"` clause.
/// App names follow the `mockforge-<orghash>-<slug>` pattern which is already
/// a subset of \[a-z0-9-\], but we keep this defensive in case the slug ever
/// admits other characters.
fn quote_label_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Process-wide lazily-initialised client. `None` means Fly Prometheus isn't
/// configured (local dev, self-hosted, or missing env vars) and callers should
/// fall back to the local Postgres counters. The env vars are read once per
/// process — if operators rotate credentials they must restart the server.
static GLOBAL: OnceLock<Option<FlyMetricsClient>> = OnceLock::new();

/// Resolve the process-wide client, initialising on first call.
pub fn global() -> Option<&'static FlyMetricsClient> {
    GLOBAL.get_or_init(FlyMetricsClient::from_env).as_ref()
}

#[derive(Debug, Deserialize)]
struct PromQueryResponse {
    data: PromQueryData,
}

#[derive(Debug, Deserialize)]
struct PromQueryData {
    result: Vec<PromQueryVector>,
}

/// Fly Managed Prometheus returns the standard Prometheus `instant` query
/// shape: `result[].value = [timestamp, "scalar as string"]`.
#[derive(Debug, Deserialize)]
struct PromQueryVector {
    value: (f64, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_value_escaping_is_defensive() {
        assert_eq!(quote_label_value("simple-name"), "simple-name");
        assert_eq!(quote_label_value("has\"quote"), "has\\\"quote");
        assert_eq!(quote_label_value("has\\back"), "has\\\\back");
    }

    #[test]
    fn from_env_returns_none_without_required_vars() {
        // SAFETY: tests run in a subprocess in parallel by default; to keep this
        // deterministic we unset the vars before checking.
        std::env::remove_var("FLY_PROMETHEUS_URL");
        std::env::remove_var("FLY_PROMETHEUS_TOKEN");
        assert!(FlyMetricsClient::from_env().is_none());
    }
}
