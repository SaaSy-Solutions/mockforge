//! Federation scenario runtime polling helpers.
//!
//! This module supplies the contract between the registry-side federation
//! scenario activation endpoints and the runtime workspace-side code that
//! observes and applies overrides. A runtime embedder spawns a
//! [`FederationScenarioPoller`], hands it a [`ScenarioApplicator`] that knows
//! how to push overrides into local actuators (chaos engine, latency
//! injector, reality-level resolver), and the poller does the rest on a
//! cadence.
//!
//! The DTOs here match the wire shape returned by
//! `GET /api/v1/workspaces/{id}/active-scenarios` exactly — both the
//! registry server and the runtime client deserialize the same types.

use crate::manifest::ServiceScenarioOverride;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

/// One override entry that applies to a workspace for a specific federation
/// service boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceActiveScenarioEntry {
    /// Activation row ID — used by the runtime when reporting back status.
    pub activation_id: Uuid,
    /// Federation that owns the activation.
    pub federation_id: Uuid,
    /// Display name of the federation, populated best-effort.
    pub federation_name: String,
    /// Display name of the scenario (from the source manifest).
    pub scenario_name: String,
    /// Federation service boundary name this workspace plays. A single
    /// workspace can back multiple service names.
    pub service_name: String,
    /// Resolved per-service override. `None` means "no override for this
    /// service — observe only the scenario manifest's global config."
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_config: Option<ServiceScenarioOverride>,
}

/// Response shape for the workspace poll endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceActiveScenariosResponse {
    /// Workspace being polled.
    pub workspace_id: Uuid,
    /// Every override currently applying to the workspace, across every
    /// federation it's a member of.
    #[serde(default)]
    pub entries: Vec<WorkspaceActiveScenarioEntry>,
}

/// Runtime-side hook. Implementations push scenario overrides into concrete
/// actuators (chaos, latency, reality level) and return when the apply has
/// completed (or failed). Called once per poll tick.
#[async_trait]
pub trait ScenarioApplicator: Send + Sync {
    /// Apply the current set of active scenario overrides for this workspace.
    ///
    /// The applicator is responsible for:
    /// - Computing the diff from the previous apply (usually by tracking
    ///   activation_id) and rolling back overrides that disappear.
    /// - Applying each entry's override to the appropriate actuator.
    /// - Reporting per-entry success/failure; the poller uses the return
    ///   value to call the registry's `report` endpoint.
    async fn apply(&self, entries: &[WorkspaceActiveScenarioEntry]) -> Vec<ApplyOutcome>;
}

/// Result of applying one entry.
#[derive(Debug, Clone)]
pub struct ApplyOutcome {
    /// Activation that produced the entry — used to correlate with the
    /// federation when reporting status back.
    pub activation_id: Uuid,
    /// Federation service boundary name the entry targeted.
    pub service_name: String,
    /// Final status of the apply.
    pub status: ApplyStatus,
    /// Human-readable error when `status == Failed`.
    pub error: Option<String>,
}

/// Per-entry apply status; mirrors the `status` string the runtime reports
/// back to the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyStatus {
    /// Override was pushed to the local actuator successfully.
    Applied,
    /// Override could not be applied — see [`ApplyOutcome::error`].
    Failed,
}

impl ApplyStatus {
    /// Wire-format string used by the registry report endpoint.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::Failed => "failed",
        }
    }
}

/// HTTP transport abstraction. The default implementation uses `reqwest`
/// under the `reqwest-client` feature; tests and embedders with a pre-built
/// HTTP client can provide their own.
#[async_trait]
pub trait PollTransport: Send + Sync {
    /// `GET /api/v1/workspaces/{workspace_id}/active-scenarios`.
    async fn fetch(
        &self,
        workspace_id: Uuid,
    ) -> Result<WorkspaceActiveScenariosResponse, PollError>;

    /// `POST /api/v1/federation/{federation_id}/scenarios/active/report`.
    async fn report(
        &self,
        federation_id: Uuid,
        service_name: &str,
        status: ApplyStatus,
        error: Option<&str>,
    ) -> Result<(), PollError>;
}

/// Transport or protocol error raised by the poller.
#[derive(Debug, thiserror::Error)]
pub enum PollError {
    /// HTTP status code was not 2xx.
    #[error("HTTP status {status}: {body}")]
    Http {
        /// Observed HTTP status code.
        status: u16,
        /// Response body (truncated if very large).
        body: String,
    },
    /// Transport-level failure (connection refused, DNS, timeout).
    #[error("transport error: {0}")]
    Transport(String),
    /// Response body was not valid JSON or didn't match the DTO schema.
    #[error("deserialize error: {0}")]
    Deserialize(String),
}

