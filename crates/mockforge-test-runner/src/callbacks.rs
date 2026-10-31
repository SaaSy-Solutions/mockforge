//! HTTP callbacks from the runner back into the registry.
//!
//! All registry-side state transitions (run started / event appended /
//! run finished / runner-seconds reported) go through these endpoints
//! so the registry stays the single source of truth. Routes are
//! `/api/v1/internal/*` and authenticate via a bearer token (mTLS will
//! land later — see RunnerConfig::registry_internal_token).

use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(test)]
use std::sync::{Arc, Mutex};

use crate::error::{Error, Result};
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
    /// First wait between `/finish` attempts; doubles each retry.
    finish_retry_base: Duration,
    #[cfg(test)]
    test_state: Option<Arc<TestCallbackState>>,
}

#[cfg(test)]
#[derive(Default)]
struct TestCallbackState {
    capture_exchanges: Vec<CaptureExchange>,
    fail_capture_fetch: bool,
    fail_endpoint_hits_fetch: bool,
    fail_chaos_toggle: bool,
    contract_score_requests: Mutex<Vec<serde_json::Value>>,
}

/// Attempts at the `/finish` callback before giving up. With a 1s base the
/// waits are 1, 2, 4, 8 and 16 seconds.
const FINISH_ATTEMPTS: u32 = 6;

