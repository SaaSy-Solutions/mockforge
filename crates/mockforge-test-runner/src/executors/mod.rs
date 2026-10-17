//! Per-kind executors. The dispatcher hands each `RunJob` to the
//! executor registered for its `kind` and reports the outcome back via
//! `RegistryCallbacks`.
//!
//! No executor may report `passed` without doing the work (#1264):
//!
//! - A kind (or payload shape) with no real implementation finishes
//!   `errored` with `summary.status_reason = "not_implemented"` via
//!   [`unsupported`]. It never emits fake progress events.
//! - Every outcome carries [`JobOutcome::work_count`], the number of
//!   cases, requests, probes or comparisons actually executed.
//!   [`JobOutcome::finalize`] turns a `passed` outcome with zero work
//!   into `failed` with `status_reason = "no_work_executed"`.
//! - [`KIND_SUPPORT`] declares, per kind, whether a real executor exists.
//!   A test fails when the registry and that table disagree.
//!
//! Module layout:
//!
//! - `test`           — #4 unit/integration/conformance/bench/owasp/...
//! - `chaos`          — #7 chaos_campaign
//! - `clone_train`    — #6 behavioral_clone (not implemented)
//! - `snapshot`       — #10 snapshot_capture / snapshot_restore (not implemented)
//! - `contract`       — #8 contract_diff / verification_suite (not implemented)
//! - `fitness`        — fitness_evaluation
//! - `flow`           — #9 scenario / orchestration / state_machine (not
//!   implemented) / chain
//! - `replay`         — #6 replay

pub mod chain;
pub mod chaos;
pub mod clone_train;
pub mod contract;
pub mod fitness;
pub mod flow;
pub mod integration;
pub mod replay;
pub mod smoke;
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
    /// Units of real work executed: test cases, HTTP requests, probes,
    /// endpoints compared, etc. A `passed` outcome with zero work is
    /// downgraded by [`JobOutcome::finalize`].
    pub work_count: u64,
}

/// `summary.status_reason` for kinds or payload shapes the runner does
/// not implement.
pub const REASON_NOT_IMPLEMENTED: &str = "not_implemented";

/// `summary.status_reason` for a `passed` outcome that executed nothing.
pub const REASON_NO_WORK: &str = "no_work_executed";

impl JobOutcome {
    /// Enforce the runner's reporting invariants before the outcome is
    /// sent to the registry:
    ///
    /// - `passed` requires `work_count > 0`; otherwise the outcome becomes
    ///   `failed` with `status_reason = "no_work_executed"`.
    /// - `summary.work_count` is always set, so API/UI consumers can see
    ///   how much was executed.
    pub fn finalize(mut self) -> Self {
        let mut summary = match self.summary.take() {
            Some(serde_json::Value::Object(map)) => map,
            Some(other) => {
                let mut map = serde_json::Map::new();
                map.insert("detail".into(), other);
                map
            }
            None => serde_json::Map::new(),
        };
        if self.status == JobStatus::Passed && self.work_count == 0 {
            self.status = JobStatus::Failed;
            summary.insert("status_reason".into(), REASON_NO_WORK.into());
            summary.insert(
                "error".into(),
                "run reported success but executed zero cases or requests".into(),
            );
        }
        summary.insert("work_count".into(), self.work_count.into());
        self.summary = Some(serde_json::Value::Object(summary));
        self
    }
}

/// Wall-clock seconds for the billing meter, rounded up, minimum 1.
pub(crate) fn runner_secs(started: std::time::Instant) -> i32 {
    (started.elapsed().as_secs_f64().ceil() as i32).max(1)
}

/// Finish a run that did no work because the runner could not do it:
/// emits one error `log` event at `seq` and returns `errored` with
/// `summary.status_reason = reason` and `summary.error = message`.
pub(crate) async fn errored_with_reason(
    job: &RunJob,
    callbacks: &RegistryCallbacks,
    started: std::time::Instant,
    seq: u32,
    reason: &str,
    message: String,
) -> Result<JobOutcome> {
    callbacks
        .run_event(
            job.run_id,
            seq,
            "log",
            serde_json::json!({
                "level": "error",
                "message": message,
                "status_reason": reason,
            }),
        )
        .await?;
    Ok(JobOutcome {
        status: JobStatus::Errored,
        runner_seconds: runner_secs(started),
        summary: Some(serde_json::json!({
            "executor_phase": "unsupported",
            "kind": job.kind,
            "status_reason": reason,
            "error": message,
        })),
        work_count: 0,
    })
}

