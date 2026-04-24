//! `/api/v1/users/me` — profile, preferences, and notification toggles
//! for the currently-authenticated user.

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    error::{ApiError, ApiResult},
    middleware::AuthUser,
    AppState,
};

fn serialize_user(user: mockforge_registry_core::models::User) -> UserResponse {
    UserResponse {
        user_id: user.id.to_string(),
        username: user.username,
        email: user.email,
        is_verified: user.is_verified,
        is_admin: user.is_admin,
        two_factor_enabled: user.two_factor_enabled,
        email_notifications: user.email_notifications,
        security_alerts: user.security_alerts,
        preferences: user.preferences,
        created_at: user.created_at,
        updated_at: user.updated_at,
    }
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub is_verified: bool,
    pub is_admin: bool,
    pub two_factor_enabled: bool,
    pub email_notifications: bool,
    pub security_alerts: bool,
    pub preferences: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// `GET /api/v1/users/me`
pub async fn get_me(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> ApiResult<Json<UserResponse>> {
    let user = state
        .store
        .find_user_by_id(user_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;
    Ok(Json(serialize_user(user)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub username: Option<String>,
    pub email: Option<String>,
}

/// `PATCH /api/v1/users/me` — update username and/or email.
pub async fn update_me(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(request): Json<UpdateProfileRequest>,
) -> ApiResult<Json<UserResponse>> {
    if request.username.is_none() && request.email.is_none() {
        return Err(ApiError::InvalidRequest(
            "At least one of `username` or `email` is required".to_string(),
        ));
    }

    let username = request
        .username
        .as_ref()
        .map(|u| u.trim().to_string())
        .filter(|u| !u.is_empty());
    if let Some(ref u) = username {
        if u.len() < 3 {
            return Err(ApiError::InvalidRequest(
                "Username must be at least 3 characters".to_string(),
            ));
        }
        if let Some(existing) = state.store.find_user_by_username(u).await? {
            if existing.id != user_id {
                return Err(ApiError::InvalidRequest("Username is already taken".to_string()));
            }
        }
    }

    let email = request
        .email
        .as_ref()
        .map(|e| e.trim().to_lowercase())
        .filter(|e| !e.is_empty());
    if let Some(ref e) = email {
        // Very permissive check — the registration endpoint uses the same
        // rule elsewhere (`contains('@')`) so we stay consistent.
        if !e.contains('@') || !e.contains('.') {
            return Err(ApiError::InvalidRequest("Email address is not valid".to_string()));
        }
        if let Some(existing) = state.store.find_user_by_email(e).await? {
            if existing.id != user_id {
                return Err(ApiError::InvalidRequest("Email is already in use".to_string()));
            }
        }
    }

    let updated = state
        .store
        .update_user_profile(user_id, username.as_deref(), email.as_deref())
        .await?;

    Ok(Json(serialize_user(updated)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateNotificationsRequest {
    pub email_notifications: Option<bool>,
    pub security_alerts: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct NotificationsResponse {
    pub email_notifications: bool,
    pub security_alerts: bool,
}

/// `PATCH /api/v1/users/me/notifications` — update notification toggles.
pub async fn update_notifications(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(request): Json<UpdateNotificationsRequest>,
) -> ApiResult<Json<NotificationsResponse>> {
    let user = state
        .store
        .find_user_by_id(user_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    let email_notifications = request.email_notifications.unwrap_or(user.email_notifications);
    let security_alerts = request.security_alerts.unwrap_or(user.security_alerts);

    state
        .store
        .update_user_notification_prefs(user_id, email_notifications, security_alerts)
        .await?;

    Ok(Json(NotificationsResponse {
        email_notifications,
        security_alerts,
    }))
}

#[derive(Debug, Serialize)]
pub struct PreferencesResponse {
    pub preferences: serde_json::Value,
}

/// `GET /api/v1/users/me/preferences`
pub async fn get_preferences(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> ApiResult<Json<PreferencesResponse>> {
    let user = state
        .store
        .find_user_by_id(user_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;
    Ok(Json(PreferencesResponse {
        preferences: user.preferences,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdatePreferencesRequest {
    /// Partial update — merged into the stored preferences blob at the top
    /// level. Nested objects are replaced, not deep-merged.
    pub preferences: serde_json::Value,
}

/// `PATCH /api/v1/users/me/preferences` — merge-update the preferences blob.
pub async fn update_preferences(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(request): Json<UpdatePreferencesRequest>,
) -> ApiResult<Json<PreferencesResponse>> {
    let mut current = state
        .store
        .find_user_by_id(user_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?
        .preferences;

    if !current.is_object() {
        current = serde_json::Value::Object(serde_json::Map::new());
    }

    match request.preferences {
        serde_json::Value::Object(patch) => {
            if let Some(obj) = current.as_object_mut() {
                for (k, v) in patch {
                    obj.insert(k, v);
                }
            }
        }
        other => {
            // Non-object body overwrites the blob outright.
            current = other;
        }
    }

    state.store.update_user_preferences(user_id, &current).await?;
    Ok(Json(PreferencesResponse {
        preferences: current,
    }))
}