impl RegistryCallbacks {
    /// Construct from runner config.
    pub fn new(base_url: String, token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
            token,
            finish_retry_base: Duration::from_secs(1),
            #[cfg(test)]
            test_state: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn for_test() -> Self {
        Self::for_test_state(TestCallbackState::default())
    }

    #[cfg(test)]
    pub(crate) fn for_test_failing_capture_fetch() -> Self {
        Self::for_test_state(TestCallbackState {
            fail_capture_fetch: true,
            ..TestCallbackState::default()
        })
    }

    #[cfg(test)]
    pub(crate) fn for_test_failing_chaos_toggle() -> Self {
        Self::for_test_state(TestCallbackState {
            fail_chaos_toggle: true,
            ..TestCallbackState::default()
        })
    }

    #[cfg(test)]
    pub(crate) fn for_test_failing_endpoint_hits_fetch() -> Self {
        Self::for_test_state(TestCallbackState {
            fail_endpoint_hits_fetch: true,
            ..TestCallbackState::default()
        })
    }

    #[cfg(test)]
    fn for_test_state(state: TestCallbackState) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: String::new(),
            token: String::new(),
            finish_retry_base: Duration::from_secs(1),
            test_state: Some(Arc::new(state)),
        }
    }

    #[cfg(test)]
    pub(crate) fn contract_score_requests(&self) -> Vec<serde_json::Value> {
        self.test_state
            .as_ref()
            .unwrap()
            .contract_score_requests
            .lock()
            .unwrap()
            .clone()
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
    ///
    /// Retried on 429, 5xx and connection failures: a dropped finish leaves
    /// the run in `running` forever (#1286). The registry side is idempotent,
    /// so a retry after a lost response is harmless.
    pub async fn run_finished(&self, run_id: Uuid, outcome: &JobOutcome) -> Result<()> {
        let path = format!("/api/v1/internal/test-runs/{run_id}/finish");
        let body = FinishBody {
            status: outcome.status.as_str(),
            runner_seconds: outcome.billable_runner_seconds(),
            summary: &outcome.summary,
        };
        let mut delay = self.finish_retry_base;
        for attempt in 1.. {
            match self.post(&path, &body).await {
                Err(Error::Callback(e)) if attempt < FINISH_ATTEMPTS && is_transient(&e) => {
                    tracing::warn!(
                        run_id = %run_id,
                        attempt,
                        error = %e,
                        "run_finished callback failed, retrying"
                    );
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                }
                result => return result,
            }
        }
        unreachable!("the loop returns by the last attempt")
    }

    #[cfg(test)]
    fn with_finish_retry_base(mut self, base: Duration) -> Self {
        self.finish_retry_base = base;
        self
    }

    async fn post<B: Serialize>(&self, path: &str, body: &B) -> Result<()> {
        #[cfg(test)]
        if self.test_state.is_some() {
            return Ok(());
        }
        let url = format!("{}{path}", self.base_url.trim_end_matches('/'));
        let resp = self.http.post(&url).bearer_auth(&self.token).json(body).send().await?;
        resp.error_for_status()?;
        Ok(())
    }

    /// Pull the captured exchanges for a session so the replay
    /// executor can replay them. Empty vec means the session has no
    /// members; callers decide whether an empty capture set is valid.
    pub async fn fetch_capture_exchanges(&self, session_id: Uuid) -> Result<Vec<CaptureExchange>> {
        #[cfg(test)]
        if let Some(state) = &self.test_state {
            if state.fail_capture_fetch {
                return Err(Error::Executor("test capture fetch failure".to_string()));
            }
            return Ok(state.capture_exchanges.clone());
        }
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
        #[cfg(test)]
        if let Some(state) = &self.test_state {
            if state.fail_endpoint_hits_fetch {
                return Err(Error::Executor("test endpoint-hits fetch failure".to_string()));
            }
            return Ok(Vec::new());
        }
        let url = format!(
            "{}/api/v1/internal/workspaces/{workspace_id}/endpoint-hits",
            self.base_url.trim_end_matches('/'),
        );
        let resp = self.http.get(&url).bearer_auth(&self.token).send().await?;
        let resp = resp.error_for_status()?;
        let rows: Vec<EndpointHit> = resp.json().await?;
        Ok(rows)
    }

    /// Run the AI-assisted contract drift second pass (#348). The
    /// registry samples recent captures for each (method, path), feeds
    /// them to an LLM with the spec excerpt, and returns scored
    /// `breaking | non_breaking | cosmetic` findings the runner emits
    /// as additional `diff_finding` events. Returns an empty findings
    /// list (with `no_traffic=true`) when no exchanges were captured —
    /// the structural pass already handled the "spec without traffic"
    /// case so this is just a quiet skip.
    pub async fn score_contract_drift(
        &self,
        body: &ContractDriftScoreRequest,
    ) -> Result<ContractDriftScoreResponse> {
        #[cfg(test)]
        if let Some(state) = &self.test_state {
            state
                .contract_score_requests
                .lock()
                .unwrap()
                .push(serde_json::to_value(body).unwrap());
            return Ok(ContractDriftScoreResponse {
                findings: Vec::new(),
                tokens_used: 17,
                provider: "test".to_string(),
                no_traffic: false,
            });
        }
        let request = self.contract_drift_score_request(body)?;
        let resp = self.http.execute(request).await?;
        let resp = resp.error_for_status()?;
        let payload: ContractDriftScoreResponse = resp.json().await?;
        Ok(payload)
    }

    pub(crate) fn contract_drift_score_request(
        &self,
        body: &ContractDriftScoreRequest,
    ) -> Result<reqwest::Request> {
        let url =
            format!("{}/api/v1/internal/contract-diff/score", self.base_url.trim_end_matches('/'),);
        Ok(self.http.post(&url).bearer_auth(&self.token).json(body).build()?)
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
    /// the container's `/__mockforge/chaos/toggle`. `run_id` lets the
    /// registry refuse a deployment outside the run's org.
    pub async fn toggle_hosted_chaos(
        &self,
        run_id: Uuid,
        deployment_id: Uuid,
        enabled: bool,
    ) -> Result<()> {
        #[cfg(test)]
        if let Some(state) = &self.test_state {
            if state.fail_chaos_toggle {
                return Err(Error::Executor("test chaos toggle failure".to_string()));
            }
            return Ok(());
        }
        let url = format!(
            "{}/api/v1/internal/hosted-mocks/{deployment_id}/chaos",
            self.base_url.trim_end_matches('/'),
        );
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.token)
            .json(&serde_json::json!({ "enabled": enabled, "run_id": run_id }))
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

/// Endpoint identifier the AI scoring callback samples captures for.
/// Format matches what `runtime_captures` stores: method case-
/// insensitive, path matched literally.
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct ContractDriftEndpoint {
    pub method: String,
    pub path: String,
}

/// Body for `POST /api/v1/internal/contract-diff/score`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct ContractDriftScoreRequest {
    /// Persisted run identity; the registry resolves the originating actor.
    pub run_id: Uuid,
    pub org_id: Uuid,
    pub workspace_id: Uuid,
    pub spec_excerpt: String,
    pub endpoints: Vec<ContractDriftEndpoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_samples_per_endpoint: Option<i64>,
}

/// One AI-scored finding mirrored from
/// `crates/mockforge-registry-server/src/ai/contract_diff.rs::AiFinding`.
/// Kept in lockstep so the runner can emit the same shape directly.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AiContractFinding {
    pub severity: String,
    pub endpoint: String,
    pub description: String,
    pub confidence: f64,
    #[serde(default)]
    pub rationale: String,
}

