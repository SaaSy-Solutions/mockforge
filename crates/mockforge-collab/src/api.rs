//! REST API endpoints for collaboration

use crate::auth::{AuthService, Credentials};
use crate::error::{CollabError, Result};
use crate::middleware::{auth_middleware, AuthUser};
use crate::models::UserRole;
use crate::user::UserService;
use crate::workspace::WorkspaceService;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// API state
#[derive(Clone)]
pub struct ApiState {
    pub auth: Arc<AuthService>,
    pub user: Arc<UserService>,
    pub workspace: Arc<WorkspaceService>,
}

/// Create API router
pub fn create_router(state: ApiState) -> Router {
    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check));

    // Protected routes (authentication required)
    let protected_routes = Router::new()
        // Workspaces
        .route("/workspaces", post(create_workspace))
        .route("/workspaces", get(list_workspaces))
        .route("/workspaces/:id", get(get_workspace))
        .route("/workspaces/:id", put(update_workspace))
        .route("/workspaces/:id", delete(delete_workspace))
        // Members
        .route("/workspaces/:id/members", post(add_member))
        .route("/workspaces/:id/members/:user_id", delete(remove_member))
        .route("/workspaces/:id/members/:user_id/role", put(change_role))
        .route("/workspaces/:id/members", get(list_members))
        .route_layer(middleware::from_fn_with_state(
            state.auth.clone(),
            auth_middleware,
        ));

    // Combine routes
    Router::new().merge(public_routes).merge(protected_routes).with_state(state)
}

// ===== Request/Response Types =====

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: Uuid,
    pub role: UserRole,
}

#[derive(Debug, Deserialize)]
pub struct ChangeRoleRequest {
    pub role: UserRole,
}

// ===== Error Handling =====

impl IntoResponse for CollabError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            CollabError::AuthenticationFailed(msg) => (StatusCode::UNAUTHORIZED, msg),
            CollabError::AuthorizationFailed(msg) => (StatusCode::FORBIDDEN, msg),
            CollabError::WorkspaceNotFound(msg) => (StatusCode::NOT_FOUND, msg),
            CollabError::UserNotFound(msg) => (StatusCode::NOT_FOUND, msg),
            CollabError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg),
            CollabError::AlreadyExists(msg) => (StatusCode::CONFLICT, msg),
            CollabError::Timeout(msg) => (StatusCode::REQUEST_TIMEOUT, msg),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

// ===== Handler Functions =====

/// Register a new user
async fn register(
    State(state): State<ApiState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>> {
    // Create user
    let user = state
        .user
        .create_user(payload.username, payload.email, payload.password)
        .await?;

    // Generate token
    let token = state.auth.generate_token(&user)?;

    Ok(Json(AuthResponse {
        access_token: token.access_token,
        token_type: token.token_type,
        expires_at: token.expires_at.to_rfc3339(),
    }))
}

/// Login user
async fn login(
    State(state): State<ApiState>,
    Json(payload): Json<Credentials>,
) -> Result<Json<AuthResponse>> {
    // Authenticate user
    let user = state.user.authenticate(&payload.username, &payload.password).await?;

    // Generate token
    let token = state.auth.generate_token(&user)?;

    Ok(Json(AuthResponse {
        access_token: token.access_token,
        token_type: token.token_type,
        expires_at: token.expires_at.to_rfc3339(),
    }))
}

/// Create a new workspace
async fn create_workspace(
    State(state): State<ApiState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateWorkspaceRequest>,
) -> Result<Json<serde_json::Value>> {
    // Create workspace
    let workspace = state
        .workspace
        .create_workspace(payload.name, payload.description, auth_user.user_id)
        .await?;

    Ok(Json(serde_json::to_value(workspace)?))
}

/// List user's workspaces
async fn list_workspaces(
    State(state): State<ApiState>,
    auth_user: AuthUser,
) -> Result<Json<serde_json::Value>> {
    // List workspaces
    let workspaces = state.workspace.list_user_workspaces(auth_user.user_id).await?;

    Ok(Json(serde_json::to_value(workspaces)?))
}

/// Get workspace by ID
async fn get_workspace(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
    auth_user: AuthUser,
) -> Result<Json<serde_json::Value>> {
    // Verify user is a member
    let _member = state.workspace.get_member(id, auth_user.user_id).await?;

    // Get workspace
    let workspace = state.workspace.get_workspace(id).await?;

    Ok(Json(serde_json::to_value(workspace)?))
}

/// Update workspace
async fn update_workspace(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
    auth_user: AuthUser,
    Json(payload): Json<UpdateWorkspaceRequest>,
) -> Result<Json<serde_json::Value>> {
    // Update workspace (permission check inside)
    let workspace = state
        .workspace
        .update_workspace(id, auth_user.user_id, payload.name, payload.description, None)
        .await?;

    Ok(Json(serde_json::to_value(workspace)?))
}

/// Delete workspace
async fn delete_workspace(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
    auth_user: AuthUser,
) -> Result<StatusCode> {
    // Delete workspace (permission check inside)
    state.workspace.delete_workspace(id, auth_user.user_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Add member to workspace
async fn add_member(
    State(state): State<ApiState>,
    Path(workspace_id): Path<Uuid>,
    auth_user: AuthUser,
    Json(payload): Json<AddMemberRequest>,
) -> Result<Json<serde_json::Value>> {
    // Add member (permission check inside)
    let member = state
        .workspace
        .add_member(workspace_id, auth_user.user_id, payload.user_id, payload.role)
        .await?;

    Ok(Json(serde_json::to_value(member)?))
}

/// Remove member from workspace
async fn remove_member(
    State(state): State<ApiState>,
    Path((workspace_id, member_user_id)): Path<(Uuid, Uuid)>,
    auth_user: AuthUser,
) -> Result<StatusCode> {
    // Remove member (permission check inside)
    state
        .workspace
        .remove_member(workspace_id, auth_user.user_id, member_user_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Change member role
async fn change_role(
    State(state): State<ApiState>,
    Path((workspace_id, member_user_id)): Path<(Uuid, Uuid)>,
    auth_user: AuthUser,
    Json(payload): Json<ChangeRoleRequest>,
) -> Result<Json<serde_json::Value>> {
    // Change role (permission check inside)
    let member = state
        .workspace
        .change_role(workspace_id, auth_user.user_id, member_user_id, payload.role)
        .await?;

    Ok(Json(serde_json::to_value(member)?))
}

/// List workspace members
async fn list_members(
    State(state): State<ApiState>,
    Path(workspace_id): Path<Uuid>,
    auth_user: AuthUser,
) -> Result<Json<serde_json::Value>> {
    // Verify user is a member
    let _member = state.workspace.get_member(workspace_id, auth_user.user_id).await?;

    // List all members
    let members = state.workspace.list_members(workspace_id).await?;

    Ok(Json(serde_json::to_value(members)?))
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

/// Readiness check endpoint
async fn readiness_check() -> impl IntoResponse {
    // TODO: Check database connection
    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        // Just ensure router can be created
        let state = ApiState {
            auth: Arc::new(AuthService::new("test".to_string())),
            workspace: Arc::new(WorkspaceService::new(todo!())),
        };
        let _router = create_router(state);
    }
}
