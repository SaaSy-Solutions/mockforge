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

use crate::deployment::cloudflare::{self, CloudflareMocks};
use crate::deployment::fly_services::fly_services;
use crate::deployment::flyio::{FlyioCheck, FlyioClient, FlyioMachineConfig, FlyioRegistryAuth};
use crate::deployment::machine_config::hosted_mock_runtime_env;
use crate::models::{DeploymentLog, DeploymentStatus, HostedMock, HostedMockPlugin};

/// Default OCI image for plugin-disabled hosted-mocks. Same as the
/// existing main mockforge image — preserves prior behavior.
const DEFAULT_BASE_IMAGE: &str = "ghcr.io/saasy-solutions/mockforge:latest";

/// Default OCI image for plugin-enabled hosted-mocks. Bundles
/// main mockforge + mockforge-plugin-host + mockforge-plugin-egress
/// per `Dockerfile.cloud-plugins`. Operators can override via
/// `MOCKFORGE_CLOUD_PLUGINS_IMAGE` env var.
const DEFAULT_CLOUD_PLUGINS_IMAGE: &str = "ghcr.io/saasy-solutions/mockforge-cloud-plugins:latest";

/// Pick the right OCI image for a deployment based on whether
/// any plugins are attached. Reads:
///
/// - `MOCKFORGE_CLOUD_PLUGINS_IMAGE` — overrides the default
///   plugin-enabled image
/// - `MOCKFORGE_DOCKER_IMAGE` — overrides the default base image
///
/// Pairs with [`crate::deployment::flyio::FlyioGuest::for_hosted_mock`]
/// so the orchestrator picks both the right image AND the right
/// machine size for the deployment's tier.
async fn resolve_image_for_deployment(pool: &PgPool, deployment_id: Uuid) -> String {
    let plugins_attached = match HostedMockPlugin::count_active_by_deployment(pool, deployment_id)
        .await
    {
        Ok(count) => count > 0,
        Err(err) => {
            // A query failure shouldn't block deploy. Fall back to
            // the base image — the deploy proceeds without plugin
            // support, and the next attach call surfaces the
            // failure on the control-plane API.
            warn!(
                error = %err,
                deployment_id = %deployment_id,
                "could not count attached plugins for image selection; falling back to base image"
            );
            false
        }
    };

    if plugins_attached {
        std::env::var("MOCKFORGE_CLOUD_PLUGINS_IMAGE")
            .unwrap_or_else(|_| DEFAULT_CLOUD_PLUGINS_IMAGE.to_string())
    } else {
        std::env::var("MOCKFORGE_DOCKER_IMAGE").unwrap_or_else(|_| DEFAULT_BASE_IMAGE.to_string())
    }
}

/// Deployment orchestrator that manages hosted mock deployments
pub struct DeploymentOrchestrator {
    db: Arc<PgPool>,
    flyio_client: Option<FlyioClient>,
    flyio_org_slug: Option<String>,
    storage: Option<crate::storage::PluginStorage>,
    cloudflare: Option<CloudflareMocks>,
}

impl DeploymentOrchestrator {
    pub fn new(
        db: Arc<PgPool>,
        flyio_token: Option<String>,
        flyio_org_slug: Option<String>,
    ) -> Self {
        let flyio_client = flyio_token.map(FlyioClient::new);

        Self {
            db,
            flyio_client,
            flyio_org_slug: flyio_org_slug.or_else(|| std::env::var("FLYIO_ORG_SLUG").ok()),
            storage: None,
            cloudflare: CloudflareMocks::from_env(),
        }
    }

    /// Orchestrator configured from the environment: Fly from
    /// `FLYIO_API_TOKEN`/`FLYIO_ORG_SLUG`, Cloudflare Containers from
    /// `MOCKFORGE_CF_MOCKS_URL`/`MOCKFORGE_CF_MOCKS_EDGE_TOKEN`.
    pub fn from_env(db: Arc<PgPool>) -> Self {
        Self::new(db, std::env::var("FLYIO_API_TOKEN").ok(), std::env::var("FLYIO_ORG_SLUG").ok())
    }

