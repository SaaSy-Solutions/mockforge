//! Routing rule CRUD (cloud-enablement task #3 / Phase 1, follow-up).
//!
//! Routes incoming incidents to notification channels based on
//! (severity × source × workspace) matching. The dispatcher worker
//! evaluates rules in priority order; first match wins.
//!
//! Routes:
//!   GET    /api/v1/organizations/{org_id}/routing-rules
//!   POST   /api/v1/organizations/{org_id}/routing-rules
//!   PATCH  /api/v1/organizations/{org_id}/routing-rules/{id}
//!   DELETE /api/v1/organizations/{org_id}/routing-rules/{id}

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use mockforge_registry_core::models::routing_rule::CreateRoutingRule;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::RoutingRule,
    AppState,
};

const ALLOWED_SEVERITIES: &[&str] = &["critical", "high", "medium", "low"];

/// `GET /api/v1/organizations/{org_id}/routing-rules`
pub async fn list_rules(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<RoutingRule>>> {
    authorize_org(&state, user_id, &headers, org_id).await?;
    let rules = RoutingRule::list_by_org(state.db.pool(), org_id)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(rules))
}

#[derive(Debug, Deserialize)]
pub struct CreateRuleRequest {
    pub priority: i32,
    #[serde(default)]
    pub match_severity: Vec<String>,
    #[serde(default)]
    pub match_source: Vec<String>,
    #[serde(default)]
    pub match_workspace_id: Option<Uuid>,
    pub channel_ids: Vec<Uuid>,
}

/// `POST /api/v1/organizations/{org_id}/routing-rules`
pub async fn create_rule(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateRuleRequest>,
) -> ApiResult<Json<RoutingRule>> {
    authorize_org(&state, user_id, &headers, org_id).await?;
    validate_severities(&request.match_severity)?;
    if request.channel_ids.is_empty() {
        return Err(ApiError::InvalidRequest(
            "channel_ids must include at least one channel".into(),
        ));
    }

    let rule = RoutingRule::create(
        state.db.pool(),
        CreateRoutingRule {
            org_id,
            priority: request.priority,
            match_severity: &request.match_severity,
            match_source: &request.match_source,
            match_workspace_id: request.match_workspace_id,
            channel_ids: &request.channel_ids,
        },
    )
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(rule))
}

#[derive(Debug, Deserialize)]
pub struct UpdateRuleRequest {
    #[serde(default)]
    pub priority: Option<i32>,
    #[serde(default)]
    pub match_severity: Option<Vec<String>>,
    #[serde(default)]
    pub match_source: Option<Vec<String>>,
    #[serde(default)]
    pub channel_ids: Option<Vec<Uuid>>,
}

/// `PATCH /api/v1/organizations/{org_id}/routing-rules/{id}`
pub async fn update_rule(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((org_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(request): Json<UpdateRuleRequest>,
) -> ApiResult<Json<RoutingRule>> {
    authorize_org(&state, user_id, &headers, org_id).await?;
    load_authorized_rule(&state, org_id, id).await?;

    if let Some(severities) = &request.match_severity {
        validate_severities(severities)?;
    }
    if let Some(channels) = &request.channel_ids {
        if channels.is_empty() {
            return Err(ApiError::InvalidRequest(
                "channel_ids cannot be set to an empty list".into(),
            ));
        }
    }

    let updated = RoutingRule::update(
        state.db.pool(),
        id,
        request.priority,
        request.match_severity.as_deref(),
        request.match_source.as_deref(),
        request.channel_ids.as_deref(),
    )
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::InvalidRequest("Routing rule not found".into()))?;
    Ok(Json(updated))
}

/// `DELETE /api/v1/organizations/{org_id}/routing-rules/{id}`
pub async fn delete_rule(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((org_id, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    authorize_org(&state, user_id, &headers, org_id).await?;
    load_authorized_rule(&state, org_id, id).await?;

    let deleted = RoutingRule::delete(state.db.pool(), id).await.map_err(ApiError::Database)?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Routing rule not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

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
        return Err(ApiError::InvalidRequest(
            "Cannot access routing rules for a different org".into(),
        ));
    }
    Ok(())
}

async fn load_authorized_rule(state: &AppState, org_id: Uuid, id: Uuid) -> ApiResult<RoutingRule> {
    let rule = RoutingRule::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Routing rule not found".into()))?;
    if rule.org_id != org_id {
        return Err(ApiError::InvalidRequest("Routing rule not found".into()));
    }
    Ok(rule)
}

fn validate_severities(values: &[String]) -> ApiResult<()> {
    for v in values {
        if !ALLOWED_SEVERITIES.contains(&v.as_str()) {
            return Err(ApiError::InvalidRequest(format!(
                "match_severity entries must be one of: {}",
                ALLOWED_SEVERITIES.join(", ")
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_severities_accepts_canonical_values() {
        let ok = vec!["critical".to_string(), "low".to_string()];
        assert!(validate_severities(&ok).is_ok());
    }

    #[test]
    fn validate_severities_rejects_unknowns() {
        let bad = vec!["critical".to_string(), "URGENT".to_string()];
        assert!(validate_severities(&bad).is_err());
    }

    #[test]
    fn validate_severities_accepts_empty() {
        // Empty vec is the wildcard case.
        assert!(validate_severities(&[]).is_ok());
    }
}
