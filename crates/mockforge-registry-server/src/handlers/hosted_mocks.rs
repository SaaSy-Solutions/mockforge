//! Hosted Mocks deployment handlers
//!
//! Provides endpoints for deploying, managing, and monitoring cloud-hosted mock services

use axum::{
    extract::{Multipart, Path, Query, State},
    http::HeaderMap,
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use chrono::{DateTime, Utc};
use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use uuid::Uuid;

use crate::{
    deployment::flyio::FlyioClient,
    error::{ApiError, ApiResult},
    fly_logs::LogEntry,
    middleware::{
        permission_check::PermissionChecker, permissions::Permission, resolve_org_context, AuthUser,
    },
    models::{
        feature_usage::FeatureType, AuditEventType, DeploymentLog, DeploymentMetrics,
        DeploymentStatus, HostedMock,
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
        let existing = state.store.list_hosted_mocks_by_org(org_ctx.org_id).await?;

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
    if state.store.find_hosted_mock_by_slug(org_ctx.org_id, slug).await?.is_some() {
        return Err(ApiError::InvalidRequest(format!(
            "A deployment with slug '{}' already exists",
            slug
        )));
    }

    // Resolve and plan-gate the protocol set. HTTP is always present and is
    // free; gRPC requires Pro; brokers (SMTP/MQTT/Kafka/AMQP/TCP) require
    // Team. Reject with an explicit error so the UI can surface "upgrade to
    // enable X" rather than a generic 400.
    let mut enabled_protocols = request.enabled_protocols.clone().unwrap_or_default();
    if !enabled_protocols.contains(&crate::models::Protocol::Http) {
        enabled_protocols.insert(0, crate::models::Protocol::Http);
    }
    let plan = org_ctx.org.plan.clone();
    if !crate::models::protocols_allowed_on_plan(&enabled_protocols, &plan) {
        let blocked: Vec<String> = enabled_protocols
            .iter()
            .filter(|p| !crate::models::protocols_allowed_on_plan(&[**p], &plan))
            .map(|p| format!("{:?}", p))
            .collect();
        return Err(ApiError::InvalidRequest(format!(
            "These protocols require a higher plan than '{}': {}",
            plan,
            blocked.join(", ")
        )));
    }

    // Persist enabled protocols into the deployment's config_json so the
    // orchestrator can read them when building the Fly machine. Done as an
    // additive merge to preserve any keys the caller already supplied.
    let mut config_json = request.config_json.clone();
    if let Some(obj) = config_json.as_object_mut() {
        obj.insert(
            "enabled_protocols".to_string(),
            serde_json::to_value(&enabled_protocols).unwrap_or(serde_json::Value::Null),
        );
    } else {
        // Caller passed a non-object (e.g. null) — replace with a fresh
        // object that only carries the protocol list.
        config_json = serde_json::json!({ "enabled_protocols": enabled_protocols });
    }

    // Create deployment record
    let deployment = state
        .store
        .create_hosted_mock(
            org_ctx.org_id,
            request.project_id,
            &request.name,
            slug,
            request.description.as_deref(),
            config_json,
            request.openapi_spec_url.as_deref(),
            request.region.as_deref(),
        )
        .await?;

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
    state
        .store
        .update_hosted_mock_status(deployment.id, DeploymentStatus::Pending, None)
        .await?;

    // Track feature usage
    state
        .store
        .record_feature_usage(
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

    state
        .store
        .record_audit_event(
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
    let deployment = state.store.find_hosted_mock_by_id(deployment.id).await?.ok_or_else(|| {
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
    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Get all deployments
    let deployments = state.store.list_hosted_mocks_by_org(org_ctx.org_id).await?;

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
    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Get deployment
    let deployment = state
        .store
        .find_hosted_mock_by_id(deployment_id)
        .await?
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
    let deployment = state
        .store
        .find_hosted_mock_by_id(deployment_id)
        .await?
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

    state
        .store
        .update_hosted_mock_status(deployment_id, status, request.error_message.as_deref())
        .await?;

    // Update URLs if provided
    if request.deployment_url.is_some() || request.internal_url.is_some() {
        state
            .store
            .update_hosted_mock_urls(
                deployment_id,
                request.deployment_url.as_deref(),
                request.internal_url.as_deref(),
            )
            .await?;
    }

    // Get updated deployment
    let deployment = state.store.find_hosted_mock_by_id(deployment_id).await?.ok_or_else(|| {
        ApiError::Internal(anyhow::anyhow!("Failed to retrieve updated deployment"))
    })?;

    // Send deployment status notification email (non-blocking)
    if let Ok(Some(org)) = state.store.find_organization_by_id(deployment.org_id).await {
        if let Ok(Some(owner)) = state.store.find_user_by_id(org.owner_id).await {
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
    let deployment = state
        .store
        .find_hosted_mock_by_id(deployment_id)
        .await?
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

    state
        .store
        .record_audit_event(
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
    state
        .store
        .update_hosted_mock_status(deployment_id, DeploymentStatus::Deleting, None)
        .await?;

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
    state.store.delete_hosted_mock(deployment_id).await?;

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
    // pool kept for DeploymentLog + the spawned redeploy orchestration task below

    // Get existing deployment
    let deployment = state
        .store
        .find_hosted_mock_by_id(deployment_id)
        .await?
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
    state
        .store
        .update_hosted_mock_status(deployment_id, DeploymentStatus::Deploying, None)
        .await?;

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

fn deployment_app_name(deployment: &HostedMock) -> String {
    format!(
        "mockforge-{}-{}",
        deployment
            .org_id
            .to_string()
            .replace('-', "")
            .chars()
            .take(8)
            .collect::<String>(),
        deployment.slug
    )
}

/// Stop a running hosted mock deployment.
///
/// Gracefully stops the Fly.io machine (without deleting it) and marks
/// the deployment as `stopped`. Only active deployments can be stopped.
pub async fn stop_deployment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(deployment_id): Path<Uuid>,
) -> ApiResult<Json<DeploymentResponse>> {
    let pool = state.db.pool();

    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let checker = PermissionChecker::new(&state);
    checker
        .require_permission(user_id, org_ctx.org_id, Permission::HostedMockUpdate)
        .await?;

    let deployment = state
        .store
        .find_hosted_mock_by_id(deployment_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".to_string()))?;

    if deployment.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest("Deployment not found".to_string()));
    }

    let status = deployment.status();
    if !matches!(status, DeploymentStatus::Active) {
        return Err(ApiError::InvalidRequest(format!(
            "Cannot stop a deployment with status '{}'. Must be 'active'.",
            status
        )));
    }

    if let Ok(flyio_token) = std::env::var("FLYIO_API_TOKEN") {
        let flyio_client = FlyioClient::new(flyio_token);
        let app_name = deployment_app_name(&deployment);
        let machine_id = deployment.metadata_json.get("flyio_machine_id").and_then(|v| v.as_str());

        if let Some(machine_id) = machine_id {
            flyio_client.stop_machine(&app_name, machine_id).await.map_err(|e| {
                ApiError::Internal(anyhow::anyhow!("Failed to stop machine: {}", e))
            })?;
        } else {
            warn!(
                "No Fly.io machine ID found for deployment {}; marking as stopped anyway",
                deployment_id
            );
        }
    }

    state
        .store
        .update_hosted_mock_status(deployment_id, DeploymentStatus::Stopped, None)
        .await?;

    DeploymentLog::create(pool, deployment_id, "info", "Deployment stopped", None)
        .await
        .ok();

    let updated = state.store.find_hosted_mock_by_id(deployment_id).await?.ok_or_else(|| {
        ApiError::Internal(anyhow::anyhow!("Failed to retrieve updated deployment"))
    })?;

    Ok(Json(DeploymentResponse::from(updated)))
}

/// Start a stopped hosted mock deployment.
pub async fn start_deployment(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(deployment_id): Path<Uuid>,
) -> ApiResult<Json<DeploymentResponse>> {
    let pool = state.db.pool();

    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let checker = PermissionChecker::new(&state);
    checker
        .require_permission(user_id, org_ctx.org_id, Permission::HostedMockUpdate)
        .await?;

    let deployment = state
        .store
        .find_hosted_mock_by_id(deployment_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".to_string()))?;

    if deployment.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest("Deployment not found".to_string()));
    }

    let status = deployment.status();
    if !matches!(status, DeploymentStatus::Stopped) {
        return Err(ApiError::InvalidRequest(format!(
            "Cannot start a deployment with status '{}'. Must be 'stopped'.",
            status
        )));
    }

    if let Ok(flyio_token) = std::env::var("FLYIO_API_TOKEN") {
        let flyio_client = FlyioClient::new(flyio_token);
        let app_name = deployment_app_name(&deployment);
        let machine_id = deployment.metadata_json.get("flyio_machine_id").and_then(|v| v.as_str());

        if let Some(machine_id) = machine_id {
            flyio_client.start_machine(&app_name, machine_id).await.map_err(|e| {
                ApiError::Internal(anyhow::anyhow!("Failed to start machine: {}", e))
            })?;
        } else {
            return Err(ApiError::InvalidRequest(
                "No Fly.io machine ID found in deployment metadata; cannot start".to_string(),
            ));
        }
    }

    state
        .store
        .update_hosted_mock_status(deployment_id, DeploymentStatus::Active, None)
        .await?;

    DeploymentLog::create(pool, deployment_id, "info", "Deployment started", None)
        .await
        .ok();

    let updated = state.store.find_hosted_mock_by_id(deployment_id).await?.ok_or_else(|| {
        ApiError::Internal(anyhow::anyhow!("Failed to retrieve updated deployment"))
    })?;

    Ok(Json(DeploymentResponse::from(updated)))
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

/// Query parameters for the runtime-logs endpoint.
#[derive(Debug, Deserialize)]
pub struct RuntimeLogsQuery {
    /// Maximum entries to return. Falls back to the env-configured default.
    pub limit: Option<u32>,
    /// RFC3339 timestamp; only entries strictly newer than this are returned.
    pub since: Option<String>,
}

/// Get runtime logs for a deployment by polling the Fly logs API.
///
/// This is the new "logs" surface (#224). The historical
/// `GET /api/v1/hosted-mocks/{id}/logs` endpoint stays as deployment events
/// (lifecycle entries from the local `deployment_logs` table) so the UI can
/// surface both views: an "Events" tab from the existing endpoint and a
/// "Logs" tab backed by this one.
pub async fn get_runtime_logs(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(deployment_id): Path<Uuid>,
    Query(params): Query<RuntimeLogsQuery>,
) -> ApiResult<Json<Vec<LogEntry>>> {
    let pool = state.db.pool();

    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".to_string()))?;

    if deployment.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "You don't have access to this deployment".to_string(),
        ));
    }

    let Some(client) = crate::fly_logs::global() else {
        // No Fly token configured — return empty rather than 500. Operators
        // see this in self-hosted / dev; the UI shows a "not configured"
        // hint when the list is empty and the configured flag (sent in a
        // header below) is false.
        return Ok(Json(Vec::new()));
    };

    let since = params
        .since
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc));

    let app_name = deployment.fly_app_name();
    let entries = client.fetch_recent(&app_name, since, params.limit).await.map_err(|e| {
        warn!(error = %e, app_name = %app_name, "Fly runtime logs fetch failed");
        ApiError::Internal(anyhow::anyhow!("Fly logs query failed: {}", e))
    })?;

    Ok(Json(entries))
}

/// SSE stream of runtime logs. Polls the Fly logs API every few seconds and
/// emits new entries as `data:` events. Closes on the first transient error
/// after surfacing it as a `data:` event so the browser can render a banner.
pub async fn stream_runtime_logs(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(deployment_id): Path<Uuid>,
) -> ApiResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    let pool = state.db.pool();
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".to_string()))?;

    if deployment.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "You don't have access to this deployment".to_string(),
        ));
    }

    let app_name = deployment.fly_app_name();
    let stream = build_runtime_logs_stream(app_name);

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

