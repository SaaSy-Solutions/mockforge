//! Runner configuration. All values are pulled from environment
//! variables so the worker can be deployed as a single binary with
//! zero-config-file Fly.io / k8s manifests.

use crate::error::{Error, Result};

/// Runtime configuration for the runner. Construct via `from_env`.
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    /// Redis URL (e.g. `redis://default:password@host:6379/0`).
    pub redis_url: String,
    /// Redis list key the registry pushes new run-job descriptors onto.
    pub queue_key: String,
    /// Base URL for the registry server's internal mTLS routes.
    /// Callbacks like `/api/v1/internal/test-runs/{id}/start` are made
    /// against this.
    pub registry_internal_base_url: String,
    /// Bearer token (or mTLS cert thumbprint, future) authenticating the
    /// runner to the registry's internal routes.
    pub registry_internal_token: String,
    /// Max concurrent jobs this runner instance will execute. Combined
    /// with `max_concurrent_runs` per org plan-limit, the registry sets
    /// the upper bound on Fly machine count.
    pub max_concurrent_jobs: usize,
    /// Polling timeout for `BLPOP`. Shorter values cost more Redis ops
    /// but make graceful shutdown faster.
    pub poll_timeout_secs: usize,
}

impl RunnerConfig {
    /// Read configuration from `MOCKFORGE_RUNNER_*` env vars. Returns
    /// `Error::Config` when a required value is missing.
    pub fn from_env() -> Result<Self> {
        let redis_url = required_env("MOCKFORGE_RUNNER_REDIS_URL")?;
        let queue_key = std::env::var("MOCKFORGE_RUNNER_QUEUE_KEY")
            .unwrap_or_else(|_| "test_runs:queued".to_string());
        let registry_internal_base_url =
            required_env("MOCKFORGE_RUNNER_REGISTRY_INTERNAL_BASE_URL")?;
        let registry_internal_token = required_env("MOCKFORGE_RUNNER_REGISTRY_INTERNAL_TOKEN")?;
        let max_concurrent_jobs = std::env::var("MOCKFORGE_RUNNER_MAX_CONCURRENT_JOBS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4);
        let poll_timeout_secs = std::env::var("MOCKFORGE_RUNNER_POLL_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);

        Ok(Self {
            redis_url,
            queue_key,
            registry_internal_base_url,
            registry_internal_token,
            max_concurrent_jobs,
            poll_timeout_secs,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    std::env::var(name).map_err(|_| Error::Config(format!("missing env var {name}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_env_complains_about_missing_required() {
        // Just ensures the error path is reachable. We don't manipulate
        // process env in unit tests — that races with parallel tests.
        let err = required_env("__MOCKFORGE_RUNNER_DEFINITELY_UNSET__")
            .expect_err("expected missing-env error");
        assert!(matches!(err, Error::Config(_)));
    }
}
