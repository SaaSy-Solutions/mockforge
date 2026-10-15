//! Publish authored static requests to the configured HTTP runtime.
use super::projection::workspace_runtime_mocks;
use super::transfer::authorize_workspace_transfer;
use super::*;

/// HTTP runtime address supplied by the admin server's own configuration.
#[derive(Clone)]
pub struct WorkspaceHttpRuntime {
    /// Main HTTP listener; clients cannot choose a publication destination.
    pub address: Option<std::net::SocketAddr>,
}

/// Publish namespaced static responses to the configured HTTP runtime.
pub async fn set_active_workspace(
    State(state): State<WorkspaceState>,
    Path(workspace_id): Path<String>,
    runtime: Option<axum::Extension<WorkspaceHttpRuntime>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<serde_json::Value>>, Response> {
    authorize_workspace_transfer(
        &headers,
        mockforge_collab::permissions::Permission::ManageSettings,
    )?;
    let address = runtime
        .and_then(|runtime| runtime.address)
        .ok_or_else(|| StatusCode::SERVICE_UNAVAILABLE.into_response())?;
    let workspace = state
        .registry
        .read()
        .await
        .get_workspace(&workspace_id)
        .map_err(|_| StatusCode::NOT_FOUND.into_response())?
        .workspace;
    let mocks = workspace_runtime_mocks(&workspace, &workspace_id)?;
    let address = if address.ip().is_unspecified() {
        std::net::SocketAddr::new(
            if address.is_ipv4() {
                std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
            } else {
                std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)
            },
            address.port(),
        )
    } else {
        address
    };
    let response = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .post(format!(
            "http://{address}/__mockforge/api/workspaces/{}/activate",
            urlencoding::encode(&workspace_id)
        ))
        .json(&mocks)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY.into_response())?;
    if !response.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY.into_response());
    }
    let result = response.json().await.map_err(|_| StatusCode::BAD_GATEWAY.into_response())?;
    Ok(Json(ApiResponse::success(result)))
}
