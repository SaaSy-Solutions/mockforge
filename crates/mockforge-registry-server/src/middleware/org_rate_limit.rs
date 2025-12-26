//! Organization-aware rate limiting middleware
//!
//! This middleware enforces rate limits based on organization plan limits.
//! It tracks usage in Redis and checks against plan limits before allowing requests.

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    middleware::resolve_org_context,
    models::{Organization, Plan, UsageCounter},
    redis::{current_month_period, org_usage_key, RedisPool},
    AppState,
};

/// Check if organization has exceeded plan limits
pub async fn check_org_limits(
    pool: &sqlx::PgPool,
    redis: Option<&RedisPool>,
    org: &Organization,
    user_id: Uuid,
) -> Result<(), RateLimitError> {
    let plan = org.plan();
    let limits = &org.limits_json;

    // Get current month period
    let period = current_month_period();

    // Get or create usage counter
    let usage = UsageCounter::get_or_create_current(pool, org.id)
        .await
        .map_err(|_| RateLimitError::Database)?;

    // Check request limit
    let requests_limit = limits
        .get("requests_per_30d")
        .and_then(|v| v.as_i64())
        .unwrap_or(10000);

    if usage.requests >= requests_limit {
        return Err(RateLimitError::LimitExceeded {
            limit_type: "requests".to_string(),
            limit: requests_limit,
            used: usage.requests,
            reset_period: period.clone(),
        });
    }

    // Check storage limit
    let storage_limit_gb = limits
        .get("storage_gb")
        .and_then(|v| v.as_i64())
        .unwrap_or(1);
    let storage_limit_bytes = storage_limit_gb * 1_000_000_000;

    if usage.storage_bytes >= storage_limit_bytes {
        return Err(RateLimitError::LimitExceeded {
            limit_type: "storage".to_string(),
            limit: storage_limit_bytes,
            used: usage.storage_bytes,
            reset_period: period.clone(),
        });
    }

    Ok(())
}

/// Increment usage counter for a request
pub async fn increment_usage(
    pool: &sqlx::PgPool,
    redis: Option<&RedisPool>,
    org_id: Uuid,
    request_size_bytes: i64,
) -> Result<(), RateLimitError> {
    // Increment in Redis first (fast path)
    if let Some(redis_pool) = redis {
        let period = current_month_period();
        let requests_key = format!("usage:{}:{}:requests", org_id, period);
        let _ = redis_pool.increment_with_expiry(&requests_key, 2592000).await; // 30 days
        if request_size_bytes > 0 {
            let egress_key = format!("usage:{}:{}:egress", org_id, period);
            let _ = redis_pool.increment_with_expiry(&egress_key, 2592000).await;
        }
    }

    // Increment in database (slower, but persistent)
    UsageCounter::increment_requests(pool, org_id, 1)
        .await
        .map_err(|_| RateLimitError::Database)?;

    if request_size_bytes > 0 {
        UsageCounter::increment_egress(pool, org_id, request_size_bytes)
            .await
            .map_err(|_| RateLimitError::Database)?;
    }

    Ok(())
}

