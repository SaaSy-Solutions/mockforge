//! AI quota check + usage recording helpers.
//!
//! These are functions, not Axum middleware. The AI request flow needs
//! to know the platform-vs-BYOK choice *before* deciding whether to
//! enforce the platform token quota — that's a per-request decision the
//! handler makes after running `pick_provider`. So we expose `check_ai_quota`
//! and `record_ai_usage` as helpers the handler calls explicitly.

use crate::ai::provider::ProviderSelection;
use crate::error::{ApiError, ApiResult};
use crate::handlers::usage::effective_limits;
use crate::models::{Organization, Plan, UsageCounter};
use crate::store::with_org_context;
use crate::AppState;
use uuid::Uuid;

/// Result of a pre-call quota check.
#[derive(Debug, Clone)]
pub struct QuotaCheck {
    /// Token quota for the current month (-1 = unlimited).
    pub limit: i64,
    /// Tokens consumed or durably reserved by outstanding/uncertain calls this month.
    pub used: i64,
    /// Credits held by pending or uncertain provider calls.
    pub reserved: i64,
    /// True if the request is allowed to proceed.
    pub allowed: bool,
    /// Human-readable reason when `allowed = false`.
    pub deny_reason: Option<String>,
}

impl QuotaCheck {
    /// Convert a denied check into the matching API error so handlers can `?`-propagate.
    /// Maps to 403 (`ResourceLimitExceeded`) — monthly quota is not a transient
    /// rate-limit, the user must upgrade or add BYOK to recover.
    pub fn into_error(self) -> ApiError {
        let reason = self.deny_reason.unwrap_or_else(|| "AI quota exceeded".to_string());
        ApiError::ResourceLimitExceeded(reason)
    }
}

/// Decide whether an AI request is allowed under the org's plan + usage state.
///
/// BYOK requests skip the token quota (the user pays their own provider bill);
/// rate caps for BYOK are enforced separately. Platform requests are gated by
/// the `ai_tokens_per_month` plan limit.
pub async fn check_ai_quota(
    state: &AppState,
    org: &Organization,
    selection: ProviderSelection,
) -> ApiResult<QuotaCheck> {
    let limits = effective_limits(state, org).await?;
    let limit = limits.get("ai_tokens_per_month").and_then(|v| v.as_i64()).unwrap_or(0);

    let usage = state.store.get_or_create_current_usage_counter(org.id).await?;
    let counter_id = usage.id;
    let period_start = usage.period_start;
    let org_id = org.id;
    let mut reserved: i64 = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(sqlx::query_scalar("SELECT COALESCE(SUM(budget),0)::bigint FROM ai_token_reservations WHERE usage_counter_id=$1 AND org_id=$2 AND state <> 'completed'")
                .bind(counter_id).bind(org_id).fetch_one(&mut **tx).await?)
        })
    }).await?;
    let mut used = usage.ai_tokens_used.saturating_add(reserved);

    if matches!(selection, ProviderSelection::Platform) {
        match crate::handlers::entitlements::require_verified_free_owner(state, org).await {
            Ok(()) => {}
            Err(ApiError::ValidationFailed(message)) => {
                return Ok(QuotaCheck {
                    limit,
                    used,
                    reserved,
                    allowed: false,
                    deny_reason: Some(message),
                });
            }
            Err(error) => return Err(error),
        }
    }

    if matches!(selection, ProviderSelection::Platform)
        && limit >= 0
        && crate::handlers::entitlements::effective_plan(state, org).await? == Plan::Free
    {
        let shared: Option<(i64, i64, i64)> = with_org_context(state.db.runtime_pool(), org_id, |tx| Box::pin(async move {
            Ok(sqlx::query_as("SELECT f.granted_tokens,f.used_tokens,f.reserved_tokens FROM free_ai_credit_periods f JOIN organizations o ON o.owner_id=f.owner_user_id WHERE o.id=$1 AND f.period_start=$2")
                .bind(org_id).bind(period_start).fetch_optional(&mut **tx).await?)
        })).await?;
        if let Some((grant, consumed, held)) = shared {
            if grant >= 0 {
                let available = grant
                    .max(limit)
                    .saturating_sub(consumed)
                    .saturating_sub(held)
                    .max(0)
                    .min(limit.saturating_sub(used).max(0));
                used = limit.saturating_sub(available);
                reserved = reserved.max(held).min(used);
            }
        }
    }

    match selection {
        ProviderSelection::Disabled => Ok(QuotaCheck {
            limit,
            used,
            reserved,
            allowed: false,
            deny_reason: Some(
                "AI features require a platform token allowance or an enabled BYOK provider key. \
                 Review your quota or add a BYOK key in Settings → BYOK."
                    .into(),
            ),
        }),
        ProviderSelection::Byok => {
            // BYOK bypasses the token quota; rate caps live elsewhere.
            Ok(QuotaCheck {
                limit,
                used,
                reserved,
                allowed: true,
                deny_reason: None,
            })
        }
        ProviderSelection::Platform => {
            // -1 means unlimited; 0 means no platform credits on this plan.
            if limit < 0 {
                Ok(QuotaCheck {
                    limit,
                    used,
                    reserved,
                    allowed: true,
                    deny_reason: None,
                })
            } else if used >= limit {
                Ok(QuotaCheck {
                    limit,
                    used,
                    reserved,
                    allowed: false,
                    deny_reason: Some(format!(
                        "Monthly AI token quota exhausted ({used} / {limit}). \
                         Add a BYOK provider key, upgrade your plan, or buy a top-up pack."
                    )),
                })
            } else {
                Ok(QuotaCheck {
                    limit,
                    used,
                    reserved,
                    allowed: true,
                    deny_reason: None,
                })
            }
        }
    }
}

/// Record post-call token usage. Only billed for `Platform` requests —
/// BYOK requests don't consume the platform's token quota.
///
/// Legacy accounting helper for already-authoritative external usage records.
/// Studio and workers use `reservation::call_with_budget` for admission and
/// exact-period settlement instead of this post-call-only helper.
pub async fn record_ai_usage(
    state: &AppState,
    org_id: Uuid,
    selection: ProviderSelection,
    tokens: i64,
) -> ApiResult<()> {
    if tokens <= 0 || !matches!(selection, ProviderSelection::Platform) {
        return Ok(());
    }
    // Request-path (RLS) pool bound to the org being charged. Also called from
    // the test-generation worker, which passes the job's org explicitly.
    with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            UsageCounter::increment_ai_tokens(tx, org_id, tokens).await?;
            Ok(())
        })
    })
    .await?;
    Ok(())
}