/// Build the SSE event stream for runtime logs. Each tick polls Fly with a
/// `since` cursor that advances past the latest entry observed so we don't
/// duplicate events on refetch.
fn build_runtime_logs_stream(app_name: String) -> impl Stream<Item = Result<Event, Infallible>> {
    use futures_util::stream::unfold;

    /// Per-stream poll state: cursor + tick count for an initial empty-poll
    /// quietness. We start the cursor a few seconds in the past so the first
    /// poll picks up "what's happening now" without dumping the whole window.
    struct State {
        cursor: DateTime<Utc>,
        client: Option<&'static crate::fly_logs::FlyLogsClient>,
        app_name: String,
        emitted_unconfigured: bool,
    }

    let state = State {
        cursor: Utc::now() - chrono::Duration::seconds(30),
        client: crate::fly_logs::global(),
        app_name,
        emitted_unconfigured: false,
    };

    unfold(state, |mut st| async move {
        // Slow poll loop. 2 seconds keeps the UI responsive without hammering
        // Fly's API; tighten once we move to the NATS subscription path
        // (#232).
        if st.client.is_none() && !st.emitted_unconfigured {
            st.emitted_unconfigured = true;
            let event = Event::default()
                .event("config")
                .data("Fly runtime logs are not configured (FLYIO_API_TOKEN unset)");
            return Some((Ok(event), st));
        }

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let Some(client) = st.client else {
            // Configured-flag already emitted; keep stream alive with a
            // periodic comment so the connection doesn't close.
            let event = Event::default().comment("idle");
            return Some((Ok(event), st));
        };

        match client.fetch_recent(&st.app_name, Some(st.cursor), None).await {
            Ok(entries) if entries.is_empty() => {
                let event = Event::default().comment("no-new-logs");
                Some((Ok(event), st))
            }
            Ok(entries) => {
                // Advance cursor to the newest entry we just emitted.
                if let Some(latest) = entries.iter().map(|e| e.timestamp).max() {
                    st.cursor = latest;
                }
                let payload = serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string());
                let event = Event::default().event("logs").data(payload);
                Some((Ok(event), st))
            }
            Err(err) => {
                let payload = serde_json::json!({ "error": err.to_string() }).to_string();
                let event = Event::default().event("error").data(payload);
                Some((Ok(event), st))
            }
        }
    })
}

