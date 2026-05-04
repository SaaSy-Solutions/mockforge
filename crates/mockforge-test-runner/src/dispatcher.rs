//! Dispatcher: pulls jobs off the queue and routes them to the
//! per-kind executor. The crate's main loop.

use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

use crate::callbacks::RegistryCallbacks;
use crate::config::RunnerConfig;
use crate::error::{Error, Result};
use crate::executors::{ExecutorRegistry, JobOutcome, JobStatus, RunJob};
use crate::queue::Consumer;

/// Top-level worker. Owns the queue consumer, the executor registry,
/// and a concurrency semaphore that bounds in-flight jobs.
pub struct Dispatcher {
    consumer: Consumer,
    registry: ExecutorRegistry,
    callbacks: Arc<RegistryCallbacks>,
    in_flight: Arc<Semaphore>,
}

impl Dispatcher {
    /// Wire up from config. Connects to Redis up-front so config errors
    /// surface before the main loop starts.
    pub async fn new(cfg: RunnerConfig) -> Result<Self> {
        let consumer =
            Consumer::connect(&cfg.redis_url, cfg.queue_key.clone(), cfg.poll_timeout_secs).await?;
        let callbacks = Arc::new(RegistryCallbacks::new(
            cfg.registry_internal_base_url.clone(),
            cfg.registry_internal_token.clone(),
        ));
        let in_flight = Arc::new(Semaphore::new(cfg.max_concurrent_jobs));
        Ok(Self {
            consumer,
            registry: ExecutorRegistry::default(),
            callbacks,
            in_flight,
        })
    }

    /// Run the consumer loop until cancellation. Blocks on Redis BLPOP;
    /// when a job arrives, acquires a semaphore permit and spawns a
    /// task to execute it. Permits cap concurrency at `max_concurrent_jobs`
    /// so a single replica never spawns more work than configured.
    pub async fn run(mut self) -> Result<()> {
        info!("dispatcher main loop starting");
        loop {
            let descriptor = match self.consumer.pop().await {
                Ok(Some(d)) => d,
                Ok(None) => {
                    // BLPOP timeout — loop again. (Lets the runtime
                    // process shutdown signals between polls.)
                    continue;
                }
                Err(Error::Redis(e)) => {
                    error!(error = %e, "redis pop failed; backing off 1s");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                }
                Err(e) => {
                    error!(error = %e, "queue pop returned non-redis error; treating as fatal");
                    return Err(e);
                }
            };

            let permit = self
                .in_flight
                .clone()
                .acquire_owned()
                .await
                .map_err(|e| Error::Other(anyhow::anyhow!("semaphore closed: {e}")))?;

            let job: RunJob = descriptor.into();
            let kind = job.kind.clone();
            let run_id = job.run_id;
            let callbacks = self.callbacks.clone();

            // Resolve before spawning so an unknown-kind job marks the run
            // errored without blocking a concurrency permit on the bad
            // kind every time.
            let executor: &dyn crate::executors::Executor = match self.registry.lookup(&kind) {
                Ok(e) => e,
                Err(e) => {
                    warn!(run_id = %run_id, kind = %kind, "{e}");
                    let cb_clone = callbacks.clone();
                    tokio::spawn(async move {
                        report_unknown_kind(&cb_clone, run_id, &kind).await;
                        drop(permit);
                    });
                    continue;
                }
            };

            // Re-clone the boxed executor by moving via trait object;
            // the executor lives inside the registry so we can't move
            // it. Instead we synchronously execute on the dispatcher
            // task. (Future: per-kind &'static dyn refs that live for
            // the dispatcher's lifetime, then the spawn is feasible.)
            let outcome = executor.execute(job.clone(), &callbacks).await;
            drop(permit);

            match outcome {
                Ok(o) => {
                    if let Err(e) = callbacks.run_finished(run_id, &o).await {
                        error!(run_id = %run_id, error = %e, "run_finished callback failed");
                    }
                }
                Err(e) => {
                    error!(run_id = %run_id, kind = %kind, error = %e, "executor errored");
                    let outcome = JobOutcome {
                        status: JobStatus::Errored,
                        runner_seconds: 0,
                        summary: Some(serde_json::json!({ "error": e.to_string() })),
                    };
                    if let Err(e) = callbacks.run_finished(run_id, &outcome).await {
                        error!(run_id = %run_id, error = %e, "errored-finish callback also failed");
                    }
                }
            }
        }
    }
}

async fn report_unknown_kind(callbacks: &RegistryCallbacks, run_id: uuid::Uuid, kind: &str) {
    let outcome = JobOutcome {
        status: JobStatus::Errored,
        runner_seconds: 0,
        summary: Some(serde_json::json!({
            "error": format!("no executor registered for kind '{kind}'")
        })),
    };
    if let Err(e) = callbacks.run_finished(run_id, &outcome).await {
        error!(run_id = %run_id, error = %e, "unknown-kind callback failed");
    }
}
