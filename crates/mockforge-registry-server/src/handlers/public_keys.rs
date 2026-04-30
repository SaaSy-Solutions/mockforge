//! Publisher public-key management for SBOM attestation.
//!
//! Routes:
//!   * `GET    /api/v1/users/me/public-keys` — list keys, with `usage_count`
//!     and (when `?includeRevoked=true`) revoked rows.
//!   * `POST   /api/v1/users/me/public-keys` — register a new key. Optional
//!     `orgId` tags the key to an org; the caller must be Owner/Admin.
//!     Per-plan quota enforced via `effective_limits`.
//!   * `POST   /api/v1/users/me/public-keys/{id}/rotate` — atomic rotation:
//!     register a new key with the same org tag and revoke the old one in a
//!     single transaction; emits a `PublisherKeyRotated` audit event.
//!   * `DELETE /api/v1/users/me/public-keys/{id}` — soft-revoke a key the
//!     caller owns (or, if the key is org-scoped, an org Owner/Admin).
//!   * `GET    /api/v1/organizations/{org_id}/public-keys` — list keys
//!     tagged to an org; visible to org Owners/Admins.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use base64::Engine;
use mockforge_registry_core::models::{AuditEventType, OrgRole};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    handlers::usage::effective_limits,
    middleware::AuthUser,
    AppState,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicKeyResponse {
    pub id: Uuid,
    pub algorithm: String,
    pub public_key_b64: String,
    pub label: String,
    pub created_at: String,
    pub revoked_at: Option<String>,
    /// Number of plugin versions whose SBOM signature was verified by
    /// this key. Lets the UI show a "signed N versions" pill.
    pub usage_count: i64,
    /// Optional org the key is scoped to. `None` = personal key.
    /// Serializes as `null` (not omitted) so clients can rely on the
    /// field being present when reading older records back.
    pub org_id: Option<Uuid>,
}