/// One captured request/response event. Mirrors `RequestLogEvent` in
/// `mockforge-observability::log_shipper` — both sides serialize via serde
/// so the wire format is the canonical contract.
#[derive(Debug, Deserialize, Serialize)]
pub struct RuntimeRequestEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub latency_ms: u32,
    #[serde(default)]
    pub matched_route: Option<String>,
    #[serde(default)]
    pub client_ip: Option<String>,
    #[serde(default)]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub bytes_in: Option<i64>,
    #[serde(default)]
    pub bytes_out: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct IngestPayload {
    pub events: Vec<RuntimeRequestEvent>,
}

#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub accepted: usize,
}

/// Ingest a batch of structured request logs from a hosted-mock container.
///
/// This endpoint is **not** behind the user-scoped `auth_middleware` — it
/// authenticates with a deployment-scoped JWT minted by the orchestrator
/// at deploy time and passed to the container as `MOCKFORGE_LOG_INGEST_TOKEN`.
/// The token's subject embeds the deployment_id; we reject mismatches.
pub async fn ingest_runtime_logs(
    State(state): State<AppState>,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
    Json(payload): Json<IngestPayload>,
) -> ApiResult<Json<IngestResponse>> {
    // Bearer token from the in-container shipper.
    let auth = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::InvalidRequest("Missing deployment ingest token".to_string()))?;

    let token_deployment_id = mockforge_registry_core::auth::verify_deployment_ingest_token(
        auth,
        &state.config.jwt_secret,
    )
    .map_err(|e| {
        tracing::warn!(error = %e, "Deployment ingest token rejected");
        ApiError::InvalidRequest("Invalid deployment ingest token".to_string())
    })?;

    if token_deployment_id != deployment_id {
        return Err(ApiError::InvalidRequest(
            "Token deployment id does not match URL path".to_string(),
        ));
    }

    if payload.events.is_empty() {
        return Ok(Json(IngestResponse { accepted: 0 }));
    }

    // Cap accepted batch size as a defence against runaway shippers — a
    // misbehaving container shouldn't be able to flood the table with one
    // request. Matches the shipper's default flush size.
    const MAX_BATCH: usize = 500;
    let events: Vec<RuntimeRequestEvent> = payload.events.into_iter().take(MAX_BATCH).collect();
    let accepted = events.len();

    // Bulk insert. Building one INSERT with many VALUES rows would be
    // marginally faster but uglier with sqlx; the row count per batch is
    // small (50 by default), so per-row inserts in a transaction is fine.
    let pool = state.db.pool();
    let mut tx = pool.begin().await.map_err(ApiError::Database)?;
    for evt in &events {
        sqlx::query(
            r#"
            INSERT INTO runtime_request_logs (
                deployment_id, occurred_at, method, path, status, latency_ms,
                matched_route, client_ip, user_agent, request_id, bytes_in, bytes_out
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(deployment_id)
        .bind(evt.timestamp)
        .bind(&evt.method)
        .bind(&evt.path)
        .bind(evt.status as i16)
        .bind(evt.latency_ms as i32)
        .bind(evt.matched_route.as_ref())
        .bind(evt.client_ip.as_ref())
        .bind(evt.user_agent.as_ref())
        .bind(evt.request_id.as_ref())
        .bind(evt.bytes_in)
        .bind(evt.bytes_out)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;
    }
    tx.commit().await.map_err(ApiError::Database)?;

    Ok(Json(IngestResponse { accepted }))
}

// === Capture ingest (#234 part 2) ===
//
// The recorder ships completed exchanges (request + response) here. The
// wire format mirrors `mockforge_recorder::models::RecordedExchange` —
// we duplicate just the fields rather than depend on the recorder crate
// directly, since the registry server doesn't need any of the recorder's
// behaviour. Both sides round-trip through serde, so renames on the
// recorder side stay compatible as long as the JSON field names match.

#[derive(Debug, Deserialize)]
pub struct CaptureIngestRequest {
    pub id: String,
    pub protocol: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub query_params: Option<String>,
    pub headers: String,
    #[serde(default)]
    pub body: Option<String>,
    pub body_encoding: String,
    #[serde(default)]
    pub client_ip: Option<String>,
    #[serde(default)]
    pub trace_id: Option<String>,
    #[serde(default)]
    pub span_id: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<i64>,
    #[serde(default)]
    pub status_code: Option<i32>,
    #[serde(default)]
    pub tags: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CaptureIngestResponse {
    pub status_code: i32,
    pub headers: String,
    #[serde(default)]
    pub body: Option<String>,
    pub body_encoding: String,
    pub size_bytes: i64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CaptureIngestExchange {
    pub request: CaptureIngestRequest,
    #[serde(default)]
    pub response: Option<CaptureIngestResponse>,
}

#[derive(Debug, Deserialize)]
pub struct CaptureIngestPayload {
    pub exchanges: Vec<CaptureIngestExchange>,
}

/// Persist a batch of recorder captures shipped by an in-container
/// `CaptureCloudSyncHandle`. Same deployment-scoped JWT contract as the
/// log and OTLP ingest endpoints. Inserts use ON CONFLICT DO NOTHING so
/// that re-shipped exchanges (retries, container restarts with cached
/// buffer) don't produce duplicates.
pub async fn ingest_runtime_captures(
    State(state): State<AppState>,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
    Json(payload): Json<CaptureIngestPayload>,
) -> ApiResult<Json<IngestResponse>> {
    let auth = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::InvalidRequest("Missing deployment ingest token".to_string()))?;

    let token_deployment_id = mockforge_registry_core::auth::verify_deployment_ingest_token(
        auth,
        &state.config.jwt_secret,
    )
    .map_err(|e| {
        tracing::warn!(error = %e, "Capture ingest token rejected");
        ApiError::InvalidRequest("Invalid deployment ingest token".to_string())
    })?;

    if token_deployment_id != deployment_id {
        return Err(ApiError::InvalidRequest(
            "Token deployment id does not match URL path".to_string(),
        ));
    }

    if payload.exchanges.is_empty() {
        return Ok(Json(IngestResponse { accepted: 0 }));
    }

    // Capture rows are larger than log rows (full request + response
    // bodies), so the cap is tighter — a runaway shipper shouldn't fill
    // a single transaction with megabytes of payload.
    const MAX_BATCH: usize = 100;
    let exchanges: Vec<CaptureIngestExchange> =
        payload.exchanges.into_iter().take(MAX_BATCH).collect();
    let accepted = exchanges.len();

    let pool = state.db.pool();
    let mut tx = pool.begin().await.map_err(ApiError::Database)?;
    for exchange in &exchanges {
        let req = &exchange.request;
        let resp = exchange.response.as_ref();
        sqlx::query(
            r#"
            INSERT INTO runtime_captures (
                deployment_id, capture_id, protocol, occurred_at, method, path,
                query_params, request_headers, request_body, request_body_encoding,
                client_ip, trace_id, span_id, duration_ms, status_code, tags,
                response_status_code, response_headers, response_body,
                response_body_encoding, response_size_bytes, response_timestamp
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
                $15, $16, $17, $18, $19, $20, $21, $22
            )
            ON CONFLICT (deployment_id, capture_id) DO NOTHING
            "#,
        )
        .bind(deployment_id)
        .bind(&req.id)
        .bind(&req.protocol)
        .bind(req.timestamp)
        .bind(&req.method)
        .bind(&req.path)
        .bind(req.query_params.as_ref())
        .bind(&req.headers)
        .bind(req.body.as_ref())
        .bind(&req.body_encoding)
        .bind(req.client_ip.as_ref())
        .bind(req.trace_id.as_ref())
        .bind(req.span_id.as_ref())
        .bind(req.duration_ms)
        .bind(req.status_code)
        .bind(req.tags.as_ref())
        .bind(resp.map(|r| r.status_code))
        .bind(resp.map(|r| r.headers.as_str()))
        .bind(resp.and_then(|r| r.body.as_deref()))
        .bind(resp.map(|r| r.body_encoding.as_str()))
        .bind(resp.map(|r| r.size_bytes))
        .bind(resp.map(|r| r.timestamp))
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;
    }
    tx.commit().await.map_err(ApiError::Database)?;

    Ok(Json(IngestResponse { accepted }))
}

#[derive(Debug, Deserialize)]
pub struct RuntimeRequestsQuery {
    /// Maximum entries to return. Capped at 500 server-side.
    pub limit: Option<u32>,
    /// RFC3339 timestamp; only entries strictly newer than this are returned.
    pub since: Option<String>,
}

/// Read back recent runtime request logs for a deployment. Powers the
/// admin UI's "Requests" tab.
pub async fn get_runtime_requests(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(deployment_id): Path<Uuid>,
    Query(params): Query<RuntimeRequestsQuery>,
) -> ApiResult<Json<Vec<RuntimeRequestEvent>>> {
    let pool = state.db.pool();

    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".to_string()))?;

    if deployment.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "You don't have access to this deployment".to_string(),
        ));
    }

    let limit = params.limit.unwrap_or(100).min(500) as i64;
    let since = params
        .since
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&chrono::Utc));

    type RuntimeRequestRow = (
        chrono::DateTime<chrono::Utc>,
        String,
        String,
        i16,
        i32,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<i64>,
    );

    let rows: Vec<RuntimeRequestRow> = if let Some(since) = since {
        sqlx::query_as(
            r#"
            SELECT occurred_at, method, path, status, latency_ms,
                   matched_route, client_ip, user_agent, request_id,
                   bytes_in, bytes_out
            FROM runtime_request_logs
            WHERE deployment_id = $1 AND occurred_at > $2
            ORDER BY occurred_at DESC
            LIMIT $3
            "#,
        )
        .bind(deployment_id)
        .bind(since)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(ApiError::Database)?
    } else {
        sqlx::query_as(
            r#"
            SELECT occurred_at, method, path, status, latency_ms,
                   matched_route, client_ip, user_agent, request_id,
                   bytes_in, bytes_out
            FROM runtime_request_logs
            WHERE deployment_id = $1
            ORDER BY occurred_at DESC
            LIMIT $2
            "#,
        )
        .bind(deployment_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(ApiError::Database)?
    };

    let events: Vec<RuntimeRequestEvent> = rows
        .into_iter()
        .map(|row| RuntimeRequestEvent {
            timestamp: row.0,
            method: row.1,
            path: row.2,
            status: row.3 as u16,
            latency_ms: row.4 as u32,
            matched_route: row.5,
            client_ip: row.6,
            user_agent: row.7,
            request_id: row.8,
            bytes_in: row.9,
            bytes_out: row.10,
        })
        .collect();

    Ok(Json(events))
}

