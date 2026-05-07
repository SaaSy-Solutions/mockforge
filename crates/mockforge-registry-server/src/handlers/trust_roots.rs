//! Per-org Ed25519 trust roots — control plane (Issue #416).
//!
//! The schema (`organization_trust_roots`) and audit-log enum variants
//! shipped in migration `20250101000074`; this is the HTTP surface that
//! org admins use to register and revoke them. Trust roots authorize
//! org-private cloud-plugin signatures (RFC §7.1, two-tier trust).
//!
//! Routes (org_id is path-scoped, mirroring `public_keys::list_org_public_keys`
//! rather than the header-scoped pattern used by hosted-mock handlers):
//!   GET    /api/v1/organizations/{org_id}/trust-roots
//!   POST   /api/v1/organizations/{org_id}/trust-roots
//!   POST   /api/v1/organizations/{org_id}/trust-roots/{root_id}/revoke
//!
//! Authorization: caller must be a member of the org and hold
//! `Permission::OrgUpdate`. Missing org returns 404; non-member returns
//! 403 (the `PermissionChecker` distinguishes the two).
//!
//! Trust-roots are not metered — no `feature_usage` emission, only audit
//! events (`AuditEventType::OrgTrustRoot{Created,Revoked}`).

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use base64::Engine;
use chrono::{DateTime, Utc};
use mockforge_registry_core::models::{
    organization_trust_root::CreateOrganizationTrustRoot, AuditEventType, OrganizationTrustRoot,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{permission_check::PermissionChecker, permissions::Permission, AuthUser},
    AppState,
};

/// Ed25519 raw public-key length. Hard-coded rather than pulled from
/// `ed25519_dalek::PUBLIC_KEY_LENGTH` because adding the dep just for a
/// constant isn't worth the compile-time cost.
const ED25519_PUBLIC_KEY_LEN: usize = 32;

/// Matches `VARCHAR(128)` on the `name` column.
const MAX_NAME_LEN: usize = 128;

/// Hard cap on the free-text revoke reason. Keeps the audit-log payload
/// bounded — the column itself is `TEXT` (unbounded).
const MAX_REVOKE_REASON_LEN: usize = 1_000;

// ─── Request / response shapes ───────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustRootResponse {
    pub id: Uuid,
    pub org_id: Uuid,
    /// Standard-base64 encoding of the raw 32-byte public key. UI
    /// renders this in a copy-to-clipboard chip; CLI fingerprints it.
    pub public_key_b64: String,
    pub name: String,
    /// Convenience: matches the partial index `idx_org_trust_roots_active`.
    /// Equivalent to `revoked_at.is_none()`.
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoked_reason: Option<String>,
    pub revoked_by: Option<Uuid>,
}