/// Runtime-side poller. Hit one `tick()` at a time (for tests) or spawn the
/// background loop with [`FederationScenarioPoller::run`].
pub struct FederationScenarioPoller<T: PollTransport, A: ScenarioApplicator> {
    workspace_id: Uuid,
    transport: T,
    applicator: A,
    interval: Duration,
}

impl<T: PollTransport, A: ScenarioApplicator> FederationScenarioPoller<T, A> {
    /// Build a poller. `interval` is the poll cadence used by `run()`.
    #[must_use]
    pub fn new(workspace_id: Uuid, transport: T, applicator: A, interval: Duration) -> Self {
        Self {
            workspace_id,
            transport,
            applicator,
            interval,
        }
    }

    /// Poll once, apply, and report. Returns the outcomes for inspection —
    /// tests use this to drive assertions without needing the background
    /// loop.
    pub async fn tick(&self) -> Result<Vec<ApplyOutcome>, PollError> {
        let response = self.transport.fetch(self.workspace_id).await?;
        let outcomes = self.applicator.apply(&response.entries).await;

        // Report each outcome back. The registry's `report` endpoint is
        // keyed by federation + service name, so we look up the federation
        // from the matching entry.
        for outcome in &outcomes {
            let federation_id = response
                .entries
                .iter()
                .find(|e| {
                    e.activation_id == outcome.activation_id
                        && e.service_name == outcome.service_name
                })
                .map(|e| e.federation_id);
            if let Some(federation_id) = federation_id {
                // Best-effort report — a failed report shouldn't crash the
                // poller. Log and continue.
                if let Err(err) = self
                    .transport
                    .report(
                        federation_id,
                        &outcome.service_name,
                        outcome.status,
                        outcome.error.as_deref(),
                    )
                    .await
                {
                    tracing::warn!(
                        error = %err,
                        activation_id = %outcome.activation_id,
                        service = %outcome.service_name,
                        "Failed to report scenario apply outcome"
                    );
                }
            }
        }

        Ok(outcomes)
    }

    /// Run the poll loop until cancelled. Returns when `cancel` resolves.
    ///
    /// Each iteration runs one `tick` and swallows poll errors after
    /// logging — a temporary registry outage shouldn't kill the runtime.
    pub async fn run<F: std::future::Future<Output = ()>>(self, cancel: F) {
        let mut cancel = Box::pin(cancel);
        let mut ticker = tokio::time::interval(self.interval);
        loop {
            tokio::select! {
                _ = &mut cancel => {
                    tracing::info!(workspace_id = %self.workspace_id, "Federation scenario poller stopping");
                    return;
                }
                _ = ticker.tick() => {
                    if let Err(err) = self.tick().await {
                        tracing::warn!(error = %err, "Federation scenario poll failed");
                    }
                }
            }
        }
    }
}

/// HTTP transport backed by `reqwest`. The shared client reuses connections
/// across polls; rebuild it per poller instance if you need per-instance
/// TLS or header customization beyond the bearer token.
pub struct ReqwestPollTransport {
    client: reqwest::Client,
    /// Base URL of the registry API, e.g. `https://app.mockforge.dev`.
    /// Endpoints are appended as `{base}/api/v1/...`.
    base_url: String,
    /// Bearer token for the `Authorization` header. Blank disables auth —
    /// only appropriate for local registries without middleware.
    auth_token: String,
}

impl ReqwestPollTransport {
    /// Build a transport pointing at `base_url` and authenticating with the
    /// given token. Strips a single trailing slash for concatenation safety.
    pub fn new(base_url: impl Into<String>, auth_token: impl Into<String>) -> Self {
        let mut base = base_url.into();
        if base.ends_with('/') {
            base.pop();
        }
        Self {
            client: reqwest::Client::new(),
            base_url: base,
            auth_token: auth_token.into(),
        }
    }

    fn authed(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if self.auth_token.is_empty() {
            builder
        } else {
            builder.bearer_auth(&self.auth_token)
        }
    }
}

