//! GitOps integration for sync operations
//!
//! This module provides functionality to create Git branches and PRs instead of
//! directly updating the database when sync changes are detected.

use crate::{
    database::RecorderDatabase,
    models::RecordedRequest,
    sync::{DetectedChange, GitOpsConfig},
    Result,
};
use mockforge_core::pr_generation::{
    PRFileChange, PRFileChangeType, PRGenerator, PRProvider, PRRequest,
};
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// GitOps sync handler
pub struct GitOpsSyncHandler {
    config: GitOpsConfig,
    pr_generator: Option<PRGenerator>,
    fixtures_dir: PathBuf,
}

impl GitOpsSyncHandler {
    /// Create a new GitOps sync handler
    pub fn new(config: GitOpsConfig, fixtures_dir: PathBuf) -> Result<Self> {
        let pr_generator = if config.enabled && config.token.is_some() {
            let provider = match config.pr_provider.to_lowercase().as_str() {
                "gitlab" => PRProvider::GitLab,
                _ => PRProvider::GitHub,
            };

            let token = config.token.as_ref().ok_or_else(|| {
                crate::RecorderError::InvalidFilter("GitOps token not provided".to_string())
            })?;

            Some(match provider {
                PRProvider::GitHub => PRGenerator::new_github(
                    config.repo_owner.clone(),
                    config.repo_name.clone(),
                    token.clone(),
                    config.base_branch.clone(),
                ),
                PRProvider::GitLab => PRGenerator::new_gitlab(
                    config.repo_owner.clone(),
                    config.repo_name.clone(),
                    token.clone(),
                    config.base_branch.clone(),
                ),
            })
        } else {
            None
        };

        Ok(Self {
            config,
            pr_generator,
            fixtures_dir,
        })
    }

    /// Process sync changes and create a PR if GitOps mode is enabled
    pub async fn process_sync_changes(
        &self,
        database: &RecorderDatabase,
        changes: &[DetectedChange],
        sync_cycle_id: &str,
    ) -> Result<Option<mockforge_core::pr_generation::PRResult>> {
        if !self.config.enabled {
            return Ok(None);
        }

        if changes.is_empty() {
            debug!("No changes detected, skipping GitOps PR creation");
            return Ok(None);
        }

        let pr_generator = self.pr_generator.as_ref().ok_or_else(|| {
            crate::RecorderError::InvalidFilter("PR generator not configured".to_string())
        })?;

        info!("Processing {} changes for GitOps PR creation", changes.len());

        // Collect file changes
        let mut file_changes = Vec::new();

        for change in changes {
            // Get the request to determine fixture path
            if let Ok(Some(request)) = database.get_request(&change.request_id).await {
                if self.config.update_fixtures {
                    if let Some(fixture_change) =
                        self.create_fixture_file_change(database, &request, change).await?
                    {
                        file_changes.push(fixture_change);
                    }
                }
            }
        }

        if file_changes.is_empty() {
            warn!("No file changes to commit, skipping PR creation");
            return Ok(None);
        }

        // Create PR
        let branch = format!(
            "{}/sync-{}",
            self.config.base_branch,
            sync_cycle_id.split('_').next_back().unwrap_or(sync_cycle_id)
        );

        let title =
            format!("Auto-sync: Update fixtures from upstream API changes ({})", sync_cycle_id);

        let body = self.generate_pr_body(changes);

        let pr_request = PRRequest {
            title,
            body,
            branch,
            files: file_changes,
            labels: vec!["automated".to_string(), "contract-update".to_string()],
            reviewers: vec![],
        };

        match pr_generator.create_pr(pr_request).await {
            Ok(result) => {
                info!("Created GitOps PR: {} - {}", result.number, result.url);
                Ok(Some(result))
            }
            Err(e) => {
                warn!("Failed to create GitOps PR: {}", e);
                Err(crate::RecorderError::InvalidFilter(format!("Failed to create PR: {}", e)))
            }
        }
    }

    /// Create a file change for a fixture update
    async fn create_fixture_file_change(
        &self,
        database: &RecorderDatabase,
        request: &RecordedRequest,
        change: &DetectedChange,
    ) -> Result<Option<PRFileChange>> {
        // Determine fixture file path
        let fixture_path = self.get_fixture_path(request);

        // Get the updated response from the database
        let response = database.get_response(&change.request_id).await?.ok_or_else(|| {
            crate::RecorderError::NotFound(format!(
                "Response not found for request {}",
                change.request_id
            ))
        })?;

        // Serialize the updated fixture
        let fixture_content = serde_json::to_string_pretty(&serde_json::json!({
            "id": request.id,
            "method": request.method,
            "path": request.path,
            "headers": request.headers,
            "body": request.body,
            "response": {
                "status_code": response.status_code,
                "headers": response.headers,
                "body": response.body,
                "body_encoding": response.body_encoding,
            },
            "timestamp": request.timestamp,
        }))?;

        // Determine if this is a create or update
        let change_type = if std::path::Path::new(&fixture_path).exists() {
            PRFileChangeType::Update
        } else {
            PRFileChangeType::Create
        };

        Ok(Some(PRFileChange {
            path: fixture_path,
            content: fixture_content,
            change_type,
        }))
    }

    /// Get the fixture file path for a request
    fn get_fixture_path(&self, request: &RecordedRequest) -> String {
        let method = request.method.to_lowercase();
        let path_hash = request.path.replace(['/', ':'], "_");

        // Use a simple hash of the path for the filename
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        request.path.hash(&mut hasher);
        let hash = format!("{:x}", hasher.finish());

        // Return relative path from repo root
        format!("fixtures/http/{}/{}/{}.json", method, path_hash, hash)
    }

    /// Generate PR body with change summary
    fn generate_pr_body(&self, changes: &[DetectedChange]) -> String {
        let mut body = String::from("## Auto-sync: Upstream API Changes\n\n");
        body.push_str("This PR was automatically generated by MockForge sync to update fixtures based on detected upstream API changes.\n\n");

        body.push_str("### Summary\n\n");
        body.push_str(&format!("- **Total changes**: {}\n", changes.len()));
        body.push_str(&format!(
            "- **Endpoints affected**: {}\n",
            self.count_unique_endpoints(changes)
        ));

        body.push_str("\n### Changes\n\n");
        for change in changes {
            body.push_str(&format!(
                "- `{} {}`: {} differences detected\n",
                change.method,
                change.path,
                change.comparison.differences.len()
            ));
        }

        body.push_str("\n### What Changed\n\n");
        body.push_str("- Updated fixture files with new response data\n");
        if self.config.update_docs {
            body.push_str("- Updated OpenAPI specifications\n");
        }
        if self.config.regenerate_sdks {
            body.push_str("- Regenerated SDKs\n");
        }

        body.push_str("\n---\n");
        body.push_str("*This PR was automatically created by MockForge sync. Please review the changes before merging.*\n");

        body
    }

    /// Count unique endpoints in changes
    fn count_unique_endpoints(&self, changes: &[DetectedChange]) -> usize {
        let mut endpoints = std::collections::HashSet::new();
        for change in changes {
            endpoints.insert(format!("{} {}", change.method, change.path));
        }
        endpoints.len()
    }
}
