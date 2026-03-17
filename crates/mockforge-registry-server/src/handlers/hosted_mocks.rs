//! Hosted Mocks deployment handlers
//!
//! Provides endpoints for deploying, managing, and monitoring cloud-hosted mock services

use axum::{
    extract::{Multipart, Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    deployment::flyio::FlyioClient,
    error::{ApiError, ApiResult},
    middleware::{
        permission_check::PermissionChecker, permissions::Permission, resolve_org_context, AuthUser,
    },
    models::{
        feature_usage::{FeatureType, FeatureUsage},
        record_audit_event, AuditEventType, DeploymentLog, DeploymentMetrics, DeploymentStatus,
        HostedMock, Organization, User,
    },
    AppState,
};
use tracing::warn;

/// Create a new hosted mock deployment
pub async fn create_deployment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<CreateDeploymentRequest>,
) -> ApiResult<Json<DeploymentResponse>> {
    let pool = state.db.pool();

    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Check permission
    let checker = PermissionChecker::new(&state);
    checker
        .require_permission(user_id, org_ctx.org_id, Permission::HostedMockCreate)
        .await?;

    // Check plan limits
    let limits = &org_ctx.org.limits_json;
    let max_hosted_mocks = limits.get("max_hosted_mocks").and_then(|v| v.as_i64()).unwrap_or(0);

    if max_hosted_mocks >= 0 {
        // Count existing active deployments
        let existing = HostedMock::find_by_org(pool, org_ctx.org_id)
            .await
            .map_err(ApiError::Database)?;

        let active_count = existing
            .iter()
            .filter(|m| {
                matches!(m.status(), DeploymentStatus::Active | DeploymentStatus::Deploying)
            })
            .count();

        if active_count as i64 >= max_hosted_mocks {
            return Err(ApiError::InvalidRequest(format!(
                "Hosted mocks limit exceeded. Your plan allows {} active deployments. Upgrade to deploy more.",
                max_hosted_mocks
            )));
        }
    }

    // Validate slug format
    let generated_slug;
    let slug = match request.slug.as_deref() {
        Some(s) => s,
        None => {
            generated_slug = request
                .name
                .to_lowercase()
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '-' {
                        c
                    } else {
                        '-'
                    }
                })
                .collect::<String>()
                .trim_matches('-')
                .replace("--", "-");
            &generated_slug
        }
    };

    if !slug.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err(ApiError::InvalidRequest(
            "Slug must contain only alphanumeric characters and hyphens".to_string(),
        ));
    }

    // Check if slug is already taken
    if HostedMock::find_by_slug(pool, org_ctx.org_id, slug)
        .await
        .map_err(ApiError::Database)?
        .is_some()
    {
        return Err(ApiError::InvalidRequest(format!(
            "A deployment with slug '{}' already exists",
            slug
        )));
    }

    // Create deployment record
    let deployment = HostedMock::create(
        pool,
        org_ctx.org_id,
        request.project_id,
        &request.name,
        slug,
        request.description.as_deref(),
        request.config_json,
        request.openapi_spec_url.as_deref(),
        request.region.as_deref(),
    )
    .await
    .map_err(ApiError::Database)?;

    // Log deployment creation
    DeploymentLog::create(
        pool,
        deployment.id,
        "info",
        "Deployment created",
        Some(serde_json::json!({
            "name": request.name,
            "slug": slug,
        })),
    )
    .await
    .map_err(ApiError::Database)?;

    // Mark as pending - the deployment orchestrator will pick it up and deploy it
    // The orchestrator polls for pending/deploying deployments every 10 seconds
    HostedMock::update_status(pool, deployment.id, DeploymentStatus::Pending, None)
        .await
        .map_err(ApiError::Database)?;

    // Track feature usage
    let _ = FeatureUsage::record(
        pool,
        org_ctx.org_id,
        Some(user_id),
        FeatureType::HostedMockDeploy,
        Some(serde_json::json!({
            "deployment_id": deployment.id,
            "name": request.name,
            "slug": slug,
        })),
    )
    .await;

    // Record audit log
    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    record_audit_event(
        pool,
        org_ctx.org_id,
        Some(user_id),
        AuditEventType::DeploymentCreated,
        format!("Hosted mock deployment '{}' created", request.name),
        Some(serde_json::json!({
            "deployment_id": deployment.id,
            "name": request.name,
            "slug": slug,
            "project_id": request.project_id,
        })),
        ip_address,
        user_agent,
    )
    .await;

    // Return deployment info
    let deployment = HostedMock::find_by_id(pool, deployment.id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            ApiError::Internal(anyhow::anyhow!("Failed to retrieve created deployment"))
        })?;

    Ok(Json(DeploymentResponse::from(deployment)))
}

