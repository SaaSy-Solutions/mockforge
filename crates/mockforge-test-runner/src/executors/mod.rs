//! Per-kind executors. The dispatcher hands each `RunJob` to the
//! executor registered for its `kind` and reports the outcome back via
//! `RegistryCallbacks`.
//!
//! Only kinds backed by real work are registered. Unknown or unfinished
//! kinds are rejected by the dispatcher instead of reporting a successful
//! run without executing checks.

pub mod chain;
pub mod chaos;
pub mod contract;
pub mod fitness;
pub mod integration;
pub mod replay;
pub mod smoke;
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

/// Maps `kind` strings to executors. Multiple kinds can share an impl.
pub struct ExecutorRegistry {
    by_kind: HashMap<&'static str, Box<dyn Executor>>,
}

impl Default for ExecutorRegistry {
    fn default() -> Self {
        let mut by_kind: HashMap<&'static str, Box<dyn Executor>> = HashMap::new();

        // Test execution suite (#4) — kinds that share the TestExecutor.
        for k in [
            "integration",
            "conformance",
            "bench",
            "owasp",
            "security",
            "wafbench",
            "crud_flow",
        ] {
            by_kind.insert(k, Box::new(test::TestExecutor::for_kind(k)));
        }

        by_kind.insert("chaos_campaign", Box::new(chaos::ChaosExecutor));
        by_kind.insert("contract_diff", Box::new(contract::ContractExecutor));
        by_kind.insert("fitness_evaluation", Box::new(fitness::FitnessExecutor));
        by_kind.insert("chain", Box::new(chain::ChainExecutor));
        by_kind.insert("replay", Box::new(replay::ReplayExecutor));
        by_kind.insert("smoke", Box::new(smoke::SmokeTestExecutor));

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
    fn registry_covers_runnable_kinds() {
        let reg = ExecutorRegistry::default();
        let kinds = [
            "integration",
            "conformance",
            "bench",
            "owasp",
            "security",
            "wafbench",
            "crud_flow",
            "chaos_campaign",
            "contract_diff",
            "fitness_evaluation",
            "chain",
            "replay",
            "smoke",
        ];
        for k in kinds {
            reg.lookup(k).unwrap_or_else(|_| panic!("missing executor for kind {k}"));
        }
    }

    #[test]
    fn registry_rejects_kinds_without_real_executors() {
        let reg = ExecutorRegistry::default();
        for kind in [
            "unit",
            "behavioral_clone",
            "snapshot_capture",
            "snapshot_restore",
            "verification_suite",
            "scenario",
            "orchestration",
            "state_machine",
        ] {
            assert!(
                matches!(reg.lookup(kind), Err(Error::UnknownKind(rejected)) if rejected == kind),
                "{kind} must not be registered"
            );
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

    #[test]
    fn registered_kinds_match_registry_runnable_kinds() {
        let registry_source = include_str!("../../../mockforge-registry-server/src/run_queue.rs");
        let declaration = registry_source
            .split_once("pub const RUNNABLE_TEST_RUN_KINDS")
            .expect("registry runnable-kind const must exist")
            .1;
        let array = declaration
            .split_once("= &[")
            .expect("runnable-kind const must be a slice")
            .1
            .split_once("];")
            .expect("runnable-kind const must end with ];")
            .0;
        let expected: std::collections::BTreeSet<String> = array
            .split(',')
            .filter_map(|entry| {
                let entry = entry.trim();
                entry
                    .strip_prefix('"')
                    .and_then(|value| value.strip_suffix('"'))
                    .map(str::to_string)
            })
            .collect();
        let actual: std::collections::BTreeSet<String> = ExecutorRegistry::default()
            .by_kind
            .keys()
            .map(|kind| (*kind).to_string())
            .collect();
        assert_eq!(actual, expected, "runner and registry runnable kinds drifted");
    }
}