/// Proxy a request to the deployment's local recorder API.
///
/// The mockforge-recorder library is mounted on the deployed `http_app` at
/// `/api/recorder/*` (#234), but the deployed instance has no per-user auth
/// on those routes. This handler is the cloud-side gate: it verifies the
/// caller has access to the deployment, then forwards the request to the
/// deployment's internal URL.
///
/// Captures stay ephemeral on the deployment's local SQLite (Fly machines
/// don't have a persistent volume mounted by default). For long-term
/// retention we'd want either a Fly volume mount or a forwarder pattern
/// like the log shipper — both bigger and tracked separately.
async fn proxy_to_deployment_recorder(
    deployment: &HostedMock,
    path_and_query: &str,
) -> ApiResult<axum::http::Response<axum::body::Body>> {
    let base = deployment.internal_url.as_deref().or(deployment.deployment_url.as_deref());
    let Some(base) = base else {
        return Err(ApiError::InvalidRequest("Deployment has no resolved URL yet".to_string()));
    };
    let url = format!("{}{}", base.trim_end_matches('/'), path_and_query);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("HTTP client init failed: {}", e)))?;

    let resp =
        client.get(&url).send().await.map_err(|e| {
            ApiError::Internal(anyhow::anyhow!("Recorder proxy fetch failed: {}", e))
        })?;

    let status = resp.status();
    let headers = resp.headers().clone();
    let body = resp.bytes().await.map_err(|e| {
        ApiError::Internal(anyhow::anyhow!("Recorder proxy read body failed: {}", e))
    })?;

    let mut builder = axum::http::Response::builder().status(status);
    if let Some(content_type) = headers.get(axum::http::header::CONTENT_TYPE) {
        builder = builder.header(axum::http::header::CONTENT_TYPE, content_type);
    }
    builder.body(axum::body::Body::from(body)).map_err(|e| {
        ApiError::Internal(anyhow::anyhow!("Recorder proxy response build failed: {}", e))
    })
}

