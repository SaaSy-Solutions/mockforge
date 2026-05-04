//! Provider routing for cloud AI calls.
//!
//! `pick_provider` is a pure function so routing logic is unit-testable
//! without database fixtures. It encodes the rules from
//! `docs/cloud/CLOUD_AI_STUDIO_DESIGN.md`:
//!
//! - Free + BYOK         → Byok    (free + own key is allowed; rate-cap applies)
//! - Free + no BYOK      → Disabled (must upgrade or add BYOK)
//! - Paid + BYOK         → Byok    (paid prefers BYOK; tokens not metered)
//! - Paid + no BYOK      → Platform (falls back to platform key, metered)

use mockforge_registry_core::models::BYOKConfig;

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
/// - `is_paid_plan`: true for Pro/Team/Enterprise, false for Free.
/// - `byok`: the org's BYOK setting if configured (already decrypted is
///   not required here — we just need to know whether one exists and is
///   `enabled`). Pass `None` if missing or `enabled = false`.
pub fn pick_provider(is_paid_plan: bool, byok: Option<BYOKConfig>) -> Provider {
    match (is_paid_plan, byok) {
        // BYOK takes precedence on every plan when enabled.
        (_, Some(cfg)) if cfg.enabled => Provider::Byok(cfg),
        // Paid plan without BYOK falls through to platform credits.
        (true, _) => Provider::Platform,
        // Free plan with no usable BYOK is locked.
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
    fn free_without_byok_is_disabled() {
        let p = pick_provider(false, None);
        assert!(matches!(p, Provider::Disabled));
    }

    #[test]
    fn free_with_disabled_byok_is_disabled() {
        // BYOK row exists but enabled=false should not unlock Free tier.
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
