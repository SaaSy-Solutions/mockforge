//! Usage tracking and statistics handlers
//!
//! Provides endpoints for organizations to view their current usage
//! and limits across requests, storage, AI tokens, etc.

use axum::{extract::State, http::HeaderMap, Json};
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::UsageCounter,
    AppState,
};

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

    // Get plan limits
    let limits = &org_ctx.org.limits_json;

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
