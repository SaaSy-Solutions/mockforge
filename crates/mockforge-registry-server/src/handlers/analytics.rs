//! Admin analytics handlers
//!
//! Provides comprehensive analytics for admin users

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::AuthUser,
    models::User,
    AppState,
};

#[derive(Debug, Serialize)]
pub struct AnalyticsResponse {
    pub users: UserAnalytics,
    pub subscriptions: SubscriptionAnalytics,
    pub usage: UsageAnalytics,
    pub features: FeatureAnalytics,
    pub growth: GrowthAnalytics,
    pub activity: ActivityAnalytics,
}

#[derive(Debug, Serialize)]
pub struct UserAnalytics {
    pub total: i64,
    pub verified: i64,
    pub unverified: i64,
    pub by_auth_provider: Vec<AuthProviderCount>,
    pub new_users_last_7d: i64,
    pub new_users_last_30d: i64,
}

#[derive(Debug, Serialize)]
pub struct AuthProviderCount {
    pub provider: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionAnalytics {
    pub total_orgs: i64,
    pub by_plan: Vec<PlanCount>,
    pub active_subscriptions: i64,
    pub trial_orgs: i64,
    pub revenue_estimate: f64, // Monthly recurring revenue estimate
}

#[derive(Debug, Serialize)]
pub struct PlanCount {
    pub plan: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct UsageAnalytics {
    pub total_requests: i64,
    pub total_storage_gb: f64,
    pub total_ai_tokens: i64,
    pub avg_requests_per_org: f64,
    pub top_orgs_by_usage: Vec<OrgUsage>,
}

#[derive(Debug, Serialize)]
pub struct OrgUsage {
    pub org_id: String,
    pub org_name: String,
    pub plan: String,
    pub requests: i64,
    pub storage_gb: f64,
}

#[derive(Debug, Serialize)]
pub struct FeatureAnalytics {
    pub hosted_mocks: FeatureUsage,
    pub plugins_published: FeatureUsage,
    pub templates_published: FeatureUsage,
    pub scenarios_published: FeatureUsage,
    pub api_tokens_created: FeatureUsage,
}

#[derive(Debug, Serialize)]
pub struct FeatureUsage {
    pub total: i64,
    pub active_orgs: i64, // Orgs that have used this feature
    pub last_30d: i64,
}

#[derive(Debug, Serialize)]
pub struct GrowthAnalytics {
    pub user_growth_7d: Vec<DailyCount>,
    pub user_growth_30d: Vec<DailyCount>,
    pub org_growth_7d: Vec<DailyCount>,
    pub org_growth_30d: Vec<DailyCount>,
}

#[derive(Debug, Serialize)]
pub struct DailyCount {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct ActivityAnalytics {
    pub logins_last_24h: i64,
    pub logins_last_7d: i64,
    pub api_requests_last_24h: i64,
    pub api_requests_last_7d: i64,
}

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub period: Option<String>, // "7d", "30d", "90d", "all"
}

/// Get comprehensive analytics (admin only)
pub async fn get_analytics(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Query(query): Query<AnalyticsQuery>,
) -> ApiResult<Json<AnalyticsResponse>> {
    let pool = state.db.pool();

    // Check if user is admin
    let user = User::find_by_id(pool, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    if !user.is_admin {
        return Err(ApiError::PermissionDenied);
    }

    // User Analytics
    let total_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Database(e))?;

    let verified_users: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE is_verified = TRUE")
            .fetch_one(pool)
            .await
            .map_err(|e| ApiError::Database(e))?;

    let unverified_users = total_users.0 - verified_users.0;

    let auth_providers = sqlx::query_as::<_, (Option<String>, i64)>(
        "SELECT auth_provider, COUNT(*) FROM users GROUP BY auth_provider",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let new_users_7d: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE created_at > NOW() - INTERVAL '7 days'")
            .fetch_one(pool)
            .await
            .map_err(|e| ApiError::Database(e))?;

    let new_users_30d: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE created_at > NOW() - INTERVAL '30 days'")
            .fetch_one(pool)
            .await
            .map_err(|e| ApiError::Database(e))?;

    // Subscription Analytics
    let total_orgs: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM organizations")
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Database(e))?;

    let plan_distribution = sqlx::query_as::<_, (String, i64)>(
        "SELECT plan, COUNT(*) FROM organizations GROUP BY plan",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let active_subs: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM subscriptions WHERE status IN ('active', 'trialing')")
            .fetch_one(pool)
            .await
            .map_err(|e| ApiError::Database(e))?;

    let trial_orgs: (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT org_id) FROM subscriptions WHERE status = 'trialing'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Revenue estimate (Pro: $29, Team: $99)
    let revenue_estimate = plan_distribution
        .iter()
        .map(|(plan, count)| match plan.as_str() {
            "pro" => *count as f64 * 29.0,
            "team" => *count as f64 * 99.0,
            _ => 0.0,
        })
        .sum::<f64>();

    // Usage Analytics
    let total_requests: (Option<i64>,) = sqlx::query_as(
        "SELECT SUM(requests) FROM usage_counters WHERE period_start >= DATE_TRUNC('month', NOW())",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let total_storage: (Option<i64>,) = sqlx::query_as(
        "SELECT SUM(storage_bytes) FROM usage_counters WHERE period_start >= DATE_TRUNC('month', NOW())"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let total_ai_tokens: (Option<i64>,) = sqlx::query_as(
        "SELECT SUM(ai_tokens_used) FROM usage_counters WHERE period_start >= DATE_TRUNC('month', NOW())"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Top orgs by usage
    let top_orgs = sqlx::query_as::<_, (Uuid, String, String, i64, i64)>(
        r#"
        SELECT
            o.id,
            o.name,
            o.plan,
            COALESCE(SUM(uc.requests), 0) as requests,
            COALESCE(SUM(uc.storage_bytes), 0) as storage_bytes
        FROM organizations o
        LEFT JOIN usage_counters uc ON o.id = uc.org_id
        WHERE uc.period_start >= DATE_TRUNC('month', NOW())
        GROUP BY o.id, o.name, o.plan
        ORDER BY requests DESC
        LIMIT 10
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Feature Analytics
    let hosted_mocks_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM hosted_mocks WHERE deleted_at IS NULL")
            .fetch_one(pool)
            .await
            .map_err(|e| ApiError::Database(e))?;

    let hosted_mocks_orgs: (i64,) =
        sqlx::query_as("SELECT COUNT(DISTINCT org_id) FROM hosted_mocks WHERE deleted_at IS NULL")
            .fetch_one(pool)
            .await
            .map_err(|e| ApiError::Database(e))?;

    let hosted_mocks_30d: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM hosted_mocks WHERE created_at > NOW() - INTERVAL '30 days' AND deleted_at IS NULL"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let plugins_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM plugins")
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Database(e))?;

    let plugins_orgs: (i64,) =
        sqlx::query_as("SELECT COUNT(DISTINCT org_id) FROM plugins WHERE org_id IS NOT NULL")
            .fetch_one(pool)
            .await
            .map_err(|e| ApiError::Database(e))?;

    let plugins_30d: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM plugins WHERE created_at > NOW() - INTERVAL '30 days'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let templates_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM templates")
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Database(e))?;

    let templates_orgs: (i64,) =
        sqlx::query_as("SELECT COUNT(DISTINCT org_id) FROM templates WHERE org_id IS NOT NULL")
            .fetch_one(pool)
            .await
            .map_err(|e| ApiError::Database(e))?;

    let templates_30d: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM templates WHERE created_at > NOW() - INTERVAL '30 days'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let scenarios_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM scenarios")
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Database(e))?;

    let scenarios_orgs: (i64,) =
        sqlx::query_as("SELECT COUNT(DISTINCT org_id) FROM scenarios WHERE org_id IS NOT NULL")
            .fetch_one(pool)
            .await
            .map_err(|e| ApiError::Database(e))?;

    let scenarios_30d: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM scenarios WHERE created_at > NOW() - INTERVAL '30 days'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let api_tokens_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM api_tokens")
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Database(e))?;

    let api_tokens_orgs: (i64,) = sqlx::query_as("SELECT COUNT(DISTINCT org_id) FROM api_tokens")
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Database(e))?;

