//! Usage tracking and statistics handlers
//!
//! Provides endpoints for organizations to view their current usage
//! and limits across requests, storage, AI tokens, etc.

use axum::{
    extract::State,
    http::HeaderMap,
    Json,
};
use chrono::Datelike;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{AuthUser, resolve_org_context},
    models::{UsageCounter, Organization},
    AppState,
};

/// Get current usage statistics for the organization
pub async fn get_usage(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<UsageResponse>> {
    let pool = state.db.pool();

    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None).await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Get current usage counter
    let usage = UsageCounter::get_or_create_current(pool, org_ctx.org_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Get plan limits
    let limits = &org_ctx.org.limits_json;

    // Build response with usage and limits
    Ok(Json(UsageResponse {
        org_id: org_ctx.org_id,
        period_start: usage.period_start,
        period_end: {
            // Calculate period end (last day of current month)
            let next_month = usage.period_start
                .with_month(usage.period_start.month() + 1)
                .unwrap_or_else(|| {
                    // If month overflow, go to next year
                    usage.period_start
                        .with_year(usage.period_start.year() + 1)
                        .unwrap()
                        .with_month(1)
                        .unwrap()
                });
            next_month.pred_opt().unwrap_or(usage.period_start)
        },
        usage: UsageBreakdown {
            requests: UsageMetric {
                used: usage.requests,
                limit: limits
                    .get("requests_per_30d")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(10000),
                unit: "requests".to_string(),
            },
            storage: UsageMetric {
                used: usage.storage_bytes,
                limit: limits
                    .get("storage_gb")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1) * 1_000_000_000, // Convert GB to bytes
                unit: "bytes".to_string(),
            },
            egress: UsageMetric {
                used: usage.egress_bytes,
                limit: -1, // Egress typically not limited separately, but tracked
                unit: "bytes".to_string(),
            },
            ai_tokens: UsageMetric {
                used: usage.ai_tokens_used,
                limit: limits
                    .get("ai_tokens_per_month")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                unit: "tokens".to_string(),
            },
        },
        limits: limits.clone(),
        plan: org_ctx.org.plan().to_string(),
    }))
}

/// Get usage history for the organization
pub async fn get_usage_history(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<UsageHistoryResponse>> {
    let pool = state.db.pool();

    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None).await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Get all usage counters for this org
    let counters = UsageCounter::get_all_for_org(pool, org_ctx.org_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Convert to response format
    let history: Vec<UsagePeriod> = counters
        .into_iter()
        .map(|counter| UsagePeriod {
            period_start: counter.period_start,
            period_end: {
                // Calculate period end (last day of month)
                let next_month = counter.period_start
                    .with_month(counter.period_start.month() + 1)
                    .unwrap_or_else(|| {
                        counter.period_start
                            .with_year(counter.period_start.year() + 1)
                            .unwrap()
                            .with_month(1)
                            .unwrap()
                    });
                next_month.pred_opt().unwrap_or(counter.period_start)
            },
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
    pub limits: serde_json::Value,
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
