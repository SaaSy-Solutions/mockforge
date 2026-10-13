//! Explicit, tenant-checked workspace attribution for hosted deployments.
use super::{DeploymentResponse, HostedMock};
use crate::{
    error::{ApiError, ApiResult},
    middleware::{
        permission_check::PermissionChecker, permissions::Permission, resolve_org_context, AuthUser,
    },
    AppState,
};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

pub(super) async fn validate(
    state: &AppState,
    org: Uuid,
    workspace: Option<Uuid>,
) -> ApiResult<()> {
    let Some(workspace) = workspace else {
        return Ok(());
    };
    let owned: bool = crate::store::with_org_context(state.db.runtime_pool(), org, move |tx| {
        Box::pin(async move {
            Ok(sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM workspaces WHERE id=$1 AND org_id=$2)",
            )
            .bind(workspace)
            .bind(org)
            .fetch_one(&mut **tx)
            .await?)
        })
    })
    .await?;
    if !owned {
        return Err(ApiError::InvalidRequest(
            "Workspace does not belong to this organization".into(),
        ));
    }
    Ok(())
}

pub(super) async fn response(
    state: &AppState,
    org: Uuid,
    deployment: HostedMock,
) -> ApiResult<DeploymentResponse> {
    let id = deployment.id;
    let workspace: Option<Uuid> =
        crate::store::with_org_context(state.db.runtime_pool(), org, move |tx| {
            Box::pin(async move {
                Ok(sqlx::query_scalar::<_, Option<Uuid>>(
                    "SELECT workspace_id FROM hosted_mocks WHERE id=$1 AND org_id=$2",
                )
                .bind(id)
                .bind(org)
                .fetch_optional(&mut **tx)
                .await?
                .flatten())
            })
        })
        .await?;
    let mut response = DeploymentResponse::from(deployment);
    response.workspace_id = workspace;
    Ok(response)
}

pub(super) async fn responses(
    state: &AppState,
    org: Uuid,
    deployments: Vec<HostedMock>,
) -> ApiResult<Vec<DeploymentResponse>> {
    if deployments.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<(Uuid, Option<Uuid>)> = crate::store::with_org_context(state.db.runtime_pool(), org, move |tx| {
        Box::pin(async move {
            Ok(sqlx::query_as("SELECT id,workspace_id FROM hosted_mocks WHERE org_id=$1 AND deleted_at IS NULL")
                .bind(org).fetch_all(&mut **tx).await?)
        })
    }).await?;
    let workspaces: std::collections::HashMap<_, _> = rows.into_iter().collect();
    Ok(deployments
        .into_iter()
        .map(|deployment| {
            let workspace = workspaces.get(&deployment.id).copied().flatten();
            let mut response = DeploymentResponse::from(deployment);
            response.workspace_id = workspace;
            response
        })
        .collect())
}

/// Required field: an explicit null detaches; an omitted key is rejected.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceRequest {
    pub workspace_id: serde_json::Value,
}

/// Change attribution for future capture ingestion; historical rows retain scope.
pub async fn update(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(request): Json<WorkspaceRequest>,
) -> ApiResult<Json<DeploymentResponse>> {
    let org = resolve_org_context(&state, user, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?
        .org_id;
    PermissionChecker::new(&state)
        .require_permission(user, org, Permission::HostedMockUpdate)
        .await?;
    let workspace: Option<Uuid> = serde_json::from_value(request.workspace_id)
        .map_err(|_| ApiError::InvalidRequest("workspace_id must be a UUID or null".into()))?;
    validate(&state, org, workspace).await?;
    let deployment = crate::store::with_org_context(state.db.runtime_pool(), org, move |tx| {
        Box::pin(async move {
            Ok(sqlx::query_as::<_, HostedMock>("UPDATE hosted_mocks SET workspace_id=$3,updated_at=NOW() WHERE id=$1 AND org_id=$2 AND deleted_at IS NULL RETURNING *")
                .bind(id).bind(org).bind(workspace).fetch_optional(&mut **tx).await?)
        })
    }).await?.ok_or_else(|| ApiError::InvalidRequest("Deployment not found".into()))?;
    super::log_deployment_event(
        state.db.runtime_pool(),
        org,
        id,
        "info",
        "Workspace attribution updated",
        Some(serde_json::json!({"workspace_id":workspace,"user_id":user})),
    )
    .await?;
    let mut response = DeploymentResponse::from(deployment);
    response.workspace_id = workspace;
    Ok(Json(response))
}