/// Finish a run whose kind (or payload shape) has no real executor.
/// Reports `errored` with `status_reason = "not_implemented"`; never
/// `passed`.
pub(crate) async fn unsupported(
    job: &RunJob,
    callbacks: &RegistryCallbacks,
    started: std::time::Instant,
    seq: u32,
    detail: &str,
) -> Result<JobOutcome> {
    let message = format!("'{}' runs are not implemented by the cloud runner: {detail}", job.kind);
    errored_with_reason(job, callbacks, started, seq, REASON_NOT_IMPLEMENTED, message).await
}

/// Whether a kind has a real executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindSupport {
    /// A real executor runs when the payload carries the inputs it needs;
    /// other payload shapes finish `errored` (`not_implemented` or a
    /// config error), never `passed`.
    Real,
    /// No executor exists; every run finishes `errored` with
    /// `status_reason = "not_implemented"`.
    NotImplemented,
}

/// Every kind the registry may enqueue and whether it is implemented.
/// `ExecutorRegistry::default()` must register exactly these kinds.
pub const KIND_SUPPORT: &[(&str, KindSupport)] = &[
    ("unit", KindSupport::NotImplemented),
    ("integration", KindSupport::Real),
    ("conformance", KindSupport::Real),
    ("bench", KindSupport::Real),
    ("owasp", KindSupport::Real),
    ("security", KindSupport::Real),
    ("wafbench", KindSupport::Real),
    ("crud_flow", KindSupport::Real),
    ("chaos_campaign", KindSupport::Real),
    ("behavioral_clone", KindSupport::NotImplemented),
    ("snapshot_capture", KindSupport::NotImplemented),
    ("snapshot_restore", KindSupport::NotImplemented),
    ("contract_diff", KindSupport::Real),
    ("verification_suite", KindSupport::NotImplemented),
    ("fitness_evaluation", KindSupport::Real),
    ("scenario", KindSupport::NotImplemented),
    ("orchestration", KindSupport::NotImplemented),
    ("state_machine", KindSupport::NotImplemented),
    ("chain", KindSupport::Real),
    ("replay", KindSupport::Real),
    ("smoke", KindSupport::Real),
];

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
        // conformance/security/wafbench/crud_flow when payloads opt in;
        // payloads with no real path (and every `unit` run) finish as
        // not_implemented.
        for k in [
            "unit",
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
        // fitness_evaluation has its own executor now (#355). The
        // ContractExecutor's synthetic fallback for this kind has been
        // retired — FitnessExecutor fetches the function definition,
        // dispatches on `kind` (latency_threshold, error_rate, etc.),
        // and queries the matching data source via internal callbacks.
        by_kind.insert("fitness_evaluation", Box::new(fitness::FitnessExecutor));

        for k in ["scenario", "orchestration", "state_machine", "chain"] {
            by_kind.insert(k, Box::new(flow::FlowExecutor::for_kind(k)));
        }

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

    /// Every registered kind, sorted.
    pub fn kinds(&self) -> Vec<&'static str> {
        let mut kinds: Vec<&'static str> = self.by_kind.keys().copied().collect();
        kinds.sort_unstable();
        kinds
    }
}

