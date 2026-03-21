//! Re-export of traffic shaping types from mockforge-core.
//!
//! These types originated in mockforge-core and are being migrated to mockforge-chaos
//! as part of the chaos module extraction (Phase 6a). During the transition period,
//! both paths work, but prefer importing from `mockforge_chaos::core_traffic_shaping`.

pub use mockforge_core::traffic_shaping::{
    BandwidthConfig, BurstLossConfig, TrafficShaper, TrafficShapingConfig,
};

// Also re-export internal types used by tests and downstream consumers
pub use mockforge_core::traffic_shaping::{BandwidthStats, BurstLossOverride, BurstLossStats};
