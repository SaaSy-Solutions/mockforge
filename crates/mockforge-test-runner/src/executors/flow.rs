//! Flow executor (#9 / Phase 2). Handles `scenario | orchestration |
//! state_machine | chain`.
//!
//! Synthetic-pass mode: emits a `node_visited` event per synthetic
//! node and reports `passed`. Real impl will load the flow's
//! current_version_id config and dispatch to the kind-specific
//! runtime (mockforge-scenarios / mockforge-pipelines).

use async_trait::async_trait;
use std::time::Instant;

use crate::callbacks::RegistryCallbacks;
use crate::error::Result;
use crate::executors::{Executor, JobOutcome, JobStatus, RunJob};

/// Executor for the four flow kinds.
pub struct FlowExecutor {
    kind: &'static str,
}

impl FlowExecutor {
    /// Construct for one of scenario/orchestration/state_machine/chain.
    pub fn for_kind(kind: &'static str) -> Self {
        Self { kind }
    }

    fn synthetic_node_count(payload: &serde_json::Value) -> u32 {
        let raw = payload.get("synthetic_nodes").and_then(|v| v.as_u64()).unwrap_or(4);
        raw.clamp(1, 100) as u32
    }

    fn synthetic_node_ms(payload: &serde_json::Value) -> u64 {
        let raw = payload.get("synthetic_node_ms").and_then(|v| v.as_u64()).unwrap_or(75);
        raw.min(2000)
    }
}

#[async_trait]
impl Executor for FlowExecutor {
    fn kind(&self) -> &'static str {
        self.kind
    }

    async fn execute(&self, job: RunJob, callbacks: &RegistryCallbacks) -> Result<JobOutcome> {
        let started = Instant::now();
        callbacks.run_started(job.run_id).await?;

        let nodes = Self::synthetic_node_count(&job.payload);
        let node_ms = Self::synthetic_node_ms(&job.payload);
        let flow_name =
            job.payload.get("flow_name").and_then(|v| v.as_str()).unwrap_or("(unnamed)");

        callbacks
            .run_event(
                job.run_id,
                1,
                "log",
                serde_json::json!({
                    "level": "info",
                    "message": format!(
                        "Synthetic flow execution: kind='{}', flow='{}', nodes={}, node_ms={}",
                        self.kind, flow_name, nodes, node_ms,
                    ),
                    "synthetic": true,
                    "tracking_task": 9,
                }),
            )
            .await?;

        let mut next_seq: u32 = 2;
        for i in 1..=nodes {
            callbacks
                .run_event(
                    job.run_id,
                    next_seq,
                    "node_visited",
                    serde_json::json!({
                        "node_index": i,
                        "node_name": format!("synthetic-node-{i}"),
                        "duration_ms": node_ms,
                    }),
                )
                .await?;
            next_seq += 1;
            tokio::time::sleep(std::time::Duration::from_millis(node_ms)).await;
        }

        let elapsed = started.elapsed();
        let secs = (elapsed.as_secs_f64().ceil() as i32).max(1);

        Ok(JobOutcome {
            status: JobStatus::Passed,
            runner_seconds: secs,
            summary: Some(serde_json::json!({
                "executor_phase": "synthetic",
                "tracking_task": 9,
                "kind": self.kind,
                "flow_name": flow_name,
                "nodes_visited": nodes,
                "wall_ms": elapsed.as_millis() as u64,
            })),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn node_count_default() {
        assert_eq!(FlowExecutor::synthetic_node_count(&json!({})), 4);
    }

    #[test]
    fn node_count_clamps() {
        assert_eq!(FlowExecutor::synthetic_node_count(&json!({ "synthetic_nodes": 9999 })), 100);
        assert_eq!(FlowExecutor::synthetic_node_count(&json!({ "synthetic_nodes": 0 })), 1);
    }
}
