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
    FlyioCheck, FlyioClient, FlyioMachineConfig, FlyioPort, FlyioRegistryAuth, FlyioService,
};
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

        // Find new deployments (pending/deploying that haven't been deployed yet).
        // Exclude deployments that already have a deployment_url — those are
        // redeployments handled directly by the redeploy handler.
        let deployments = sqlx::query_as::<_, HostedMock>(
            r#"
            SELECT * FROM hosted_mocks
            WHERE status IN ('pending', 'deploying')
            AND deployment_url IS NULL
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

        // Build machine config
        let mut env = HashMap::new();
        env.insert("MOCKFORGE_DEPLOYMENT_ID".to_string(), deployment.id.to_string());
        env.insert("MOCKFORGE_ORG_ID".to_string(), deployment.org_id.to_string());
        env.insert("MOCKFORGE_CONFIG".to_string(), serde_json::to_string(&deployment.config_json)?);
        env.insert("PORT".to_string(), "3000".to_string());

        if let Some(ref spec_url) = deployment.openapi_spec_url {
            env.insert("MOCKFORGE_OPENAPI_SPEC_URL".to_string(), spec_url.clone());
        }

        // Inject env vars + extra Fly services for any protocols the
        // deployment opted into beyond HTTP. HTTP/WS/GraphQL all share port
        // 3000 (router merge happens in mockforge-cli/src/serve.rs) so they
        // never produce a Fly service entry; gRPC and the broker family do.
        let enabled_protocols = deployment.enabled_protocols();
        for protocol in &enabled_protocols {
            if let Some((key, value)) = protocol.enable_env() {
                env.insert(key.to_string(), value);
            }
        }

        // Kafka clients respect the broker's advertised listener over the
        // bootstrap address — without this env var they'd loop on the
        // internal Fly address and fail. The mockforge-kafka broker must
        // honor `config.kafka.advertised_host` for this to be functional
        // end-to-end (#231 broker-side todo).
        if enabled_protocols.contains(&crate::models::Protocol::Kafka) {
            let advertised_host = if let Ok(domain) = std::env::var("MOCKFORGE_MOCKS_DOMAIN") {
                format!("{}.{}", deployment.slug, domain)
            } else {
                format!("{}.fly.dev", app_name)
            };
            env.insert("MOCKFORGE_KAFKA_ADVERTISED_HOST".to_string(), advertised_host);
            env.insert("MOCKFORGE_KAFKA_ADVERTISED_PORT".to_string(), "9092".to_string());
        }

        // Wire the in-container request log shipper (#232) and the recorder
        // capture cloud-sync (#234 part 2) to forward to MockForge Cloud.
        // We only set the env vars when:
        //   * a JWT secret is available (production deploys always have one)
        //   * a public ingest base URL is configured via MOCKFORGE_LOG_INGEST_BASE_URL
        // Both shippers reuse the same deployment-scoped JWT — they're
        // ingest-only, so a single token with the deployment subject
        // suffices for both. When the base URL is absent the shippers
        // auto-disable and the deployment runs without cloud forwarding.
        if let Ok(jwt_secret) = std::env::var("JWT_SECRET") {
            if let Ok(ingest_base) = std::env::var("MOCKFORGE_LOG_INGEST_BASE_URL") {
                let trimmed_base = ingest_base.trim_end_matches('/');
                let token = mockforge_registry_core::auth::create_deployment_ingest_token(
                    deployment.id,
                    &jwt_secret,
                    30, // 30 days; rotated on every redeploy
                )
                .ok();
                if let Some(token) = token {
                    env.insert(
                        "MOCKFORGE_LOG_INGEST_URL".to_string(),
                        format!(
                            "{}/api/v1/hosted-mocks/{}/log-ingest",
                            trimmed_base, deployment.id
                        ),
                    );
                    env.insert("MOCKFORGE_LOG_INGEST_TOKEN".to_string(), token.clone());
                    env.insert(
                        "MOCKFORGE_CAPTURE_INGEST_URL".to_string(),
                        format!(
                            "{}/api/v1/hosted-mocks/{}/captures/ingest",
                            trimmed_base, deployment.id
                        ),
                    );
                    env.insert("MOCKFORGE_CAPTURE_INGEST_TOKEN".to_string(), token);
                }
            }
        }

        // OTLP tracing export (#233). When the registry server has a
        // collector reachable at `MOCKFORGE_OTLP_INGEST_ENDPOINT` we tell
        // the deployment to ship spans there. Identifying labels (deployment
        // id, org id) are already set above as MOCKFORGE_DEPLOYMENT_ID /
        // _ORG_ID; the receiver uses those for multi-tenant routing.
        if let Ok(otlp_endpoint) = std::env::var("MOCKFORGE_OTLP_INGEST_ENDPOINT") {
            if !otlp_endpoint.trim().is_empty() {
                env.insert("MOCKFORGE_OTLP_ENDPOINT".to_string(), otlp_endpoint);
                // Deployment-scoped service name keeps spans separable when
                // multiple hosted mocks share a backend.
                env.insert(
                    "MOCKFORGE_OTLP_SERVICE_NAME".to_string(),
                    format!("hosted-mock/{}", deployment.slug),
                );
            }
        }

        // Reality-driven proxy upstream (#222). When set, the deployed
        // mockforge-cli's `reality_proxy` middleware forwards a per-request
        // probabilistic share of traffic to this URL based on the
        // workspace's reality_continuum_ratio. Unset = always-mock,
        // current behaviour preserved.
        if let Some(upstream) = deployment.upstream_url() {
            env.insert("MOCKFORGE_PROXY_UPSTREAM".to_string(), upstream);
        }

        // Use MockForge Docker image
        let image = resolve_image_for_deployment(pool, deployment.id).await;

        // Always-on HTTP service (also serves WS upgrade and /graphql).
        let mut services = vec![FlyioService {
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

        // Per-protocol Fly services. Skip protocols already covered by the
        // HTTP listener above and skip duplicates; the orchestrator is the
        // one place we synthesize Fly machine configs, so we keep this
        // verbose for clarity.
        let mut bound_internal_ports: std::collections::HashSet<u16> =
            std::collections::HashSet::new();
        bound_internal_ports.insert(3000);
        for protocol in deployment.enabled_protocols() {
            let Some(internal) = protocol.internal_port() else {
                continue; // HTTP / WS / GraphQL — share 3000.
            };
            if !bound_internal_ports.insert(internal) {
                continue;
            }
            let public = protocol.public_port().unwrap_or(internal);
            let handlers: Vec<String> =
                protocol.fly_handlers().iter().map(|s| (*s).to_string()).collect();
            services.push(FlyioService {
                protocol: "tcp".to_string(),
                internal_port: internal,
                ports: vec![FlyioPort {
                    port: public,
                    handlers,
                }],
            });
        }

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

        HostedMock::update_status(pool, deployment.id, DeploymentStatus::Deploying, None)
            .await
            .context("Failed to update deployment status")?;

        DeploymentLog::create(pool, deployment.id, "info", "Starting redeployment", None)
            .await
            .context("Failed to create deployment log")?;

        if let Some(ref flyio_client) = self.flyio_client {
            self.redeploy_to_flyio(flyio_client, deployment).await?;
        } else {
            // For multitenant router, just restart
            self.deploy_to_multitenant_router(deployment).await?;
        }

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

        let app_name = format!(
            "mockforge-{}-{}",
            deployment
                .org_id
                .to_string()
                .replace('-', "")
                .chars()
                .take(8)
                .collect::<String>(),
            deployment.slug
        );

        DeploymentLog::create(
            pool,
            deployment.id,
            "info",
            &format!("Updating Fly.io machine {} in app {}", machine_id, app_name),
            None,
        )
        .await?;

        // Build updated machine config (same structure as deploy)
        let mut env = HashMap::new();
        env.insert("MOCKFORGE_DEPLOYMENT_ID".to_string(), deployment.id.to_string());
        env.insert("MOCKFORGE_ORG_ID".to_string(), deployment.org_id.to_string());
        env.insert("MOCKFORGE_CONFIG".to_string(), serde_json::to_string(&deployment.config_json)?);
        env.insert("PORT".to_string(), "3000".to_string());

        if let Some(ref spec_url) = deployment.openapi_spec_url {
            env.insert("MOCKFORGE_OPENAPI_SPEC_URL".to_string(), spec_url.clone());
        }

        let image = resolve_image_for_deployment(pool, deployment.id).await;

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

        // For multitenant router, we just need to set the deployment URL
        // The router will handle routing based on org/project/env
        let base_url = std::env::var("MOCKFORGE_BASE_URL")
            .unwrap_or_else(|_| "https://mocks.mockforge.dev".to_string());

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
                // Reconstruct app name from org_id + slug (same logic as deploy)
                let app_name = format!(
                    "mockforge-{}-{}",
                    deployment
                        .org_id
                        .to_string()
                        .replace('-', "")
                        .chars()
                        .take(8)
                        .collect::<String>(),
                    deployment.slug
                );

                if let Err(e) = flyio_client.delete_machine(&app_name, machine_id).await {
                    warn!("Failed to delete Fly.io machine: {}", e);
                }
                // Delete the Fly.io app after removing the machine
                if let Err(e) = flyio_client.delete_app(&app_name).await {
                    warn!("Failed to delete Fly.io app {}: {}", app_name, e);
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
