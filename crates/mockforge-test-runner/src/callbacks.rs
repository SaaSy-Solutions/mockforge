//! HTTP callbacks from the runner back into the registry.
//!
//! All registry-side state transitions (run started / event appended /
//! run finished / runner-seconds reported) go through these endpoints
//! so the registry stays the single source of truth. Routes are
//! `/api/v1/internal/*` and authenticate via a bearer token (mTLS will
//! land later — see RunnerConfig::registry_internal_token).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;
use crate::executors::JobOutcome;

/// One captured exchange returned by the registry's internal
/// capture-exchanges endpoint. Mirrors the Postgres schema's
/// runtime_captures columns the replay executor cares about.
#[allow(missing_docs)] // wire-format struct mirroring runtime_captures columns
#[derive(Debug, Clone, Deserialize)]
pub struct CaptureExchange {
    pub capture_id: String,
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub query_params: Option<String>,
    pub request_headers: String,
    #[serde(default)]
    pub request_body: Option<String>,
    pub request_body_encoding: String,
    #[serde(default)]
    pub response_status_code: Option<i32>,
    #[serde(default)]
    pub response_headers: Option<String>,
    #[serde(default)]
    pub response_body: Option<String>,
    #[serde(default)]
    pub response_body_encoding: Option<String>,
}

/// Thin client over reqwest::Client that knows how to talk to the
/// registry's internal callback routes.
pub struct RegistryCallbacks {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

impl RegistryCallbacks {
    /// Construct from runner config.
    pub fn new(base_url: String, token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
            token,
        }
    }

    /// Mark a run as `running` (registry-side `mark_running`).
    pub async fn run_started(&self, run_id: Uuid) -> Result<()> {
        self.post(&format!("/api/v1/internal/test-runs/{run_id}/start"), &EmptyBody {})
            .await
    }