#[derive(Debug, Deserialize)]
pub struct RecorderCapturesQuery {
    pub limit: Option<u32>,
    pub since: Option<String>,
}

/// List recently captured request/response pairs for a deployment.
/// Proxies through to the deployment's `/api/recorder/requests` endpoint.
pub async fn list_recorder_captures(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(deployment_id): Path<Uuid>,
    Query(params): Query<RecorderCapturesQuery>,
) -> ApiResult<axum::http::Response<axum::body::Body>> {
    let pool = state.db.pool();

    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".to_string()))?;

    if deployment.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "You don't have access to this deployment".to_string(),
        ));
    }

    let mut qs = String::from("/api/recorder/requests");
    let mut sep = '?';
    if let Some(limit) = params.limit {
        qs.push(sep);
        qs.push_str(&format!("limit={}", limit));
        sep = '&';
    }
    if let Some(since) = params.since.as_deref() {
        qs.push(sep);
        qs.push_str(&format!("since={}", urlencoding::encode(since)));
    }

    let _ = state; // currently unused inside the proxy itself; kept for symmetry/future use
    proxy_to_deployment_recorder(&deployment, &qs).await
}

/// Get a single capture by id.
pub async fn get_recorder_capture(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((deployment_id, capture_id)): Path<(Uuid, String)>,
) -> ApiResult<axum::http::Response<axum::body::Body>> {
    let pool = state.db.pool();

    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".to_string()))?;

    if deployment.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "You don't have access to this deployment".to_string(),
        ));
    }

    // Validate the capture id is path-safe — defence in depth even though
    // we already URL-encode below.
    if capture_id.contains('/') || capture_id.contains('?') || capture_id.contains('#') {
        return Err(ApiError::InvalidRequest("Invalid capture id".to_string()));
    }

    let path = format!("/api/recorder/requests/{}", urlencoding::encode(&capture_id));
    let _ = state;
    proxy_to_deployment_recorder(&deployment, &path).await
}