/// List all deployments for the organization
pub async fn list_deployments(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<DeploymentResponse>>> {
    let pool = state.db.pool();

    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Get all deployments
    let deployments = HostedMock::find_by_org(pool, org_ctx.org_id)
        .await
        .map_err(ApiError::Database)?;

    let responses: Vec<DeploymentResponse> =
        deployments.into_iter().map(DeploymentResponse::from).collect();

    Ok(Json(responses))
}

/// Get deployment details
pub async fn get_deployment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(deployment_id): Path<Uuid>,
) -> ApiResult<Json<DeploymentResponse>> {
    let pool = state.db.pool();

    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Get deployment
    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".to_string()))?;

    // Verify access
    if deployment.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "You don't have access to this deployment".to_string(),
        ));
    }

    Ok(Json(DeploymentResponse::from(deployment)))
}

/// Update deployment status (internal/admin use)
pub async fn update_deployment_status(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(deployment_id): Path<Uuid>,
    Json(request): Json<UpdateStatusRequest>,
) -> ApiResult<Json<DeploymentResponse>> {
    let pool = state.db.pool();

    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Check permission
    let checker = PermissionChecker::new(&state);
    checker
        .require_permission(user_id, org_ctx.org_id, Permission::HostedMockUpdate)
        .await?;

    // Get deployment
    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".to_string()))?;

    // Verify access
    if deployment.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "You don't have access to this deployment".to_string(),
        ));
    }

    // Update status
    let status = DeploymentStatus::from_str(&request.status)
        .ok_or_else(|| ApiError::InvalidRequest("Invalid status".to_string()))?;

    HostedMock::update_status(pool, deployment_id, status, request.error_message.as_deref())
        .await
        .map_err(ApiError::Database)?;

    // Update URLs if provided
    if request.deployment_url.is_some() || request.internal_url.is_some() {
        HostedMock::update_urls(
            pool,
            deployment_id,
            request.deployment_url.as_deref(),
            request.internal_url.as_deref(),
        )
        .await
        .map_err(ApiError::Database)?;
    }

    // Get updated deployment
    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            ApiError::Internal(anyhow::anyhow!("Failed to retrieve updated deployment"))
        })?;

    // Send deployment status notification email (non-blocking)
    if let Ok(Some(org)) = Organization::find_by_id(pool, deployment.org_id).await {
        if let Ok(Some(owner)) = User::find_by_id(pool, org.owner_id).await {
            let status_str = format!("{:?}", deployment.status()).to_lowercase();
            let email_msg = crate::email::EmailService::generate_deployment_status_email(
                &owner.username,
                &owner.email,
                &deployment.name,
                &status_str,
                deployment.deployment_url.as_deref(),
                request.error_message.as_deref(),
            );

            tokio::spawn(async move {
                match crate::email::EmailService::from_env() {
                    Ok(email_service) => {
                        if let Err(e) = email_service.send(email_msg).await {
                            tracing::warn!("Failed to send deployment status email: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create email service: {}", e);
                    }
                }
            });
        }
    }

    Ok(Json(DeploymentResponse::from(deployment)))
}

