//! Scenario storage and versioning
//!
//! This module provides storage, versioning, and export/import functionality
//! for behavioral scenarios.

use super::scenario_types::BehavioralScenario;
use crate::database::RecorderDatabase;
use anyhow::Result;
use std::path::Path;
use tracing::info;

/// Storage for behavioral scenarios
pub struct ScenarioStorage {
    db: RecorderDatabase,
}

impl ScenarioStorage {
    /// Create a new scenario storage
    pub fn new(db: RecorderDatabase) -> Self {
        Self { db }
    }

    /// Store a scenario with version
    pub async fn store_scenario(&self, scenario: &BehavioralScenario, version: &str) -> Result<()> {
        self.db
            .store_scenario(scenario, version)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to store scenario: {}", e))?;

        info!("Stored scenario: {} v{}", scenario.id, version);
        Ok(())
    }

    /// Store a scenario with auto-versioning (increments from existing version)
    pub async fn store_scenario_auto_version(
        &self,
        scenario: &BehavioralScenario,
    ) -> Result<String> {
        // Get latest version for this scenario name
        let scenarios = self
            .db
            .list_scenarios(None)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list scenarios: {}", e))?;

        let latest_version = scenarios
            .iter()
            .filter(|s| s.name == scenario.name)
            .map(|s| s.version.clone())
            .max_by(|a, b| {
                // Simple version comparison (semver-like)
                compare_versions(a, b)
            })
            .unwrap_or_else(|| "1.0.0".to_string());

        // Increment patch version
        let new_version = increment_version(&latest_version);
        self.store_scenario(scenario, &new_version).await?;
        Ok(new_version)
    }

    /// Get a scenario by ID
    pub async fn get_scenario(&self, scenario_id: &str) -> Result<Option<BehavioralScenario>> {
        self.db
            .get_scenario(scenario_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get scenario: {}", e))
    }

    /// Get a scenario by name and version
    pub async fn get_scenario_by_name_version(
        &self,
        name: &str,
        version: &str,
    ) -> Result<Option<BehavioralScenario>> {
        self.db
            .get_scenario_by_name_version(name, version)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get scenario: {}", e))
    }

    /// List all scenarios
    pub async fn list_scenarios(&self, limit: Option<usize>) -> Result<Vec<ScenarioInfo>> {
        let limit = limit.map(|l| l as i64);
        let rows = self
            .db
            .list_scenarios(limit)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list scenarios: {}", e))?;

        Ok(rows
            .into_iter()
            .map(|row| ScenarioInfo {
                id: row.id,
                name: row.name,
                version: row.version,
                description: row.description,
                created_at: row.created_at,
                updated_at: row.updated_at,
                tags: serde_json::from_str(&row.tags).unwrap_or_default(),
            })
            .collect())
    }

    /// Export scenario to YAML/JSON
    pub async fn export_scenario(&self, scenario_id: &str, format: &str) -> Result<String> {
        let scenario = self
            .get_scenario(scenario_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Scenario not found: {}", scenario_id))?;

        match format.to_lowercase().as_str() {
            "yaml" | "yml" => serde_yaml::to_string(&scenario)
                .map_err(|e| anyhow::anyhow!("Failed to serialize to YAML: {}", e)),
            "json" => serde_json::to_string_pretty(&scenario)
                .map_err(|e| anyhow::anyhow!("Failed to serialize to JSON: {}", e)),
            _ => Err(anyhow::anyhow!("Unsupported format: {}", format)),
        }
    }

    /// Export scenario to file
    pub async fn export_scenario_to_file(
        &self,
        scenario_id: &str,
        output_path: &Path,
    ) -> Result<()> {
        let format = output_path.extension().and_then(|ext| ext.to_str()).unwrap_or("yaml");
        let content = self.export_scenario(scenario_id, format).await?;
        tokio::fs::write(output_path, content)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to write file: {}", e))?;
        Ok(())
    }

    /// Import scenario from YAML/JSON string
    pub async fn import_scenario(&self, data: &str, format: &str) -> Result<BehavioralScenario> {
        let scenario = match format.to_lowercase().as_str() {
            "yaml" | "yml" => serde_yaml::from_str(data)
                .map_err(|e| anyhow::anyhow!("Failed to parse YAML: {}", e))?,
            "json" => serde_json::from_str(data)
                .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?,
            _ => return Err(anyhow::anyhow!("Unsupported format: {}", format)),
        };

        Ok(scenario)
    }

    /// Import scenario from file
    pub async fn import_scenario_from_file(&self, input_path: &Path) -> Result<BehavioralScenario> {
        let content = tokio::fs::read_to_string(input_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;

        let format = input_path.extension().and_then(|ext| ext.to_str()).unwrap_or("yaml");

        self.import_scenario(&content, format).await
    }

    /// Delete a scenario
    pub async fn delete_scenario(&self, scenario_id: &str) -> Result<()> {
        self.db
            .delete_scenario(scenario_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to delete scenario: {}", e))?;
        Ok(())
    }
}

/// Scenario information for listing
#[derive(Debug, Clone)]
pub struct ScenarioInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<String>,
}

/// Increment version string (simple patch version increment)
fn increment_version(version: &str) -> String {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 3 {
        if let Ok(patch) = parts[2].parse::<u64>() {
            return format!("{}.{}.{}", parts[0], parts[1], patch + 1);
        }
    }
    format!("{}.0.1", version)
}

// Simple version comparison helper
fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<u64> = a.split('.').filter_map(|p| p.parse().ok()).collect();
    let b_parts: Vec<u64> = b.split('.').filter_map(|p| p.parse().ok()).collect();

    for (a_val, b_val) in a_parts.iter().zip(b_parts.iter()) {
        match a_val.cmp(b_val) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    a_parts.len().cmp(&b_parts.len())
}
