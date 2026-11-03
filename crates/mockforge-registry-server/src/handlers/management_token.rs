//! A hosted mock's management token (#1085): what the machine is told to
//! require on control-plane writes, and what the registry sends when it
//! calls those routes on the owner's behalf.

use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderValue},
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
    token_with_secret(deployment_id, std::env::var("JWT_SECRET").ok().as_deref())
}

fn token_with_secret(deployment_id: Uuid, secret: Option<&str>) -> Option<String> {
    secret
        .filter(|secret| !secret.is_empty())
        .map(|secret| deployment_management_token(deployment_id, secret))
}

/// The env entry that gives a machine its management token. Without it the
/// runtime locks its control plane (#1011), so a missing `JWT_SECRET` is
/// logged: the mock would serve traffic but nobody could manage it.
pub fn env_entry(deployment_id: Uuid) -> Option<(String, String)> {
    let entry = env_entry_with_secret(deployment_id, std::env::var("JWT_SECRET").ok().as_deref());
    if entry.is_none() {
        tracing::error!(
            %deployment_id,
            "JWT_SECRET is unset: hosted mock gets no management token, so its control plane \
             will be locked"
        );
    }
    entry
}

fn env_entry_with_secret(deployment_id: Uuid, secret: Option<&str>) -> Option<(String, String)> {
    token_with_secret(deployment_id, secret).map(|token| (MANAGEMENT_TOKEN_ENV.to_string(), token))
}

/// A client for calling one deployment's runtime, carrying its token
pub fn proxy_client(deployment_id: Uuid, timeout: Duration) -> reqwest::Result<reqwest::Client> {
    client_with_token(token_for(deployment_id), timeout)
}

fn client_with_token(token: Option<String>, timeout: Duration) -> reqwest::Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    if let Some(value) = token.and_then(|t| HeaderValue::from_str(&t).ok()) {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The runtime refuses tokens shorter than this
    /// (`mockforge_http::management_auth::MIN_TOKEN_LEN`).
    const RUNTIME_MIN_TOKEN_LEN: usize = 32;

    #[test]
    fn provisioning_delivers_a_token_the_runtime_accepts() {
        let deployment = Uuid::new_v4();
        let (name, token) =
            env_entry_with_secret(deployment, Some("registry-jwt-secret")).expect("token");
        assert_eq!(name, "MOCKFORGE_MANAGEMENT_TOKEN");
        assert!(token.len() >= RUNTIME_MIN_TOKEN_LEN, "{token}");
        assert!(token.starts_with("mfm_"));
        // HeaderValue-safe, and usable as a WebSocket subprotocol token.
        assert!(token.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
        // Stable per deployment, distinct across deployments.
        assert_eq!(
            env_entry_with_secret(deployment, Some("registry-jwt-secret")).unwrap().1,
            token
        );
        assert_ne!(
            env_entry_with_secret(Uuid::new_v4(), Some("registry-jwt-secret")).unwrap().1,
            token
        );
        // The owner-facing endpoint and the machine see the same value.
        assert_eq!(deployment_management_token(deployment, "registry-jwt-secret"), token);
    }

    #[test]
    fn no_secret_means_no_token() {
        assert!(env_entry_with_secret(Uuid::new_v4(), None).is_none());
        assert!(env_entry_with_secret(Uuid::new_v4(), Some("")).is_none());
    }

    #[tokio::test]
    async fn proxy_client_sends_the_token() {
        use axum::{http::HeaderMap as AxumHeaders, routing::get, Router};
        let app = Router::new().route(
            "/__mockforge/routes",
            get(|headers: AxumHeaders| async move {
                headers
                    .get(MANAGEMENT_TOKEN_HEADER)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("missing")
                    .to_string()
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await });
        let token = token_with_secret(Uuid::new_v4(), Some("registry-jwt-secret")).unwrap();
        let client = client_with_token(Some(token.clone()), Duration::from_secs(5)).unwrap();
        let echoed = client
            .get(format!("http://{addr}/__mockforge/routes"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert_eq!(echoed, token);
        server.abort();
    }
}
