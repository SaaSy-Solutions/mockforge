//! Re-export of network profile types from mockforge-core.
//!
//! These types originated in mockforge-core and are being migrated to mockforge-chaos
//! as part of the chaos module extraction (Phase 6a). During the transition period,
//! both paths work, but prefer importing from `mockforge_chaos::core_network_profiles`.

pub use mockforge_core::network_profiles::{NetworkProfile, NetworkProfileCatalog};
