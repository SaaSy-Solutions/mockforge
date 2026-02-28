//! Hosted Mocks deployment handlers
//!
//! Provides endpoints for deploying, managing, and monitoring cloud-hosted mock services

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    deployment::flyio::FlyioClient,
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
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

    // Check plan limits
    let limits = &org_ctx.org.limits_json;
    let max_hosted_mocks = limits.get("max_hosted_mocks").and_then(|v| v.as_i64()).unwrap_or(0);

    if max_hosted_mocks >= 0 {
        // Count existing active deployments
        let existing = HostedMock::find_by_org(pool, org_ctx.org_id)
            .await
            .map_err(|e| ApiError::Database(e))?;

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
    if HostedMock::find_by_slug(pool, org_ctx.org_id, &slug)
        .await
        .map_err(|e| ApiError::Database(e))?
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
        &slug,
        request.description.as_deref(),
        request.config_json,
        request.openapi_spec_url.as_deref(),
    )
    .await
    .map_err(|e| ApiError::Database(e))?;

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
    .map_err(|e| ApiError::Database(e))?;

    // Mark as pending - the deployment orchestrator will pick it up and deploy it
    // The orchestrator polls for pending/deploying deployments every 10 seconds
    HostedMock::update_status(pool, deployment.id, DeploymentStatus::Pending, None)
        .await
        .map_err(|e| ApiError::Database(e))?;

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
        .map_err(|e| ApiError::Database(e))?
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
        .map_err(|e| ApiError::Database(e))?;

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
        .map_err(|e| ApiError::Database(e))?
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

    // Get deployment
    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(|e| ApiError::Database(e))?
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
        .map_err(|e| ApiError::Database(e))?;

    // Update URLs if provided
    if request.deployment_url.is_some() || request.internal_url.is_some() {
        HostedMock::update_urls(
            pool,
            deployment_id,
            request.deployment_url.as_deref(),
            request.internal_url.as_deref(),
        )
        .await
        .map_err(|e| ApiError::Database(e))?;
    }

    // Get updated deployment
    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| {
            ApiError::Internal(anyhow::anyhow!("Failed to retrieve updated deployment"))
        })?;

    // Send deployment status notification email (non-blocking)
    if let Ok(org) = Organization::find_by_id(pool, deployment.org_id).await {
        if let Some(org) = org {
            if let Ok(owner) = User::find_by_id(pool, org.owner_id).await {
                if let Some(owner) = owner {
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

    // Get deployment
    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(|e| ApiError::Database(e))?
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
        .map_err(|e| ApiError::Database(e))?;

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
                    .ok(); // Log but don't fail on log error

                    // Check if there are other machines for this app
                    // If not, we could delete the app, but for now we'll leave it
                    // (apps can be reused for future deployments)
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
                    // Continue with database deletion even if Fly.io deletion fails
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
                    // Continue with database deletion
                }
            }
        }
    }

    // Soft delete from database
    HostedMock::delete(pool, deployment_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    DeploymentLog::create(pool, deployment_id, "info", "Deployment deleted successfully", None)
        .await
        .ok(); // Log but don't fail on log error

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Deployment deleted"
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
        .map_err(|e| ApiError::Database(e))?
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
        .map_err(|e| ApiError::Database(e))?;

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
        .map_err(|e| ApiError::Database(e))?
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
        .map_err(|e| ApiError::Database(e))?;

    Ok(Json(MetricsResponse::from(metrics)))
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
