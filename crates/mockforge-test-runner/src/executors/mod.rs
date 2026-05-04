//! Per-kind executors. The dispatcher hands each `RunJob` to the
//! executor registered for its `kind` and reports the outcome back via
//! `RegistryCallbacks`.
//!
//! Phase 1 layout: every executor is a stub that immediately reports
//! `errored` with a "not implemented" message. Slices owning each task
//! fill in real logic without blocking each other:
//!
//! - `test`           — #4 unit/integration/conformance/bench/owasp
//! - `chaos`          — #7 chaos_campaign
//! - `clone_train`    — #6 behavioral_clone
//! - `snapshot`       — #10 snapshot_capture / snapshot_restore
//! - `contract`       — #8 contract_diff / verification_suite / fitness_evaluation
//! - `flow`           — #9 scenario / orchestration / state_machine / chain
//! - `replay`         — #6 replay

pub mod chaos;
pub mod clone_train;
pub mod contract;
pub mod flow;
pub mod replay;
pub mod snapshot;
pub mod test;

use async_trait::async_trait;
use std::collections::HashMap;
use uuid::Uuid;

use crate::callbacks::RegistryCallbacks;
use crate::error::{Error, Result};

/// A job dispatched to an executor.
#[derive(Debug, Clone)]
pub struct RunJob {
    /// Primary key of the test_runs row.
    pub run_id: Uuid,
    /// Owning org (for billing meter callbacks).
    pub org_id: Uuid,
    /// The resource that produced this run (suite_id, flow_id, etc.).
    pub source_id: Uuid,
    /// Drives executor selection.
    pub kind: String,
    /// Per-kind opaque payload.
    pub payload: serde_json::Value,
}

/// Terminal outcome an executor returns. The dispatcher passes this to
/// `RegistryCallbacks::run_finished` which updates the `test_runs` row
/// and increments `runner_seconds_used`.
#[derive(Debug, Clone)]
pub struct JobOutcome {
    /// Terminal status. One of `passed | failed | cancelled | errored`.
    pub status: JobStatus,
    /// Wall-clock seconds spent in this run (for the billing meter).
    pub runner_seconds: i32,
    /// Optional kind-specific summary JSON for the run row's `summary`
    /// column.
    pub summary: Option<serde_json::Value>,
}

/// Subset of test_runs.status values an executor can produce. `queued`
/// and `running` aren't terminal so the executor never returns them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    /// Tests/checks all passed.
    Passed,
    /// At least one assertion failed (still a clean run, just red).
    Failed,
    /// User aborted via `POST /test-runs/{id}/cancel`.
    Cancelled,
    /// Executor itself crashed or hit infrastructure failure (distinct
    /// from `Failed` which is a clean negative result).
    Errored,
}

impl JobStatus {
    /// String representation used in the test_runs.status column.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Errored => "errored",
        }
    }
}

/// Per-kind work. Implementors stream events via `callbacks` as work
/// progresses, then return the terminal outcome. Errors thrown
/// up-front (before any events) are mapped to `errored` by the
/// dispatcher.
#[async_trait]
pub trait Executor: Send + Sync {
    /// Human-readable kind label this executor handles. Used both for
    /// registry lookups and for log messages.
    fn kind(&self) -> &'static str;

    /// Execute one job to terminal status.
    async fn execute(&self, job: RunJob, callbacks: &RegistryCallbacks) -> Result<JobOutcome>;
}

/// Maps `kind` strings to executors. Multiple kinds can share an impl
/// (e.g. all of unit/integration/conformance/bench/owasp use the
/// `TestExecutor`).
pub struct ExecutorRegistry {
    by_kind: HashMap<&'static str, Box<dyn Executor>>,
}

impl Default for ExecutorRegistry {
    fn default() -> Self {
        let mut by_kind: HashMap<&'static str, Box<dyn Executor>> = HashMap::new();

        // Test execution suite (#4) — kinds that share the TestExecutor
        // impl. The cloud_api path (run_cloud_*) handles bench/owasp/
        // conformance/security/wafbench/crud_flow/data_driven when
        // payloads opt in; unit/integration fall through to synthetic mode.
        for k in [
            "unit",
            "integration",
            "conformance",
            "bench",
            "owasp",
            "security",
            "wafbench",
            "crud_flow",
            "data_driven",
        ] {
            by_kind.insert(k, Box::new(test::TestExecutor::for_kind(k)));
        }

        by_kind.insert("chaos_campaign", Box::new(chaos::ChaosExecutor));
        by_kind.insert("behavioral_clone", Box::new(clone_train::CloneTrainExecutor));
        by_kind.insert(
            "snapshot_capture",
            Box::new(snapshot::SnapshotExecutor::for_kind("snapshot_capture")),
        );
        by_kind.insert(
            "snapshot_restore",
            Box::new(snapshot::SnapshotExecutor::for_kind("snapshot_restore")),
        );
        by_kind.insert(
            "contract_diff",
            Box::new(contract::ContractExecutor::for_kind("contract_diff")),
        );
        by_kind.insert(
            "verification_suite",
            Box::new(contract::ContractExecutor::for_kind("verification_suite")),
        );
        by_kind.insert(
            "fitness_evaluation",
            Box::new(contract::ContractExecutor::for_kind("fitness_evaluation")),
        );

        for k in ["scenario", "orchestration", "state_machine", "chain"] {
            by_kind.insert(k, Box::new(flow::FlowExecutor::for_kind(k)));
        }

        by_kind.insert("replay", Box::new(replay::ReplayExecutor));

        Self { by_kind }
    }
}

impl ExecutorRegistry {
    /// Resolve the executor for a given kind. Returns `Error::UnknownKind`
    /// when no executor is registered.
    pub fn lookup(&self, kind: &str) -> Result<&dyn Executor> {
        self.by_kind
            .get(kind)
            .map(|b| b.as_ref())
            .ok_or_else(|| Error::UnknownKind(kind.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_status_as_str_round_trips() {
        for s in [
            JobStatus::Passed,
            JobStatus::Failed,
            JobStatus::Cancelled,
            JobStatus::Errored,
        ] {
            let name = s.as_str();
            assert!(["passed", "failed", "cancelled", "errored"].contains(&name));
        }
    }

    #[test]
    fn registry_covers_every_documented_kind() {
        let reg = ExecutorRegistry::default();
        let kinds = [
            "unit",
            "integration",
            "conformance",
            "bench",
            "owasp",
            "security",
            "wafbench",
            "crud_flow",
            "data_driven",
            "chaos_campaign",
            "behavioral_clone",
            "snapshot_capture",
            "snapshot_restore",
            "contract_diff",
            "verification_suite",
            "fitness_evaluation",
            "scenario",
            "orchestration",
            "state_machine",
            "chain",
            "replay",
        ];
        for k in kinds {
            reg.lookup(k).unwrap_or_else(|_| panic!("missing executor for kind {k}"));
        }
    }

    #[test]
    fn registry_rejects_unknown_kind() {
        let reg = ExecutorRegistry::default();
        // expect_err would require the Ok variant to be Debug, but it's a
        // trait object — match on the result instead.
        match reg.lookup("not_a_real_kind") {
            Err(Error::UnknownKind(k)) => assert_eq!(k, "not_a_real_kind"),
            Err(other) => panic!("expected UnknownKind, got {other:?}"),
            Ok(_) => panic!("expected UnknownKind"),
        }
    }
}