impl From<mockforge_registry_core::models::UserPublicKeyWithUsage> for PublicKeyResponse {
    fn from(k: mockforge_registry_core::models::UserPublicKeyWithUsage) -> Self {
        Self {
            id: k.key.id,
            algorithm: k.key.algorithm,
            public_key_b64: k.key.public_key_b64,
            label: k.key.label,
            created_at: k.key.created_at.to_rfc3339(),
            revoked_at: k.key.revoked_at.map(|dt| dt.to_rfc3339()),
            usage_count: k.usage_count,
            org_id: k.key.org_id,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPublicKeysResponse {
    pub keys: Vec<PublicKeyResponse>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPublicKeysQuery {
    /// When `true`, the response also includes soft-revoked keys so the
    /// UI can render a revocation history. Defaults to `false`.
    #[serde(default)]
    pub include_revoked: bool,
}

pub async fn list_my_public_keys(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Query(q): Query<ListPublicKeysQuery>,
) -> ApiResult<Json<ListPublicKeysResponse>> {
    let keys = state.store.list_user_public_keys_with_usage(user_id, q.include_revoked).await?;
    Ok(Json(ListPublicKeysResponse {
        keys: keys.into_iter().map(Into::into).collect(),
    }))
}

pub async fn list_org_public_keys(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    Query(q): Query<ListPublicKeysQuery>,
) -> ApiResult<Json<ListPublicKeysResponse>> {
    require_org_admin(&state, org_id, user_id).await?;
    let keys = state.store.list_org_public_keys_with_usage(org_id, q.include_revoked).await?;
    Ok(Json(ListPublicKeysResponse {
        keys: keys.into_iter().map(Into::into).collect(),
    }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePublicKeyRequest {
    /// Currently only `"ed25519"` is accepted. Kept client-supplied so
    /// the API shape is ready for additional algorithms without a bump.
    #[serde(default = "default_algorithm")]
    pub algorithm: String,
    /// Raw 32-byte Ed25519 public key, base64-encoded. Accepts both
    /// standard and URL-safe base64 (no padding).
    pub public_key_b64: String,
    /// Short human label the user sees in their key list. Required so
    /// a returning user can distinguish their own keys.
    pub label: String,
    /// Optional org the key should be scoped to. The caller must be
    /// Owner/Admin of that org.
    #[serde(default)]
    pub org_id: Option<Uuid>,
}

fn default_algorithm() -> String {
    "ed25519".to_string()
}

pub async fn create_my_public_key(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Json(request): Json<CreatePublicKeyRequest>,
) -> ApiResult<Json<PublicKeyResponse>> {
    let algorithm = request.algorithm.trim().to_ascii_lowercase();
    if algorithm != "ed25519" {
        return Err(ApiError::InvalidRequest(format!(
            "unsupported key algorithm '{}': only 'ed25519' is accepted",
            algorithm
        )));
    }

    let label = request.label.trim();
    if label.is_empty() || label.len() > 128 {
        return Err(ApiError::InvalidRequest(
            "label must be between 1 and 128 characters".to_string(),
        ));
    }

    let key_b64 = request.public_key_b64.trim();
    // Length-check the decoded bytes eagerly so we reject garbage at the
    // HTTP boundary rather than surfacing it at verification time.
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(key_b64)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(key_b64))
        .map_err(|e| ApiError::InvalidRequest(format!("public_key_b64 is not base64: {}", e)))?;
    if decoded.len() != ed25519_dalek::PUBLIC_KEY_LENGTH {
        return Err(ApiError::InvalidRequest(format!(
            "ed25519 public key must be {} bytes, got {}",
            ed25519_dalek::PUBLIC_KEY_LENGTH,
            decoded.len()
        )));
    }

    // Org tag: confirm the caller is Owner/Admin of the requested org
    // before we let them attach a key to it.
    if let Some(org_id) = request.org_id {
        require_org_admin(&state, org_id, user_id).await?;
    }

    enforce_publisher_key_quota(&state, user_id).await?;

    let saved = state
        .store
        .create_user_public_key(user_id, &algorithm, key_b64, label, request.org_id)
        .await?;

    // Audit the create. org_id on the audit row is the audit log's own
    // scope (always nil for personal-style events) — independent of the
    // key's optional org tag, which goes in metadata so org admins can
    // correlate later.
    state
        .store
        .record_audit_event(
            uuid::Uuid::nil(),
            Some(user_id),
            AuditEventType::PublisherKeyCreated,
            format!("Publisher key '{}' created", saved.label),
            Some(serde_json::json!({
                "key_id": saved.id,
                "label": saved.label,
                "algorithm": saved.algorithm,
                "key_org_id": saved.org_id,
            })),
            None,
            None,
        )
        .await;

    Ok(Json(PublicKeyResponse {
        id: saved.id,
        algorithm: saved.algorithm,
        public_key_b64: saved.public_key_b64,
        label: saved.label,
        created_at: saved.created_at.to_rfc3339(),
        revoked_at: saved.revoked_at.map(|dt| dt.to_rfc3339()),
        usage_count: 0,
        org_id: saved.org_id,
    }))
}

pub async fn revoke_my_public_key(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(key_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    // First try the user-scoped revoke. If that misses (returns false)
    // and the key is tagged to an org the caller administers, fall
    // through to the org-scoped revoke.
    let revoked = if state.store.revoke_user_public_key(user_id, key_id).await? {
        true
    } else {
        // Look up the key to learn its org tag, then check the
        // caller's role on that org.
        match state.store.find_user_public_key_by_id(key_id).await? {
            Some(k) => match k.org_id {
                Some(org_id) => {
                    require_org_admin(&state, org_id, user_id).await?;
                    state.store.revoke_org_public_key(org_id, key_id).await?
                }
                None => false,
            },
            None => false,
        }
    };

    if !revoked {
        return Err(ApiError::InvalidRequest(
            "key does not exist, is already revoked, or you don't have permission to revoke it"
                .to_string(),
        ));
    }

    state
        .store
        .record_audit_event(
            uuid::Uuid::nil(),
            Some(user_id),
            AuditEventType::PublisherKeyRevoked,
            format!("Publisher key {} revoked", key_id),
            Some(serde_json::json!({"key_id": key_id})),
            None,
            None,
        )
        .await;

    Ok(Json(serde_json::json!({ "revoked": true, "id": key_id })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RotatePublicKeyRequest {
    pub new_public_key_b64: String,
    pub new_label: String,
    /// Caller may pin the algorithm; defaults to ed25519 (the only one
    /// supported today). Kept symmetric with `CreatePublicKeyRequest`.
    #[serde(default = "default_algorithm")]
    pub algorithm: String,
}

pub async fn rotate_my_public_key(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(old_key_id): Path<Uuid>,
    Json(request): Json<RotatePublicKeyRequest>,
) -> ApiResult<Json<PublicKeyResponse>> {
    let algorithm = request.algorithm.trim().to_ascii_lowercase();
    if algorithm != "ed25519" {
        return Err(ApiError::InvalidRequest(format!(
            "unsupported key algorithm '{}': only 'ed25519' is accepted",
            algorithm
        )));
    }

    let new_label = request.new_label.trim();
    if new_label.is_empty() || new_label.len() > 128 {
        return Err(ApiError::InvalidRequest(
            "new_label must be between 1 and 128 characters".to_string(),
        ));
    }

    let new_key_b64 = request.new_public_key_b64.trim();
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(new_key_b64)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(new_key_b64))
        .map_err(|e| {
            ApiError::InvalidRequest(format!("new_public_key_b64 is not base64: {}", e))
        })?;
    if decoded.len() != ed25519_dalek::PUBLIC_KEY_LENGTH {
        return Err(ApiError::InvalidRequest(format!(
            "ed25519 public key must be {} bytes, got {}",
            ed25519_dalek::PUBLIC_KEY_LENGTH,
            decoded.len()
        )));
    }

    // Quota: rotation creates one key but also revokes one in the same
    // transaction, so net active count doesn't change. We still gate
    // the create on the quota in case the user is *over* quota due to
    // a downgrade and is rotating to fix that — they're allowed.
    // No-op for now.

    let new_key = state
        .store
        .rotate_user_public_key(user_id, old_key_id, &algorithm, new_key_b64, new_label)
        .await
        .map_err(|e| match e {
            mockforge_registry_core::error::StoreError::NotFound => ApiError::InvalidRequest(
                "key does not exist, is already revoked, or doesn't belong to you".to_string(),
            ),
            other => other.into(),
        })?;

    state
        .store
        .record_audit_event(
            uuid::Uuid::nil(),
            Some(user_id),
            AuditEventType::PublisherKeyRotated,
            format!("Publisher key rotated from {} to {}", old_key_id, new_key.id),
            Some(serde_json::json!({
                "old_key_id": old_key_id,
                "new_key_id": new_key.id,
                "label": new_key.label,
                "key_org_id": new_key.org_id,
            })),
            None,
            None,
        )
        .await;

    Ok(Json(PublicKeyResponse {
        id: new_key.id,
        algorithm: new_key.algorithm,
        public_key_b64: new_key.public_key_b64,
        label: new_key.label,
        created_at: new_key.created_at.to_rfc3339(),
        revoked_at: new_key.revoked_at.map(|dt| dt.to_rfc3339()),
        usage_count: 0,
        org_id: new_key.org_id,
    }))
}

/// Resolve a Plan limit on `max_publisher_keys` for the user's owner
/// org and reject if the user is at or over it. Falls back to the Free
/// default when the user has no owned org (e.g. before personal-org
/// backfill ran). `-1` means unlimited.
async fn enforce_publisher_key_quota(state: &AppState, user_id: Uuid) -> ApiResult<()> {
    let pool = state.db.pool();
    let orgs = mockforge_registry_core::models::Organization::find_by_user(pool, user_id)
        .await
        .map_err(ApiError::Database)?;
    let owner_org = orgs.into_iter().find(|o| o.owner_id == user_id);

    let limit = if let Some(org) = owner_org.as_ref() {
        let limits = effective_limits(state, org).await?;
        limits.get("max_publisher_keys").and_then(|v| v.as_i64()).unwrap_or(3)
    } else {
        // No owner org yet — apply the Free default conservatively so
        // users without an org still hit a sane cap.
        3
    };

    if limit < 0 {
        return Ok(());
    }

    // Count active keys via the existing list helper. Keys per user
    // typically <50, so the cost of running the JOIN here is fine.
    let active = state.store.list_user_public_keys(user_id).await?;
    if active.len() as i64 >= limit {
        return Err(ApiError::ResourceLimitExceeded(format!(
            "publisher key limit reached ({}/{}) — revoke an old key or upgrade your plan",
            active.len(),
            limit
        )));
    }

    Ok(())
}

/// Bail with `PermissionDenied` unless `user_id` is the org's Owner or
/// has an Admin/Owner role in `org_members`.
async fn require_org_admin(state: &AppState, org_id: Uuid, user_id: Uuid) -> ApiResult<()> {
    let org = state
        .store
        .find_organization_by_id(org_id)
        .await?
        .ok_or(ApiError::OrganizationNotFound)?;
    if org.owner_id == user_id {
        return Ok(());
    }
    let member = state.store.find_org_member(org_id, user_id).await?;
    let role = member.as_ref().map(|m| m.role());
    if matches!(role, Some(OrgRole::Owner) | Some(OrgRole::Admin)) {
        Ok(())
    } else {
        Err(ApiError::PermissionDenied)
    }
}
