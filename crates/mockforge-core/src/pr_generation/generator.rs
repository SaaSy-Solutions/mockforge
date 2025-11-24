//! PR generator for creating pull requests
//!
//! This module provides a unified interface for generating PRs across different providers.

use crate::pr_generation::templates::{PRTemplate, PRTemplateContext};
use crate::pr_generation::types::{PRProvider, PRRequest, PRResult};
use crate::pr_generation::{GitHubPRClient, GitLabPRClient};

/// PR generator that works with multiple providers
#[derive(Debug, Clone)]
pub struct PRGenerator {
    provider: PRProvider,
    github_client: Option<GitHubPRClient>,
    gitlab_client: Option<GitLabPRClient>,
}

impl PRGenerator {
    /// Create a new PR generator for GitHub
    pub fn new_github(owner: String, repo: String, token: String, base_branch: String) -> Self {
        Self {
            provider: PRProvider::GitHub,
            github_client: Some(GitHubPRClient::new(owner, repo, token, base_branch)),
            gitlab_client: None,
        }
    }

    /// Create a new PR generator for GitLab
    pub fn new_gitlab(owner: String, repo: String, token: String, base_branch: String) -> Self {
        Self {
            provider: PRProvider::GitLab,
            github_client: None,
            gitlab_client: Some(GitLabPRClient::new(owner, repo, token, base_branch)),
        }
    }

    /// Generate and create a PR from template context
    pub async fn create_pr_from_context(
        &self,
        context: PRTemplateContext,
        files: Vec<crate::pr_generation::types::PRFileChange>,
        labels: Vec<String>,
        reviewers: Vec<String>,
    ) -> crate::Result<PRResult> {
        let title = PRTemplate::generate_title(&context);
        let body = PRTemplate::generate_body(&context);

        let branch = format!(
            "{}-{}-{}",
            "mockforge/contract-update",
            context.method.to_lowercase(),
            &uuid::Uuid::new_v4().to_string()[..8]
        );

        let request = PRRequest {
            title,
            body,
            branch,
            files,
            labels,
            reviewers,
        };

        self.create_pr(request).await
    }

    /// Create a PR
    pub async fn create_pr(&self, request: PRRequest) -> crate::Result<PRResult> {
        match self.provider {
            PRProvider::GitHub => {
                let client = self
                    .github_client
                    .as_ref()
                    .ok_or_else(|| crate::Error::generic("GitHub client not configured"))?;
                client.create_pr(request).await
            }
            PRProvider::GitLab => {
                let client = self
                    .gitlab_client
                    .as_ref()
                    .ok_or_else(|| crate::Error::generic("GitLab client not configured"))?;
                client.create_pr(request).await
            }
        }
    }
}