/// Delete a deployment
pub async fn delete_deployment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(deployment_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let pool = state.db.pool();

    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Check permission
    let checker = PermissionChecker::new(&state);
    checker
        .require_permission(user_id, org_ctx.org_id, Permission::HostedMockDelete)
        .await?;

    // Get deployment
    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".to_string()))?;

    // Verify access
    if deployment.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "You don't have access to this deployment".to_string(),
        ));
    }

    // Record audit log before deletion
    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    record_audit_event(
        pool,
        org_ctx.org_id,
        Some(user_id),
        AuditEventType::DeploymentDeleted,
        format!("Hosted mock deployment '{}' deleted", deployment.name),
        Some(serde_json::json!({
            "deployment_id": deployment.id,
            "name": deployment.name,
            "slug": deployment.slug,
        })),
        ip_address,
        user_agent,
    )
    .await;

    // Update status to deleting before cleanup
    HostedMock::update_status(pool, deployment_id, DeploymentStatus::Deleting, None)
        .await
        .map_err(ApiError::Database)?;

    // Trigger actual deletion (stop service, cleanup resources, etc.)
    // Try to delete from Fly.io if configured
    if let Ok(flyio_token) = std::env::var("FLYIO_API_TOKEN") {
        let flyio_client = FlyioClient::new(flyio_token);

        // Generate app name (same as in orchestrator)
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

        // Try to get machine_id from metadata
        let machine_id = deployment.metadata_json.get("flyio_machine_id").and_then(|v| v.as_str());

        if let Some(machine_id) = machine_id {
            // Delete the specific machine
            match flyio_client.delete_machine(&app_name, machine_id).await {
                Ok(_) => {
                    DeploymentLog::create(
                        pool,
                        deployment_id,
                        "info",
                        &format!("Deleted Fly.io machine: {}", machine_id),
                        None,
                    )
                    .await
                    .ok();
                }
                Err(e) => {
                    warn!("Failed to delete Fly.io machine {}: {}", machine_id, e);
                    DeploymentLog::create(
                        pool,
                        deployment_id,
                        "warning",
                        &format!("Failed to delete Fly.io machine: {}", e),
                        None,
                    )
                    .await
                    .ok();
                    // Continue with app deletion and database deletion
                }
            }
        } else {
            // Machine ID not found in metadata, try to list and delete all machines
            warn!(
                "Machine ID not found in metadata for deployment {}, attempting to list machines",
                deployment_id
            );
            match flyio_client.list_machines(&app_name).await {
                Ok(machines) => {
                    for machine in machines {
                        if let Err(e) = flyio_client.delete_machine(&app_name, &machine.id).await {
                            warn!("Failed to delete Fly.io machine {}: {}", machine.id, e);
                        } else {
                            DeploymentLog::create(
                                pool,
                                deployment_id,
                                "info",
                                &format!("Deleted Fly.io machine: {}", machine.id),
                                None,
                            )
                            .await
                            .ok();
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to list Fly.io machines for app {}: {}", app_name, e);
                    // Continue with app deletion and database deletion
                }
            }
        }

        // Delete the Fly.io app itself to avoid empty apps piling up
        match flyio_client.delete_app(&app_name).await {
            Ok(_) => {
                DeploymentLog::create(
                    pool,
                    deployment_id,
                    "info",
                    &format!("Deleted Fly.io app: {}", app_name),
                    None,
                )
                .await
                .ok();
            }
            Err(e) => {
                warn!("Failed to delete Fly.io app {}: {}", app_name, e);
            }
        }
    }

    // Soft delete from database
    HostedMock::delete(pool, deployment_id).await.map_err(ApiError::Database)?;

    DeploymentLog::create(pool, deployment_id, "info", "Deployment deleted successfully", None)
        .await
        .ok(); // Log but don't fail on log error

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Deployment deleted"
    })))
}

/// Request body for redeployment (all fields optional)
#[derive(Debug, Deserialize, Default)]
pub struct RedeployRequest {
    /// Updated OpenAPI spec (replaces existing config)
    pub config_json: Option<serde_json::Value>,
    /// Updated spec URL
    pub openapi_spec_url: Option<String>,
}

