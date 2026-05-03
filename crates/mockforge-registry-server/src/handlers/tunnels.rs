//! Tunnel reservation handlers (cloud-enablement task #5 / Phase 1).
//!
//! Reservations are durable subdomain claims; the relay binary
//! (mockforge-tunnel server, separate deployment) reads them on
//! connection auth and writes session rows back through internal mTLS
//! routes (separate slice).
//!
//! Routes:
//!   GET    /api/v1/organizations/{org_id}/tunnels
//!   POST   /api/v1/organizations/{org_id}/tunnels
//!   GET    /api/v1/tunnels/{id}
//!   PATCH  /api/v1/tunnels/{id}
//!   DELETE /api/v1/tunnels/{id}
//!   POST   /api/v1/tunnels/{id}/verify-custom-domain   (DNS proof check)

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use mockforge_registry_core::models::tunnel::{is_valid_subdomain, CreateTunnelReservation};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    handlers::usage::effective_limits,
    middleware::{resolve_org_context, AuthUser},
    models::TunnelReservation,
    AppState,
};

/// `GET /api/v1/organizations/{org_id}/tunnels`
pub async fn list_tunnels(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<TunnelReservation>>> {
    authorize_org(&state, user_id, &headers, org_id).await?;
    let tunnels = TunnelReservation::list_by_org(state.db.pool(), org_id)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(tunnels))
}

#[derive(Debug, Deserialize)]
pub struct CreateTunnelRequest {
    pub name: String,
    pub subdomain: String,
    #[serde(default)]
    pub workspace_id: Option<Uuid>,
    #[serde(default)]
    pub custom_domain: Option<String>,
}

/// `POST /api/v1/organizations/{org_id}/tunnels`
///
/// Enforces both `max_tunnel_reservations` and `max_custom_domains` plan
/// limits before insert. Subdomain uniqueness is enforced at the DB
/// level by the unique index; we surface a friendlier error here when
/// the conflict is observable up-front.
pub async fn create_tunnel(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateTunnelRequest>,
) -> ApiResult<Json<TunnelReservation>> {
    let ctx = resolve_org_context_for_org(&state, user_id, &headers, org_id).await?;

    // 1. Validate subdomain shape.
    if !is_valid_subdomain(&request.subdomain) {
        return Err(ApiError::InvalidRequest(
            "subdomain must be 3-40 lowercase alphanumeric chars (hyphens \
             allowed in the middle)"
                .into(),
        ));
    }

    // 2. Plan-limit checks.
    let limits = effective_limits(&state, &ctx.org).await?;
    let max_reservations =
        limits.get("max_tunnel_reservations").and_then(|v| v.as_i64()).unwrap_or(0);
    if max_reservations == 0 {
        return Err(ApiError::ResourceLimitExceeded(
            "Tunnels are not enabled on this plan — upgrade to Pro or Team".into(),
        ));
    }
    if max_reservations > 0 {
        let used = TunnelReservation::count_by_org(state.db.pool(), org_id)
            .await
            .map_err(ApiError::Database)?;
        if used >= max_reservations {
            return Err(ApiError::ResourceLimitExceeded(format!(
                "Tunnel reservation limit reached ({used}/{max_reservations}). \
                 Delete an unused tunnel or upgrade your plan."
            )));
        }
    }

    if request.custom_domain.is_some() {
        let max_custom = limits.get("max_custom_domains").and_then(|v| v.as_i64()).unwrap_or(0);
        if max_custom == 0 {
            return Err(ApiError::ResourceLimitExceeded(
                "Custom domains are not available on this plan".into(),
            ));
        }
    }

    // 3. Subdomain pre-check (the unique index is the authoritative guard;
    //    this is for a friendlier error before hitting it).
    if let Some(existing) =
        TunnelReservation::find_by_subdomain(state.db.pool(), &request.subdomain)
            .await
            .map_err(ApiError::Database)?
    {
        let _ = existing;
        return Err(ApiError::InvalidRequest(format!(
            "Subdomain '{}' is already taken",
            request.subdomain
        )));
    }

    // 4. Create.
    let tunnel = TunnelReservation::create(
        state.db.pool(),
        CreateTunnelReservation {
            org_id,
            workspace_id: request.workspace_id,
            name: &request.name,
            subdomain: &request.subdomain,
            custom_domain: request.custom_domain.as_deref(),
            created_by: Some(user_id),
        },
    )
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(tunnel))
}

