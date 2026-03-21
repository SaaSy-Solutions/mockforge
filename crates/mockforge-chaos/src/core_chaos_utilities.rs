//! Re-export of chaos utility types from mockforge-core.
//!
//! These types originated in mockforge-core and are being migrated to mockforge-chaos
//! as part of the chaos module extraction (Phase 6a). During the transition period,
//! both paths work, but prefer importing from `mockforge_chaos::core_chaos_utilities`.

pub use mockforge_core::chaos_utilities::{ChaosConfig, ChaosEngine, ChaosResult, ChaosStatistics};
