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

    /// Fetch a fitness function definition (id, name, kind, config)
    /// for the FitnessExecutor (#355). Called once per run before
    /// dispatching to the kind-specific evaluator.
    pub async fn fetch_fitness_function(
        &self,
        function_id: Uuid,
    ) -> Result<FitnessFunctionDefinition> {
        let url = format!(
            "{}/api/v1/internal/fitness-functions/{function_id}",
            self.base_url.trim_end_matches('/'),
        );
        let resp = self.http.get(&url).bearer_auth(&self.token).send().await?;
        let resp = resp.error_for_status()?;
        let row: FitnessFunctionDefinition = resp.json().await?;
        Ok(row)
    }

    /// Pull latency-aggregate stats for a deployment over a window.
    /// Used by `kind='latency_threshold'` fitness checks. The optional
    /// `path` filter narrows to a single endpoint when the function's
    /// config specifies one.
    pub async fn fetch_deployment_latency_stats(
        &self,
        deployment_id: Uuid,
        window_minutes: i64,
        path: Option<&str>,
    ) -> Result<DeploymentLatencyStats> {
        let url = format!(
            "{}/api/v1/internal/deployments/{deployment_id}/latency-stats",
            self.base_url.trim_end_matches('/'),
        );
        // `reqwest::RequestBuilder::query` urlencodes both keys and
        // values — handles paths with `/`, spaces, etc. without us
        // pulling in an extra urlencoding dep.
        let mut req = self
            .http
            .get(&url)
            .bearer_auth(&self.token)
            .query(&[("window_minutes", window_minutes.to_string())]);
        if let Some(p) = path {
            req = req.query(&[("path", p)]);
        }
        let resp = req.send().await?;
        let resp = resp.error_for_status()?;
        let stats: DeploymentLatencyStats = resp.json().await?;
        Ok(stats)
    }

    /// Pull contract-stability aggregates for a monitored service over
    /// a window. Used by `kind='contract_stability'` fitness checks.
    /// Returns severity counts (breaking / non_breaking / cosmetic)
    /// plus run count + latest run timestamp; the executor asserts
    /// thresholds against the breaking count.
    pub async fn fetch_monitored_service_contract_stability(
        &self,
        monitored_service_id: Uuid,
        window_minutes: i64,
    ) -> Result<MonitoredServiceContractStability> {
        let url = format!(
            "{}/api/v1/internal/monitored-services/{monitored_service_id}/contract-stability",
            self.base_url.trim_end_matches('/'),
        );
        let resp = self
            .http
            .get(&url)
            .bearer_auth(&self.token)
            .query(&[("window_minutes", window_minutes.to_string())])
            .send()
            .await?;
        let resp = resp.error_for_status()?;
        let stats: MonitoredServiceContractStability = resp.json().await?;
        Ok(stats)
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

/// Subset of `FitnessFunction` the runner needs to dispatch + evaluate.
/// We don't pull the full model row (last_evaluated_at, last_status,
/// timestamps) since the runner only consumes id, name, kind, config.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
pub struct FitnessFunctionDefinition {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub kind: String,
    pub config: serde_json::Value,
}

/// Aggregate latency stats over a window for one deployment. Mirrors
/// the registry handler's `DeploymentLatencyStats` shape exactly.
/// Percentile / max / avg fields are `None` when `count == 0`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
pub struct DeploymentLatencyStats {
    pub count: i64,
    pub error_count: i64,
    pub p50_ms: Option<f64>,
    pub p95_ms: Option<f64>,
    pub p99_ms: Option<f64>,
    pub max_ms: Option<f64>,
    pub avg_ms: Option<f64>,
}

/// Aggregate of contract-diff findings for a monitored service over
/// a window. Mirrors the registry handler's
/// `MonitoredServiceContractStability` shape; used by
/// `kind='contract_stability'` fitness checks.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
pub struct MonitoredServiceContractStability {
    pub breaking_count: i64,
    pub non_breaking_count: i64,
    pub cosmetic_count: i64,
    pub run_count: i64,
    /// Most recent diff-run timestamp inside the window. `None` when
    /// no runs happened — distinguishes "no data" (e.g. nothing
    /// scheduled) from "lots of data but no findings".
    pub latest_run_at: Option<String>,
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