/// Redeploy an existing hosted mock deployment
///
/// Updates the machine image and optionally the spec, then restarts.
pub async fn redeploy_deployment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(deployment_id): Path<Uuid>,
    body: Option<Json<RedeployRequest>>,
) -> ApiResult<Json<serde_json::Value>> {
    let pool = state.db.pool();

    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Check permission (reuse deploy permission)
    let checker = PermissionChecker::new(&state);
    checker
        .require_permission(user_id, org_ctx.org_id, Permission::HostedMockCreate)
        .await?;

    // Get existing deployment
    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".to_string()))?;

    // Verify ownership
    if deployment.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest("Deployment not found".to_string()));
    }

    // Only allow redeployment of active or failed deployments
    let status = deployment.status();
    if !matches!(status, DeploymentStatus::Active | DeploymentStatus::Failed) {
        return Err(ApiError::InvalidRequest(format!(
            "Cannot redeploy a deployment with status '{}'. Must be 'active' or 'failed'.",
            status
        )));
    }

    // Update spec if provided
    let request = body.map(|b| b.0).unwrap_or_default();
    if request.config_json.is_some() || request.openapi_spec_url.is_some() {
        let mut query = String::from("UPDATE hosted_mocks SET updated_at = NOW()");
        let mut param_count = 0;

        if request.config_json.is_some() {
            param_count += 1;
            query.push_str(&format!(", config_json = ${}", param_count));
        }
        if request.openapi_spec_url.is_some() {
            param_count += 1;
            query.push_str(&format!(", openapi_spec_url = ${}", param_count));
        }
        param_count += 1;
        query.push_str(&format!(" WHERE id = ${}", param_count));

        let mut q = sqlx::query(&query);
        if let Some(ref config) = request.config_json {
            q = q.bind(config);
        }
        if let Some(ref spec_url) = request.openapi_spec_url {
            q = q.bind(spec_url);
        }
        q = q.bind(deployment_id);
        q.execute(pool).await.map_err(|e| {
            ApiError::Internal(anyhow::anyhow!("Failed to update deployment: {}", e))
        })?;
    }

    // Update status to deploying
    HostedMock::update_status(pool, deployment_id, DeploymentStatus::Deploying, None)
        .await
        .map_err(ApiError::Database)?;

    DeploymentLog::create(pool, deployment_id, "info", "Redeployment initiated", None)
        .await
        .ok();

    // Trigger redeployment in background
    let pool_clone = pool.clone();
    let deployment_id_clone = deployment_id;
    tokio::spawn(async move {
        let pool = &pool_clone;

        // Fetch the updated deployment
        let updated_deployment = match HostedMock::find_by_id(pool, deployment_id_clone).await {
            Ok(Some(d)) => d,
            Ok(None) => {
                tracing::error!("Deployment {} not found during redeploy", deployment_id_clone);
                return;
            }
            Err(e) => {
                tracing::error!(
                    "Failed to fetch deployment {} for redeploy: {}",
                    deployment_id_clone,
                    e
                );
                return;
            }
        };

        // Try to redeploy via Fly.io if configured
        if let Ok(flyio_token) = std::env::var("FLYIO_API_TOKEN") {
            let flyio_client = FlyioClient::new(flyio_token);

            let machine_id = updated_deployment
                .metadata_json
                .get("flyio_machine_id")
                .and_then(|v| v.as_str());

            if let Some(machine_id) = machine_id {
                let app_name = format!(
                    "mockforge-{}-{}",
                    updated_deployment
                        .org_id
                        .to_string()
                        .replace('-', "")
                        .chars()
                        .take(8)
                        .collect::<String>(),
                    updated_deployment.slug
                );

                // Build updated machine config
                let mut env = std::collections::HashMap::new();
                env.insert(
                    "MOCKFORGE_DEPLOYMENT_ID".to_string(),
                    updated_deployment.id.to_string(),
                );
                env.insert("MOCKFORGE_ORG_ID".to_string(), updated_deployment.org_id.to_string());
                if let Ok(config_str) = serde_json::to_string(&updated_deployment.config_json) {
                    env.insert("MOCKFORGE_CONFIG".to_string(), config_str);
                }
                env.insert("PORT".to_string(), "3000".to_string());

                if let Some(ref spec_url) = updated_deployment.openapi_spec_url {
                    env.insert("MOCKFORGE_OPENAPI_SPEC_URL".to_string(), spec_url.clone());
                }

                let image = std::env::var("MOCKFORGE_DOCKER_IMAGE")
                    .unwrap_or_else(|_| "ghcr.io/saasy-solutions/mockforge:latest".to_string());

                use crate::deployment::flyio::{
                    FlyioCheck, FlyioMachineConfig, FlyioPort, FlyioRegistryAuth, FlyioService,
                };

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

                let mut checks = std::collections::HashMap::new();
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
                        password: flyio_client.api_token().to_string(),
                    })
                } else {
                    None
                };

                match flyio_client
                    .update_machine(&app_name, machine_id, machine_config, registry_auth)
                    .await
                {
                    Ok(_) => {
                        let _ = DeploymentLog::create(
                            pool,
                            deployment_id_clone,
                            "info",
                            "Machine updated and restarting",
                            None,
                        )
                        .await;
                    }
                    Err(e) => {
                        tracing::error!("Redeployment failed for {}: {:#}", deployment_id_clone, e);
                        let _ = HostedMock::update_status(
                            pool,
                            deployment_id_clone,
                            DeploymentStatus::Failed,
                            Some(&format!("Redeployment failed: {}", e)),
                        )
                        .await;
                        let _ = DeploymentLog::create(
                            pool,
                            deployment_id_clone,
                            "error",
                            &format!("Redeployment failed: {}", e),
                            None,
                        )
                        .await;
                        return;
                    }
                }
            } else {
                tracing::error!(
                    "No Fly.io machine ID found for deployment {}",
                    deployment_id_clone
                );
                let _ = HostedMock::update_status(
                    pool,
                    deployment_id_clone,
                    DeploymentStatus::Failed,
                    Some("No Fly.io machine ID found in deployment metadata"),
                )
                .await;
                return;
            }
        }

        // Mark as active
        let _ =
            HostedMock::update_status(pool, deployment_id_clone, DeploymentStatus::Active, None)
                .await;

        let _ = DeploymentLog::create(
            pool,
            deployment_id_clone,
            "info",
            "Redeployment completed successfully",
            None,
        )
        .await;

        tracing::info!("Successfully redeployed mock service: {}", deployment_id_clone);
    });

    Ok(Json(serde_json::json!({
        "id": deployment_id,
        "status": "deploying",
        "message": "Redeployment initiated"
    })))
}

