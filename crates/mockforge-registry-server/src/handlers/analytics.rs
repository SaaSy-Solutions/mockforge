//! Admin analytics handlers
//!
//! Provides comprehensive analytics for admin users

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::{ApiError, ApiResult},
    middleware::AuthUser,
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
    Query(_query): Query<AnalyticsQuery>,
) -> ApiResult<Json<AnalyticsResponse>> {
    // Check if user is admin
    let user = state
        .store
        .find_user_by_id(user_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("User not found".to_string()))?;

    if !user.is_admin {
        return Err(ApiError::PermissionDenied);
    }

    let snap = state.store.get_admin_analytics_snapshot().await?;

    // Revenue estimate (Pro: $29, Team: $99)
    let revenue_estimate = snap
        .plan_distribution
        .iter()
        .map(|(plan, count)| match plan.as_str() {
            "pro" => *count as f64 * 29.0,
            "team" => *count as f64 * 99.0,
            _ => 0.0,
        })
        .sum::<f64>();

    Ok(Json(AnalyticsResponse {
        users: UserAnalytics {
            total: snap.total_users,
            verified: snap.verified_users,
            unverified: snap.total_users - snap.verified_users,
            by_auth_provider: snap
                .auth_providers
                .into_iter()
                .map(|(provider, count)| AuthProviderCount {
                    provider: provider.unwrap_or_else(|| "email".to_string()),
                    count,
                })
                .collect(),
            new_users_last_7d: snap.new_users_7d,
            new_users_last_30d: snap.new_users_30d,
        },
        subscriptions: SubscriptionAnalytics {
            total_orgs: snap.total_orgs,
            by_plan: snap
                .plan_distribution
                .into_iter()
                .map(|(plan, count)| PlanCount { plan, count })
                .collect(),
            active_subscriptions: snap.active_subs,
            trial_orgs: snap.trial_orgs,
            revenue_estimate,
        },
        usage: UsageAnalytics {
            total_requests: snap.total_requests.unwrap_or(0),
            total_storage_gb: (snap.total_storage.unwrap_or(0) as f64) / 1_000_000_000.0,
            total_ai_tokens: snap.total_ai_tokens.unwrap_or(0),
            avg_requests_per_org: if snap.total_orgs > 0 {
                (snap.total_requests.unwrap_or(0) as f64) / (snap.total_orgs as f64)
            } else {
                0.0
            },
            top_orgs_by_usage: snap
                .top_orgs
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
                total: snap.hosted_mocks_count,
                active_orgs: snap.hosted_mocks_orgs,
                last_30d: snap.hosted_mocks_30d,
            },
            plugins_published: FeatureUsage {
                total: snap.plugins_count,
                active_orgs: snap.plugins_orgs,
                last_30d: snap.plugins_30d,
            },
            templates_published: FeatureUsage {
                total: snap.templates_count,
                active_orgs: snap.templates_orgs,
                last_30d: snap.templates_30d,
            },
            scenarios_published: FeatureUsage {
                total: snap.scenarios_count,
                active_orgs: snap.scenarios_orgs,
                last_30d: snap.scenarios_30d,
            },
            api_tokens_created: FeatureUsage {
                total: snap.api_tokens_count,
                active_orgs: snap.api_tokens_orgs,
                last_30d: snap.api_tokens_30d,
            },
        },
        growth: GrowthAnalytics {
            user_growth_7d: {
                let cutoff = (chrono::Utc::now() - chrono::Duration::days(7)).date_naive();
                snap.user_growth_30d
                    .iter()
                    .filter(|(date, _)| *date >= cutoff)
                    .map(|(date, count)| DailyCount {
                        date: date.to_string(),
                        count: *count,
                    })
                    .collect()
            },
            user_growth_30d: snap
                .user_growth_30d
                .iter()
                .map(|(date, count)| DailyCount {
                    date: date.to_string(),
                    count: *count,
                })
                .collect(),
            org_growth_7d: {
                let cutoff = (chrono::Utc::now() - chrono::Duration::days(7)).date_naive();
                snap.org_growth_30d
                    .iter()
                    .filter(|(date, _)| *date >= cutoff)
                    .map(|(date, count)| DailyCount {
                        date: date.to_string(),
                        count: *count,
                    })
                    .collect()
            },
            org_growth_30d: snap
                .org_growth_30d
                .iter()
                .map(|(date, count)| DailyCount {
                    date: date.to_string(),
                    count: *count,
                })
                .collect(),
        },
        activity: ActivityAnalytics {
            logins_last_24h: snap.logins_24h,
            logins_last_7d: snap.logins_7d,
            api_requests_last_24h: snap.api_requests_24h,
            api_requests_last_7d: snap.api_requests_7d,
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
    // Check if user is admin
    let user = state
        .store
        .find_user_by_id(user_id)
        .await?
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

    let snap = state.store.get_conversion_funnel_snapshot(interval).await?;

    let mut stages = Vec::new();
    let signup_count = snap.signups as f64;

    stages.push(FunnelStage {
        stage: "Signup".to_string(),
        count: snap.signups,
        conversion_rate: 100.0,
        drop_off: 0.0,
    });

    let verified_rate = if signup_count > 0.0 {
        (snap.verified as f64 / signup_count) * 100.0
    } else {
        0.0
    };
    stages.push(FunnelStage {
        stage: "Email Verified".to_string(),
        count: snap.verified,
        conversion_rate: verified_rate,
        drop_off: 100.0 - verified_rate,
    });

    let login_rate = if signup_count > 0.0 {
        (snap.logged_in as f64 / signup_count) * 100.0
    } else {
        0.0
    };
    stages.push(FunnelStage {
        stage: "First Login".to_string(),
        count: snap.logged_in,
        conversion_rate: login_rate,
        drop_off: verified_rate - login_rate,
    });

    let org_rate = if signup_count > 0.0 {
        (snap.org_created as f64 / signup_count) * 100.0
    } else {
        0.0
    };
    stages.push(FunnelStage {
        stage: "Organization Created".to_string(),
        count: snap.org_created,
        conversion_rate: org_rate,
        drop_off: login_rate - org_rate,
    });

    let feature_rate = if signup_count > 0.0 {
        (snap.feature_users as f64 / signup_count) * 100.0
    } else {
        0.0
    };
    stages.push(FunnelStage {
        stage: "First Feature Use".to_string(),
        count: snap.feature_users,
        conversion_rate: feature_rate,
        drop_off: org_rate - feature_rate,
    });

    let checkout_rate = if signup_count > 0.0 {
        (snap.checkout_initiated as f64 / signup_count) * 100.0
    } else {
        0.0
    };
    stages.push(FunnelStage {
        stage: "Checkout Initiated".to_string(),
        count: snap.checkout_initiated,
        conversion_rate: checkout_rate,
        drop_off: feature_rate - checkout_rate,
    });

    let paid_rate = if signup_count > 0.0 {
        (snap.paid_subscribers as f64 / signup_count) * 100.0
    } else {
        0.0
    };
    stages.push(FunnelStage {
        stage: "Paid Subscription".to_string(),
        count: snap.paid_subscribers,
        conversion_rate: paid_rate,
        drop_off: checkout_rate - paid_rate,
    });

    Ok(Json(ConversionFunnelResponse {
        period: period.to_string(),
        stages,
        overall_conversion_rate: paid_rate,
        time_to_convert: snap.time_to_convert_days,
    }))
}