    let api_tokens_30d: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM api_tokens WHERE created_at > NOW() - INTERVAL '30 days'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Growth Analytics (last 30 days daily)
    let user_growth_30d = sqlx::query_as::<_, (chrono::NaiveDate, i64)>(
        r#"
        SELECT DATE(created_at) as date, COUNT(*) as count
        FROM users
        WHERE created_at > NOW() - INTERVAL '30 days'
        GROUP BY DATE(created_at)
        ORDER BY date ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let org_growth_30d = sqlx::query_as::<_, (chrono::NaiveDate, i64)>(
        r#"
        SELECT DATE(created_at) as date, COUNT(*) as count
        FROM organizations
        WHERE created_at > NOW() - INTERVAL '30 days'
        GROUP BY DATE(created_at)
        ORDER BY date ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Activity Analytics
    let logins_24h: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM login_attempts WHERE success = TRUE AND created_at > NOW() - INTERVAL '24 hours'"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let logins_7d: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM login_attempts WHERE success = TRUE AND created_at > NOW() - INTERVAL '7 days'"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    // API requests (approximate from usage counters)
    let api_requests_24h: (i64,) = sqlx::query_as(
        r#"
        SELECT COALESCE(SUM(requests), 0) FROM usage_counters
        WHERE updated_at > NOW() - INTERVAL '24 hours'
        "#,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    let api_requests_7d: (i64,) = sqlx::query_as(
        r#"
        SELECT COALESCE(SUM(requests), 0) FROM usage_counters
        WHERE updated_at > NOW() - INTERVAL '7 days'
        "#,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    Ok(Json(AnalyticsResponse {
        users: UserAnalytics {
            total: total_users.0,
            verified: verified_users.0,
            unverified: unverified_users,
            by_auth_provider: auth_providers
                .into_iter()
                .map(|(provider, count)| AuthProviderCount {
                    provider: provider.unwrap_or_else(|| "email".to_string()),
                    count,
                })
                .collect(),
            new_users_last_7d: new_users_7d.0,
            new_users_last_30d: new_users_30d.0,
        },
        subscriptions: SubscriptionAnalytics {
            total_orgs: total_orgs.0,
            by_plan: plan_distribution
                .into_iter()
                .map(|(plan, count)| PlanCount { plan, count })
                .collect(),
            active_subscriptions: active_subs.0,
            trial_orgs: trial_orgs.0,
            revenue_estimate,
        },
        usage: UsageAnalytics {
            total_requests: total_requests.0.unwrap_or(0),
            total_storage_gb: (total_storage.0.unwrap_or(0) as f64) / 1_000_000_000.0,
            total_ai_tokens: total_ai_tokens.0.unwrap_or(0),
            avg_requests_per_org: if total_orgs.0 > 0 {
                (total_requests.0.unwrap_or(0) as f64) / (total_orgs.0 as f64)
            } else {
                0.0
            },
            top_orgs_by_usage: top_orgs
                .into_iter()
                .map(|(id, name, plan, requests, storage_bytes)| OrgUsage {
                    org_id: id.to_string(),
                    org_name: name,
                    plan,
                    requests,
                    storage_gb: (storage_bytes as f64) / 1_000_000_000.0,
                })
                .collect(),
        },
        features: FeatureAnalytics {
            hosted_mocks: FeatureUsage {
                total: hosted_mocks_count.0,
                active_orgs: hosted_mocks_orgs.0,
                last_30d: hosted_mocks_30d.0,
            },
            plugins_published: FeatureUsage {
                total: plugins_count.0,
                active_orgs: plugins_orgs.0,
                last_30d: plugins_30d.0,
            },
            templates_published: FeatureUsage {
                total: templates_count.0,
                active_orgs: templates_orgs.0,
                last_30d: templates_30d.0,
            },
            scenarios_published: FeatureUsage {
                total: scenarios_count.0,
                active_orgs: scenarios_orgs.0,
                last_30d: scenarios_30d.0,
            },
            api_tokens_created: FeatureUsage {
                total: api_tokens_count.0,
                active_orgs: api_tokens_orgs.0,
                last_30d: api_tokens_30d.0,
            },
        },
        growth: GrowthAnalytics {
            user_growth_7d: vec![], // Can be calculated from 30d data
            user_growth_30d: user_growth_30d
                .into_iter()
                .map(|(date, count)| DailyCount {
                    date: date.to_string(),
                    count,
                })
                .collect(),
            org_growth_7d: vec![],
            org_growth_30d: org_growth_30d
                .into_iter()
                .map(|(date, count)| DailyCount {
                    date: date.to_string(),
                    count,
                })
                .collect(),
        },
        activity: ActivityAnalytics {
            logins_last_24h: logins_24h.0,
            logins_last_7d: logins_7d.0,
            api_requests_last_24h: api_requests_24h.0,
            api_requests_last_7d: api_requests_7d.0,
        },
    }))
}

/// Conversion funnel stages
#[derive(Debug, Serialize)]
pub struct ConversionFunnelResponse {
    pub period: String, // "7d", "30d", "90d", "all"
    pub stages: Vec<FunnelStage>,
    pub overall_conversion_rate: f64, // Signup to paid conversion
    pub time_to_convert: Option<f64>, // Average days from signup to paid (if available)
}

#[derive(Debug, Serialize)]
pub struct FunnelStage {
    pub stage: String,
    pub count: i64,
    pub conversion_rate: f64, // Percentage of previous stage
    pub drop_off: f64,        // Percentage lost from previous stage
}

/// Get conversion funnel analysis (admin only)
/// Tracks user journey from signup to paid subscription
pub async fn get_conversion_funnel(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Query(query): Query<AnalyticsQuery>,
) -> ApiResult<Json<ConversionFunnelResponse>> {
    let pool = state.db.pool();

    // Check if user is admin
    let user = User::find_by_id(pool, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    if !user.is_admin {
        return Err(ApiError::PermissionDenied);
    }

    // Determine time period
    let period = query.period.as_deref().unwrap_or("30d");
    let interval = match period {
        "7d" => "7 days",
        "30d" => "30 days",
        "90d" => "90 days",
        "all" => "1000 years", // Effectively all time
        _ => "30 days",
    };

    // Stage 1: Signups (users created)
    let signups: (i64,) = sqlx::query_as(&format!(
        "SELECT COUNT(*) FROM users WHERE created_at > NOW() - INTERVAL '{}'",
        interval
    ))
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Stage 2: Email Verified
    let verified: (i64,) = sqlx::query_as(&format!(
        "SELECT COUNT(*) FROM users WHERE is_verified = TRUE AND created_at > NOW() - INTERVAL '{}'",
        interval
    ))
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Stage 3: First Login (users who have logged in at least once)
    // Note: login_attempts uses email, not user_id, so we join via email
    let logged_in: (i64,) = sqlx::query_as(&format!(
        r#"
        SELECT COUNT(DISTINCT u.id)
        FROM users u
        INNER JOIN login_attempts la ON u.email = la.email
        WHERE la.success = TRUE
        AND u.created_at > NOW() - INTERVAL '{}'
        "#,
        interval
    ))
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Stage 4: Organization Created (users who created an org)
    let org_created: (i64,) = sqlx::query_as(&format!(
        r#"
        SELECT COUNT(DISTINCT u.id)
        FROM users u
        INNER JOIN organization_members om ON u.id = om.user_id
        INNER JOIN organizations o ON om.org_id = o.id
        WHERE om.role = 'admin'
        AND u.created_at > NOW() - INTERVAL '{}'
        "#,
        interval
    ))
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Stage 5: First Feature Use (users who used any feature)
    let feature_users: (i64,) = sqlx::query_as(&format!(
        r#"
        SELECT COUNT(DISTINCT u.id)
        FROM users u
        INNER JOIN feature_usage fu ON u.id = fu.user_id
        WHERE u.created_at > NOW() - INTERVAL '{}'
        "#,
        interval
    ))
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Stage 6: Checkout Initiated (users who started checkout)
    let checkout_initiated: (i64,) = sqlx::query_as(&format!(
        r#"
        SELECT COUNT(DISTINCT u.id)
        FROM users u
        INNER JOIN feature_usage fu ON u.id = fu.user_id
        WHERE fu.feature = 'billing_checkout'
        AND u.created_at > NOW() - INTERVAL '{}'
        "#,
        interval
    ))
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Stage 7: Paid Subscription (users with active paid subscriptions)
    let paid_subscribers: (i64,) = sqlx::query_as(&format!(
        r#"
        SELECT COUNT(DISTINCT u.id)
        FROM users u
        INNER JOIN organization_members om ON u.id = om.user_id
        INNER JOIN organizations o ON om.org_id = o.id
        INNER JOIN subscriptions s ON o.id = s.org_id
        WHERE s.status IN ('active', 'trialing')
        AND s.plan IN ('pro', 'team')
        AND u.created_at > NOW() - INTERVAL '{}'
        "#,
        interval
    ))
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Calculate average time to convert (signup to paid subscription)
    let time_to_convert: Option<f64> = sqlx::query_scalar::<_, Option<f64>>(&format!(
        r#"
        SELECT AVG(EXTRACT(EPOCH FROM (s.created_at - u.created_at)) / 86400.0) as avg_days
        FROM users u
        INNER JOIN organization_members om ON u.id = om.user_id
        INNER JOIN organizations o ON om.org_id = o.id
        INNER JOIN subscriptions s ON o.id = s.org_id
        WHERE s.status IN ('active', 'trialing')
        AND s.plan IN ('pro', 'team')
        AND u.created_at > NOW() - INTERVAL '{}'
        "#,
        interval
    ))
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Build funnel stages
    let mut stages = Vec::new();
    let signup_count = signups.0 as f64;

    // Stage 1: Signups (baseline - 100%)
    stages.push(FunnelStage {
        stage: "Signup".to_string(),
        count: signups.0,
        conversion_rate: 100.0,
        drop_off: 0.0,
    });

    // Stage 2: Email Verified
    let verified_rate = if signup_count > 0.0 {
        (verified.0 as f64 / signup_count) * 100.0
    } else {
        0.0
    };
    stages.push(FunnelStage {
        stage: "Email Verified".to_string(),
        count: verified.0,
        conversion_rate: verified_rate,
        drop_off: 100.0 - verified_rate,
    });

    // Stage 3: First Login
    let login_rate = if signup_count > 0.0 {
        (logged_in.0 as f64 / signup_count) * 100.0
    } else {
        0.0
    };
    stages.push(FunnelStage {
        stage: "First Login".to_string(),
        count: logged_in.0,
        conversion_rate: login_rate,
        drop_off: verified_rate - login_rate,
    });

    // Stage 4: Organization Created
    let org_rate = if signup_count > 0.0 {
        (org_created.0 as f64 / signup_count) * 100.0
    } else {
        0.0
    };
    stages.push(FunnelStage {
        stage: "Organization Created".to_string(),
        count: org_created.0,
        conversion_rate: org_rate,
        drop_off: login_rate - org_rate,
    });

    // Stage 5: First Feature Use
    let feature_rate = if signup_count > 0.0 {
        (feature_users.0 as f64 / signup_count) * 100.0
    } else {
        0.0
    };
    stages.push(FunnelStage {
        stage: "First Feature Use".to_string(),
        count: feature_users.0,
        conversion_rate: feature_rate,
        drop_off: org_rate - feature_rate,
    });

    // Stage 6: Checkout Initiated
    let checkout_rate = if signup_count > 0.0 {
        (checkout_initiated.0 as f64 / signup_count) * 100.0
    } else {
        0.0
    };
    stages.push(FunnelStage {
        stage: "Checkout Initiated".to_string(),
        count: checkout_initiated.0,
        conversion_rate: checkout_rate,
        drop_off: feature_rate - checkout_rate,
    });

    // Stage 7: Paid Subscription
    let paid_rate = if signup_count > 0.0 {
        (paid_subscribers.0 as f64 / signup_count) * 100.0
    } else {
        0.0
    };
    stages.push(FunnelStage {
        stage: "Paid Subscription".to_string(),
        count: paid_subscribers.0,
        conversion_rate: paid_rate,
        drop_off: checkout_rate - paid_rate,
    });

    Ok(Json(ConversionFunnelResponse {
        period: period.to_string(),
        stages,
        overall_conversion_rate: paid_rate,
        time_to_convert,
    }))
}
