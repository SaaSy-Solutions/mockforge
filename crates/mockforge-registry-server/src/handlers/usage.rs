//! Usage tracking and statistics handlers
//!
//! Provides endpoints for organizations to view their current usage
//! and limits across requests, storage, AI tokens, etc.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{Organization, UsageAlert, UsageCounter},
    AppState,
};

/// Effective per-key limits = plan defaults (org.limits_json) shallow-merged with
/// the org's optional `quota` setting (org_settings.setting_key = "quota"). The
/// admin-set quota overrides plan defaults per top-level key; missing keys fall
/// back to the plan.
pub(crate) async fn effective_limits(
    state: &AppState,
    org: &Organization,
) -> ApiResult<serde_json::Value> {
    let mut limits = org.limits_json.clone();
    let setting = state.store.get_org_setting(org.id, "quota").await?;
    if let Some(setting) = setting {
        merge_quota_overrides(&mut limits, &setting.setting_value);
    }
    Ok(limits)
}

/// Shallow-merge `overrides` into `limits` in-place. Both must be JSON objects
/// for any change to occur; otherwise this is a no-op.
fn merge_quota_overrides(limits: &mut serde_json::Value, overrides: &serde_json::Value) {
    if let (Some(base), Some(over)) = (limits.as_object_mut(), overrides.as_object()) {
        for (k, v) in over {
            base.insert(k.clone(), v.clone());
        }
    }
}

/// Get current usage statistics for the organization
pub async fn get_usage(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<UsageResponse>> {
    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Get current usage counter
    let usage = state.store.get_or_create_current_usage_counter(org_ctx.org_id).await?;

    // Effective limits = plan defaults + custom org quota overrides
    let limits = effective_limits(&state, &org_ctx.org).await?;

    // Build response with usage and limits
    Ok(Json(UsageResponse {
        org_id: org_ctx.org_id,
        period_start: usage.period_start,
        period_end: calculate_period_end(usage.period_start),
        usage: UsageBreakdown {
            requests: UsageMetric {
                used: usage.requests,
                limit: limits.get("requests_per_30d").and_then(|v| v.as_i64()).unwrap_or(10000),
                unit: "requests".to_string(),
            },
            storage: UsageMetric {
                used: usage.storage_bytes,
                limit: limits.get("storage_gb").and_then(|v| v.as_i64()).unwrap_or(1)
                    * 1_000_000_000, // Convert GB to bytes
                unit: "bytes".to_string(),
            },
            egress: UsageMetric {
                used: usage.egress_bytes,
                limit: -1, // Egress typically not limited separately, but tracked
                unit: "bytes".to_string(),
            },
            ai_tokens: UsageMetric {
                used: usage.ai_tokens_used,
                limit: limits.get("ai_tokens_per_month").and_then(|v| v.as_i64()).unwrap_or(0),
                unit: "tokens".to_string(),
            },
            runner_seconds: UsageMetric {
                used: usage.runner_seconds_used,
                limit: limits.get("runner_seconds_per_month").and_then(|v| v.as_i64()).unwrap_or(0),
                unit: "seconds".to_string(),
            },
            tunnel_bytes: UsageMetric {
                used: usage.tunnel_bytes_used,
                limit: limits.get("tunnel_bytes_per_month").and_then(|v| v.as_i64()).unwrap_or(0),
                unit: "bytes".to_string(),
            },
            snapshot_bytes: UsageMetric {
                used: usage.snapshot_bytes_stored,
                limit: limits.get("snapshot_bytes_quota").and_then(|v| v.as_i64()).unwrap_or(0),
                unit: "bytes".to_string(),
            },
        },
        plan: org_ctx.org.plan().to_string(),
    }))
}

/// Get usage history for the organization
pub async fn get_usage_history(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<UsageHistoryResponse>> {
    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Get all usage counters for this org
    let counters = state.store.list_usage_counters_by_org(org_ctx.org_id).await?;

    // Convert to response format
    let history: Vec<UsagePeriod> = counters
        .into_iter()
        .map(|counter| UsagePeriod {
            period_start: counter.period_start,
            period_end: calculate_period_end(counter.period_start),
            requests: counter.requests,
            egress_bytes: counter.egress_bytes,
            storage_bytes: counter.storage_bytes,
            ai_tokens_used: counter.ai_tokens_used,
            runner_seconds_used: counter.runner_seconds_used,
            tunnel_bytes_used: counter.tunnel_bytes_used,
            snapshot_bytes_stored: counter.snapshot_bytes_stored,
        })
        .collect();

    Ok(Json(UsageHistoryResponse {
        org_id: org_ctx.org_id,
        history,
    }))
}

#[derive(Debug, Serialize)]
pub struct UsageResponse {
    pub org_id: Uuid,
    pub period_start: chrono::NaiveDate,
    pub period_end: chrono::NaiveDate,
    pub usage: UsageBreakdown,
    pub plan: String,
}

#[derive(Debug, Serialize)]
pub struct UsageBreakdown {
    pub requests: UsageMetric,
    pub storage: UsageMetric,
    pub egress: UsageMetric,
    pub ai_tokens: UsageMetric,
    pub runner_seconds: UsageMetric,
    pub tunnel_bytes: UsageMetric,
    pub snapshot_bytes: UsageMetric,
}

#[derive(Debug, Serialize)]
pub struct UsageMetric {
    pub used: i64,
    pub limit: i64, // -1 means unlimited
    pub unit: String,
}

#[derive(Debug, Serialize)]
pub struct UsageHistoryResponse {
    pub org_id: Uuid,
    pub history: Vec<UsagePeriod>,
}

#[derive(Debug, Serialize)]
pub struct UsagePeriod {
    pub period_start: chrono::NaiveDate,
    pub period_end: chrono::NaiveDate,
    pub requests: i64,
    pub egress_bytes: i64,
    pub storage_bytes: i64,
    pub ai_tokens_used: i64,
    #[serde(default)]
    pub runner_seconds_used: i64,
    #[serde(default)]
    pub tunnel_bytes_used: i64,
    #[serde(default)]
    pub snapshot_bytes_stored: i64,
}

/// Request body for reporting AI token consumption
#[derive(Debug, Deserialize)]
pub struct ReportAiTokensRequest {
    /// Number of tokens consumed
    pub tokens: i64,
    /// Optional description of the operation (e.g., "api-critique", "system-generation")
    #[serde(default)]
    pub operation: Option<String>,
}

/// Report AI token consumption for the authenticated user's organization
///
/// POST /api/v1/usage/ai-tokens
pub async fn report_ai_tokens(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<ReportAiTokensRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    if request.tokens <= 0 {
        return Err(ApiError::InvalidRequest("tokens must be a positive integer".to_string()));
    }

    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    UsageCounter::increment_ai_tokens(state.db.pool(), org_ctx.org_id, request.tokens)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(
        org_id = %org_ctx.org_id,
        tokens = request.tokens,
        operation = request.operation.as_deref().unwrap_or("unknown"),
        "AI token usage recorded"
    );

    Ok(Json(serde_json::json!({
        "recorded": true,
        "tokens": request.tokens,
        "org_id": org_ctx.org_id,
    })))
}

/// Calculate the end of a billing period (last day of the month)
fn calculate_period_end(period_start: chrono::NaiveDate) -> chrono::NaiveDate {
    use chrono::NaiveDate;

    let year = period_start.year();
    let month = period_start.month();

    // Calculate first day of next month
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };

    // Get first day of next month, then subtract one day to get last day of current month
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|d| d.pred_opt())
        .unwrap_or(period_start)
}

