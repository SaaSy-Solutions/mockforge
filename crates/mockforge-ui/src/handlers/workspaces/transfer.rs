//! Authenticate and transfer versioned live workspace configuration.
use super::*;

pub(super) fn authorize_workspace_transfer(
    headers: &axum::http::HeaderMap,
    permission: mockforge_collab::permissions::Permission,
) -> Result<(), Response> {
    // Transfers require an explicit valid token even when other local read APIs
    // are public or development mode supplies a default admin context.
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(|| StatusCode::UNAUTHORIZED.into_response())?;
    let claims =
        crate::auth::validate_token(token).map_err(|_| StatusCode::UNAUTHORIZED.into_response())?;
    let user = crate::auth::claims_to_user_context(&claims);
    if !mockforge_collab::permissions::RolePermissions::has_permission(user.role, permission) {
        return Err(StatusCode::FORBIDDEN.into_response());
    }
    Ok(())
}

/// Export authored local workspace configuration with credential references.
pub async fn export_live_workspace(
    State(state): State<WorkspaceState>,
    Path(workspace_id): Path<String>,
    headers: axum::http::HeaderMap,
) -> Result<Json<mockforge_core::workspace_export::WorkspaceExportDocument>, Response> {
    authorize_workspace_transfer(
        &headers,
        mockforge_collab::permissions::Permission::WorkspaceRead,
    )?;
    let registry = state.registry.read().await;
    let mut workspace = registry
        .get_workspace(&workspace_id)
        .map_err(|_| StatusCode::NOT_FOUND.into_response())?
        .workspace;
    workspace.id = workspace_id;
    let snapshot =
        mockforge_core::workspace_export::WorkspaceExportDocument::from_workspace(&workspace)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    Ok(Json(snapshot))
}

/// Import resolved authored configuration as a new workspace without overwriting existing state.
pub async fn import_live_workspace(
    State(state): State<WorkspaceState>,
    headers: axum::http::HeaderMap,
    Json(input): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<CreatedItemResponse>), Response> {
    authorize_workspace_transfer(
        &headers,
        mockforge_collab::permissions::Permission::ManageSettings,
    )?;
    let id = input
        .get("id")
        .and_then(serde_json::Value::as_str)
        .filter(|id| !id.is_empty() && id.len() <= 256)
        .ok_or_else(|| StatusCode::BAD_REQUEST.into_response())?
        .to_owned();
    let mut workspace: Workspace = serde_json::from_value(
        input.get("workspace").cloned().unwrap_or_default(),
    )
    .map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"Invalid workspace configuration"})),
        )
            .into_response()
    })?;
    workspace.id = id.clone();
    // Imported metadata denotes this new local authoring instance, while exports
    // normalize it so a subsequent unchanged export still has a clean Git diff.
    workspace.created_at = chrono::Utc::now();
    workspace.updated_at = workspace.created_at;
    let mut registry = state.registry.write().await;
    if registry.workspace_exists(&id) {
        return Err(StatusCode::CONFLICT.into_response());
    }
    registry
        .register_workspace(id.clone(), workspace)
        .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
    Ok((
        StatusCode::CREATED,
        Json(CreatedItemResponse {
            id,
            message: "Workspace imported".into(),
        }),
    ))
}