/// Get the response body associated with a capture. Recorder splits the
/// request and response on separate endpoints so callers can paginate
/// requests cheaply without dragging response payloads along.
pub async fn get_recorder_capture_response(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((deployment_id, capture_id)): Path<(Uuid, String)>,
) -> ApiResult<axum::http::Response<axum::body::Body>> {
    let pool = state.db.pool();

    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".to_string()))?;

    if deployment.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "You don't have access to this deployment".to_string(),
        ));
    }

    if capture_id.contains('/') || capture_id.contains('?') || capture_id.contains('#') {
        return Err(ApiError::InvalidRequest("Invalid capture id".to_string()));
    }

    let path = format!("/api/recorder/requests/{}/response", urlencoding::encode(&capture_id));
    let _ = state;
    proxy_to_deployment_recorder(&deployment, &path).await
}

/// Proxy a POST to the deployment's recorder API. Used by the
/// enable/disable/clear mutations below — same auth model as the GET
/// proxies (user JWT gates access; the deployment itself has no
/// per-user auth on `/api/recorder/*`).
async fn proxy_post_to_deployment_recorder(
    deployment: &HostedMock,
    path: &str,
) -> ApiResult<axum::http::Response<axum::body::Body>> {
    let base = deployment.internal_url.as_deref().or(deployment.deployment_url.as_deref());
    let Some(base) = base else {
        return Err(ApiError::InvalidRequest("Deployment has no resolved URL yet".to_string()));
    };
    let url = format!("{}{}", base.trim_end_matches('/'), path);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("HTTP client init failed: {}", e)))?;

    let resp =
        client.post(&url).send().await.map_err(|e| {
            ApiError::Internal(anyhow::anyhow!("Recorder proxy POST failed: {}", e))
        })?;

    let status = resp.status();
    let headers = resp.headers().clone();
    let body = resp.bytes().await.map_err(|e| {
        ApiError::Internal(anyhow::anyhow!("Recorder proxy read body failed: {}", e))
    })?;

    let mut builder = axum::http::Response::builder().status(status);
    if let Some(content_type) = headers.get(axum::http::header::CONTENT_TYPE) {
        builder = builder.header(axum::http::header::CONTENT_TYPE, content_type);
    }
    builder.body(axum::body::Body::from(body)).map_err(|e| {
        ApiError::Internal(anyhow::anyhow!("Recorder proxy response build failed: {}", e))
    })
}

