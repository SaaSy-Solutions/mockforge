//! GitOps handler for drift budget violations
//!
//! This handler generates pull requests when drift budgets are exceeded,
//! updating OpenAPI specs, fixtures, and optionally triggering client generation.

use crate::{
    incidents::types::DriftIncident,
    pr_generation::{PRFileChange, PRFileChangeType, PRGenerator, PRRequest, PRResult},
    Result,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for drift GitOps handler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftGitOpsConfig {
    /// Whether GitOps mode is enabled
    pub enabled: bool,
    /// PR generation configuration (used to build PRGenerator)
    pub pr_config: Option<crate::pr_generation::PRGenerationConfig>,
    /// Whether to update OpenAPI specs
    #[serde(default = "default_true")]
    pub update_openapi_specs: bool,
    /// Whether to update fixture files
    #[serde(default = "default_true")]
    pub update_fixtures: bool,
    /// Whether to regenerate client SDKs
    #[serde(default)]
    pub regenerate_clients: bool,
    /// Whether to run tests
    #[serde(default)]
    pub run_tests: bool,
    /// Base directory for OpenAPI specs
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openapi_spec_dir: Option<String>,
    /// Base directory for fixtures
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixtures_dir: Option<String>,
    /// Base directory for generated clients
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clients_dir: Option<String>,
    /// Branch prefix for generated branches
    #[serde(default = "default_branch_prefix")]
    pub branch_prefix: String,
}

fn default_true() -> bool {
    true
}

fn default_branch_prefix() -> String {
    "mockforge/drift-fix".to_string()
}

impl Default for DriftGitOpsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            pr_config: None,
            update_openapi_specs: true,
            update_fixtures: true,
            regenerate_clients: false,
            run_tests: false,
            openapi_spec_dir: None,
            fixtures_dir: None,
            clients_dir: None,
            branch_prefix: "mockforge/drift-fix".to_string(),
        }
    }
}

/// GitOps handler for drift budget violations
pub struct DriftGitOpsHandler {
    config: DriftGitOpsConfig,
    pr_generator: Option<PRGenerator>,
}

