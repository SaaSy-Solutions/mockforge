//! Entry point for the mockforge-test-runner worker.
//!
//! Reads `MOCKFORGE_RUNNER_*` env vars, opens a Redis connection,
//! dispatches forever. Designed to run on Fly.io as a separate app
//! or as a sidecar to the registry depending on deployment shape.

use mockforge_test_runner::{Dispatcher, RunnerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let cfg = RunnerConfig::from_env()?;
    tracing::info!(
        queue_key = %cfg.queue_key,
        max_concurrent = cfg.max_concurrent_jobs,
        "starting mockforge-test-runner",
    );

    let dispatcher = Dispatcher::new(cfg).await?;
    dispatcher.run().await?;
    Ok(())
}