/// Get deployment logs
pub async fn get_deployment_logs(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(deployment_id): Path<Uuid>,
) -> ApiResult<Json<Vec<LogResponse>>> {
    let pool = state.db.pool();

    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Get deployment
    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".to_string()))?;

    // Verify access
    if deployment.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "You don't have access to this deployment".to_string(),
        ));
    }

    // Get logs
    let logs = DeploymentLog::find_by_mock(pool, deployment_id, Some(100))
        .await
        .map_err(ApiError::Database)?;

    let responses: Vec<LogResponse> = logs.into_iter().map(LogResponse::from).collect();

    Ok(Json(responses))
}

/// Get deployment metrics
pub async fn get_deployment_metrics(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(deployment_id): Path<Uuid>,
) -> ApiResult<Json<MetricsResponse>> {
    let pool = state.db.pool();

    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Get deployment
    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".to_string()))?;

    // Verify access
    if deployment.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "You don't have access to this deployment".to_string(),
        ));
    }

    // Get current metrics
    let metrics = DeploymentMetrics::get_or_create_current(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?;

    Ok(Json(MetricsResponse::from(metrics)))
}

/// Upload an OpenAPI spec file for use in a hosted mock deployment
pub async fn upload_spec(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> ApiResult<Json<SpecUploadResponse>> {
    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Check permission
    let checker = PermissionChecker::new(&state);
    checker
        .require_permission(user_id, org_ctx.org_id, Permission::HostedMockCreate)
        .await?;

    // Extract file from multipart
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name = String::from("spec");

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::InvalidRequest(format!("Failed to read multipart field: {}", e)))?
    {
        if field.name() == Some("file") || field.name() == Some("spec") {
            if let Some(name) = field.file_name() {
                file_name =
                    name.to_string().replace(".yaml", "").replace(".yml", "").replace(".json", "");
            }
            let data = field.bytes().await.map_err(|e| {
                ApiError::InvalidRequest(format!("Failed to read file data: {}", e))
            })?;

            // Validate it's valid JSON or YAML OpenAPI spec
            let content = String::from_utf8(data.to_vec()).map_err(|_| {
                ApiError::InvalidRequest("File must be valid UTF-8 text".to_string())
            })?;

            // Try to parse as JSON first, then YAML
            let spec_value: serde_json::Value =
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                    v
                } else if let Ok(v) = serde_yaml::from_str::<serde_json::Value>(&content) {
                    v
                } else {
                    return Err(ApiError::InvalidRequest(
                        "File must be a valid JSON or YAML OpenAPI specification".to_string(),
                    ));
                };

            // Basic OpenAPI validation - check for required fields
            if spec_value.get("openapi").is_none() && spec_value.get("swagger").is_none() {
                return Err(ApiError::InvalidRequest(
                    "File must contain an 'openapi' or 'swagger' field".to_string(),
                ));
            }

            // Always store as JSON
            let json_data = serde_json::to_vec_pretty(&spec_value).map_err(|e| {
                ApiError::Internal(anyhow::anyhow!("Failed to serialize spec: {}", e))
            })?;

            file_data = Some(json_data);
        }
    }

    let data = file_data.ok_or_else(|| {
        ApiError::InvalidRequest("No 'file' or 'spec' field in upload".to_string())
    })?;

    // Upload to storage
    let url = state
        .storage
        .upload_spec(&org_ctx.org_id.to_string(), &file_name, data)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to upload spec: {}", e)))?;

    Ok(Json(SpecUploadResponse { url }))
}

