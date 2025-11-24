//! Mock environment configuration for workspace environments
//!
//! Mock environments (dev/test/prod) allow workspaces to have different
//! configurations for reality levels, chaos profiles, and drift budgets
//! per environment, similar to application environments.

use crate::chaos_utilities::ChaosConfig;
use crate::contract_drift::DriftBudgetConfig;
use crate::reality::RealityConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Mock environment name (dev/test/prod)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
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
    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            MockEnvironmentName::Dev => "dev",
            MockEnvironmentName::Test => "test",
            MockEnvironmentName::Prod => "prod",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dev" => Some(MockEnvironmentName::Dev),
            "test" => Some(MockEnvironmentName::Test),
            "prod" => Some(MockEnvironmentName::Prod),
            _ => None,
        }
    }

    /// Get all environment names in promotion order
    pub fn promotion_order() -> Vec<Self> {
        vec![
            MockEnvironmentName::Dev,
            MockEnvironmentName::Test,
            MockEnvironmentName::Prod,
        ]
    }

    /// Get the next environment in promotion order
    pub fn next(&self) -> Option<Self> {
        match self {
            MockEnvironmentName::Dev => Some(MockEnvironmentName::Test),
            MockEnvironmentName::Test => Some(MockEnvironmentName::Prod),
            MockEnvironmentName::Prod => None,
        }
    }

    /// Get the previous environment in promotion order
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

/// Mock environment configuration
///
/// Each workspace can have multiple mock environments (dev/test/prod),
/// each with its own reality settings, chaos profiles, and drift budgets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockEnvironment {
    /// Unique identifier for this environment
    pub id: String,
    /// Workspace ID this environment belongs to
    pub workspace_id: String,
    /// Environment name (dev/test/prod)
    pub name: MockEnvironmentName,
    /// Environment-specific reality configuration
    /// If None, uses workspace default
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reality_config: Option<RealityConfig>,
    /// Environment-specific chaos configuration
    /// If None, uses workspace default
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chaos_config: Option<ChaosConfig>,
    /// Environment-specific drift budget configuration
    /// If None, uses workspace default
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drift_budget_config: Option<DriftBudgetConfig>,
}

impl MockEnvironment {
    /// Create a new mock environment with default settings
    pub fn new(workspace_id: String, name: MockEnvironmentName) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id,
            name,
            reality_config: None,
            chaos_config: None,
            drift_budget_config: None,
        }
    }

    /// Create a new mock environment with all configurations
    pub fn with_configs(
        workspace_id: String,
        name: MockEnvironmentName,
        reality_config: Option<RealityConfig>,
        chaos_config: Option<ChaosConfig>,
        drift_budget_config: Option<DriftBudgetConfig>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id,
            name,
            reality_config,
            chaos_config,
            drift_budget_config,
        }
    }
}

/// Mock environment manager for a workspace
///
/// Manages multiple mock environments (dev/test/prod) for a workspace,
/// providing methods to get environment-specific configurations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockEnvironmentManager {
    /// Workspace ID
    pub workspace_id: String,
    /// All environments indexed by name
    pub environments: HashMap<MockEnvironmentName, MockEnvironment>,
    /// Active environment name
    pub active_environment: Option<MockEnvironmentName>,
}

impl MockEnvironmentManager {
    /// Create a new environment manager
    pub fn new(workspace_id: String) -> Self {
        Self {
            workspace_id,
            environments: HashMap::new(),
            active_environment: None,
        }
    }

    /// Add an environment
    pub fn add_environment(&mut self, environment: MockEnvironment) {
        self.environments.insert(environment.name, environment);
    }

    /// Get an environment by name
    pub fn get_environment(&self, name: MockEnvironmentName) -> Option<&MockEnvironment> {
        self.environments.get(&name)
    }

    /// Get the active environment
    pub fn get_active_environment(&self) -> Option<&MockEnvironment> {
        self.active_environment.and_then(|name| self.environments.get(&name))
    }

    /// Set the active environment
    pub fn set_active_environment(&mut self, name: MockEnvironmentName) -> Result<(), String> {
        if self.environments.contains_key(&name) {
            self.active_environment = Some(name);
            Ok(())
        } else {
            Err(format!("Environment '{}' not found", name.as_str()))
        }
    }

    /// List all environments
    pub fn list_environments(&self) -> Vec<&MockEnvironment> {
        let order = MockEnvironmentName::promotion_order();
        let mut result: Vec<&MockEnvironment> =
            order.iter().filter_map(|name| self.environments.get(name)).collect();
        // Add any environments not in the standard order
        for (name, env) in &self.environments {
            if !order.contains(name) {
                result.push(env);
            }
        }
        result
    }

    /// Remove an environment
    pub fn remove_environment(&mut self, name: MockEnvironmentName) -> Option<MockEnvironment> {
        if self.active_environment == Some(name) {
            self.active_environment = None;
        }
        self.environments.remove(&name)
    }
}

impl Default for MockEnvironmentManager {
    fn default() -> Self {
        Self::new(String::new())
    }
}

/// Environment-specific configuration overrides
///
/// This struct represents the configuration overrides that can be applied
/// per environment, merging with workspace defaults.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MockEnvironmentConfig {
    /// Environment-specific reality configuration override
    /// If None, uses workspace default
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reality_config: Option<RealityConfig>,
    /// Environment-specific chaos configuration override
    /// If None, uses workspace default
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chaos_config: Option<ChaosConfig>,
    /// Environment-specific drift budget configuration override
    /// If None, uses workspace default
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drift_budget_config: Option<DriftBudgetConfig>,
}