    /// The Cloudflare backend, when configured and the deployment qualifies.
    async fn cloudflare_for(&self, deployment: &HostedMock) -> Option<&CloudflareMocks> {
        let cf = self.cloudflare.as_ref()?;
        let plugins = HostedMockPlugin::count_active_by_deployment(self.db.as_ref(), deployment.id)
            .await
            .map(|n| n > 0)
            // Plugins need the cloud-plugins image; if we cannot tell, use Fly.
            .unwrap_or(true);
        cloudflare::runs_on_cloudflare(deployment, plugins).then_some(cf)
    }

    /// Use tenant-scoped upload storage for local OpenAPI mocks.
    pub fn with_storage(mut self, storage: crate::storage::PluginStorage) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Start the orchestrator background task
    pub fn start(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            // Slow provider cleanup must not block new deployment polling.
            // Both loops belong to this task, so aborting it cancels both
            // futures instead of leaving an orphan cleanup task running.
            let cleanup = async {
                let mut interval = interval(Duration::from_secs(10));
                loop {
                    interval.tick().await;
                    if let Err(e) = self.process_requested_deletions().await {
                        error!("Error reconciling requested deletions: {}", e);
                    }
                }
            };
            let provisioning = async {
                let mut interval = interval(Duration::from_secs(10));
                loop {
                    interval.tick().await;
                    if let Err(e) = self.process_pending_deployments().await {
                        error!("Error processing pending deployments: {}", e);
                    }
                }
            };
            tokio::join!(cleanup, provisioning);
        })
    }

    /// Retry only durable, authorized deletion intents, including legacy
    /// soft-deleted rows whose resources were never confirmed absent.
    async fn process_requested_deletions(&self) -> Result<()> {
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM hosted_mocks WHERE (capacity_reserved OR deleted_at IS NULL) AND deletion_requested_at IS NOT NULL ORDER BY updated_at ASC LIMIT 10",
        )
        .fetch_all(self.db.as_ref())
        .await?;
        for id in ids {
            if let Err(error) = self.cleanup_deployment(id, false).await {
                warn!("Cleanup remains unconfirmed for {}: {}", id, error);
            }
        }
        Ok(())
    }

    /// Process pending deployments
    async fn process_pending_deployments(&self) -> Result<()> {
        let pool = self.db.as_ref();

        // Find new deployments (pending/deploying that haven't been deployed yet).
        // Exclude deployments that already have a deployment_url — those are
        // redeployments handled directly by the redeploy handler.
        let deployments = sqlx::query_as::<_, HostedMock>(
            r#"
            SELECT * FROM hosted_mocks
            WHERE status IN ('pending', 'deploying')
            AND deployment_url IS NULL
            AND deleted_at IS NULL
            AND deletion_requested_at IS NULL
            ORDER BY created_at ASC
            LIMIT 10
            "#,
        )
        .fetch_all(pool)
        .await
        .context("Failed to fetch pending deployments")?;

        for deployment in deployments {
            if let Err(e) = self.deploy(&deployment).await {
                // Log the full error chain for debugging
                let error_msg = format!("{:#}", e);
                error!("Failed to deploy {}: {}", deployment.id, error_msg);

                // Update status to failed
                let _ = HostedMock::update_status(
                    pool,
                    deployment.id,
                    DeploymentStatus::Failed,
                    Some(&format!("Deployment failed: {}", error_msg)),
                )
                .await;

                // Log error
                let _ = DeploymentLog::create(
                    pool,
                    deployment.id,
                    "error",
                    &format!("Deployment failed: {}", error_msg),
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

        let mut lifecycle = match crate::handlers::hosted_capacity::LifecycleGuard::acquire(
            pool,
            deployment.org_id,
            deployment.id,
        )
        .await
        {
            Ok(guard) => guard,
            Err(crate::error::ApiError::Conflict(_)) => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        let Some(current) = lifecycle.find().await? else {
            return Ok(());
        };
        if lifecycle.deletion_requested().await? {
            return Ok(());
        }
        if !matches!(current.status(), DeploymentStatus::Pending | DeploymentStatus::Deploying)
            || current.deployment_url.is_some()
        {
            return Ok(());
        }
        let deployment = &current;

        // Update status to deploying
        HostedMock::update_status(pool, deployment.id, DeploymentStatus::Deploying, None)
            .await
            .context("Failed to update deployment status")?;

        DeploymentLog::create(pool, deployment.id, "info", "Starting deployment", None)
            .await
            .context("Failed to create deployment log")?;

        // Record the selected backend before the first provider request. A
        // missing machine ID after a timeout must not imply a local deployment.
        let org = crate::models::Organization::find_by_id(pool, deployment.org_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Organization not found"))?;
        let subscription =
            crate::models::Subscription::find_by_org(pool, deployment.org_id).await?;
        let plan = crate::handlers::entitlements::resolve_effective_plan(
            org.plan(),
            subscription.as_ref(),
            chrono::Utc::now(),
        );
        // Free mocks have no externally reachable backend that bypasses the
        // registry's hard request cap. Paid isolated hosting is unchanged.
        // Free deployments never reach a configured provider.
        let cf = if plan == crate::models::Plan::Free {
            None
        } else {
            self.cloudflare_for(deployment).await
        };
        let fly = self.flyio_client.as_ref().filter(|_| plan != crate::models::Plan::Free);
        let backend = if cf.is_some() {
            "cloudflare"
        } else {
            fly.map_or("multitenant", |_| "flyio")
        };
        sqlx::query("UPDATE hosted_mocks SET metadata_json = jsonb_set(COALESCE(metadata_json, '{}'::jsonb), '{hosting_backend}', to_jsonb($2::text)) WHERE id = $1 AND org_id = $3")
            .bind(deployment.id).bind(backend).bind(deployment.org_id).execute(pool).await?;
        if let Some(cf) = cf {
            self.deploy_to_cloudflare(cf, deployment).await?;
        } else if let Some(fly) = fly {
            self.deploy_to_flyio(fly, deployment).await?;
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

    /// Redeploy on the backend the deployment already uses. Fly deployments
    /// stay on Fly; a Cloudflare deployment that gained a raw-TCP protocol
    /// moves to Fly and its container is stopped.
    pub(crate) async fn redeploy_on_backend(&self, deployment: &HostedMock) -> Result<()> {
        if super::local_runtime::is_local(deployment) {
            return self.deploy_to_multitenant_router(deployment).await;
        }
        if !cloudflare::is_cloudflare(deployment) {
            return match self.flyio_client {
                Some(ref flyio_client) => self.redeploy_to_flyio(flyio_client, deployment).await,
                None => self.deploy_to_multitenant_router(deployment).await,
            };
        }
        if let Some(cf) = self.cloudflare_for(deployment).await {
            return self.deploy_to_cloudflare(cf, deployment).await;
        }
        let flyio_client = self.flyio_client.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "deployment needs a protocol Cloudflare cannot serve and Fly is not configured"
            )
        })?;
        self.deploy_to_flyio(flyio_client, deployment).await?;
        if let Some(cf) = self.cloudflare.as_ref() {
            if let Err(e) = cf.destroy(deployment).await {
                warn!("Failed to stop Cloudflare container for {}: {:#}", deployment.id, e);
            }
        }
        Ok(())
    }

    /// Serve a deployment from Cloudflare Containers. Nothing is provisioned up
    /// front: the Worker starts the container on the first request with the
    /// environment from [`hosted_mock_runtime_env`]. Restarting here makes a
    /// redeploy take effect on the next request.
    async fn deploy_to_cloudflare(
        &self,
        cf: &CloudflareMocks,
        deployment: &HostedMock,
    ) -> Result<()> {
        let pool = self.db.as_ref();
        cf.restart(deployment).await.context("Failed to reset Cloudflare container")?;

        let deployment_url = cf.public_url(deployment);
        sqlx::query(
            r#"
            UPDATE hosted_mocks
            SET deployment_url = $1,
                internal_url = NULL,
                health_check_url = NULL,
                metadata_json = COALESCE(metadata_json, '{}'::jsonb) || jsonb_build_object($2::text, $3::text),
                updated_at = NOW()
            WHERE id = $4
            "#,
        )
        .bind(&deployment_url)
        .bind(cloudflare::BACKEND_KEY)
        .bind(cloudflare::BACKEND_CLOUDFLARE)
        .bind(deployment.id)
        .execute(pool)
        .await
        .context("Failed to record Cloudflare deployment")?;

        DeploymentLog::create(
            pool,
            deployment.id,
            "info",
            &format!("Serving from Cloudflare Containers at {}", deployment_url),
            None,
        )
        .await?;
        Ok(())
    }

    /// Deploy to Fly.io
    async fn deploy_to_flyio(&self, client: &FlyioClient, deployment: &HostedMock) -> Result<()> {
        let pool = self.db.as_ref();
        let org_slug = self
            .flyio_org_slug
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("FLYIO_ORG_SLUG not configured"))?;

        // Generate app name (must be unique globally on Fly.io). Delegated to
        // `HostedMock::fly_app_name()` so other subsystems (fly_metrics) stay
        // in sync with whatever the orchestrator actually names the app.
        let app_name = deployment.fly_app_name();

        DeploymentLog::create(
            pool,
            deployment.id,
            "info",
            &format!("Creating Fly.io app: {}", app_name),
            None,
        )
        .await?;

        // Create or get app
        let is_new_app;
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
                is_new_app = false;
                app
            }
            Err(_) => {
                let app = client
                    .create_app(&app_name, org_slug)
                    .await
                    .context("Failed to create Fly.io app")?;
                is_new_app = true;
                app
            }
        };

        // Allocate public IPs for new apps so they're accessible via DNS
        if is_new_app {
            client.allocate_ips(&app_name).await.context("Failed to allocate public IPs")?;

            DeploymentLog::create(pool, deployment.id, "info", "Allocated public IPs", None)
                .await?;
        }

        // Kafka clients follow the broker's advertised listener, so it must be
        // the public name of this app rather than its internal Fly address.
        let kafka_host = match std::env::var("MOCKFORGE_MOCKS_DOMAIN") {
            Ok(domain) => format!("{}.{}", deployment.slug, domain),
            Err(_) => format!("{}.fly.dev", app_name),
        };
        let env = hosted_mock_runtime_env(deployment, Some(kafka_host))?;

        // Use MockForge Docker image
        let image = resolve_image_for_deployment(pool, deployment.id).await;

        let services = fly_services(deployment);

        let mut checks = HashMap::new();
        checks.insert(
            "alive".to_string(),
            FlyioCheck {
                check_type: "http".to_string(),
                port: 3000,
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
            // None = accept Fly's API default (shared-cpu-1x:256MB)
            // for legacy hosted-mocks. Cloud-plugin-enabled deploys
            // will set this via FlyioGuest::for_hosted_mock when the
            // plugin runtime ships in Phase 2.
            guest: None,
        };

        DeploymentLog::create(pool, deployment.id, "info", "Creating Fly.io machine", None).await?;

        // Build registry auth if configured (for private Docker images)
        let registry_auth = if let (Ok(server), Ok(username), Ok(password)) = (
            std::env::var("DOCKER_REGISTRY_SERVER"),
            std::env::var("DOCKER_REGISTRY_USERNAME"),
            std::env::var("DOCKER_REGISTRY_PASSWORD"),
        ) {
            Some(FlyioRegistryAuth {
                server,
                username,
                password,
            })
        } else if machine_config.image.starts_with("registry.fly.io/") {
            // Auto-detect Fly.io registry images and use the Fly.io API token
            Some(FlyioRegistryAuth {
                server: "registry.fly.io".to_string(),
                username: "x".to_string(),
                password: client.api_token().to_string(),
            })
        } else if machine_config.image.starts_with("ghcr.io/") {
            // Auto-detect GitHub Container Registry images
            if let Ok(token) = std::env::var("GHCR_TOKEN") {
                Some(FlyioRegistryAuth {
                    server: "ghcr.io".to_string(),
                    username: std::env::var("GHCR_USERNAME")
                        .unwrap_or_else(|_| "mockforge".to_string()),
                    password: token,
                })
            } else {
                tracing::warn!(
                    "GHCR image '{}' requires GHCR_TOKEN env var for authentication",
                    machine_config.image
                );
                None
            }
        } else {
            None
        };

        // Create machine
        let machine = client
            .create_machine(&app_name, machine_config, &deployment.region, registry_auth)
            .await
            .context("Failed to create Fly.io machine")?;

        // If MOCKFORGE_MOCKS_DOMAIN is set, use <slug>.<domain> as the public URL.
        // A wildcard TLS cert on the registry app covers all subdomains, so no
        // per-deployment certificate management is needed.
        let deployment_url = if let Ok(domain) = std::env::var("MOCKFORGE_MOCKS_DOMAIN") {
            format!("https://{}.{}", deployment.slug, domain)
        } else {
            format!("https://{}.fly.dev", app_name)
        };
        // Use the public fly.dev URL for proxying — .internal DNS doesn't
        // reliably resolve cross-app in all Fly.io configurations.
        let internal_url = format!("https://{}.fly.dev", app_name);
        let health_check_url = format!("https://{}.fly.dev/health/live", app_name);

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

    /// Get a reference to the database pool
    pub fn db(&self) -> &PgPool {
        &self.db
    }

    /// Redeploy an existing deployment with updated image/config
    pub async fn redeploy(&self, deployment: &HostedMock) -> Result<()> {
        info!("Redeploying mock service: {} ({})", deployment.name, deployment.id);

        let pool = self.db.as_ref();

        let mut lifecycle = crate::handlers::hosted_capacity::LifecycleGuard::acquire(
            pool,
            deployment.org_id,
            deployment.id,
        )
        .await?;
        let current =
            lifecycle.find().await?.ok_or_else(|| anyhow::anyhow!("Deployment not found"))?;
        lifecycle.ensure_not_deleting().await?;
        let deployment = &current;

        HostedMock::update_status(pool, deployment.id, DeploymentStatus::Deploying, None)
            .await
            .context("Failed to update deployment status")?;

        DeploymentLog::create(pool, deployment.id, "info", "Starting redeployment", None)
            .await
            .context("Failed to create deployment log")?;

        self.redeploy_on_backend(deployment).await?;

        HostedMock::update_status(pool, deployment.id, DeploymentStatus::Active, None)
            .await
            .context("Failed to update deployment status")?;

        DeploymentLog::create(
            pool,
            deployment.id,
            "info",
            "Redeployment completed successfully",
            None,
        )
        .await
        .context("Failed to create deployment log")?;

        info!("Successfully redeployed mock service: {}", deployment.id);

        Ok(())
    }

    /// Redeploy to Fly.io by updating the machine config
    async fn redeploy_to_flyio(&self, client: &FlyioClient, deployment: &HostedMock) -> Result<()> {
        let pool = self.db.as_ref();

        // Extract machine ID from metadata
        let machine_id = deployment
            .metadata_json
            .get("flyio_machine_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("No Fly.io machine ID found in deployment metadata"))?;

        let app_name = deployment.fly_app_name();

        DeploymentLog::create(
            pool,
            deployment.id,
            "info",
            &format!("Updating Fly.io machine {} in app {}", machine_id, app_name),
            None,
        )
        .await?;

        // Same environment as the first deploy. This used to be a shorter copy
        // that dropped protocol, ingest, tracing and upstream settings.
        let kafka_host = match std::env::var("MOCKFORGE_MOCKS_DOMAIN") {
            Ok(domain) => format!("{}.{}", deployment.slug, domain),
            Err(_) => format!("{}.fly.dev", app_name),
        };
        let env = hosted_mock_runtime_env(deployment, Some(kafka_host))?;

        let image = resolve_image_for_deployment(pool, deployment.id).await;

        let services = fly_services(deployment);

        let mut checks = HashMap::new();
        checks.insert(
            "alive".to_string(),
            FlyioCheck {
                check_type: "http".to_string(),
                port: 3000,
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
            // None = accept Fly's API default (shared-cpu-1x:256MB)
            // for legacy hosted-mocks. Cloud-plugin-enabled deploys
            // will set this via FlyioGuest::for_hosted_mock when the
            // plugin runtime ships in Phase 2.
            guest: None,
        };

        // Build registry auth
        let registry_auth = if let (Ok(server), Ok(username), Ok(password)) = (
            std::env::var("DOCKER_REGISTRY_SERVER"),
            std::env::var("DOCKER_REGISTRY_USERNAME"),
            std::env::var("DOCKER_REGISTRY_PASSWORD"),
        ) {
            Some(FlyioRegistryAuth {
                server,
                username,
                password,
            })
        } else if machine_config.image.starts_with("registry.fly.io/") {
            Some(FlyioRegistryAuth {
                server: "registry.fly.io".to_string(),
                username: "x".to_string(),
                password: client.api_token().to_string(),
            })
        } else if machine_config.image.starts_with("ghcr.io/") {
            if let Ok(token) = std::env::var("GHCR_TOKEN") {
                Some(FlyioRegistryAuth {
                    server: "ghcr.io".to_string(),
                    username: std::env::var("GHCR_USERNAME")
                        .unwrap_or_else(|_| "mockforge".to_string()),
                    password: token,
                })
            } else {
                tracing::warn!(
                    "GHCR image '{}' requires GHCR_TOKEN env var for authentication",
                    machine_config.image
                );
                None
            }
        } else {
            None
        };

        client
            .update_machine(&app_name, machine_id, machine_config, registry_auth)
            .await
            .context("Failed to update Fly.io machine")?;

        DeploymentLog::create(pool, deployment.id, "info", "Machine updated and restarting", None)
            .await?;

        Ok(())
    }

    /// Deploy to multitenant router (single process routing)
    async fn deploy_to_multitenant_router(&self, deployment: &HostedMock) -> Result<()> {
        let pool = self.db.as_ref();

        let refreshed = if deployment.openapi_spec_url.is_some() {
            let storage = self
                .storage
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Uploaded specification storage is unavailable"))?;
            super::spec_source::refresh(pool, storage, deployment).await?
        } else {
            deployment.clone()
        };
        let deployment = &refreshed;

        // Invalid configuration cannot be reported as an active deployment.
        let _router = super::local_runtime::prepare(deployment).map_err(anyhow::Error::msg)?;

        // For multitenant router, we just need to set the deployment URL
        // The router will handle routing based on org/project/env
        let base_url = std::env::var("MOCKFORGE_BASE_URL")
            .unwrap_or_else(|_| "https://registry.mockforge.dev".to_string());
        let base_url = base_url.trim_end_matches('/');

        let deployment_url =
            format!("{}/mocks/{}/{}", base_url, deployment.org_id, deployment.slug);
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
        self.cleanup_deployment(deployment_id, true).await
    }

    async fn cleanup_deployment(&self, deployment_id: Uuid, request_new: bool) -> Result<()> {
        let pool = self.db.as_ref();

        let deployment =
            sqlx::query_as::<_, HostedMock>("SELECT * FROM hosted_mocks WHERE id = $1")
                .bind(deployment_id)
                .fetch_optional(pool)
                .await
                .context("Failed to find deployment")?
                .ok_or_else(|| anyhow::anyhow!("Deployment not found"))?;

        let mut lifecycle = crate::handlers::hosted_capacity::LifecycleGuard::acquire(
            pool,
            deployment.org_id,
            deployment.id,
        )
        .await?;
        let deployment = lifecycle
            .find_for_cleanup()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Deployment not found"))?;

        if !request_new && !lifecycle.deletion_requested().await? {
            return Ok(());
        }
        if deployment.deleted_at.is_some() {
            let reserved: bool = sqlx::query_scalar(
                "SELECT capacity_reserved FROM hosted_mocks WHERE id = $1 AND org_id = $2",
            )
            .bind(deployment_id)
            .bind(deployment.org_id)
            .fetch_one(pool)
            .await?;
            if !reserved {
                return Ok(());
            }
        }
        // This also rotates unsuccessful attempts behind other queued intents.
        lifecycle.request_deletion().await?;

        DeploymentLog::create(pool, deployment_id, "info", "Starting deletion", None).await?;

        let local =
            deployment.metadata_json.get("hosting_backend").and_then(|value| value.as_str())
                == Some("multitenant");
        if cloudflare::is_cloudflare(&deployment) {
            let cf = self.cloudflare.as_ref().ok_or_else(|| {
                anyhow::anyhow!("Cloudflare is not configured; cannot confirm cleanup")
            })?;
            cf.destroy(&deployment).await?;
        } else if !local {
            let flyio_client = self.flyio_client.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "Hosted deployment provider is not configured; cannot confirm cleanup"
                )
            })?;
            let app_name = format!(
                "mockforge-{}-{}",
                &deployment.org_id.simple().to_string()[..8],
                deployment.slug
            );
            flyio_client.delete_app_confirmed(&app_name).await?;
        }

        lifecycle.complete_deletion().await?;

        DeploymentLog::create(pool, deployment_id, "info", "Deletion completed", None).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deployment::cloudflare::tests::deployment;
    use crate::deployment::machine_config::{apply_sentry_env, hosted_mock_cors_env};

    #[test]
    fn every_fly_service_stops_when_idle_and_starts_on_demand() {
        let services = fly_services(&deployment(&["http", "grpc", "kafka"], serde_json::json!({})));
        assert_eq!(services.len(), 3, "HTTP plus one service per TCP protocol");
        for service in &services {
            assert_eq!(service.autostop.as_deref(), Some("stop"));
            assert_eq!(service.autostart, Some(true));
            assert_eq!(service.min_machines_running, Some(0));
        }
        let json = serde_json::to_value(&services[0]).unwrap();
        assert_eq!(json["autostop"], "stop");
        assert_eq!(json["autostart"], true);
    }

    #[test]
    fn runtime_env_carries_protocols_and_kafka_host_for_every_deploy_path() {
        let d = deployment(&["http", "kafka"], serde_json::json!({}));
        let env = hosted_mock_runtime_env(&d, Some("shop.example".into())).unwrap();
        assert_eq!(env["MOCKFORGE_DEPLOYMENT_ID"], d.id.to_string());
        assert_eq!(env["PORT"], "3000");
        assert_eq!(env["MOCKFORGE_KAFKA_ADVERTISED_HOST"], "shop.example");
        for protocol in d.enabled_protocols() {
            if let Some((key, value)) = protocol.enable_env() {
                assert_eq!(env.get(key), Some(&value), "{key}");
            }
        }
        let without_host = hosted_mock_runtime_env(&d, None).unwrap();
        assert!(!without_host.contains_key("MOCKFORGE_KAFKA_ADVERTISED_HOST"));
    }

    #[test]
    fn hosted_mocks_configure_cors_origins_explicitly() {
        let env: HashMap<_, _> = hosted_mock_cors_env().into_iter().collect();
        assert_eq!(env.get("MOCKFORGE_CORS_ALLOWED_ORIGINS").map(String::as_str), Some("*"));
    }

    #[test]
    fn sentry_env_forwarding_follows_dsn() {
        // One test owns the process-global SENTRY_* vars end to end; cargo
        // runs test threads in parallel, so a second test here would race.
        std::env::set_var("SENTRY_DSN", "https://k@example.ingest.sentry.io/1");
        let mut env = HashMap::new();
        apply_sentry_env(&mut env);
        assert_eq!(
            env.get("SENTRY_DSN").map(String::as_str),
            Some("https://k@example.ingest.sentry.io/1")
        );

        std::env::remove_var("SENTRY_DSN");
        let mut env = HashMap::new();
        apply_sentry_env(&mut env);
        assert!(!env.contains_key("SENTRY_DSN"));
    }
}

#[cfg(test)]
#[path = "orchestrator_cleanup_tests.rs"]
mod cleanup_reconciliation_tests;

#[cfg(test)]
#[path = "orchestrator_free_boundary_tests.rs"]
mod free_provider_boundary_tests;
