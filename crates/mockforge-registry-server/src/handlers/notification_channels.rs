//! Notification channel CRUD (cloud-enablement task #3 / Phase 1, follow-up).
//!
//! Each org configures one or more places where incident notifications
//! land: email recipients, Slack webhook, PagerDuty integration key,
//! generic outbound webhook. The dispatcher worker (separate slice) reads
//! these rows when fanning out a triggered incident.
//!
//! Routes:
//!   GET    /api/v1/organizations/{org_id}/notification-channels
//!   POST   /api/v1/organizations/{org_id}/notification-channels
//!   PATCH  /api/v1/organizations/{org_id}/notification-channels/{id}
//!   DELETE /api/v1/organizations/{org_id}/notification-channels/{id}

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use mockforge_registry_core::models::notification_channel::CreateNotificationChannel;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::NotificationChannel,
    AppState,
};

const VALID_KINDS: &[&str] = &["email", "slack", "pagerduty", "webhook"];

/// `GET /api/v1/organizations/{org_id}/notification-channels`
pub async fn list_channels(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<NotificationChannel>>> {
    authorize_org(&state, user_id, &headers, org_id).await?;
    let channels = NotificationChannel::list_by_org(state.db.pool(), org_id)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(channels))
}

#[derive(Debug, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    pub kind: String,
    pub config: serde_json::Value,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// `POST /api/v1/organizations/{org_id}/notification-channels`
pub async fn create_channel(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateChannelRequest>,
) -> ApiResult<Json<NotificationChannel>> {
    authorize_org(&state, user_id, &headers, org_id).await?;

    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("name must not be empty".into()));
    }
    if !VALID_KINDS.contains(&request.kind.as_str()) {
        return Err(ApiError::InvalidRequest(format!(
            "kind must be one of: {}",
            VALID_KINDS.join(", ")
        )));
    }

    let channel = NotificationChannel::create(
        state.db.pool(),
        CreateNotificationChannel {
            org_id,
            name: &request.name,
            kind: &request.kind,
            config: &request.config,
            enabled: request.enabled,
        },
    )
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(channel))
}

#[derive(Debug, Deserialize)]
pub struct UpdateChannelRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

/// `PATCH /api/v1/organizations/{org_id}/notification-channels/{id}`
pub async fn update_channel(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((org_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(request): Json<UpdateChannelRequest>,
) -> ApiResult<Json<NotificationChannel>> {
    authorize_org(&state, user_id, &headers, org_id).await?;
    let existing = load_authorized_channel(&state, org_id, id).await?;
    let _ = existing; // existence check; the UPDATE re-fetches

    let updated = NotificationChannel::update(
        state.db.pool(),
        id,
        request.name.as_deref(),
        request.config.as_ref(),
        request.enabled,
    )
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::InvalidRequest("Notification channel not found".into()))?;
    Ok(Json(updated))
}

/// `POST /api/v1/organizations/{org_id}/notification-channels/{id}/test-fire`
///
/// Fires a synthetic dispatch through this channel only — bypassing
/// routing rules — so an operator can validate webhook URLs / Slack
/// hooks without raising a real incident. Returns the dispatch result
/// inline so the UI can render success / error in the channel form.
pub async fn test_fire_channel(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((org_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    authorize_org(&state, user_id, &headers, org_id).await?;
    let channel = load_authorized_channel(&state, org_id, id).await?;
    if !channel.enabled {
        return Err(ApiError::InvalidRequest("Channel is disabled — enable it first".into()));
    }

    let result = crate::workers::incident_dispatcher::test_fire(&channel).await;
    Ok(Json(result))
}

/// `DELETE /api/v1/organizations/{org_id}/notification-channels/{id}`
pub async fn delete_channel(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((org_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    authorize_org(&state, user_id, &headers, org_id).await?;
    load_authorized_channel(&state, org_id, id).await?;

    let deleted = NotificationChannel::delete(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Notification channel not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// Verify caller belongs to the org. Errors are mapped to InvalidRequest
/// so non-members can't probe for existence.
async fn authorize_org(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    org_id: Uuid,
) -> ApiResult<()> {
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != org_id {
        return Err(ApiError::InvalidRequest("Cannot access channels for a different org".into()));
    }
    Ok(())
}

/// Fetch a channel and verify it belongs to `org_id`. Cross-org reads are
/// reported as "not found" rather than "forbidden" to avoid leaking
/// existence.
async fn load_authorized_channel(
    state: &AppState,
    org_id: Uuid,
    id: Uuid,
) -> ApiResult<NotificationChannel> {
    let channel = NotificationChannel::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Notification channel not found".into()))?;
    if channel.org_id != org_id {
        return Err(ApiError::InvalidRequest("Notification channel not found".into()));
    }
    Ok(channel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_kinds_are_recognized() {
        assert!(VALID_KINDS.contains(&"email"));
        assert!(VALID_KINDS.contains(&"slack"));
        assert!(VALID_KINDS.contains(&"pagerduty"));
        assert!(VALID_KINDS.contains(&"webhook"));
    }

    #[test]
    fn unknown_kinds_are_rejected_in_validation() {
        // The validation lives inline in create_channel; this test exists to
        // anchor the kind list as load-bearing — adding/removing values here
        // will need a dispatcher-side update too.
        assert!(!VALID_KINDS.contains(&"sms"));
        assert!(!VALID_KINDS.contains(&""));
        assert!(!VALID_KINDS.contains(&"SLACK"));
    }
}
