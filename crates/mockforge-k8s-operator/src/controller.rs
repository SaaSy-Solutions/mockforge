//! Controller for managing ChaosOrchestration resources

use crate::crd::ChaosOrchestration;
use crate::reconciler::Reconciler;
use crate::{OperatorError, Result};
use futures::StreamExt;
use kube::{
    api::ListParams,
    runtime::{controller::{Action, Controller as KubeController}, watcher::Config},
    Api, Client, ResourceExt,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info};

/// Main controller for the operator
pub struct Controller {
    client: Client,
    reconciler: Arc<Reconciler>,
}

impl Controller {
    /// Create a new controller
    pub fn new(client: Client) -> Self {
        let reconciler = Arc::new(Reconciler::new(client.clone()));

        Self {
            client,
            reconciler,
        }
    }

    /// Run the controller
    pub async fn run(&self, namespace: Option<String>) -> Result<()> {
        info!("Starting MockForge Kubernetes Operator");

        let api: Api<ChaosOrchestration> = if let Some(ns) = namespace {
            info!("Watching namespace: {}", ns);
            Api::namespaced(self.client.clone(), &ns)
        } else {
            info!("Watching all namespaces");
            Api::all(self.client.clone())
        };

        let reconciler = self.reconciler.clone();

        KubeController::new(api.clone(), Config::default())
            .shutdown_on_signal()
            .run(
                move |orchestration, _ctx| {
                    let reconciler = reconciler.clone();
                    async move {
                        Self::reconcile(orchestration, reconciler).await
                    }
                },
                |_orchestration, error, _ctx| {
                    error!("Reconciliation error: {:?}", error);
                    Action::requeue(Duration::from_secs(60))
                },
                Arc::new(()),
            )
            .for_each(|res| async move {
                match res {
                    Ok(o) => debug!("Reconciled: {:?}", o),
                    Err(e) => error!("Reconcile error: {:?}", e),
                }
            })
            .await;

        Ok(())
    }

    /// Reconcile a single orchestration
    async fn reconcile(
        orchestration: Arc<ChaosOrchestration>,
        reconciler: Arc<Reconciler>,
    ) -> std::result::Result<Action, OperatorError> {
        let name = orchestration.name_any();
        let namespace = orchestration.namespace().unwrap_or_else(|| "default".to_string());

        info!("Reconciling ChaosOrchestration: {}/{}", namespace, name);

        match orchestration.metadata.deletion_timestamp {
            Some(_) => {
                // Handle deletion
                reconciler.cleanup(&name).await?;
                Ok(Action::await_change())
            }
            None => {
                // Normal reconciliation
                reconciler.reconcile(orchestration, &namespace).await?;

                // Requeue after 30 seconds to check status
                Ok(Action::requeue(Duration::from_secs(30)))
            }
        }
    }

    /// Watch for ChaosOrchestration resources
    pub async fn watch(&self, namespace: Option<String>) -> Result<()> {
        let api: Api<ChaosOrchestration> = if let Some(ns) = &namespace {
            Api::namespaced(self.client.clone(), ns)
        } else {
            Api::all(self.client.clone())
        };

        let lp = ListParams::default();
        let mut stream = kube::runtime::watcher(api, Config::default().any_semantic()).applied_objects();

        info!("Watching for ChaosOrchestration resources...");

        while let Some(event) = stream.next().await {
            match event {
                Ok(orchestration) => {
                    info!(
                        "Detected change: {}/{}",
                        orchestration.namespace().unwrap_or_else(|| "default".to_string()),
                        orchestration.name_any()
                    );
                }
                Err(e) => {
                    error!("Watch error: {:?}", e);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_creation() {
        // This test requires a Kubernetes cluster, so it's just a placeholder
        // In a real test environment, you would:
        // 1. Create a test Kubernetes cluster (e.g., using kind)
        // 2. Create a Client
        // 3. Instantiate the Controller
        // 4. Test reconciliation logic
    }
}
