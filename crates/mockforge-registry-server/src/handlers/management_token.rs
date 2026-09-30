//! A hosted mock's management token (#1085): what the machine is told to
//! require on control-plane writes, and what the registry sends when it
//! calls those routes on the owner's behalf.

use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use mockforge_registry_core::auth::deployment_management_token;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    error::ApiResult,
    handlers::resilience::authorize_deployment,
    middleware::{permission_check::PermissionChecker, permissions::Permission, AuthUser},
    AppState,
};

/// Env var the runtime reads (`mockforge_http::management_auth`)
pub const MANAGEMENT_TOKEN_ENV: &str = "MOCKFORGE_MANAGEMENT_TOKEN";
/// Header the runtime accepts the token in
pub const MANAGEMENT_TOKEN_HEADER: &str = "x-mockforge-management-token";

/// The deployment's token, or `None` when `JWT_SECRET` is unset
pub fn token_for(deployment_id: Uuid) -> Option<String> {
    std::env::var("JWT_SECRET")
        .ok()
        .filter(|secret| !secret.is_empty())
        .map(|secret| deployment_management_token(deployment_id, &secret))
}

/// The env entry that turns the gate on for a machine
pub fn env_entry(deployment_id: Uuid) -> Option<(String, String)> {
    token_for(deployment_id).map(|token| (MANAGEMENT_TOKEN_ENV.to_string(), token))
}

/// A client for calling one deployment's runtime, carrying its token
pub fn proxy_client(deployment_id: Uuid, timeout: Duration) -> reqwest::Result<reqwest::Client> {
    let mut headers = reqwest::header::HeaderMap::new();
    if let Some(value) =
        token_for(deployment_id).and_then(|t| reqwest::header::HeaderValue::from_str(&t).ok())
    {
        headers.insert(MANAGEMENT_TOKEN_HEADER, value);
    }
    reqwest::Client::builder().timeout(timeout).default_headers(headers).build()
}

/// Response for `GET /api/v1/hosted-mocks/{id}/management-token`
#[derive(Debug, Serialize)]
pub struct ManagementTokenResponse {
    /// Send as `X-MockForge-Management-Token` on control-plane writes
    pub token: String,
    /// The header name to send it in
    pub header: &'static str,
}

/// `GET /api/v1/hosted-mocks/{deployment_id}/management-token`, for owners
/// who drive their hosted mock with an SDK or curl.
pub async fn get_management_token(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<ManagementTokenResponse>> {
    let deployment = authorize_deployment(&state, user_id, &headers, deployment_id).await?;
    PermissionChecker::new(&state)
        .require_permission(user_id, deployment.org_id, Permission::HostedMockUpdate)
        .await?;
    Ok(Json(ManagementTokenResponse {
        token: deployment_management_token(deployment.id, &state.config.jwt_secret),
        header: "X-MockForge-Management-Token",
    }))
}