impl From<OrganizationTrustRoot> for TrustRootResponse {
    fn from(row: OrganizationTrustRoot) -> Self {
        Self {
            active: row.is_active(),
            id: row.id,
            org_id: row.org_id,
            public_key_b64: base64::engine::general_purpose::STANDARD.encode(&row.public_key),
            name: row.name,
            created_at: row.created_at,
            created_by: row.created_by,
            revoked_at: row.revoked_at,
            revoked_reason: row.revoked_reason,
            revoked_by: row.revoked_by,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTrustRootsResponse {
    pub trust_roots: Vec<TrustRootResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTrustRootRequest {
    /// Raw 32-byte Ed25519 public key, base64-encoded. Accepts both
    /// standard and URL-safe (no-padding) variants — same lenience
    /// `public_keys::create_my_public_key` applies.
    pub public_key_b64: String,
    /// Human-readable label shown in the org-settings UI.
    pub name: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeTrustRootRequest {
    /// Optional free-text reason persisted on the row and echoed to the
    /// audit log. Trimmed; empty/whitespace-only collapses to `None`.
    #[serde(default)]
    pub reason: Option<String>,
}

// ─── Routes ──────────────────────────────────────────────────────────

/// `GET /api/v1/organizations/{org_id}/trust-roots`
///
/// Returns active and revoked roots, ordered newest-first. Revoked rows
/// stay in the response so the UI can render a revocation history (the
/// row's `active` flag distinguishes the two).
pub async fn list_trust_roots(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
) -> ApiResult<Json<ListTrustRootsResponse>> {
    authorize(&state, user_id, org_id).await?;

    let rows = OrganizationTrustRoot::list_by_org(state.db.pool(), org_id)
        .await
        .map_err(ApiError::Database)?;

    Ok(Json(ListTrustRootsResponse {
        trust_roots: rows.into_iter().map(TrustRootResponse::from).collect(),
    }))
}

/// `POST /api/v1/organizations/{org_id}/trust-roots`
pub async fn create_trust_root(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateTrustRootRequest>,
) -> ApiResult<Json<TrustRootResponse>> {
    authorize(&state, user_id, org_id).await?;

    let name = validate_name(&request.name)?;
    let public_key = decode_public_key(&request.public_key_b64)?;

    let row = OrganizationTrustRoot::create(
        state.db.pool(),
        CreateOrganizationTrustRoot {
            org_id,
            public_key: &public_key,
            name: &name,
            created_by: Some(user_id),
        },
    )
    .await
    .map_err(ApiError::Database)?;

    let (ip, ua) = client_metadata(&headers);
    state
        .store
        .record_audit_event(
            org_id,
            Some(user_id),
            AuditEventType::OrgTrustRootCreated,
            format!("Trust root '{}' registered for org {}", row.name, org_id),
            Some(serde_json::json!({
                "trust_root_id": row.id,
                "name": row.name,
                "public_key_b64": base64::engine::general_purpose::STANDARD.encode(&row.public_key),
            })),
            ip.as_deref(),
            ua.as_deref(),
        )
        .await;

    Ok(Json(TrustRootResponse::from(row)))
}

/// `POST /api/v1/organizations/{org_id}/trust-roots/{root_id}/revoke`
pub async fn revoke_trust_root(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((org_id, root_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    request: Option<Json<RevokeTrustRootRequest>>,
) -> ApiResult<Json<TrustRootResponse>> {
    authorize(&state, user_id, org_id).await?;

    // Fetch first so we can disambiguate "not found" from "wrong org" and
    // "already revoked" — the SQL UPDATE in `OrganizationTrustRoot::revoke`
    // returns `None` for either of the latter two, which we want to treat
    // as 409 (already revoked) rather than 404.
    let existing = OrganizationTrustRoot::find_by_id(state.db.pool(), root_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Trust root not found".into()))?;

    if existing.org_id != org_id {
        // Cross-org access surfaces as not-found to avoid leaking
        // existence (matches `cloud_plugin_attachments::load_authorized_attachment`).
        return Err(ApiError::InvalidRequest("Trust root not found".into()));
    }
    if existing.revoked_at.is_some() {
        return Err(ApiError::Conflict("Trust root is already revoked".into()));
    }

    let reason = sanitize_reason(request.and_then(|Json(r)| r.reason).as_deref())?;

    let row = OrganizationTrustRoot::revoke(
        state.db.pool(),
        root_id,
        reason.as_deref(),
        Some(user_id),
    )
    .await
    .map_err(ApiError::Database)?
    // Lost a race with another concurrent revoke — same end-state as a
    // double-revoke, surface as 409 too.
    .ok_or_else(|| ApiError::Conflict("Trust root is already revoked".into()))?;

    let (ip, ua) = client_metadata(&headers);
    state
        .store
        .record_audit_event(
            org_id,
            Some(user_id),
            AuditEventType::OrgTrustRootRevoked,
            format!("Trust root '{}' revoked", row.name),
            Some(serde_json::json!({
                "trust_root_id": row.id,
                "name": row.name,
                "reason": reason,
            })),
            ip.as_deref(),
            ua.as_deref(),
        )
        .await;

    Ok(Json(TrustRootResponse::from(row)))
}

// ─── Helpers ─────────────────────────────────────────────────────────

/// Verify (a) the org exists and (b) the caller has `OrgUpdate` on it.
/// Existence comes first so a non-member of a *missing* org sees 404
/// rather than 403 — matches the convention `OrganizationNotFound`
/// uses elsewhere.
async fn authorize(state: &AppState, user_id: Uuid, org_id: Uuid) -> ApiResult<()> {
    let _org = state
        .store
        .find_organization_by_id(org_id)
        .await?
        .ok_or(ApiError::OrganizationNotFound)?;

    let checker = PermissionChecker::new(state);
    checker.require_permission(user_id, org_id, Permission::OrgUpdate).await?;
    Ok(())
}

/// Trim, reject empty/whitespace-only, cap at `MAX_NAME_LEN` *characters*
/// (not bytes — same lenience `cloud_plugins::sanitize_use_case` applies).
fn validate_name(raw: &str) -> ApiResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ApiError::InvalidRequest("name must not be empty".into()));
    }
    if trimmed.chars().count() > MAX_NAME_LEN {
        return Err(ApiError::InvalidRequest(format!(
            "name must be {} characters or fewer",
            MAX_NAME_LEN
        )));
    }
    Ok(trimmed.to_string())
}

/// Decode standard or URL-safe (no-padding) base64 and reject anything
/// that doesn't decode to exactly 32 bytes.
fn decode_public_key(raw: &str) -> ApiResult<Vec<u8>> {
    let trimmed = raw.trim();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(trimmed)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(trimmed))
        .map_err(|e| ApiError::InvalidRequest(format!("public_key_b64 is not base64: {}", e)))?;
    if bytes.len() != ED25519_PUBLIC_KEY_LEN {
        return Err(ApiError::ValidationFailed(format!(
            "ed25519 public key must be {} bytes, got {}",
            ED25519_PUBLIC_KEY_LEN,
            bytes.len()
        )));
    }
    Ok(bytes)
}

fn sanitize_reason(raw: Option<&str>) -> ApiResult<Option<String>> {
    let Some(text) = raw else {
        return Ok(None);
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > MAX_REVOKE_REASON_LEN {
        return Err(ApiError::InvalidRequest(format!(
            "reason must be {} characters or fewer",
            MAX_REVOKE_REASON_LEN
        )));
    }
    Ok(Some(trimmed.to_string()))
}

fn client_metadata(headers: &HeaderMap) -> (Option<String>, Option<String>) {
    let ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string());
    let ua = headers.get("user-agent").and_then(|h| h.to_str().ok()).map(|s| s.to_string());
    (ip, ua)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_name_trims_and_rejects_empty() {
        assert!(matches!(validate_name(""), Err(ApiError::InvalidRequest(_))));
        assert!(matches!(validate_name("   "), Err(ApiError::InvalidRequest(_))));
        assert_eq!(validate_name("  CI signing key  ").unwrap(), "CI signing key");
    }

    #[test]
    fn validate_name_rejects_too_long() {
        let too_long: String = "x".repeat(MAX_NAME_LEN + 1);
        assert!(matches!(validate_name(&too_long), Err(ApiError::InvalidRequest(_))));
    }

    #[test]
    fn validate_name_accepts_max_length() {
        let exact: String = "x".repeat(MAX_NAME_LEN);
        assert_eq!(validate_name(&exact).unwrap(), exact);
    }

    #[test]
    fn validate_name_counts_chars_not_bytes() {
        let s: String = "🔐".repeat(MAX_NAME_LEN);
        assert_eq!(validate_name(&s).unwrap(), s);
    }

    #[test]
    fn decode_public_key_accepts_standard_b64() {
        let bytes: Vec<u8> = (0..32u8).collect();
        let s = base64::engine::general_purpose::STANDARD.encode(&bytes);
        assert_eq!(decode_public_key(&s).unwrap(), bytes);
    }

    #[test]
    fn decode_public_key_accepts_url_safe_b64() {
        let bytes: Vec<u8> = (0..32u8).collect();
        let s = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes);
        assert_eq!(decode_public_key(&s).unwrap(), bytes);
    }

    #[test]
    fn decode_public_key_rejects_short() {
        let bytes = vec![0u8; 31];
        let s = base64::engine::general_purpose::STANDARD.encode(&bytes);
        match decode_public_key(&s).unwrap_err() {
            ApiError::ValidationFailed(msg) => assert!(msg.contains("32 bytes")),
            other => panic!("expected ValidationFailed, got {:?}", other),
        }
    }

    #[test]
    fn decode_public_key_rejects_long() {
        let bytes = vec![0u8; 64];
        let s = base64::engine::general_purpose::STANDARD.encode(&bytes);
        assert!(matches!(decode_public_key(&s), Err(ApiError::ValidationFailed(_))));
    }

    #[test]
    fn decode_public_key_rejects_non_base64() {
        assert!(matches!(decode_public_key("not-base64-!!!"), Err(ApiError::InvalidRequest(_))));
    }

    #[test]
    fn sanitize_reason_trims_and_collapses_empty() {
        assert_eq!(sanitize_reason(None).unwrap(), None);
        assert_eq!(sanitize_reason(Some("")).unwrap(), None);
        assert_eq!(sanitize_reason(Some("   ")).unwrap(), None);
        assert_eq!(
            sanitize_reason(Some("  key compromise  ")).unwrap(),
            Some("key compromise".to_string())
        );
    }

    #[test]
    fn sanitize_reason_rejects_too_long() {
        let too_long: String = "x".repeat(MAX_REVOKE_REASON_LEN + 1);
        assert!(matches!(sanitize_reason(Some(&too_long)), Err(ApiError::InvalidRequest(_))));
    }
}