/// Test helpers: a tiny HTTP server standing in for the registry's
/// internal callback API (or any target), recording every request.
#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// One request the fake server received.
    #[derive(Debug, Clone)]
    pub struct Recorded {
        /// e.g. `POST /api/v1/internal/test-runs/<id>/events`
        pub request_line: String,
        /// Raw body, parsed as JSON when possible.
        pub body: serde_json::Value,
    }

    type Responder = dyn Fn(&str) -> (u16, String) + Send + Sync;

    /// A running fake HTTP server.
    pub struct FakeHttp {
        /// `http://127.0.0.1:<port>`
        pub base_url: String,
        requests: Arc<Mutex<Vec<Recorded>>>,
    }

    impl FakeHttp {
        /// Serve every request with `respond(request_line)`.
        pub async fn start(
            respond: impl Fn(&str) -> (u16, String) + Send + Sync + 'static,
        ) -> Self {
            let listener =
                tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind loopback");
            let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
            let requests = Arc::new(Mutex::new(Vec::new()));
            let respond: Arc<Responder> = Arc::new(respond);
            let recorded = requests.clone();
            tokio::spawn(async move {
                while let Ok((socket, _)) = listener.accept().await {
                    let recorded = recorded.clone();
                    let respond = respond.clone();
                    tokio::spawn(handle(socket, recorded, respond));
                }
            });
            Self { base_url, requests }
        }

        /// A fake registry: every callback succeeds with `{}`.
        pub async fn registry() -> Self {
            Self::start(|_| (200, "{}".to_string())).await
        }

        /// Snapshot of everything received so far.
        pub fn requests(&self) -> Vec<Recorded> {
            self.requests.lock().expect("fake server lock").clone()
        }

        /// Bodies of the `events` callbacks, in order.
        pub fn events(&self) -> Vec<serde_json::Value> {
            self.requests()
                .into_iter()
                .filter(|r| r.request_line.contains("/events "))
                .map(|r| r.body)
                .collect()
        }
    }

    async fn handle(
        mut socket: tokio::net::TcpStream,
        recorded: Arc<Mutex<Vec<Recorded>>>,
        respond: Arc<Responder>,
    ) {
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 4096];
        let (head_end, length) = loop {
            let Ok(n) = socket.read(&mut chunk).await else {
                return;
            };
            if n == 0 {
                return;
            }
            bytes.extend_from_slice(&chunk[..n]);
            if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                let head = String::from_utf8_lossy(&bytes[..end]).to_string();
                let length = head
                    .lines()
                    .find_map(|l| {
                        let (k, v) = l.split_once(':')?;
                        k.eq_ignore_ascii_case("content-length").then(|| v.trim().parse().ok())?
                    })
                    .unwrap_or(0usize);
                break (end + 4, length);
            }
        };
        while bytes.len() < head_end + length {
            let Ok(n) = socket.read(&mut chunk).await else {
                return;
            };
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..n]);
        }
        let head = String::from_utf8_lossy(&bytes[..head_end]).to_string();
        let request_line = head.lines().next().unwrap_or_default().to_string();
        let body_bytes = &bytes[head_end..(head_end + length).min(bytes.len())];
        let body = serde_json::from_slice(body_bytes).unwrap_or(serde_json::Value::Null);
        recorded.lock().expect("fake server lock").push(Recorded {
            request_line: request_line.clone(),
            body,
        });
        let (status, body) = respond(&request_line);
        let response = format!(
            "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = socket.write_all(response.as_bytes()).await;
    }

    /// A job of `kind` with `payload`.
    pub fn job(kind: &str, payload: serde_json::Value) -> super::RunJob {
        super::RunJob {
            run_id: uuid::Uuid::new_v4(),
            org_id: uuid::Uuid::new_v4(),
            source_id: uuid::Uuid::new_v4(),
            kind: kind.to_string(),
            payload,
        }
    }

    /// `summary.status_reason`, if any.
    pub fn reason(outcome: &super::JobOutcome) -> Option<String> {
        outcome
            .summary
            .as_ref()
            .and_then(|s| s.get("status_reason"))
            .and_then(|v| v.as_str())
            .map(String::from)
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

    fn outcome(status: JobStatus, work_count: u64) -> JobOutcome {
        JobOutcome {
            status,
            runner_seconds: 1,
            summary: Some(serde_json::json!({ "executor_phase": "real" })),
            work_count,
        }
    }

    #[test]
    fn finalize_turns_zero_work_pass_into_failure() {
        let o = outcome(JobStatus::Passed, 0).finalize();
        assert_eq!(o.status, JobStatus::Failed);
        assert_eq!(test_support::reason(&o).as_deref(), Some(REASON_NO_WORK));
        assert_eq!(o.summary.as_ref().and_then(|s| s.get("work_count")), Some(&0.into()));
    }

    #[test]
    fn finalize_keeps_pass_with_work_and_records_count() {
        let o = outcome(JobStatus::Passed, 7).finalize();
        assert_eq!(o.status, JobStatus::Passed);
        assert_eq!(test_support::reason(&o), None);
        let summary = o.summary.expect("finalize always sets a summary");
        assert_eq!(summary["work_count"], 7);
        assert_eq!(summary["executor_phase"], "real");
    }

    #[test]
    fn finalize_leaves_non_pass_status_alone() {
        for status in [JobStatus::Failed, JobStatus::Errored, JobStatus::Cancelled] {
            let o = outcome(status, 0).finalize();
            assert_eq!(o.status, status);
            assert_eq!(o.summary.expect("summary set")["work_count"], 0);
        }
        let o = JobOutcome {
            status: JobStatus::Errored,
            runner_seconds: 0,
            summary: None,
            work_count: 0,
        }
        .finalize();
        assert_eq!(o.summary.expect("summary set")["work_count"], 0);
    }

    /// Fails when a kind is registered without declaring whether it has a
    /// real executor, or declared without being registered.
    #[test]
    fn kind_support_table_matches_registry() {
        let mut declared: Vec<&str> = KIND_SUPPORT.iter().map(|(k, _)| *k).collect();
        declared.sort_unstable();
        let before = declared.len();
        declared.dedup();
        assert_eq!(before, declared.len(), "KIND_SUPPORT lists a kind twice");
        assert_eq!(
            ExecutorRegistry::default().kinds(),
            declared,
            "every registered kind needs a KIND_SUPPORT entry (Real or NotImplemented)"
        );
    }

    /// Runs every registered kind with an empty payload — no target, no
    /// spec, no config — so no kind can do real work. None may pass, and
    /// NotImplemented kinds must say so and emit no fake progress events.
    #[tokio::test]
    async fn no_kind_passes_without_doing_work() {
        let registry = ExecutorRegistry::default();
        // Each kind's expected (status, status_reason) for an empty payload.
        // `None` reason: the executor's own pre-flight error summary.
        let expected: &[(&str, JobStatus, Option<&str>)] = &[
            ("unit", JobStatus::Errored, Some(REASON_NOT_IMPLEMENTED)),
            ("integration", JobStatus::Errored, Some(REASON_NOT_IMPLEMENTED)),
            ("conformance", JobStatus::Errored, Some(REASON_NOT_IMPLEMENTED)),
            ("bench", JobStatus::Errored, Some(REASON_NOT_IMPLEMENTED)),
            ("owasp", JobStatus::Errored, Some(REASON_NOT_IMPLEMENTED)),
            ("security", JobStatus::Errored, Some(REASON_NOT_IMPLEMENTED)),
            ("wafbench", JobStatus::Errored, Some(REASON_NOT_IMPLEMENTED)),
            ("crud_flow", JobStatus::Errored, Some(REASON_NOT_IMPLEMENTED)),
            ("chaos_campaign", JobStatus::Errored, Some("chaos_target_invalid")),
            ("behavioral_clone", JobStatus::Errored, Some(REASON_NOT_IMPLEMENTED)),
            ("snapshot_capture", JobStatus::Errored, Some(REASON_NOT_IMPLEMENTED)),
            ("snapshot_restore", JobStatus::Errored, Some(REASON_NOT_IMPLEMENTED)),
            ("contract_diff", JobStatus::Errored, Some(contract::REASON_SPEC_MISSING)),
            ("verification_suite", JobStatus::Errored, Some(REASON_NOT_IMPLEMENTED)),
            ("fitness_evaluation", JobStatus::Errored, None),
            ("scenario", JobStatus::Errored, Some(REASON_NOT_IMPLEMENTED)),
            ("orchestration", JobStatus::Errored, Some(REASON_NOT_IMPLEMENTED)),
            ("state_machine", JobStatus::Errored, Some(REASON_NOT_IMPLEMENTED)),
            ("chain", JobStatus::Errored, Some(REASON_NOT_IMPLEMENTED)),
            ("replay", JobStatus::Errored, Some("replay_target_missing")),
            ("smoke", JobStatus::Errored, None),
        ];
        let mut covered: Vec<&str> = expected.iter().map(|(k, _, _)| *k).collect();
        covered.sort_unstable();
        assert_eq!(registry.kinds(), covered, "add the new kind to this table");

        for (kind, want_status, want_reason) in expected {
            let fake = test_support::FakeHttp::registry().await;
            let callbacks = RegistryCallbacks::new(fake.base_url.clone(), "t".into());
            let executor = registry.lookup(kind).expect("kind is registered");
            let job = test_support::job(kind, serde_json::json!({}));
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                executor.execute(job, &callbacks),
            )
            .await
            .unwrap_or_else(|_| panic!("{kind} did not finish"));
            let o = result.unwrap_or_else(|e| panic!("{kind} returned Err: {e}")).finalize();

            assert_ne!(o.status, JobStatus::Passed, "{kind} passed without doing work");
            assert_eq!(o.status, *want_status, "{kind} status");
            assert_eq!(o.work_count, 0, "{kind} work_count");
            if let Some(reason) = want_reason {
                assert_eq!(test_support::reason(&o).as_deref(), Some(*reason), "{kind} reason");
            }

            let support = KIND_SUPPORT
                .iter()
                .find(|(k, _)| k == kind)
                .map(|(_, s)| *s)
                .expect("kind declared");
            if support == KindSupport::NotImplemented {
                assert_eq!(
                    test_support::reason(&o).as_deref(),
                    Some(REASON_NOT_IMPLEMENTED),
                    "{kind} is declared NotImplemented"
                );
                let events = fake.events();
                assert_eq!(events.len(), 1, "{kind} emitted fake progress: {events:?}");
                assert_eq!(events[0]["event_type"], "log");
                assert_eq!(events[0]["payload"]["level"], "error");
            }
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
            "smoke",
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