async fn check_org_access(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    deployment_id: Uuid,
) -> ApiResult<HostedMock> {
    let pool = state.db.pool();

    let org_ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".to_string()))?;

    if deployment.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "You don't have access to this deployment".to_string(),
        ));
    }
    Ok(deployment)
}

/// Enable recording on the deployment. Proxies POST /api/recorder/enable.
pub async fn enable_recorder(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(deployment_id): Path<Uuid>,
) -> ApiResult<axum::http::Response<axum::body::Body>> {
    let deployment = check_org_access(&state, user_id, &headers, deployment_id).await?;
    proxy_post_to_deployment_recorder(&deployment, "/api/recorder/enable").await
}

/// Disable recording on the deployment. Proxies POST /api/recorder/disable.
pub async fn disable_recorder(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(deployment_id): Path<Uuid>,
) -> ApiResult<axum::http::Response<axum::body::Body>> {
    let deployment = check_org_access(&state, user_id, &headers, deployment_id).await?;
    proxy_post_to_deployment_recorder(&deployment, "/api/recorder/disable").await
}

/// Clear all captures on the deployment. Proxies DELETE /api/recorder/clear.
/// Note: the deployment's clear endpoint is DELETE; we POST through to it
/// here because typing the verb at the cloud layer doesn't change much
/// and POST is friendlier for browser fetch without preflight.
pub async fn clear_recorder(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(deployment_id): Path<Uuid>,
) -> ApiResult<axum::http::Response<axum::body::Body>> {
    let deployment = check_org_access(&state, user_id, &headers, deployment_id).await?;
    // Build a DELETE on the wire since that's what the recorder defines.
    let base = deployment.internal_url.as_deref().or(deployment.deployment_url.as_deref());
    let Some(base) = base else {
        return Err(ApiError::InvalidRequest("Deployment has no resolved URL yet".to_string()));
    };
    let url = format!("{}/api/recorder/clear", base.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("HTTP client init failed: {}", e)))?;
    let resp =
        client.delete(&url).send().await.map_err(|e| {
            ApiError::Internal(anyhow::anyhow!("Recorder clear proxy failed: {}", e))
        })?;
    let status = resp.status();
    let headers_resp = resp.headers().clone();
    let body = resp.bytes().await.map_err(|e| {
        ApiError::Internal(anyhow::anyhow!("Recorder clear read body failed: {}", e))
    })?;
    let mut builder = axum::http::Response::builder().status(status);
    if let Some(content_type) = headers_resp.get(axum::http::header::CONTENT_TYPE) {
        builder = builder.header(axum::http::header::CONTENT_TYPE, content_type);
    }
    builder.body(axum::body::Body::from(body)).map_err(|e| {
        ApiError::Internal(anyhow::anyhow!("Recorder clear response build failed: {}", e))
    })
}

/// Replay a captured request against the deployment. The recorder
/// records the request envelope; replay re-executes it and returns the
/// fresh response — useful for "did the bug we captured get fixed yet"
/// or "check whether a chaos rule still triggers."
pub async fn replay_recorder_capture(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((deployment_id, capture_id)): Path<(Uuid, String)>,
) -> ApiResult<axum::http::Response<axum::body::Body>> {
    let deployment = check_org_access(&state, user_id, &headers, deployment_id).await?;
    if capture_id.contains('/') || capture_id.contains('?') || capture_id.contains('#') {
        return Err(ApiError::InvalidRequest("Invalid capture id".to_string()));
    }
    let path = format!("/api/recorder/replay/{}", urlencoding::encode(&capture_id));
    proxy_post_to_deployment_recorder(&deployment, &path).await
}

