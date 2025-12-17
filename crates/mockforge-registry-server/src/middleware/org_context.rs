//! Organization context middleware for multi-tenancy
//!
//! This middleware extracts organization context from requests and provides
//! it to handlers via extractors. Supports:
//! - X-Organization-Id header
//! - X-Organization-Slug header
//! - Default org from user's personal org

use axum::http::{HeaderMap, StatusCode};
use uuid::Uuid;

use crate::{models::Organization, AppState};

/// Verify user has access to organization
async fn verify_org_access(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    user_id: Uuid,
) -> Result<(), StatusCode> {
    use crate::models::OrgMember;

    // Check if user is owner
    let org = Organization::find_by_id(pool, org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if org.owner_id == user_id {
        return Ok(());
    }

    // Check if user is a member
    let member = OrgMember::find(pool, org_id, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if member.is_some() {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

/// Organization context
#[derive(Debug, Clone)]
pub struct OrgContext {
    pub org_id: Uuid,
    pub org: Organization,
}

/// Helper function to resolve org context from State and AuthUser
/// Use this in handlers instead of the extractor if you need more control
///
/// Also checks request extensions for org_id set by API token auth
pub async fn resolve_org_context(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    request_extensions: Option<&axum::http::Extensions>, // Optional extensions from request
) -> Result<OrgContext, StatusCode> {
    let pool = state.db.pool();

    // Check if org_id was set by API token auth (for faster lookup)
    let api_token_org_id = request_extensions.and_then(|ext| {
        // Try to get org_id from extensions
        ext.get::<String>().and_then(|s| {
            if s.starts_with("org_id:") {
                Uuid::parse_str(&s[7..]).ok()
            } else {
                None
            }
        })
    });

    // Try to get org from API token first, then header, then default
    let org = if let Some(org_id) = api_token_org_id {
        // Use org_id from API token (fastest path)
        // Try cache first
        let org = Organization::find_by_id(pool, org_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        // Verify user has access (already verified by API token, but double-check)
        verify_org_access(pool, org_id, user_id)
            .await
            .map_err(|_| StatusCode::FORBIDDEN)?;

        org
    } else if let Some(org_id_header) = headers.get("X-Organization-Id") {
        // Resolve by ID
        let org_id_str = org_id_header.to_str().map_err(|_| StatusCode::BAD_REQUEST)?;
        let org_id = Uuid::parse_str(org_id_str).map_err(|_| StatusCode::BAD_REQUEST)?;

        // Try cache first
        let org = Organization::find_by_id(pool, org_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        // Verify user has access to this org
        verify_org_access(pool, org_id, user_id)
            .await
            .map_err(|_| StatusCode::FORBIDDEN)?;

        org
    } else if let Some(org_slug_header) = headers.get("X-Organization-Slug") {
        // Resolve by slug
        let slug = org_slug_header.to_str().map_err(|_| StatusCode::BAD_REQUEST)?;

        let org = Organization::find_by_slug(pool, slug)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        // Verify user has access to this org
        verify_org_access(pool, org.id, user_id)
            .await
            .map_err(|_| StatusCode::FORBIDDEN)?;

        org
    } else {
        // Get user's default/personal org
        // For now, get the first org where user is owner
        // In the future, we might store a "default_org_id" on users
        let orgs = Organization::find_by_user(pool, user_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        orgs.into_iter().find(|o| o.owner_id == user_id).ok_or(StatusCode::NOT_FOUND)?
    };

    Ok(OrgContext {
        org_id: org.id,
        org,
    })
}