    /// Append an event to a run's stream. The registry persists it to
    /// `test_run_events` and pubsubs it for SSE listeners.
    pub async fn run_event(
        &self,
        run_id: Uuid,
        seq: u32,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<()> {
        self.post(
            &format!("/api/v1/internal/test-runs/{run_id}/events"),
            &EventBody {
                seq,
                event_type,
                payload,
            },
        )
        .await
    }

    /// Mark a run as terminal (passed/failed/cancelled/errored) and
    /// report runner_seconds for billing.
    pub async fn run_finished(&self, run_id: Uuid, outcome: &JobOutcome) -> Result<()> {
        self.post(
            &format!("/api/v1/internal/test-runs/{run_id}/finish"),
            &FinishBody {
                status: outcome.status.as_str(),
                runner_seconds: outcome.runner_seconds,
                summary: &outcome.summary,
            },
        )
        .await
    }

    async fn post<B: Serialize>(&self, path: &str, body: &B) -> Result<()> {
        let url = format!("{}{path}", self.base_url.trim_end_matches('/'));
        let resp = self.http.post(&url).bearer_auth(&self.token).json(body).send().await?;
        resp.error_for_status()?;
        Ok(())
    }

    /// Pull the captured exchanges for a session so the replay
    /// executor can replay them. Empty vec means the session has no
    /// members; the executor falls back to synthetic mode in that case.
    pub async fn fetch_capture_exchanges(&self, session_id: Uuid) -> Result<Vec<CaptureExchange>> {
        let url = format!(
            "{}/api/v1/internal/capture-sessions/{session_id}/exchanges",
            self.base_url.trim_end_matches('/'),
        );
        let resp = self.http.get(&url).bearer_auth(&self.token).send().await?;
        let resp = resp.error_for_status()?;
        let rows: Vec<CaptureExchange> = resp.json().await?;
        Ok(rows)
    }

    /// Pull endpoint-hit counts for a workspace from recent traffic
    /// (last 24h). Used by the contract diff executor to compare
    /// declared spec endpoints against what's actually being hit.
    pub async fn fetch_workspace_endpoint_hits(
        &self,
        workspace_id: Uuid,
    ) -> Result<Vec<EndpointHit>> {
        let url = format!(
            "{}/api/v1/internal/workspaces/{workspace_id}/endpoint-hits",
            self.base_url.trim_end_matches('/'),
        );
        let resp = self.http.get(&url).bearer_auth(&self.token).send().await?;
        let resp = resp.error_for_status()?;
        let rows: Vec<EndpointHit> = resp.json().await?;
        Ok(rows)
    }

    /// Pull aggregate latency / error stats for a workspace. Used by
    /// the FitnessExecutor (#355) to evaluate latency_threshold and
    /// error_rate fitness functions. `path_prefix` is optional ("" =
    /// no filter); `window_minutes` clamped to 1..=10080 server-side.
    pub async fn fetch_workspace_runtime_stats(
        &self,
        workspace_id: Uuid,
        window_minutes: i64,
        path_prefix: Option<&str>,
    ) -> Result<WorkspaceRuntimeStats> {
        let mut url = format!(
            "{}/api/v1/internal/workspaces/{workspace_id}/runtime-stats?window_minutes={window_minutes}",
            self.base_url.trim_end_matches('/'),
        );
        if let Some(p) = path_prefix {
            if !p.is_empty() {
                url.push_str("&path_prefix=");
                url.push_str(&urlencoding::encode(p));
            }
        }
        let resp = self.http.get(&url).bearer_auth(&self.token).send().await?;
        let resp = resp.error_for_status()?;
        let stats: WorkspaceRuntimeStats = resp.json().await?;
        Ok(stats)
    }

    /// Raise an incident from the runner side. Used by the
    /// FitnessExecutor (#355) when a fitness function evaluates as
    /// failed — same dedupe semantics as raise_incident_external on
    /// the public surface, but auth'd via the internal token.
    pub async fn raise_incident(&self, body: RaiseIncidentBody<'_>) -> Result<()> {
        let url = format!(
            "{}/api/v1/internal/incidents/raise-from-runner",
            self.base_url.trim_end_matches('/'),
        );
        let resp = self.http.post(&url).bearer_auth(&self.token).json(&body).send().await?;
        resp.error_for_status()?;
        Ok(())
    }

    /// Toggle chaos on a hosted-mock deployment via the registry's
    /// internal proxy. Used by the chaos executor for
    /// target_kind=hosted_mock — the registry resolves the
    /// deployment's internal Fly URL and forwards the request to
    /// the container's `/__mockforge/chaos/toggle`.
    pub async fn toggle_hosted_chaos(&self, deployment_id: Uuid, enabled: bool) -> Result<()> {
        let url = format!(
            "{}/api/v1/internal/hosted-mocks/{deployment_id}/chaos",
            self.base_url.trim_end_matches('/'),
        );
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.token)
            .json(&serde_json::json!({ "enabled": enabled }))
            .send()
            .await?;
        resp.error_for_status()?;
        Ok(())
    }
}

/// (method, path, hits) tuple from the workspace endpoint-hits endpoint.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
pub struct EndpointHit {
    pub method: String,
    pub path: String,
    pub hits: i64,
}

/// Aggregate latency / error stats for a workspace returned by
/// `/api/v1/internal/workspaces/{id}/runtime-stats`. Used by the
/// FitnessExecutor (#355).
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceRuntimeStats {
    pub window_minutes: i64,
    pub path_prefix: String,
    pub total_requests: i64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub server_errors: i64,
    pub client_errors: i64,
}

/// Body for `RegistryCallbacks::raise_incident`. Mirrors the registry's
/// internal `RaiseIncidentFromRunnerBody`.
#[allow(missing_docs)] // wire-format struct; field semantics live on the registry side
#[derive(Debug, Serialize)]
pub struct RaiseIncidentBody<'a> {
    pub workspace_id: Uuid,
    pub source: &'a str,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<&'a str>,
    pub dedupe_key: &'a str,
    pub severity: &'a str,
    pub title: &'a str,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<&'a str>,
}

#[derive(Serialize)]
struct EmptyBody {}

#[derive(Serialize)]
struct EventBody<'a> {
    seq: u32,
    event_type: &'a str,
    payload: serde_json::Value,
}

#[derive(Serialize)]
struct FinishBody<'a> {
    status: &'a str,
    runner_seconds: i32,
    summary: &'a Option<serde_json::Value>,
}
