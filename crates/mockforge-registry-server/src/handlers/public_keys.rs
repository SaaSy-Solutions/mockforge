//! Publisher public-key management for SBOM attestation.
//!
//! Routes (all authenticated, scoped to the calling user):
//!   * `GET    /api/v1/users/me/public-keys` — list active keys.
//!   * `POST   /api/v1/users/me/public-keys` — register a new key.
//!   * `DELETE /api/v1/users/me/public-keys/{id}` — soft-revoke.
//!
//! These are the minimum surface a publisher needs to opt into signing
//! SBOMs. The verification logic lives in
//! `mockforge-registry-core::models::attestation`; this handler just
//! owns the user-facing CRUD.

use axum::{
    extract::{Path, State},
    Json,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
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
}

impl From<mockforge_registry_core::models::UserPublicKey> for PublicKeyResponse {
    fn from(k: mockforge_registry_core::models::UserPublicKey) -> Self {
        Self {
            id: k.id,
            algorithm: k.algorithm,
            public_key_b64: k.public_key_b64,
            label: k.label,
            created_at: k.created_at.to_rfc3339(),
            revoked_at: k.revoked_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPublicKeysResponse {
    pub keys: Vec<PublicKeyResponse>,
}

pub async fn list_my_public_keys(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
) -> ApiResult<Json<ListPublicKeysResponse>> {
    let keys = state.store.list_user_public_keys(user_id).await?;
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

    let saved = state.store.create_user_public_key(user_id, &algorithm, key_b64, label).await?;
    Ok(Json(saved.into()))
}

pub async fn revoke_my_public_key(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(key_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let revoked = state.store.revoke_user_public_key(user_id, key_id).await?;
    if !revoked {
        return Err(ApiError::InvalidRequest(
            "key does not exist, is already revoked, or does not belong to the calling user"
                .to_string(),
        ));
    }
    Ok(Json(serde_json::json!({ "revoked": true, "id": key_id })))
}
