//! A/B Testing for Mocks
//!
//! This module provides functionality for defining multiple mock variants
//! for a single endpoint and routing traffic to different variants based
//! on configuration (e.g., 10% to variant=new_user, 90% to variant=existing_user).

pub mod analytics;
pub mod manager;
pub mod middleware;
pub mod types;

pub use analytics::{ABTestReport, VariantComparison};
pub use manager::VariantManager;
pub use middleware::{apply_variant_to_response, select_variant, ABTestingMiddlewareState};
pub use types::{
    ABTestConfig, MockVariant, VariantAllocation, VariantAnalytics, VariantSelectionStrategy,
};