#[async_trait]
impl PollTransport for ReqwestPollTransport {
    async fn fetch(
        &self,
        workspace_id: Uuid,
    ) -> Result<WorkspaceActiveScenariosResponse, PollError> {
        let url = format!("{}/api/v1/workspaces/{workspace_id}/active-scenarios", self.base_url);
        let response = self
            .authed(self.client.get(&url))
            .send()
            .await
            .map_err(|e| PollError::Transport(e.to_string()))?;

        let status = response.status();
        let body = response.text().await.map_err(|e| PollError::Transport(e.to_string()))?;

        if !status.is_success() {
            return Err(PollError::Http {
                status: status.as_u16(),
                body,
            });
        }
        serde_json::from_str(&body).map_err(|e| PollError::Deserialize(e.to_string()))
    }

    async fn report(
        &self,
        federation_id: Uuid,
        service_name: &str,
        status: ApplyStatus,
        error: Option<&str>,
    ) -> Result<(), PollError> {
        let url =
            format!("{}/api/v1/federation/{federation_id}/scenarios/active/report", self.base_url);
        let mut body = serde_json::json!({
            "service_name": service_name,
            "status": status.as_str(),
        });
        if let Some(err) = error {
            body["error"] = serde_json::Value::String(err.to_string());
        }

        let response = self
            .authed(self.client.post(&url).json(&body))
            .send()
            .await
            .map_err(|e| PollError::Transport(e.to_string()))?;

        let status_code = response.status();
        if !status_code.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(PollError::Http {
                status: status_code.as_u16(),
                body,
            });
        }
        Ok(())
    }
}

/// Default applicator that logs every entry it receives without touching any
/// actuator. Good starting point for embedders who want to observe scenario
/// traffic before writing real apply logic.
///
/// Structured log fields: `activation_id`, `federation_id`, `federation_name`,
/// `scenario_name`, `service_name`, plus each override field when present.
#[derive(Debug, Default, Clone)]
pub struct LoggingApplicator;

#[async_trait]
impl ScenarioApplicator for LoggingApplicator {
    async fn apply(&self, entries: &[WorkspaceActiveScenarioEntry]) -> Vec<ApplyOutcome> {
        entries
            .iter()
            .map(|entry| {
                let override_summary = entry
                    .override_config
                    .as_ref()
                    .map(|o| {
                        serde_json::to_string(o).unwrap_or_else(|_| "<unserializable>".to_string())
                    })
                    .unwrap_or_else(|| "(no override)".to_string());
                tracing::info!(
                    activation_id = %entry.activation_id,
                    federation_id = %entry.federation_id,
                    federation_name = %entry.federation_name,
                    scenario_name = %entry.scenario_name,
                    service_name = %entry.service_name,
                    override = %override_summary,
                    "LoggingApplicator observed federation scenario override"
                );
                ApplyOutcome {
                    activation_id: entry.activation_id,
                    service_name: entry.service_name.clone(),
                    status: ApplyStatus::Applied,
                    error: None,
                }
            })
            .collect()
    }
}

