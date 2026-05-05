//! Cloud Plugins beta interest endpoints (Phase 0 demand validation).
//!
//! These endpoints back the "Request beta access" CTA on the cloud
//! `/plugin-registry` page. They are deliberately tiny: a single UPSERT
//! and a single point-read. Aggregate analysis for the go/no-go review
//! is done off-line via SQL against `cloud_plugin_beta_interest`.
//!
//! Routes:
//!   POST /api/v1/cloud-plugins/beta-interest
//!   GET  /api/v1/cloud-plugins/beta-interest/me
//!
//! The endpoint is NOT org-scoped on purpose: registering interest is a
//! per-user action, not a per-org one. We still snapshot the user's
//! current org_id + plan so the go/no-go review can segment by tier.

use axum::{extract::State, http::HeaderMap, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::CloudPluginBetaInterest,
    AppState,
};

/// Free-text caps. Trimmed and length-limited server-side so a runaway
/// client can't dump unlimited text into the table.
const MAX_USE_CASE_LEN: usize = 2_000;

#[derive(Debug, Deserialize)]
pub struct BetaInterestRequest {
    /// Optional "what would you build with cloud plugins?" answer.
    #[serde(default)]
    pub use_case: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BetaInterestResponse {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct BetaInterestStatusResponse {
    pub signed_up: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Echo back the user's last submitted use case so the form can
    /// pre-populate when they revisit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_case: Option<String>,
}

/// `POST /api/v1/cloud-plugins/beta-interest`
///
/// UPSERT — second submission updates `use_case` instead of erroring.
pub async fn submit_interest(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<BetaInterestRequest>,
) -> ApiResult<Json<BetaInterestResponse>> {
    let use_case = sanitize_use_case(request.use_case.as_deref())?;

    // Best-effort org context — we don't fail the signup if the user has
    // no current org, we just store NULL.
    let (org_id, plan_at_signup) = match resolve_org_context(&state, user_id, &headers, None).await
    {
        Ok(ctx) => (Some(ctx.org_id), Some(ctx.org.plan)),
        Err(_) => (None, None),
    };

    let row = CloudPluginBetaInterest::upsert(
        state.db.pool(),
        crate::models::cloud_plugin_beta_interest::UpsertCloudPluginBetaInterest {
            user_id,
            org_id,
            use_case: use_case.as_deref(),
            plan_at_signup: plan_at_signup.as_deref(),
        },
    )
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(BetaInterestResponse {
        id: row.id.to_string(),
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
}

/// `GET /api/v1/cloud-plugins/beta-interest/me`
pub async fn get_my_interest(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> ApiResult<Json<BetaInterestStatusResponse>> {
    let existing = CloudPluginBetaInterest::find_by_user(state.db.pool(), user_id)
        .await
        .map_err(ApiError::Database)?;

    Ok(Json(match existing {
        Some(row) => BetaInterestStatusResponse {
            signed_up: true,
            created_at: Some(row.created_at),
            use_case: row.use_case,
        },
        None => BetaInterestStatusResponse {
            signed_up: false,
            created_at: None,
            use_case: None,
        },
    }))
}

fn sanitize_use_case(raw: Option<&str>) -> ApiResult<Option<String>> {
    let Some(text) = raw else {
        return Ok(None);
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > MAX_USE_CASE_LEN {
        return Err(ApiError::InvalidRequest(format!(
            "use_case must be {} characters or fewer",
            MAX_USE_CASE_LEN
        )));
    }
    Ok(Some(trimmed.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_trims_and_drops_empty() {
        assert_eq!(sanitize_use_case(None).unwrap(), None);
        assert_eq!(sanitize_use_case(Some("")).unwrap(), None);
        assert_eq!(sanitize_use_case(Some("   ")).unwrap(), None);
        assert_eq!(sanitize_use_case(Some("  hello  ")).unwrap(), Some("hello".to_string()));
    }

    #[test]
    fn sanitize_rejects_too_long() {
        let too_long: String = "x".repeat(MAX_USE_CASE_LEN + 1);
        let err = sanitize_use_case(Some(&too_long)).unwrap_err();
        assert!(matches!(err, ApiError::InvalidRequest(_)));
    }

    #[test]
    fn sanitize_accepts_max_length() {
        let exact: String = "x".repeat(MAX_USE_CASE_LEN);
        assert_eq!(sanitize_use_case(Some(&exact)).unwrap(), Some(exact));
    }

    #[test]
    fn sanitize_counts_chars_not_bytes() {
        // Multi-byte UTF-8 char — 4 bytes, 1 grapheme. We count chars to
        // be lenient with non-ASCII input.
        let s: String = "🚀".repeat(MAX_USE_CASE_LEN);
        assert_eq!(sanitize_use_case(Some(&s)).unwrap(), Some(s));
    }
}
