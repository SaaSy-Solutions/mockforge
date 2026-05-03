//! HTTP callbacks from the runner back into the registry.
//!
//! All registry-side state transitions (run started / event appended /
//! run finished / runner-seconds reported) go through these endpoints
//! so the registry stays the single source of truth. Routes are
//! `/api/v1/internal/*` and authenticate via a bearer token (mTLS will
//! land later — see RunnerConfig::registry_internal_token).

use serde::Serialize;
use uuid::Uuid;

use crate::error::Result;
use crate::executors::JobOutcome;

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