/// Organization-aware rate limiting middleware
///
/// This middleware:
/// 1. Resolves organization context from request
/// 2. Checks plan limits (requests, storage, etc.)
/// 3. Increments usage counters
/// 4. Returns 429 if limits exceeded
///
/// Note: This should be applied AFTER auth_middleware
pub async fn org_rate_limit_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    // Try to get user_id from auth middleware (set in extensions)
    let user_id_str = request.extensions()
        .get::<String>()
        .cloned();

    // If no user_id, this might be a public endpoint - skip org rate limiting
    // (but still apply global rate limiting if configured)
    let user_id = if let Some(id_str) = user_id_str {
        match Uuid::parse_str(&id_str) {
            Ok(id) => id,
            Err(_) => {
                // Invalid user_id, skip org rate limiting
                return Ok(next.run(request).await);
            }
        }
    } else {
        return Ok(next.run(request).await);
    };

    // Resolve org context (pass request extensions for API token org_id lookup)
    let org_ctx = match resolve_org_context(&state, user_id, &headers, Some(request.extensions())).await {
        Ok(ctx) => ctx,
        Err(_) => {
            // No org context, skip org rate limiting
            return Ok(next.run(request).await);
        }
    };

    let pool = state.db.pool();

    // Check org limits
    if let Err(e) = check_org_limits(pool, state.redis.as_ref(), &org_ctx.org, user_id).await {
        return Err(rate_limit_error_response(e));
    }

    // Get usage info for rate limit headers
    let usage = UsageCounter::get_or_create_current(pool, org_ctx.org_id)
        .await
        .ok();

    let limits = &org_ctx.org.limits_json;
    let requests_limit = limits
        .get("requests_per_30d")
        .and_then(|v| v.as_i64())
        .unwrap_or(10000);

    let requests_remaining = usage
        .as_ref()
        .map(|u| (requests_limit - u.requests).max(0))
        .unwrap_or(requests_limit);

    // Calculate reset time (end of current month)
    let now = chrono::Utc::now();
    let next_month = if now.month() == 12 {
        chrono::NaiveDate::from_ymd_opt((now.year() + 1) as i32, 1, 1)
    } else {
        chrono::NaiveDate::from_ymd_opt(now.year() as i32, (now.month() + 1) as u32, 1)
    }
    .and_then(|d| d.and_hms_opt(0, 0, 0))
    .map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc));

    let reset_timestamp = next_month
        .map(|dt| dt.timestamp())
        .unwrap_or_else(|| now.timestamp() + 2592000); // Fallback: 30 days from now

    // Process request
    let mut response = next.run(request).await;

    // Add rate limit headers
    let headers = response.headers_mut();
    headers.insert(
        "X-RateLimit-Limit",
        axum::http::HeaderValue::from_str(&requests_limit.to_string())
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("10000")),
    );
    headers.insert(
        "X-RateLimit-Remaining",
        axum::http::HeaderValue::from_str(&requests_remaining.to_string())
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("0")),
    );
    headers.insert(
        "X-RateLimit-Reset",
        axum::http::HeaderValue::from_str(&reset_timestamp.to_string())
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("0")),
    );

    // Increment usage (only for successful requests, 2xx status)
    let status = response.status();
    if status.is_success() {
        // Estimate request size from response (approximate)
        let response_size = estimate_response_size(&response);

        // Increment usage asynchronously (don't block response)
        let pool_clone = pool.clone();
        let redis_clone = state.redis.clone();
        let org_id = org_ctx.org_id;
        tokio::spawn(async move {
            let _ = increment_usage(&pool_clone, redis_clone.as_ref(), org_id, response_size).await;
        });
    }

    Ok(response)
}

/// Estimate response size (approximate)
fn estimate_response_size(response: &Response) -> i64 {
    // This is a rough estimate - in production you might want to track actual bytes
    // For now, we'll use a default estimate
    response.headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(1024) // Default 1KB estimate
}

/// Rate limit error
#[derive(Debug)]
pub enum RateLimitError {
    Database,
    LimitExceeded {
        limit_type: String,
        limit: i64,
        used: i64,
        reset_period: String,
    },
}

/// Convert rate limit error to HTTP response
fn rate_limit_error_response(error: RateLimitError) -> impl IntoResponse {
    match error {
        RateLimitError::Database => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Internal server error",
                "message": "Failed to check rate limits"
            })),
        ),
        RateLimitError::LimitExceeded { limit_type, limit, used, reset_period } => {
            let limit_type_display = match limit_type.as_str() {
                "requests" => "Monthly request limit",
                "storage" => "Storage limit",
                _ => "Usage limit",
            };

            (
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({
                    "error": "Rate limit exceeded",
                    "message": format!("{} exceeded. Used {}/{}", limit_type_display, used, limit),
                    "limit_type": limit_type,
                    "limit": limit,
                    "used": used,
                    "reset_period": reset_period,
                    "upgrade_url": "/billing/upgrade"
                })),
            )
        }
    }
}
