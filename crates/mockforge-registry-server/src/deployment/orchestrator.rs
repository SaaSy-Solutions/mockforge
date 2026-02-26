//! Deployment orchestrator service
//!
//! Listens for deployment requests and manages the lifecycle of hosted mock services

use anyhow::{Context, Result};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::deployment::flyio::{
    FlyioCheck, FlyioClient, FlyioMachineConfig, FlyioPort, FlyioService,
};
use crate::models::{DeploymentLog, DeploymentStatus, HostedMock};

/// Deployment orchestrator that manages hosted mock deployments
pub struct DeploymentOrchestrator {
    db: Arc<PgPool>,
    flyio_client: Option<FlyioClient>,
    flyio_org_slug: Option<String>,
}

impl DeploymentOrchestrator {
    pub fn new(
        db: Arc<PgPool>,
        flyio_token: Option<String>,
        flyio_org_slug: Option<String>,
    ) -> Self {
        let flyio_client = flyio_token.map(|token| FlyioClient::new(token));

        Self {
            db,
            flyio_client,
            flyio_org_slug: flyio_org_slug.or_else(|| std::env::var("FLYIO_ORG_SLUG").ok()),
        }
    }

    /// Start the orchestrator background task
    pub fn start(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10)); // Check every 10 seconds

            loop {
                interval.tick().await;

                if let Err(e) = self.process_pending_deployments().await {
                    error!("Error processing pending deployments: {}", e);
                }
            }
        })
    }

    /// Process pending deployments
    async fn process_pending_deployments(&self) -> Result<()> {
        let pool = self.db.as_ref();

        // Find all pending or deploying deployments
        let deployments = sqlx::query_as::<_, HostedMock>(
            r#"
            SELECT * FROM hosted_mocks
            WHERE status IN ('pending', 'deploying')
            AND deleted_at IS NULL
            ORDER BY created_at ASC
            LIMIT 10
            "#,
        )
        .fetch_all(pool)
        .await
        .context("Failed to fetch pending deployments")?;

        for deployment in deployments {
            if let Err(e) = self.deploy(&deployment).await {
                error!("Failed to deploy {}: {}", deployment.id, e);

                // Update status to failed
                let _ = HostedMock::update_status(
                    pool,
                    deployment.id,
                    DeploymentStatus::Failed,
                    Some(&format!("Deployment failed: {}", e)),
                )
                .await;

                // Log error
                let _ = DeploymentLog::create(
                    pool,
                    deployment.id,
                    "error",
                    &format!("Deployment failed: {}", e),
                    None,
                )
                .await;
            }
        }

        Ok(())
    }

    /// Deploy a mock service
    async fn deploy(&self, deployment: &HostedMock) -> Result<()> {
        info!("Deploying mock service: {} ({})", deployment.name, deployment.id);

        let pool = self.db.as_ref();

        // Update status to deploying
        HostedMock::update_status(pool, deployment.id, DeploymentStatus::Deploying, None)
            .await
            .context("Failed to update deployment status")?;

        DeploymentLog::create(pool, deployment.id, "info", "Starting deployment", None)
            .await
            .context("Failed to create deployment log")?;

        // Deploy using Fly.io if configured
        if let Some(ref flyio_client) = self.flyio_client {
            self.deploy_to_flyio(flyio_client, deployment).await?;
        } else {
            // Fallback: use multitenant router (single process routing)
            self.deploy_to_multitenant_router(deployment).await?;
        }

        // Update status to active
        HostedMock::update_status(pool, deployment.id, DeploymentStatus::Active, None)
            .await
            .context("Failed to update deployment status")?;

        DeploymentLog::create(
            pool,
            deployment.id,
            "info",
            "Deployment completed successfully",
            None,
        )
        .await
        .context("Failed to create deployment log")?;

        info!("Successfully deployed mock service: {}", deployment.id);

        Ok(())
    }

    /// Deploy to Fly.io
    async fn deploy_to_flyio(&self, client: &FlyioClient, deployment: &HostedMock) -> Result<()> {
        let pool = self.db.as_ref();
        let org_slug = self
            .flyio_org_slug
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("FLYIO_ORG_SLUG not configured"))?;

        // Generate app name (must be unique globally on Fly.io)
        let app_name = format!(
            "mockforge-{}-{}",
            deployment
                .org_id
                .to_string()
                .replace("-", "")
                .chars()
                .take(8)
                .collect::<String>(),
            deployment.slug
        );

        DeploymentLog::create(
            pool,
            deployment.id,
            "info",
            &format!("Creating Fly.io app: {}", app_name),
            None,
        )
        .await?;

        // Create or get app
        let _app = match client.get_app(&app_name).await {
            Ok(app) => {
                DeploymentLog::create(
                    pool,
                    deployment.id,
                    "info",
                    "Using existing Fly.io app",
                    None,
                )
                .await?;
                app
            }
            Err(_) => client
                .create_app(&app_name, org_slug)
                .await
                .context("Failed to create Fly.io app")?,
        };

        // Build machine config
        let mut env = HashMap::new();
        env.insert("MOCKFORGE_DEPLOYMENT_ID".to_string(), deployment.id.to_string());
        env.insert("MOCKFORGE_ORG_ID".to_string(), deployment.org_id.to_string());
        env.insert("MOCKFORGE_CONFIG".to_string(), serde_json::to_string(&deployment.config_json)?);

        if let Some(ref spec_url) = deployment.openapi_spec_url {
            env.insert("MOCKFORGE_OPENAPI_SPEC_URL".to_string(), spec_url.clone());
        }

        // Use MockForge Docker image
        let image = std::env::var("MOCKFORGE_DOCKER_IMAGE")
            .unwrap_or_else(|_| "ghcr.io/saasy-solutions/mockforge:latest".to_string());

        let services = vec![FlyioService {
            protocol: "tcp".to_string(),
            internal_port: 3000,
            ports: vec![
                FlyioPort {
                    port: 80,
                    handlers: vec!["http".to_string()],
                },
                FlyioPort {
                    port: 443,
                    handlers: vec!["tls".to_string(), "http".to_string()],
                },
            ],
        }];

        let mut checks = HashMap::new();
        checks.insert(
            "alive".to_string(),
            FlyioCheck {
                grace_period: "10s".to_string(),
                interval: "15s".to_string(),
                method: "GET".to_string(),
                timeout: "2s".to_string(),
                tls_skip_verify: false,
                path: Some("/health/live".to_string()),
            },
        );

        let machine_config = FlyioMachineConfig {
            image,
            env,
            services,
            checks: Some(checks),
        };

        DeploymentLog::create(pool, deployment.id, "info", "Creating Fly.io machine", None).await?;

        // Create machine
        let machine = client
            .create_machine(&app_name, machine_config, &deployment.region)
            .await
            .context("Failed to create Fly.io machine")?;

        // Update deployment with URLs
        let deployment_url = format!("https://{}.fly.dev", app_name);
        let internal_url = format!("http://{}.internal:3000", app_name);
        let health_check_url = format!("{}/health/live", deployment_url);

        sqlx::query(
            r#"
            UPDATE hosted_mocks
            SET
                deployment_url = $1,
                internal_url = $2,
                health_check_url = $3,
                metadata_json = jsonb_set(
                    COALESCE(metadata_json, '{}'::jsonb),
                    '{flyio_machine_id}',
                    to_jsonb($4::text)
                ),
                updated_at = NOW()
            WHERE id = $5
            "#,
        )
        .bind(&deployment_url)
        .bind(&internal_url)
        .bind(&health_check_url)
        .bind(&machine.id)
        .bind(deployment.id)
        .execute(pool)
        .await
        .context("Failed to update deployment URLs")?;

        DeploymentLog::create(
            pool,
            deployment.id,
            "info",
            &format!("Deployment URL: {}", deployment_url),
            None,
        )
        .await?;

        Ok(())
    }

    /// Deploy to multitenant router (single process routing)
    async fn deploy_to_multitenant_router(&self, deployment: &HostedMock) -> Result<()> {
        let pool = self.db.as_ref();

        // For multitenant router, we just need to set the deployment URL
        // The router will handle routing based on org/project/env
        let base_url = std::env::var("MOCKFORGE_BASE_URL")
            .unwrap_or_else(|_| "https://mocks.mockforge.dev".to_string());

        let deployment_url = format!("{}/{}/{}", base_url, deployment.org_id, deployment.slug);
        let health_check_url = format!("{}/health/live", deployment_url);

        sqlx::query(
            r#"
            UPDATE hosted_mocks
            SET
                deployment_url = $1,
                health_check_url = $2,
                updated_at = NOW()
            WHERE id = $3
            "#,
        )
        .bind(&deployment_url)
        .bind(&health_check_url)
        .bind(deployment.id)
        .execute(pool)
        .await
        .context("Failed to update deployment URLs")?;

        DeploymentLog::create(
            pool,
            deployment.id,
            "info",
            &format!("Deployed to multitenant router: {}", deployment_url),
            None,
        )
        .await?;

        Ok(())
    }

    /// Delete a deployment
    pub async fn delete_deployment(&self, deployment_id: Uuid) -> Result<()> {
        let pool = self.db.as_ref();

        let deployment = HostedMock::find_by_id(pool, deployment_id)
            .await
            .context("Failed to find deployment")?
            .ok_or_else(|| anyhow::anyhow!("Deployment not found"))?;

        // Update status to deleting
        HostedMock::update_status(pool, deployment_id, DeploymentStatus::Deleting, None).await?;

        DeploymentLog::create(pool, deployment_id, "info", "Starting deletion", None).await?;

        // Delete from Fly.io if deployed there
        if let Some(ref flyio_client) = self.flyio_client {
            if let Some(machine_id) =
                deployment.metadata_json.get("flyio_machine_id").and_then(|v| v.as_str())
            {
                // Extract app name from deployment URL
                if let Some(ref deployment_url) = deployment.deployment_url {
                    if let Some(app_name) = deployment_url.strip_suffix(".fly.dev") {
                        if let Err(e) = flyio_client.delete_machine(app_name, machine_id).await {
                            warn!("Failed to delete Fly.io machine: {}", e);
                        }
                    }
                }
            }
        }

        // Soft delete in database
        sqlx::query(
            r#"
            UPDATE hosted_mocks
            SET deleted_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(deployment_id)
        .execute(pool)
        .await?;

        DeploymentLog::create(pool, deployment_id, "info", "Deletion completed", None).await?;

        Ok(())
    }
}