/// Spawn the poller reading its config from environment variables.
///
/// Reads:
/// - `MOCKFORGE_FEDERATION_POLL_URL` — registry base URL (required)
/// - `MOCKFORGE_FEDERATION_WORKSPACE_ID` — workspace UUID (required)
/// - `MOCKFORGE_FEDERATION_POLL_TOKEN` — bearer token (optional; blank = no auth)
/// - `MOCKFORGE_FEDERATION_POLL_INTERVAL_SECS` — poll cadence (default 30)
///
/// Returns `Ok(None)` when the required env vars are missing, so callers can
/// unconditionally invoke this during startup. Returns `Ok(Some(handle))`
/// with a tokio join handle when a poller is spawned; the caller owns the
/// handle and is free to abort it on shutdown.
///
/// # Errors
///
/// Returns an error only if env vars are malformed (invalid UUID, non-numeric
/// interval). A missing URL is not an error — it means "not configured".
pub fn spawn_from_env<A>(applicator: A) -> Result<Option<tokio::task::JoinHandle<()>>, String>
where
    A: ScenarioApplicator + 'static,
{
    let base_url = match std::env::var("MOCKFORGE_FEDERATION_POLL_URL") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return Ok(None),
    };
    let workspace_id = match std::env::var("MOCKFORGE_FEDERATION_WORKSPACE_ID") {
        Ok(v) if !v.trim().is_empty() => Uuid::parse_str(v.trim())
            .map_err(|e| format!("MOCKFORGE_FEDERATION_WORKSPACE_ID: {e}"))?,
        _ => return Ok(None),
    };
    let auth_token = std::env::var("MOCKFORGE_FEDERATION_POLL_TOKEN").unwrap_or_default();
    let interval_secs: u64 = std::env::var("MOCKFORGE_FEDERATION_POLL_INTERVAL_SECS")
        .ok()
        .map(|s| s.parse().map_err(|e| format!("MOCKFORGE_FEDERATION_POLL_INTERVAL_SECS: {e}")))
        .transpose()?
        .unwrap_or(30);

    let transport = ReqwestPollTransport::new(base_url, auth_token);
    let poller = FederationScenarioPoller::new(
        workspace_id,
        transport,
        applicator,
        Duration::from_secs(interval_secs),
    );

    // Cancellation channel — the caller can drop the handle to stop the
    // poller, but we also need a tokio-aware cancel future. Wire both.
    let handle = tokio::spawn(async move {
        let never: std::future::Pending<()> = std::future::pending();
        poller.run(never).await;
    });

    tracing::info!(%workspace_id, interval_secs, "Federation scenario poller spawned");
    Ok(Some(handle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// A recording transport that replays a canned response and logs reports.
    struct FakeTransport {
        canned: WorkspaceActiveScenariosResponse,
        reports: Arc<Mutex<Vec<(Uuid, String, ApplyStatus)>>>,
    }

    #[async_trait]
    impl PollTransport for FakeTransport {
        async fn fetch(&self, _: Uuid) -> Result<WorkspaceActiveScenariosResponse, PollError> {
            Ok(self.canned.clone())
        }
        async fn report(
            &self,
            federation_id: Uuid,
            service_name: &str,
            status: ApplyStatus,
            _error: Option<&str>,
        ) -> Result<(), PollError> {
            self.reports
                .lock()
                .unwrap()
                .push((federation_id, service_name.to_string(), status));
            Ok(())
        }
    }

    /// An applicator that marks every entry applied.
    struct AlwaysApplies;

    #[async_trait]
    impl ScenarioApplicator for AlwaysApplies {
        async fn apply(&self, entries: &[WorkspaceActiveScenarioEntry]) -> Vec<ApplyOutcome> {
            entries
                .iter()
                .map(|e| ApplyOutcome {
                    activation_id: e.activation_id,
                    service_name: e.service_name.clone(),
                    status: ApplyStatus::Applied,
                    error: None,
                })
                .collect()
        }
    }

    #[tokio::test]
    async fn tick_applies_and_reports_each_entry() {
        let workspace_id = Uuid::new_v4();
        let federation_id = Uuid::new_v4();
        let activation_id = Uuid::new_v4();

        let canned = WorkspaceActiveScenariosResponse {
            workspace_id,
            entries: vec![WorkspaceActiveScenarioEntry {
                activation_id,
                federation_id,
                federation_name: "shop".to_string(),
                scenario_name: "payment-outage".to_string(),
                service_name: "payments".to_string(),
                override_config: Some(ServiceScenarioOverride {
                    failure_rate: Some(0.5),
                    ..Default::default()
                }),
            }],
        };
        let reports = Arc::new(Mutex::new(Vec::new()));
        let transport = FakeTransport {
            canned,
            reports: reports.clone(),
        };

        let poller = FederationScenarioPoller::new(
            workspace_id,
            transport,
            AlwaysApplies,
            Duration::from_millis(100),
        );

        let outcomes = poller.tick().await.unwrap();
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].status, ApplyStatus::Applied);
        assert_eq!(outcomes[0].service_name, "payments");

        let reports = reports.lock().unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].0, federation_id);
        assert_eq!(reports[0].1, "payments");
        assert_eq!(reports[0].2, ApplyStatus::Applied);
    }

    #[tokio::test]
    async fn tick_with_no_entries_reports_nothing() {
        let workspace_id = Uuid::new_v4();
        let canned = WorkspaceActiveScenariosResponse {
            workspace_id,
            entries: vec![],
        };
        let reports = Arc::new(Mutex::new(Vec::new()));
        let transport = FakeTransport {
            canned,
            reports: reports.clone(),
        };

        let poller = FederationScenarioPoller::new(
            workspace_id,
            transport,
            AlwaysApplies,
            Duration::from_millis(100),
        );

        let outcomes = poller.tick().await.unwrap();
        assert!(outcomes.is_empty());
        assert!(reports.lock().unwrap().is_empty());
    }
}