/// `GET /api/v1/tunnels/{id}`
pub async fn get_tunnel(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<TunnelReservation>> {
    let tunnel = load_authorized_tunnel(&state, user_id, &headers, id).await?;
    Ok(Json(tunnel))
}

#[derive(Debug, Deserialize)]
pub struct UpdateTunnelRequest {
    #[serde(default)]
    pub name: Option<String>,
    /// Outer Option = "field present"; inner = "set to NULL" (clears
    /// the custom domain). Setting a new custom_domain resets the
    /// `custom_domain_verified` flag — verification has to re-run
    /// against the new DNS record.
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub custom_domain: Option<Option<String>>,
}

/// `PATCH /api/v1/tunnels/{id}`
pub async fn update_tunnel(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<UpdateTunnelRequest>,
) -> ApiResult<Json<TunnelReservation>> {
    load_authorized_tunnel(&state, user_id, &headers, id).await?;

    let updated = TunnelReservation::update(
        state.db.pool(),
        id,
        request.name.as_deref(),
        request.custom_domain.as_ref().map(|d| d.as_deref()),
    )
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::InvalidRequest("Tunnel not found".into()))?;
    Ok(Json(updated))
}

/// `DELETE /api/v1/tunnels/{id}`
pub async fn delete_tunnel(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    load_authorized_tunnel(&state, user_id, &headers, id).await?;

    let deleted = TunnelReservation::delete(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Tunnel not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// `POST /api/v1/tunnels/{id}/verify-custom-domain`
///
/// Stub for now — DNS proof verification (CNAME → t.mockforge.dev)
/// happens in a follow-up slice. This endpoint exists so the UI flow
/// is wired even if it currently always fails.
pub async fn verify_custom_domain(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<TunnelReservation>> {
    let tunnel = load_authorized_tunnel(&state, user_id, &headers, id).await?;
    if tunnel.custom_domain.is_none() {
        return Err(ApiError::InvalidRequest("Tunnel has no custom domain to verify".into()));
    }
    // TODO: actual DNS lookup + CNAME validation in follow-up slice.
    // For now this returns 501-style error so the UI knows verification
    // isn't ready yet.
    Err(ApiError::InvalidRequest(
        "Custom domain verification is not yet implemented".into(),
    ))
}

/// Verify caller belongs to `org_id` and return the OrgContext.
async fn resolve_org_context_for_org(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    org_id: Uuid,
) -> ApiResult<crate::middleware::org_context::OrgContext> {
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != org_id {
        return Err(ApiError::InvalidRequest("Cannot access tunnels for a different org".into()));
    }
    Ok(ctx)
}

async fn authorize_org(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    org_id: Uuid,
) -> ApiResult<()> {
    resolve_org_context_for_org(state, user_id, headers, org_id).await?;
    Ok(())
}

/// Fetch a tunnel and verify the caller belongs to its org. Cross-org
/// reads return "not found" rather than "forbidden" — same pattern as
/// other handlers.
async fn load_authorized_tunnel(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<TunnelReservation> {
    let tunnel = TunnelReservation::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Tunnel not found".into()))?;
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != tunnel.org_id {
        return Err(ApiError::InvalidRequest("Tunnel not found".into()));
    }
    Ok(tunnel)
}

/// PATCH-semantics double-option deserializer (same pattern as test_suites).
fn deserialize_double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}