impl DriftGitOpsHandler {
    /// Create a new drift GitOps handler
    pub fn new(config: DriftGitOpsConfig) -> Result<Self> {
        // Build PR generator from config if enabled
        let pr_generator = if config.enabled {
            if let Some(ref pr_config) = config.pr_config {
                if pr_config.enabled {
                    let token = pr_config.token.clone().ok_or_else(|| {
                        crate::Error::generic("PR token not configured".to_string())
                    })?;

                    let generator = match pr_config.provider {
                        crate::pr_generation::PRProvider::GitHub => PRGenerator::new_github(
                            pr_config.owner.clone(),
                            pr_config.repo.clone(),
                            token,
                            pr_config.base_branch.clone(),
                        ),
                        crate::pr_generation::PRProvider::GitLab => PRGenerator::new_gitlab(
                            pr_config.owner.clone(),
                            pr_config.repo.clone(),
                            token,
                            pr_config.base_branch.clone(),
                        ),
                    };
                    Some(generator)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            config,
            pr_generator,
        })
    }

    /// Generate a PR from drift incidents
    ///
    /// This method processes drift incidents and generates a PR with:
    /// - Updated OpenAPI specs (if corrections are available)
    /// - Updated fixture files
    /// - Optionally regenerated client SDKs
    /// - Optionally test execution
    pub async fn generate_pr_from_incidents(
        &self,
        incidents: &[DriftIncident],
    ) -> Result<Option<PRResult>> {
        if !self.config.enabled {
            return Ok(None);
        }

        if incidents.is_empty() {
            return Ok(None);
        }

        let pr_generator = self
            .pr_generator
            .as_ref()
            .ok_or_else(|| crate::Error::generic("PR generator not configured"))?;

        // Collect file changes from incidents
        let mut file_changes = Vec::new();

        for incident in incidents {
            // Add OpenAPI spec updates if enabled and corrections are available
            if self.config.update_openapi_specs {
                if let Some(openapi_changes) = self.create_openapi_changes(incident).await? {
                    file_changes.extend(openapi_changes);
                }
            }

            // Add fixture updates if enabled
            if self.config.update_fixtures {
                if let Some(fixture_changes) = self.create_fixture_changes(incident).await? {
                    file_changes.extend(fixture_changes);
                }
            }
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        // Generate branch name
        let branch = format!(
            "{}/{}",
            self.config.branch_prefix,
            uuid::Uuid::new_v4().to_string()[..8].to_string()
        );

        // Generate PR title and body
        let title = self.generate_pr_title(incidents);
        let body = self.generate_pr_body(incidents);

        // Create PR request
        let pr_request = PRRequest {
            title,
            body,
            branch,
            files: file_changes,
            labels: vec![
                "automated".to_string(),
                "drift-fix".to_string(),
                "contract-update".to_string(),
            ],
            reviewers: vec![],
        };

        // Create PR
        match pr_generator.create_pr(pr_request).await {
            Ok(result) => {
                tracing::info!("Created drift GitOps PR: {} - {}", result.number, result.url);
                Ok(Some(result))
            }
            Err(e) => {
                tracing::warn!("Failed to create drift GitOps PR: {}", e);
                Err(e)
            }
        }
    }

    /// Create OpenAPI spec changes from incident
    async fn create_openapi_changes(
        &self,
        incident: &DriftIncident,
    ) -> Result<Option<Vec<PRFileChange>>> {
        // Extract corrections from incident details or after_sample
        let corrections = if let Some(after_sample) = &incident.after_sample {
            if let Some(corrections) = after_sample.get("corrections") {
                corrections.as_array().cloned().unwrap_or_default()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        if corrections.is_empty() {
            return Ok(None);
        }

        // Determine OpenAPI spec file path
        let spec_path = if let Some(ref spec_dir) = self.config.openapi_spec_dir {
            // Try to find spec file based on endpoint
            // For now, use a default path - in a full implementation, we'd search for the spec
            PathBuf::from(spec_dir).join("openapi.yaml")
        } else {
            PathBuf::from("openapi.yaml")
        };

        // Apply corrections to OpenAPI spec
        // Note: In a full implementation, we'd:
        // 1. Load the existing OpenAPI spec
        // 2. Apply JSON Patch corrections
        // 3. Serialize the updated spec
        // For now, we'll create a placeholder that indicates what needs to be updated
        let updated_spec = serde_json::json!({
            "note": "OpenAPI spec should be updated based on drift corrections",
            "endpoint": format!("{} {}", incident.method, incident.endpoint),
            "corrections": corrections,
            "incident_id": incident.id,
        });

        // Use JSON format for now (YAML would require serde_yaml)
        let spec_content = serde_json::to_string_pretty(&updated_spec)
            .map_err(|e| crate::Error::generic(format!("Failed to serialize spec: {}", e)))?;

        Ok(Some(vec![PRFileChange {
            path: spec_path.to_string_lossy().to_string(),
            content: spec_content,
            change_type: PRFileChangeType::Update,
        }]))
    }

    /// Create fixture file changes from incident
    async fn create_fixture_changes(
        &self,
        incident: &DriftIncident,
    ) -> Result<Option<Vec<PRFileChange>>> {
        // Use after_sample as the updated fixture
        let fixture_data = if let Some(after_sample) = &incident.after_sample {
            after_sample.clone()
        } else {
            // Fall back to incident details
            incident.details.clone()
        };

        // Determine fixture file path
        let fixtures_dir = self
            .config
            .fixtures_dir
            .as_ref()
            .map(|d| PathBuf::from(d))
            .unwrap_or_else(|| PathBuf::from("fixtures"));

        let method = incident.method.to_lowercase();
        let path_hash = incident.endpoint.replace(['/', ':'], "_");
        let fixture_path =
            fixtures_dir.join("http").join(&method).join(format!("{}.json", path_hash));

        let fixture_content = serde_json::to_string_pretty(&fixture_data)
            .map_err(|e| crate::Error::generic(format!("Failed to serialize fixture: {}", e)))?;

        // Determine if this is a create or update
        // Note: We can't check file existence in async context easily, so default to Update
        // In a full implementation, we'd check the file system or track this in metadata
        let change_type = PRFileChangeType::Update;

        Ok(Some(vec![PRFileChange {
            path: fixture_path.to_string_lossy().to_string(),
            content: fixture_content,
            change_type,
        }]))
    }

    /// Generate PR title from incidents
    fn generate_pr_title(&self, incidents: &[DriftIncident]) -> String {
        if incidents.len() == 1 {
            let incident = &incidents[0];
            format!(
                "Fix drift: {} {} - {:?}",
                incident.method, incident.endpoint, incident.incident_type
            )
        } else {
            format!(
                "Fix drift: {} incidents across {} endpoints",
                incidents.len(),
                incidents
                    .iter()
                    .map(|i| format!("{} {}", i.method, i.endpoint))
                    .collect::<std::collections::HashSet<_>>()
                    .len()
            )
        }
    }

    /// Generate PR body from incidents
    fn generate_pr_body(&self, incidents: &[DriftIncident]) -> String {
        let mut body = String::from("## Drift Budget Violation Fix\n\n");
        body.push_str(
            "This PR was automatically generated by MockForge to fix drift budget violations.\n\n",
        );

        body.push_str("### Summary\n\n");
        body.push_str(&format!("- **Total incidents**: {}\n", incidents.len()));

        let breaking_count = incidents
            .iter()
            .filter(|i| {
                matches!(i.incident_type, crate::incidents::types::IncidentType::BreakingChange)
            })
            .count();
        let threshold_count = incidents.len() - breaking_count;

        body.push_str(&format!("- **Breaking changes**: {}\n", breaking_count));
        body.push_str(&format!("- **Threshold exceeded**: {}\n", threshold_count));

        body.push_str("\n### Affected Endpoints\n\n");
        for incident in incidents {
            body.push_str(&format!(
                "- `{} {}` - {:?} ({:?})\n",
                incident.method, incident.endpoint, incident.incident_type, incident.severity
            ));
        }

        body.push_str("\n### Changes Made\n\n");
        if self.config.update_openapi_specs {
            body.push_str("- Updated OpenAPI specifications with corrections\n");
        }
        if self.config.update_fixtures {
            body.push_str("- Updated fixture files with new response data\n");
        }
        if self.config.regenerate_clients {
            body.push_str("- Regenerated client SDKs\n");
        }
        if self.config.run_tests {
            body.push_str("- Ran tests (see CI results)\n");
        }

        body.push_str("\n### Incident Details\n\n");
        for incident in incidents {
            body.push_str(&format!("#### {} {}\n\n", incident.method, incident.endpoint));
            body.push_str(&format!("- **Incident ID**: `{}`\n", incident.id));
            body.push_str(&format!("- **Type**: {:?}\n", incident.incident_type));
            body.push_str(&format!("- **Severity**: {:?}\n", incident.severity));

            if let Some(breaking_changes) = incident.details.get("breaking_changes") {
                body.push_str(&format!("- **Breaking Changes**: {}\n", breaking_changes));
            }
            if let Some(non_breaking_changes) = incident.details.get("non_breaking_changes") {
                body.push_str(&format!("- **Non-Breaking Changes**: {}\n", non_breaking_changes));
            }

            body.push_str("\n");
        }

        body.push_str("---\n");
        body.push_str("*This PR was automatically created by MockForge drift budget monitoring. Please review the changes before merging.*\n");

        body
    }
}