/// Environment configuration resolver
///
/// Resolves environment-specific configurations by merging workspace defaults
/// with environment-specific overrides.
pub struct EnvironmentConfigResolver;

impl EnvironmentConfigResolver {
    /// Resolve reality configuration for an environment
    ///
    /// Merges workspace default reality config with environment-specific override.
    /// If environment override exists, it takes precedence.
    pub fn resolve_reality_config(
        workspace_default: Option<RealityConfig>,
        environment_override: Option<RealityConfig>,
    ) -> Option<RealityConfig> {
        environment_override.or(workspace_default)
    }

    /// Resolve chaos configuration for an environment
    ///
    /// Merges workspace default chaos config with environment-specific override.
    /// If environment override exists, it takes precedence.
    pub fn resolve_chaos_config(
        workspace_default: Option<ChaosConfig>,
        environment_override: Option<ChaosConfig>,
    ) -> Option<ChaosConfig> {
        environment_override.or(workspace_default)
    }

    /// Resolve drift budget configuration for an environment
    ///
    /// Merges workspace default drift budget config with environment-specific override.
    /// If environment override exists, it takes precedence.
    pub fn resolve_drift_budget_config(
        workspace_default: Option<DriftBudgetConfig>,
        environment_override: Option<DriftBudgetConfig>,
    ) -> Option<DriftBudgetConfig> {
        environment_override.or(workspace_default)
    }

    /// Resolve all configurations for an environment
    ///
    /// Returns a resolved configuration with all environment-specific overrides
    /// merged with workspace defaults.
    pub fn resolve_all_configs(
        workspace_defaults: &WorkspaceEnvironmentDefaults,
        environment_config: &MockEnvironmentConfig,
    ) -> ResolvedEnvironmentConfig {
        ResolvedEnvironmentConfig {
            reality_config: Self::resolve_reality_config(
                workspace_defaults.reality_config.clone(),
                environment_config.reality_config.clone(),
            ),
            chaos_config: Self::resolve_chaos_config(
                workspace_defaults.chaos_config.clone(),
                environment_config.chaos_config.clone(),
            ),
            drift_budget_config: Self::resolve_drift_budget_config(
                workspace_defaults.drift_budget_config.clone(),
                environment_config.drift_budget_config.clone(),
            ),
        }
    }

    /// Deserialize environment config from JSONB value
    ///
    /// Attempts to deserialize a JSONB value into MockEnvironmentConfig.
    /// Returns None if deserialization fails or value is null.
    pub fn from_jsonb(value: &Value) -> Result<MockEnvironmentConfig, serde_json::Error> {
        serde_json::from_value(value.clone())
    }

    /// Serialize environment config to JSONB value
    pub fn to_jsonb(config: &MockEnvironmentConfig) -> Value {
        serde_json::to_value(config).unwrap_or_else(|_| serde_json::json!({}))
    }
}

/// Workspace-level default configurations
///
/// These are the default configurations at the workspace level that can be
/// overridden per environment.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceEnvironmentDefaults {
    /// Default reality configuration for the workspace
    pub reality_config: Option<RealityConfig>,
    /// Default chaos configuration for the workspace
    pub chaos_config: Option<ChaosConfig>,
    /// Default drift budget configuration for the workspace
    pub drift_budget_config: Option<DriftBudgetConfig>,
}

/// Resolved environment configuration
///
/// The final resolved configuration after merging workspace defaults
/// with environment-specific overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedEnvironmentConfig {
    /// Resolved reality configuration
    pub reality_config: Option<RealityConfig>,
    /// Resolved chaos configuration
    pub chaos_config: Option<ChaosConfig>,
    /// Resolved drift budget configuration
    pub drift_budget_config: Option<DriftBudgetConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_name_parsing() {
        assert_eq!(MockEnvironmentName::from_str("dev"), Some(MockEnvironmentName::Dev));
        assert_eq!(MockEnvironmentName::from_str("test"), Some(MockEnvironmentName::Test));
        assert_eq!(MockEnvironmentName::from_str("prod"), Some(MockEnvironmentName::Prod));
        assert_eq!(MockEnvironmentName::from_str("invalid"), None);
    }

    #[test]
    fn test_promotion_order() {
        let order = MockEnvironmentName::promotion_order();
        assert_eq!(order.len(), 3);
        assert_eq!(order[0], MockEnvironmentName::Dev);
        assert_eq!(order[1], MockEnvironmentName::Test);
        assert_eq!(order[2], MockEnvironmentName::Prod);
    }

    #[test]
    fn test_next_environment() {
        assert_eq!(MockEnvironmentName::Dev.next(), Some(MockEnvironmentName::Test));
        assert_eq!(MockEnvironmentName::Test.next(), Some(MockEnvironmentName::Prod));
        assert_eq!(MockEnvironmentName::Prod.next(), None);
    }

    #[test]
    fn test_previous_environment() {
        assert_eq!(MockEnvironmentName::Dev.previous(), None);
        assert_eq!(MockEnvironmentName::Test.previous(), Some(MockEnvironmentName::Dev));
        assert_eq!(MockEnvironmentName::Prod.previous(), Some(MockEnvironmentName::Test));
    }
}
