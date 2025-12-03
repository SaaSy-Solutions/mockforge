//! MockForge Kubernetes Operator Main Entry Point

use kube::Client;
use mockforge_k8s_operator::Controller;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mockforge_k8s_operator=info,kube=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting MockForge Kubernetes Operator");

    // Create Kubernetes client
    let client = Client::try_default().await?;

    info!("Connected to Kubernetes cluster");

    // Create and run controller
    let controller = Controller::new(client);

    // Get namespace from environment variable, or watch all namespaces
    let namespace = std::env::var("WATCH_NAMESPACE").ok();

    if let Err(e) = controller.run(namespace).await {
        error!("Controller error: {:?}", e);
        std::process::exit(1);
    }

    Ok(())
}
