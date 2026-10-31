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
//!         ├─ integration / conformance / security kinds → `executors::test`
//!         ├─ kind = "chaos_campaign"                   → `executors::chaos`
//!         ├─ kind = "contract_diff"                    → `executors::contract`
//!         ├─ kind = "fitness_evaluation"               → `executors::fitness`
//!         ├─ kind = "chain"                            → `executors::chain`
//!         ├─ kind = "replay"                           → `executors::replay`
//!         └─ kind = "smoke"                            → `executors::smoke`
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

#![warn(missing_docs)]
#![allow(clippy::missing_errors_doc)]
// crate-level: errors are typed via thiserror
// `next_seq` is a run-event sequence number, not a loop counter — it is
// shared across several loops and helper calls in the executors.
#![allow(clippy::explicit_counter_loop)]

pub mod callbacks;
pub mod config;
pub mod dispatcher;
pub mod error;
pub mod executors;
pub mod queue;
pub mod target_client;

pub use config::RunnerConfig;
pub use dispatcher::Dispatcher;
pub use error::{Error, Result};
pub use executors::{Executor, ExecutorRegistry, JobOutcome, RunJob};
pub use target_client::TargetClient;