/// Response from the AI scoring endpoint.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
pub struct ContractDriftScoreResponse {
    pub findings: Vec<AiContractFinding>,
    pub tokens_used: u64,
    pub provider: String,
    pub no_traffic: bool,
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

/// Rate limiting, a registry 5xx or a failed connection can clear on retry;
/// any other 4xx will not.
fn is_transient(e: &reqwest::Error) -> bool {
    match e.status() {
        Some(status) => {
            status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
        }
        None => e.is_connect() || e.is_timeout() || e.is_request(),
    }
}

#[cfg(test)]
mod finish_retry_tests {
    use super::*;
    use crate::executors::{JobOutcome, JobStatus};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Serve one canned status per connection and count the requests.
    async fn serve(statuses: Vec<u16>) -> (String, tokio::task::JoinHandle<usize>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            for (served, status) in statuses.iter().enumerate() {
                let Ok(Ok((mut socket, _))) =
                    tokio::time::timeout(Duration::from_secs(2), listener.accept()).await
                else {
                    return served;
                };
                let mut request = Vec::new();
                loop {
                    let mut chunk = [0; 4096];
                    let n = socket.read(&mut chunk).await.unwrap();
                    request.extend_from_slice(&chunk[..n]);
                    let text = String::from_utf8_lossy(&request);
                    if let Some(end) = text.find("\r\n\r\n") {
                        let length = text[..end]
                            .lines()
                            .find_map(|l| {
                                let (k, v) = l.split_once(':')?;
                                k.eq_ignore_ascii_case("content-length")
                                    .then(|| v.trim().parse::<usize>().unwrap())
                            })
                            .unwrap_or(0);
                        if request.len() >= end + 4 + length {
                            assert!(text.starts_with("POST /api/v1/internal/test-runs/"));
                            break;
                        }
                    }
                    if n == 0 {
                        break;
                    }
                }
                let reply = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}"
                );
                socket.write_all(reply.as_bytes()).await.unwrap();
            }
            statuses.len()
        });
        (url, server)
    }

    fn outcome() -> JobOutcome {
        JobOutcome {
            status: JobStatus::Passed,
            runner_seconds: 1,
            summary: None,
        }
    }

    #[tokio::test]
    async fn finish_retries_rate_limits_and_server_errors() {
        let (url, server) = serve(vec![429, 503, 200]).await;
        let callbacks = RegistryCallbacks::new(url, "t".into())
            .with_finish_retry_base(Duration::from_millis(5));
        callbacks.run_finished(Uuid::new_v4(), &outcome()).await.unwrap();
        assert_eq!(server.await.unwrap(), 3);
    }

    #[tokio::test]
    async fn finish_does_not_retry_a_client_error() {
        let (url, server) = serve(vec![404, 200]).await;
        let callbacks = RegistryCallbacks::new(url, "t".into())
            .with_finish_retry_base(Duration::from_millis(5));
        assert!(callbacks.run_finished(Uuid::new_v4(), &outcome()).await.is_err());
        assert_eq!(server.await.unwrap(), 1);
    }

    #[tokio::test]
    async fn finish_gives_up_after_the_last_attempt() {
        let (url, server) = serve(vec![429; FINISH_ATTEMPTS as usize + 1]).await;
        let callbacks = RegistryCallbacks::new(url, "t".into())
            .with_finish_retry_base(Duration::from_millis(1));
        assert!(callbacks.run_finished(Uuid::new_v4(), &outcome()).await.is_err());
        assert_eq!(server.await.unwrap(), FINISH_ATTEMPTS as usize);
    }
}