/// First day of the current month — used to scope alerts to a billing period.
pub(crate) fn current_period_start() -> chrono::NaiveDate {
    let today = chrono::Utc::now().date_naive();
    chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today)
}

#[derive(Debug, Serialize)]
pub struct UsageAlertItem {
    pub id: Uuid,
    pub metric: String,
    pub period_start: chrono::NaiveDate,
    pub threshold_pct: i16,
    pub notified_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct ListUsageAlertsResponse {
    pub org_id: Uuid,
    pub period_start: chrono::NaiveDate,
    pub alerts: Vec<UsageAlertItem>,
}

/// GET /api/v1/usage/alerts — active (non-dismissed) alerts for the org's
/// current billing period.
pub async fn list_usage_alerts(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<ListUsageAlertsResponse>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let period_start = current_period_start();
    let rows = UsageAlert::list_active_for_period(state.db.pool(), org_ctx.org_id, period_start)
        .await
        .map_err(ApiError::Database)?;

    let alerts = rows
        .into_iter()
        .map(|a| UsageAlertItem {
            id: a.id,
            metric: a.metric,
            period_start: a.period_start,
            threshold_pct: a.threshold_pct,
            notified_at: a.notified_at,
        })
        .collect();

    Ok(Json(ListUsageAlertsResponse {
        org_id: org_ctx.org_id,
        period_start,
        alerts,
    }))
}

#[derive(Debug, Serialize)]
pub struct DismissUsageAlertResponse {
    pub dismissed: bool,
}

/// POST /api/v1/usage/alerts/{alert_id}/dismiss — soft-dismiss an alert.
pub async fn dismiss_usage_alert(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(alert_id): Path<Uuid>,
) -> ApiResult<Json<DismissUsageAlertResponse>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let result = UsageAlert::dismiss(state.db.pool(), alert_id, org_ctx.org_id)
        .await
        .map_err(ApiError::Database)?;

    Ok(Json(DismissUsageAlertResponse {
        dismissed: result.is_some(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merge_quota_overrides_replaces_existing_keys() {
        let mut limits = json!({"requests_per_30d": 10_000, "storage_gb": 1});
        let overrides = json!({"requests_per_30d": 50_000});
        merge_quota_overrides(&mut limits, &overrides);
        assert_eq!(limits["requests_per_30d"], 50_000);
        assert_eq!(limits["storage_gb"], 1);
    }

    #[test]
    fn merge_quota_overrides_adds_new_keys() {
        let mut limits = json!({"requests_per_30d": 10_000});
        let overrides = json!({"egress_gb": 5});
        merge_quota_overrides(&mut limits, &overrides);
        assert_eq!(limits["requests_per_30d"], 10_000);
        assert_eq!(limits["egress_gb"], 5);
    }

    #[test]
    fn merge_quota_overrides_noop_for_non_objects() {
        let mut limits = json!({"requests_per_30d": 10_000});
        let original = limits.clone();
        merge_quota_overrides(&mut limits, &json!("not an object"));
        assert_eq!(limits, original);

        let mut not_object = json!(42);
        merge_quota_overrides(&mut not_object, &json!({"x": 1}));
        assert_eq!(not_object, json!(42));
    }

    #[test]
    fn merge_quota_overrides_empty_override_keeps_plan() {
        let mut limits = json!({"requests_per_30d": 10_000, "storage_gb": 1});
        let original = limits.clone();
        merge_quota_overrides(&mut limits, &json!({}));
        assert_eq!(limits, original);
    }
}
