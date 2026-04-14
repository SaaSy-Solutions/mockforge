//! Workspace promotion types
//!
//! Extracted from `mockforge-core::workspace::{mock_environment, scenario_promotion}`
//! (Phase 6 / A10).
//!
//! Only the simple enum types live here. The richer `PromotionRequest`,
//! `PromotionHistory`, and `PromotionService` trait stay in `mockforge-core`
//! because their field shapes vary across consumers.

use serde::{Deserialize, Serialize};

/// Mock environment names — used to scope behavior, chaos, and promotion workflows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MockEnvironmentName {
    /// Development environment - typically permissive, high chaos for testing
    Dev,
    /// Test environment - balanced settings for integration testing
    Test,
    /// Production-like environment - strict settings, minimal chaos
    Prod,
}

impl MockEnvironmentName {
    /// Convert to string.
    pub fn as_str(&self) -> &'static str {
        match self {
            MockEnvironmentName::Dev => "dev",
            MockEnvironmentName::Test => "test",
            MockEnvironmentName::Prod => "prod",
        }
    }

    /// Parse from string.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dev" => Some(MockEnvironmentName::Dev),
            "test" => Some(MockEnvironmentName::Test),
            "prod" => Some(MockEnvironmentName::Prod),
            _ => None,
        }
    }

    /// Get all environment names in promotion order.
    pub fn promotion_order() -> Vec<Self> {
        vec![
            MockEnvironmentName::Dev,
            MockEnvironmentName::Test,
            MockEnvironmentName::Prod,
        ]
    }

    /// Get the next environment in promotion order.
    pub fn next(&self) -> Option<Self> {
        match self {
            MockEnvironmentName::Dev => Some(MockEnvironmentName::Test),
            MockEnvironmentName::Test => Some(MockEnvironmentName::Prod),
            MockEnvironmentName::Prod => None,
        }
    }

    /// Get the previous environment in promotion order.
    pub fn previous(&self) -> Option<Self> {
        match self {
            MockEnvironmentName::Dev => None,
            MockEnvironmentName::Test => Some(MockEnvironmentName::Dev),
            MockEnvironmentName::Prod => Some(MockEnvironmentName::Test),
        }
    }
}

impl std::fmt::Display for MockEnvironmentName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Type of entity being promoted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromotionEntityType {
    /// Scenario promotion
    Scenario,
    /// Persona promotion
    Persona,
    /// Configuration promotion (reality, chaos, drift budget)
    Config,
}

impl std::fmt::Display for PromotionEntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PromotionEntityType::Scenario => write!(f, "scenario"),
            PromotionEntityType::Persona => write!(f, "persona"),
            PromotionEntityType::Config => write!(f, "config"),
        }
    }
}

/// Trait for services that can perform promotions.
///
/// Allows pipeline steps and other consumers to trigger promotions without
/// creating circular dependencies between crates.
#[allow(clippy::too_many_arguments)]
#[async_trait::async_trait]
pub trait PromotionService: Send + Sync {
    /// Promote an entity from one environment to another.
    async fn promote_entity(
        &self,
        workspace_id: uuid::Uuid,
        entity_type: PromotionEntityType,
        entity_id: String,
        entity_version: Option<String>,
        from_environment: MockEnvironmentName,
        to_environment: MockEnvironmentName,
        promoted_by: uuid::Uuid,
        comments: Option<String>,
    ) -> crate::Result<uuid::Uuid>;
}
