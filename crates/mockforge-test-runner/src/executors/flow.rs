//! Flow executor (#9 / Phase 2). Handles `scenario | orchestration |
//! state_machine | chain`.
//!
//! Only `chain` with a non-empty `config.links` list has a real
//! executor (see `chain.rs`). `scenario`, `orchestration` and
//! `state_machine` need the kind-specific runtimes
//! (mockforge-scenarios / mockforge-pipelines) wired into the runner;
//! until then those runs, and chains without links, finish `errored`
//! with `status_reason = "not_implemented"` (#1264). They previously
//! emitted `node_visited` events for nodes that were never executed and
//! reported `passed`.

use async_trait::async_trait;
use std::time::Instant;

use crate::callbacks::RegistryCallbacks;
use crate::error::Result;
use crate::executors::{unsupported, Executor, JobOutcome, RunJob};

/// Executor for the four flow kinds.
pub struct FlowExecutor {
    kind: &'static str,
}

impl FlowExecutor {
    /// Construct for one of scenario/orchestration/state_machine/chain.
    pub fn for_kind(kind: &'static str) -> Self {
        Self { kind }
    }
}

/// The chain config to execute, when the payload carries one with links.
fn chain_config(payload: &serde_json::Value) -> Option<&serde_json::Value> {
    payload
        .get("config")
        .filter(|c| c.get("links").and_then(|v| v.as_array()).is_some_and(|a| !a.is_empty()))
}

#[async_trait]
impl Executor for FlowExecutor {
    fn kind(&self) -> &'static str {
        self.kind
    }

    async fn execute(&self, job: RunJob, callbacks: &RegistryCallbacks) -> Result<JobOutcome> {
        let started = Instant::now();

        if self.kind == "chain" {
            if let Some(config) = chain_config(&job.payload).cloned() {
                return crate::executors::chain::run_chain(job, callbacks, started, &config).await;
            }
        }

        callbacks.run_started(job.run_id).await?;
        let detail = if self.kind == "chain" {
            "the chain config has no links to execute"
        } else {
            "flow nodes are not executed by the runner yet"
        };
        unsupported(&job, callbacks, started, 1, detail).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn chain_config_requires_links() {
        assert!(chain_config(&json!({})).is_none());
        assert!(chain_config(&json!({ "config": { "links": [] } })).is_none());
        assert!(chain_config(&json!({ "config": { "links": [{ "id": "a" }] } })).is_some());
    }
}
