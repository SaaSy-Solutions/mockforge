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
use crate::models::{Organization, UsageCounter};
use crate::AppState;
use uuid::Uuid;

/// Result of a pre-call quota check.
#[derive(Debug, Clone)]
pub struct QuotaCheck {
    /// Token quota for the current month (-1 = unlimited).
    pub limit: i64,
    /// Tokens already consumed this month.
    pub used: i64,
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
    let used = usage.ai_tokens_used;

    match selection {
        ProviderSelection::Disabled => Ok(QuotaCheck {
            limit,
            used,
            allowed: false,
            deny_reason: Some(
                "AI features are not available on the Free plan without a BYOK provider key. \
                 Upgrade to Pro or add a BYOK key in Settings → BYOK."
                    .into(),
            ),
        }),
        ProviderSelection::Byok => {
            // BYOK bypasses the token quota; rate caps live elsewhere.
            Ok(QuotaCheck {
                limit,
                used,
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
                    allowed: true,
                    deny_reason: None,
                })
            } else if used >= limit {
                Ok(QuotaCheck {
                    limit,
                    used,
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
/// The increment is now a single atomic `UPDATE ... RETURNING` (#867), so the
/// counter can no longer be clobbered by concurrent writers. Note that the
/// pre-call [`check_ai_quota`] read and this post-call increment still form a
/// check-then-act window: many in-flight Platform requests can each pass the
/// pre-call check and then push `ai_tokens_used` past the limit. That is an
/// acceptable, bounded overshoot for a *post-call accounting* model (tokens are
/// only known after the LLM responds) — the next [`check_ai_quota`] sees the
/// now-correct total and denies. The atomic increment guarantees no usage is
/// *lost*; eliminating the overshoot entirely would require pre-reserving an
/// upper-bound token budget before the call, which is out of scope here.
pub async fn record_ai_usage(
    state: &AppState,
    org_id: Uuid,
    selection: ProviderSelection,
    tokens: i64,
) -> ApiResult<()> {
    if tokens <= 0 || !matches!(selection, ProviderSelection::Platform) {
        return Ok(());
    }
    let _new_total = UsageCounter::increment_ai_tokens(state.db.pool(), org_id, tokens)
        .await
        .map_err(ApiError::Database)?;
    Ok(())
}
