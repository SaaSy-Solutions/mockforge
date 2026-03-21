//! Re-export of failure injection types from mockforge-core.
//!
//! These types originated in mockforge-core and are being migrated to mockforge-chaos
//! as part of the chaos module extraction (Phase 6a). During the transition period,
//! both paths work, but prefer importing from `mockforge_chaos::core_failure_injection`.

pub use mockforge_core::failure_injection::{
    create_failure_injector, FailureConfig, FailureInjector, TagFailureConfig,
};
