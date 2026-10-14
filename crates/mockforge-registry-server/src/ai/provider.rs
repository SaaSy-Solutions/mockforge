//! Provider routing for cloud AI calls.
//!
//! `pick_provider` is a pure function so routing logic is unit-testable
//! without database fixtures. Enabled BYOK takes precedence; otherwise a
//! positive or unlimited platform allowance enables metered platform calls.
//! The Free plan includes a small allowance and retains BYOK support.

use mockforge_registry_core::models::{BYOKConfig, Organization};

use crate::{error::ApiResult, handlers::usage::effective_limits, AppState};

/// Resolve provider availability from subscription-aware token entitlements.
/// Every caller still checks quota before invoking or enqueuing platform AI.
pub async fn resolve_provider(
    state: &AppState,
    org: &Organization,
    byok: Option<BYOKConfig>,
) -> ApiResult<Provider> {
    let limits = effective_limits(state, org).await?;
    let credits = limits.get("ai_tokens_per_month").and_then(|v| v.as_i64()).unwrap_or(0);
    Ok(pick_provider(credits != 0, byok))
}

/// Selected provider for a single AI request.
#[derive(Debug, Clone)]
pub enum Provider {
    /// Use the org's own provider key. Tokens are not billed against
    /// the platform's `ai_tokens_per_month` quota; rate caps still apply.
    Byok(BYOKConfig),
    /// Use the platform's provider key. Tokens count against the org's
    /// monthly quota.
    Platform,
    /// AI is not available on this plan/configuration. Handler should
    /// return a 402-style error pointing the user to upgrade or add BYOK.
    Disabled,
}

/// Lightweight summary of which path was selected. Used for logging,
/// usage records, and the `provider` field exposed in responses so the
/// UI can render the "Using your key" / "Using platform credits" badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderSelection {
    Byok,
    Platform,
    Disabled,
}

impl Provider {
    pub fn selection(&self) -> ProviderSelection {
        match self {
            Provider::Byok(_) => ProviderSelection::Byok,
            Provider::Platform => ProviderSelection::Platform,
            Provider::Disabled => ProviderSelection::Disabled,
        }
    }
}

/// Pure routing decision. Inputs:
/// - `has_platform_credits`: true for a positive or unlimited platform allowance.
/// - `byok`: the org's BYOK setting if configured (already decrypted is
///   not required here — we just need to know whether one exists and is
///   `enabled`). Pass `None` if missing or `enabled = false`.
pub fn pick_provider(has_platform_credits: bool, byok: Option<BYOKConfig>) -> Provider {
    match (has_platform_credits, byok) {
        // BYOK takes precedence on every plan when enabled.
        (_, Some(cfg)) if cfg.enabled => Provider::Byok(cfg),
        // Any plan with a platform allowance uses metered credits.
        (true, _) => Provider::Platform,
        // Plans without platform credits need a usable BYOK key.
        (false, _) => Provider::Disabled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled_byok() -> BYOKConfig {
        BYOKConfig {
            provider: "openai".into(),
            api_key: "encrypted-test-key".into(),
            base_url: None,
            model: Some("gpt-4o-mini".into()),
            enabled: true,
        }
    }

    fn disabled_byok() -> BYOKConfig {
        BYOKConfig {
            enabled: false,
            ..enabled_byok()
        }
    }

    #[test]
    fn free_plus_byok_uses_byok() {
        let p = pick_provider(false, Some(enabled_byok()));
        assert!(matches!(p, Provider::Byok(_)));
        assert_eq!(p.selection(), ProviderSelection::Byok);
    }

    #[test]
    fn zero_credit_without_byok_is_disabled() {
        let p = pick_provider(false, None);
        assert!(matches!(p, Provider::Disabled));
    }

    #[test]
    fn zero_credit_with_disabled_byok_is_disabled() {
        // A disabled BYOK row does not enable calls without a platform allowance.
        let p = pick_provider(false, Some(disabled_byok()));
        assert!(matches!(p, Provider::Disabled));
    }

    #[test]
    fn paid_plus_byok_prefers_byok() {
        let p = pick_provider(true, Some(enabled_byok()));
        assert!(matches!(p, Provider::Byok(_)));
    }

    #[test]
    fn paid_without_byok_uses_platform() {
        let p = pick_provider(true, None);
        assert!(matches!(p, Provider::Platform));
    }

    #[test]
    fn paid_with_disabled_byok_uses_platform() {
        // Disabled BYOK should not block paid users from platform credits.
        let p = pick_provider(true, Some(disabled_byok()));
        assert!(matches!(p, Provider::Platform));
    }
}