#[derive(Debug, Serialize)]
pub struct SpecUploadResponse {
    pub url: String,
}

// Request/Response types

#[derive(Debug, Deserialize)]
pub struct CreateDeploymentRequest {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub project_id: Option<Uuid>,
    pub config_json: serde_json::Value,
    pub openapi_spec_url: Option<String>,
    pub region: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
    pub error_message: Option<String>,
    pub deployment_url: Option<String>,
    pub internal_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeploymentResponse {
    pub id: Uuid,
    pub org_id: Uuid,
    pub project_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub status: String,
    pub deployment_url: Option<String>,
    pub openapi_spec_url: Option<String>,
    pub health_status: String,
    pub error_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<HostedMock> for DeploymentResponse {
    fn from(mock: HostedMock) -> Self {
        let status = mock.status().to_string();
        let health_status = mock.health_status().to_string();
        Self {
            id: mock.id,
            org_id: mock.org_id,
            project_id: mock.project_id,
            name: mock.name,
            slug: mock.slug,
            description: mock.description,
            status,
            deployment_url: mock.deployment_url,
            openapi_spec_url: mock.openapi_spec_url,
            health_status,
            error_message: mock.error_message,
            created_at: mock.created_at,
            updated_at: mock.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LogResponse {
    pub id: Uuid,
    pub level: String,
    pub message: String,
    pub metadata: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<DeploymentLog> for LogResponse {
    fn from(log: DeploymentLog) -> Self {
        Self {
            id: log.id,
            level: log.level,
            message: log.message,
            metadata: log.metadata_json,
            created_at: log.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub requests: i64,
    pub requests_2xx: i64,
    pub requests_4xx: i64,
    pub requests_5xx: i64,
    pub egress_bytes: i64,
    pub avg_response_time_ms: i64,
    pub period_start: chrono::NaiveDate,
}

impl From<DeploymentMetrics> for MetricsResponse {
    fn from(metrics: DeploymentMetrics) -> Self {
        Self {
            requests: metrics.requests,
            requests_2xx: metrics.requests_2xx,
            requests_4xx: metrics.requests_4xx,
            requests_5xx: metrics.requests_5xx,
            egress_bytes: metrics.egress_bytes,
            avg_response_time_ms: metrics.avg_response_time_ms,
            period_start: metrics.period_start,
        }
    }
}
