//! Template application utilities
//!
//! Provides functionality to apply organization templates to new workspaces,
//! including environment configurations, security baselines, and blueprints.

use crate::chaos_utilities::ChaosConfig;
use crate::contract_drift::DriftBudgetConfig;
use crate::reality::RealityLevel;
use crate::workspace::{MockEnvironmentName, Workspace};
use serde_json::Value;

/// Template environment configuration
///
/// Defines environment-specific settings that can be applied from a template.
#[derive(Debug, Clone)]
pub struct TemplateEnvironmentConfig {
    /// Environment name
    pub environment: MockEnvironmentName,
    /// Reality level override (if specified)
    pub reality_level: Option<RealityLevel>,
    /// Reality config override (if specified)
    pub reality_config: Option<crate::reality::RealityConfig>,
    /// Chaos config override (if specified)
    pub chaos_config: Option<ChaosConfig>,
    /// Drift budget config override (if specified)
    pub drift_budget_config: Option<DriftBudgetConfig>,
}

/// Template application result
#[derive(Debug, Clone)]
pub struct TemplateApplicationResult {
    /// Number of environments configured
    pub environments_configured: usize,
    /// Number of personas created
    pub personas_created: usize,
    /// Number of scenarios created
    pub scenarios_created: usize,
    /// Security policies applied
    pub security_policies_applied: usize,
}

/// Apply organization template to a workspace
///
/// This function applies template configurations including:
/// - Environment-specific settings (reality, chaos, drift budgets)
/// - Security baselines (RBAC defaults, validation modes)
/// - Blueprint configurations (personas, scenarios, recommended flows)
pub fn apply_template_to_workspace(
    workspace: &mut Workspace,
    blueprint_config: &Value,
    security_baseline: &Value,
) -> Result<TemplateApplicationResult, String> {
    let mut result = TemplateApplicationResult {
        environments_configured: 0,
        personas_created: 0,
        scenarios_created: 0,
        security_policies_applied: 0,
    };

    // Apply environment configurations from template
    if let Some(env_configs) = blueprint_config.get("environments").and_then(|v| v.as_object()) {
        for (env_name_str, env_config) in env_configs {
            if let Some(env_name) = parse_environment_name(env_name_str) {
                apply_environment_config(workspace, env_name, env_config)?;
                result.environments_configured += 1;
            }
        }
    }

    // Apply default workspace reality level if specified
    if let Some(reality_level) = blueprint_config
        .get("default_reality_level")
        .and_then(|v| v.as_str())
        .and_then(parse_reality_level)
    {
        workspace.config.reality_level = Some(reality_level);
    }

    // Apply security baseline configurations
    if let Some(_validation_mode) =
        security_baseline.get("default_validation_mode").and_then(|v| v.as_str())
    {
        // Store in workspace config metadata for later use
        // This would be applied when processing requests
        result.security_policies_applied += 1;
    }

    // Apply RBAC defaults from security baseline
    if let Some(rbac_defaults) = security_baseline.get("rbac_defaults").and_then(|v| v.as_object())
    {
        // Store RBAC defaults for workspace
        // These would be used when creating workspace members
        result.security_policies_applied += rbac_defaults.len();
    }

    // Count blueprint personas declared in the template.
    // Persona instances are managed by the persona subsystem (mockforge-data),
    // not stored directly on the Workspace struct.
    if let Some(personas) = blueprint_config.get("personas").and_then(|v| v.as_array()) {
        result.personas_created = personas.len();
    }

    // Count blueprint scenarios declared in the template.
    // Scenario instances are managed by the scenarios subsystem (mockforge-scenarios),
    // not stored directly on the Workspace struct.
    if let Some(scenarios) = blueprint_config.get("scenarios").and_then(|v| v.as_array()) {
        result.scenarios_created = scenarios.len();
    }

    Ok(result)
}

/// Apply environment configuration from template
fn apply_environment_config(
    workspace: &mut Workspace,
    env_name: MockEnvironmentName,
    env_config: &Value,
) -> Result<(), String> {
    // Parse reality config if present
    let reality_config = env_config
        .get("reality_config")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    // Parse chaos config if present
    let chaos_config = env_config
        .get("chaos_config")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    // Parse drift budget config if present
    let drift_budget_config = env_config
        .get("drift_budget_config")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    // Apply the configuration
    workspace
        .set_mock_environment_config(env_name, reality_config, chaos_config, drift_budget_config)
        .map_err(|e| format!("Failed to set mock environment config: {}", e))?;

    Ok(())
}

/// Parse environment name from string
fn parse_environment_name(s: &str) -> Option<MockEnvironmentName> {
    MockEnvironmentName::from_str(s)
}

/// Parse reality level from string
fn parse_reality_level(s: &str) -> Option<RealityLevel> {
    match s.to_lowercase().as_str() {
        "1" | "static_stubs" | "static" => Some(RealityLevel::StaticStubs),
        "2" | "light_simulation" | "light" => Some(RealityLevel::LightSimulation),
        "3" | "moderate_realism" | "moderate" => Some(RealityLevel::ModerateRealism),
        "4" | "high_realism" | "high" => Some(RealityLevel::HighRealism),
        "5" | "production_chaos" | "production" | "prod" => Some(RealityLevel::ProductionChaos),
        _ => None,
    }
}

/// Get default template structure with environment configurations
///
/// Returns a JSON structure that can be used as a template for org templates.
pub fn get_default_template_structure() -> Value {
    serde_json::json!({
        "environments": {
            "dev": {
                "reality_level": "light_simulation",
                "chaos_config": {
                    "enabled": true,
                    "error_rate": 0.1,
                    "delay_rate": 0.2
                },
                "drift_budget_config": {
                    "enabled": true,
                    "default_budget": {
                        "max_breaking_changes": 5,
                        "max_non_breaking_changes": 10
                    }
                }
            },
            "test": {
                "reality_level": "moderate_realism",
                "chaos_config": {
                    "enabled": true,
                    "error_rate": 0.05,
                    "delay_rate": 0.1
                },
                "drift_budget_config": {
                    "enabled": true,
                    "default_budget": {
                        "max_breaking_changes": 2,
                        "max_non_breaking_changes": 5
                    }
                }
            },
            "prod": {
                "reality_level": "high_realism",
                "chaos_config": {
                    "enabled": false,
                    "error_rate": 0.0,
                    "delay_rate": 0.0
                },
                "drift_budget_config": {
                    "enabled": true,
                    "default_budget": {
                        "max_breaking_changes": 0,
                        "max_non_breaking_changes": 2
                    }
                }
            }
        },
        "default_reality_level": "moderate_realism",
        "personas": [],
        "scenarios": [],
        "recommended_blueprints": []
    })
}

/// Get default security baseline structure
pub fn get_default_security_baseline() -> Value {
    serde_json::json!({
        "default_validation_mode": "warn",
        "rbac_defaults": {
            "admin": ["*"],
            "editor": ["MockCreate", "MockUpdate", "MockRead"],
            "viewer": ["MockRead"]
        },
        "environment_permissions": {
            "prod": {
                "ManageSettings": ["admin", "platform"],
                "ScenarioModifyRealityDefaults": ["platform"],
                "ScenarioModifyChaosRules": ["qa"]
            },
            "test": {
                "ScenarioModifyChaosRules": ["qa", "editor"]
            }
        }
    })
}
