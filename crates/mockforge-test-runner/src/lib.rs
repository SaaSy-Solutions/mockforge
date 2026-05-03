//! Cloud worker that consumes `test_runs` rows from a Redis queue and
//! dispatches per-kind executors. The keystone for the cloud-enablement
//! plan: tasks #4, #6, #7, #8, #9, #10 all queue jobs into this runner.
//!
//! ## Architecture
//!
//! ```text
//! [ registry POST /test-suites/{id}/runs ]
//!         │  inserts test_runs row in 'queued' state +
//!         │  pushes (run_id, kind) onto Redis list
//!         ▼
//! [ mockforge-test-runner ]
//!         │  blpop on the queue
//!         ▼
//! [ Dispatcher::run(job) ]  selects executor by job.kind
//!         │
//!         ├─ kind = "unit" | "integration" | …  → see `executors::test`
//!         ├─ kind = "chaos_campaign"            → see `executors::chaos`
//!         ├─ kind = "behavioral_clone"          → see `executors::clone_train`
//!         ├─ kind = "snapshot_capture" | …      → see `executors::snapshot`
//!         ├─ kind = "contract_diff" | …         → see `executors::contract`
//!         └─ kind = "scenario" | "chain" | …    → see `executors::flow`
//!         │
//!         ▼
//! [ Executor::execute(job) ]  per-kind logic
//!         │  emits events along the way
//!         │  reports terminal status + runner_seconds
//!         ▼
//! [ Callbacks ]  POST to registry's internal mTLS routes:
//!     /api/v1/internal/test-runs/{id}/start
//!     /api/v1/internal/test-runs/{id}/events
//!     /api/v1/internal/test-runs/{id}/finish
//! ```
//!
//! Phase 1 of this crate is scaffolding — every executor returns
//! "unimplemented" so the queue drains without crashing, and slices
//! that own each kind can fill them in incrementally without blocking
//! each other.

#![warn(missing_docs)]
#![allow(clippy::missing_errors_doc)] // crate-level: errors are typed via thiserror

pub mod callbacks;
pub mod config;
pub mod dispatcher;
pub mod error;
pub mod executors;
pub mod queue;

pub use config::RunnerConfig;
pub use dispatcher::Dispatcher;
pub use error::{Error, Result};
pub use executors::{Executor, ExecutorRegistry, JobOutcome, RunJob};