/// Export the deployment's recorder captures as HAR. Proxies the
/// recorder's existing `/api/recorder/export/har` endpoint and forwards
/// the response unchanged. The browser handles the download via a blob
/// URL on the UI side.
pub async fn export_recorder_captures_har(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(deployment_id): Path<Uuid>,
) -> ApiResult<axum::http::Response<axum::body::Body>> {
    let pool = state.db.pool();

    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".to_string()))?;

    if deployment.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "You don't have access to this deployment".to_string(),
        ));
    }

    let _ = state;
    proxy_to_deployment_recorder(&deployment, "/api/recorder/export/har").await
}

/// Export the deployment's recorder captures as JSONL — one
/// `RecordedExchange` per line, all protocols. Companion to the HAR
/// export. JSONL is the format the local `mockforge-cli replay` reads,
/// so this is the round-trip path: capture in cloud → download → replay
/// locally.
pub async fn export_recorder_captures_jsonl(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(deployment_id): Path<Uuid>,
) -> ApiResult<axum::http::Response<axum::body::Body>> {
    let deployment = check_org_access(&state, user_id, &headers, deployment_id).await?;
    proxy_to_deployment_recorder(&deployment, "/api/recorder/export/jsonl").await
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

    // Prefer live metrics from Fly Managed Prometheus when configured.
    // Falls back to the local `deployment_metrics` table when:
    //   * Fly Prometheus env vars aren't set (dev / self-hosted), or
    //   * the query fails (transient network / auth error).
    if let Some(client) = crate::fly_metrics::global() {
        let app_name = deployment.fly_app_name();
        match client.snapshot_for_app(&app_name).await {
            Ok(snap) => {
                use chrono::Datelike;
                let now = chrono::Utc::now().date_naive();
                let period_start =
                    chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap_or(now);
                return Ok(Json(MetricsResponse {
                    requests: snap.requests,
                    requests_2xx: snap.requests_2xx,
                    requests_4xx: snap.requests_4xx,
                    requests_5xx: snap.requests_5xx,
                    egress_bytes: snap.egress_bytes,
                    avg_response_time_ms: snap.avg_response_time_ms,
                    period_start,
                }));
            }
            Err(err) => {
                tracing::warn!(
                    app_name = %app_name,
                    error = %err,
                    "Fly Prometheus metrics query failed; falling back to local counters"
                );
            }
        }
    }

    // Fallback: return the local aggregate counters. Until the in-container
    // log shipper lands (#232) this table has no writer and returns zeros.
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
    /// Protocols to expose on the deployment. HTTP is implicit and always
    /// included. Items beyond Free-tier (gRPC, brokers) require a higher
    /// plan and are rejected at create time if the org isn't entitled.
    /// Persisted into `config_json["enabled_protocols"]`.
    #[serde(default)]
    pub enabled_protocols: Option<Vec<crate::models::Protocol>>,
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
    pub region: String,
    pub instance_type: String,
    pub health_status: String,
    pub error_message: Option<String>,
    pub enabled_protocols: Vec<crate::models::Protocol>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<HostedMock> for DeploymentResponse {
    fn from(mock: HostedMock) -> Self {
        let status = mock.status().to_string();
        let health_status = mock.health_status().to_string();
        let enabled_protocols = mock.enabled_protocols();
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
            region: mock.region,
            instance_type: mock.instance_type,
            health_status,
            error_message: mock.error_message,
            enabled_protocols,
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

#[derive(Debug, Deserialize)]
pub struct SetDomainRequest {
    pub domain: String,
}

/// Set a custom domain for a deployment.
///
/// Adds a TLS certificate on the registry server Fly.io app so that
/// `<slug>.<domain>` terminates TLS here, then the proxy fallback
/// handler forwards traffic to the deployment's internal URL.
pub async fn set_domain(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(deployment_id): Path<Uuid>,
    Json(request): Json<SetDomainRequest>,
) -> ApiResult<Json<serde_json::Value>> {
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

    // Get deployment
    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".to_string()))?;

    // Verify ownership
    if deployment.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest("Deployment not found".to_string()));
    }

    let hostname = format!("{}.{}", deployment.slug, request.domain);

    // Update deployment URL to use the custom domain.
    // A wildcard TLS cert on the registry app covers all subdomains,
    // so no per-deployment certificate management is needed.
    let new_url = format!("https://{}", hostname);
    sqlx::query("UPDATE hosted_mocks SET deployment_url = $1, updated_at = NOW() WHERE id = $2")
        .bind(&new_url)
        .bind(deployment_id)
        .execute(pool)
        .await
        .map_err(|e| {
            ApiError::Internal(anyhow::anyhow!("Failed to update deployment URL: {}", e))
        })?;

    DeploymentLog::create(
        pool,
        deployment_id,
        "info",
        &format!("Custom domain set: {}", hostname),
        None,
    )
    .await
    .ok();

    Ok(Json(serde_json::json!({
        "hostname": hostname,
        "deployment_url": new_url,
    })))
}
