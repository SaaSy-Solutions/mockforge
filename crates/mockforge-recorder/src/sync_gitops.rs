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
        let pr_generator = if config.enabled {
            let token = config.token.as_ref().ok_or_else(|| {
                crate::RecorderError::InvalidFilter("GitOps token not provided".to_string())
            })?;

            let provider = match config.pr_provider.to_lowercase().as_str() {
                "gitlab" => PRProvider::GitLab,
                _ => PRProvider::GitHub,
            };

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{ComparisonResult, Difference, DifferenceType};
    use tempfile::TempDir;

    fn create_test_gitops_config(enabled: bool) -> GitOpsConfig {
        GitOpsConfig {
            enabled,
            pr_provider: "github".to_string(),
            repo_owner: "test-owner".to_string(),
            repo_name: "test-repo".to_string(),
            base_branch: "main".to_string(),
            update_fixtures: true,
            regenerate_sdks: false,
            update_docs: true,
            auto_merge: false,
            token: Some("test-token".to_string()),
        }
    }

    fn create_test_summary(total_differences: usize) -> crate::diff::ComparisonSummary {
        crate::diff::ComparisonSummary {
            total_differences,
            added_fields: 0,
            removed_fields: 0,
            changed_fields: total_differences,
            type_changes: 0,
        }
    }

    fn create_test_change(request_id: &str, path: &str, method: &str) -> DetectedChange {
        let differences = vec![Difference::new(
            "$.status".to_string(),
            DifferenceType::Changed {
                path: "$.status".to_string(),
                original: "200".to_string(),
                current: "201".to_string(),
            },
        )];
        DetectedChange {
            request_id: request_id.to_string(),
            path: path.to_string(),
            method: method.to_string(),
            comparison: ComparisonResult {
                matches: false,
                status_match: false,
                headers_match: true,
                body_match: true,
                differences: differences.clone(),
                summary: create_test_summary(differences.len()),
            },
            updated: false,
        }
    }

    #[tokio::test]
    async fn test_gitops_handler_creation_enabled() {
        let config = create_test_gitops_config(true);
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        let result = GitOpsSyncHandler::new(config, fixtures_dir);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_gitops_handler_creation_disabled() {
        let mut config = create_test_gitops_config(false);
        config.token = None;
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        let result = GitOpsSyncHandler::new(config, fixtures_dir);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_gitops_handler_creation_no_token() {
        let mut config = create_test_gitops_config(true);
        config.token = None;
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        let result = GitOpsSyncHandler::new(config, fixtures_dir);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_gitops_handler_creation_gitlab_provider() {
        let mut config = create_test_gitops_config(true);
        config.pr_provider = "gitlab".to_string();
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        let result = GitOpsSyncHandler::new(config, fixtures_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_fixture_path() {
        let config = create_test_gitops_config(true);
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        let handler = GitOpsSyncHandler::new(config, fixtures_dir).unwrap();

        let request = RecordedRequest {
            id: "test-123".to_string(),
            protocol: crate::models::Protocol::Http,
            timestamp: chrono::Utc::now(),
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            query_params: None,
            headers: "{}".to_string(),
            body: None,
            body_encoding: "utf8".to_string(),
            client_ip: None,
            trace_id: None,
            span_id: None,
            duration_ms: None,
            status_code: Some(200),
            tags: None,
        };

        let fixture_path = handler.get_fixture_path(&request);

        assert!(fixture_path.contains("fixtures/http/get"));
        assert!(fixture_path.ends_with(".json"));
    }

    #[test]
    fn test_generate_pr_body_single_change() {
        let config = create_test_gitops_config(true);
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        let handler = GitOpsSyncHandler::new(config, fixtures_dir).unwrap();

        let changes = vec![create_test_change("req-1", "/api/users", "GET")];
        let body = handler.generate_pr_body(&changes);

        assert!(body.contains("**Total changes**: 1"));
        assert!(body.contains("GET /api/users"));
        assert!(body.contains("1 differences detected"));
        assert!(body.contains("Updated fixture files"));
    }

    #[test]
    fn test_generate_pr_body_multiple_changes() {
        let config = create_test_gitops_config(true);
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        let handler = GitOpsSyncHandler::new(config, fixtures_dir).unwrap();

        let changes = vec![
            create_test_change("req-1", "/api/users", "GET"),
            create_test_change("req-2", "/api/posts", "POST"),
            create_test_change("req-3", "/api/users", "DELETE"),
        ];

        let body = handler.generate_pr_body(&changes);

        assert!(body.contains("**Total changes**: 3"));
        assert!(body.contains("GET /api/users"));
        assert!(body.contains("POST /api/posts"));
        assert!(body.contains("DELETE /api/users"));
    }

    #[test]
    fn test_generate_pr_body_with_docs_enabled() {
        let mut config = create_test_gitops_config(true);
        config.update_docs = true;
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        let handler = GitOpsSyncHandler::new(config, fixtures_dir).unwrap();

        let changes = vec![create_test_change("req-1", "/api/users", "GET")];
        let body = handler.generate_pr_body(&changes);

        assert!(body.contains("Updated OpenAPI specifications"));
    }

    #[test]
    fn test_generate_pr_body_with_sdks_enabled() {
        let mut config = create_test_gitops_config(true);
        config.regenerate_sdks = true;
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        let handler = GitOpsSyncHandler::new(config, fixtures_dir).unwrap();

        let changes = vec![create_test_change("req-1", "/api/users", "GET")];
        let body = handler.generate_pr_body(&changes);

        assert!(body.contains("Regenerated SDKs"));
    }

    #[test]
    fn test_count_unique_endpoints_single() {
        let config = create_test_gitops_config(true);
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        let handler = GitOpsSyncHandler::new(config, fixtures_dir).unwrap();

        let changes = vec![
            create_test_change("req-1", "/api/users", "GET"),
            create_test_change("req-2", "/api/users", "GET"),
        ];

        let count = handler.count_unique_endpoints(&changes);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_count_unique_endpoints_multiple() {
        let config = create_test_gitops_config(true);
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        let handler = GitOpsSyncHandler::new(config, fixtures_dir).unwrap();

        let changes = vec![
            create_test_change("req-1", "/api/users", "GET"),
            create_test_change("req-2", "/api/posts", "GET"),
            create_test_change("req-3", "/api/users", "POST"),
        ];

        let count = handler.count_unique_endpoints(&changes);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_count_unique_endpoints_empty() {
        let config = create_test_gitops_config(true);
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        let handler = GitOpsSyncHandler::new(config, fixtures_dir).unwrap();

        let changes = vec![];
        let count = handler.count_unique_endpoints(&changes);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_gitops_config_serialization() {
        let config = create_test_gitops_config(true);
        let json = serde_json::to_string(&config).unwrap();

        assert!(json.contains("github"));
        assert!(json.contains("test-owner"));
        assert!(json.contains("test-repo"));
        assert!(!json.contains("test-token")); // Token should be skipped
    }

    #[test]
    fn test_gitops_config_deserialization() {
        let json = r#"{
            "enabled": true,
            "pr_provider": "github",
            "repo_owner": "owner",
            "repo_name": "repo",
            "base_branch": "develop",
            "update_fixtures": true,
            "regenerate_sdks": false,
            "update_docs": true,
            "auto_merge": false
        }"#;

        let config: GitOpsConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert_eq!(config.pr_provider, "github");
        assert_eq!(config.repo_owner, "owner");
        assert_eq!(config.base_branch, "develop");
    }

    #[test]
    fn test_gitops_config_defaults() {
        let json = r#"{
            "enabled": true,
            "pr_provider": "github",
            "repo_owner": "owner",
            "repo_name": "repo"
        }"#;

        let config: GitOpsConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.base_branch, "main");
        assert!(config.update_fixtures);
        assert!(config.update_docs);
        assert!(!config.regenerate_sdks);
        assert!(!config.auto_merge);
    }
}
