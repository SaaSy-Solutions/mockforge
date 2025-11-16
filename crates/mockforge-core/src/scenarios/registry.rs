//! Scenario registry for storing and retrieving scenario definitions

use crate::scenarios::types::ScenarioDefinition;
use crate::{Error, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Registry for managing scenario definitions
#[derive(Debug, Clone)]
pub struct ScenarioRegistry {
    /// Storage for scenario definitions (scenario_id -> ScenarioDefinition)
    scenarios: Arc<RwLock<HashMap<String, ScenarioDefinition>>>,
}

impl ScenarioRegistry {
    /// Create a new scenario registry
    pub fn new() -> Self {
        Self {
            scenarios: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a scenario definition
    pub async fn register(&self, scenario: ScenarioDefinition) -> Result<()> {
        let mut scenarios = self.scenarios.write().await;
        scenarios.insert(scenario.id.clone(), scenario);
        Ok(())
    }

    /// Get a scenario by ID
    pub async fn get(&self, scenario_id: &str) -> Option<ScenarioDefinition> {
        let scenarios = self.scenarios.read().await;
        scenarios.get(scenario_id).cloned()
    }

    /// List all registered scenarios
    pub async fn list(&self) -> Vec<ScenarioDefinition> {
        let scenarios = self.scenarios.read().await;
        scenarios.values().cloned().collect()
    }

    /// Search scenarios by name or tag
    pub async fn search(&self, query: &str) -> Vec<ScenarioDefinition> {
        let scenarios = self.scenarios.read().await;
        let query_lower = query.to_lowercase();
        scenarios
            .values()
            .filter(|scenario| {
                scenario.name.to_lowercase().contains(&query_lower)
                    || scenario.id.to_lowercase().contains(&query_lower)
                    || scenario
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
                    || scenario
                        .description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    /// Remove a scenario
    pub async fn remove(&self, scenario_id: &str) -> bool {
        let mut scenarios = self.scenarios.write().await;
        scenarios.remove(scenario_id).is_some()
    }

    /// Clear all scenarios
    pub async fn clear(&self) {
        let mut scenarios = self.scenarios.write().await;
        scenarios.clear();
    }
}

impl Default for ScenarioRegistry {
    fn default() -> Self {
        Self::new()
    }
}
