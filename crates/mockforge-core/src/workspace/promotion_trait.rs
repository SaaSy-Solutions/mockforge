//! Promotion service trait
//!
//! This trait allows pipeline steps to trigger promotions without creating
//! circular dependencies between crates.

use crate::workspace::mock_environment::MockEnvironmentName;
use crate::workspace::scenario_promotion::PromotionEntityType;
use crate::Result;
use uuid::Uuid;

/// Trait for services that can perform promotions
#[async_trait::async_trait]
pub trait PromotionService: Send + Sync {
    /// Promote an entity from one environment to another
    async fn promote_entity(
        &self,
        workspace_id: Uuid,
        entity_type: PromotionEntityType,
        entity_id: String,
        entity_version: Option<String>,
        from_environment: MockEnvironmentName,
        to_environment: MockEnvironmentName,
        promoted_by: Uuid,
        comments: Option<String>,
    ) -> Result<Uuid>;
}
