//! GitHub PR client
//!
//! This module provides functionality for creating pull requests on GitHub.

use crate::pr_generation::types::{PRFileChange, PRFileChangeType, PRRequest, PRResult};
use crate::Error;
use reqwest::Client;

/// GitHub PR client
#[derive(Debug, Clone)]
pub struct GitHubPRClient {
    owner: String,
    repo: String,
    token: String,
    base_branch: String,
    client: Client,
}

impl GitHubPRClient {
    /// Create a new GitHub PR client
    pub fn new(owner: String, repo: String, token: String, base_branch: String) -> Self {
        Self {
            owner,
            repo,
            token,
            base_branch,
            client: Client::new(),
        }
    }

    /// Create a pull request
    pub async fn create_pr(&self, request: PRRequest) -> crate::Result<PRResult> {
        // Step 1: Get base branch SHA
        let base_sha = self.get_branch_sha(&self.base_branch).await?;

        // Step 2: Create new branch
        self.create_branch(&request.branch, &base_sha).await?;

        // Step 3: Create commits for file changes
        let mut current_sha = base_sha;
        for file_change in &request.files {
            current_sha = match file_change.change_type {
                PRFileChangeType::Create | PRFileChangeType::Update => {
                    self.create_file_commit(&request.branch, file_change, &current_sha).await?
                }
                PRFileChangeType::Delete => {
                    self.delete_file_commit(&request.branch, file_change, &current_sha).await?
                }
            };
        }

        // Step 4: Create pull request
        let pr = self.create_pull_request(&request, &current_sha).await?;

        // Step 5: Add labels if any
        if !request.labels.is_empty() {
            self.add_labels(pr.number, &request.labels).await?;
        }

        // Step 6: Request reviewers if any
        if !request.reviewers.is_empty() {
            self.request_reviewers(pr.number, &request.reviewers).await?;
        }

        Ok(pr)
    }

    async fn get_branch_sha(&self, branch: &str) -> crate::Result<String> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/git/ref/heads/{}",
            self.owner, self.repo, branch
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .map_err(|e| Error::generic(format!("Failed to get branch: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::generic(format!("Failed to get branch: {}", response.status())));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Error::generic(format!("Failed to parse response: {}", e)))?;

        json["object"]["sha"]
            .as_str()
            .ok_or_else(|| Error::generic("Missing SHA in response"))?
            .to_string()
            .pipe(Ok)
    }

    async fn create_branch(&self, branch: &str, sha: &str) -> crate::Result<()> {
        let url = format!("https://api.github.com/repos/{}/{}/git/refs", self.owner, self.repo);

        let body = serde_json::json!({
            "ref": format!("refs/heads/{}", branch),
            "sha": sha
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::generic(format!("Failed to create branch: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::generic(format!(
                "Failed to create branch: {} - {}",
                status, error_text
            )));
        }

        Ok(())
    }

    async fn create_file_commit(
        &self,
        branch: &str,
        file_change: &PRFileChange,
        parent_sha: &str,
    ) -> crate::Result<String> {
        // First, create blob with file content
        let blob_sha = self.create_blob(&file_change.content).await?;

        // Then, create tree with the new file
        let tree_sha = self.create_tree(parent_sha, &file_change.path, &blob_sha, "100644").await?;

        // Finally, create commit
        let commit_sha = self
            .create_commit(parent_sha, &tree_sha, &format!("Update {}", file_change.path))
            .await?;

        // Update branch reference
        self.update_branch_ref(branch, &commit_sha).await?;

        Ok(commit_sha)
    }

    async fn delete_file_commit(
        &self,
        branch: &str,
        file_change: &PRFileChange,
        parent_sha: &str,
    ) -> crate::Result<String> {
        // Create tree without the file
        let tree_sha = self.create_tree_delete(parent_sha, &file_change.path).await?;

        // Create commit
        let commit_sha = self
            .create_commit(parent_sha, &tree_sha, &format!("Delete {}", file_change.path))
            .await?;

        // Update branch reference
        self.update_branch_ref(branch, &commit_sha).await?;

        Ok(commit_sha)
    }

    async fn create_blob(&self, content: &str) -> crate::Result<String> {
        let url = format!("https://api.github.com/repos/{}/{}/git/blobs", self.owner, self.repo);

        let body = serde_json::json!({
            "content": content,
            "encoding": "utf-8"
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::generic(format!("Failed to create blob: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::generic(format!("Failed to create blob: {}", response.status())));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Error::generic(format!("Failed to parse response: {}", e)))?;

        json["sha"]
            .as_str()
            .ok_or_else(|| Error::generic("Missing SHA in response"))?
            .to_string()
            .pipe(Ok)
    }

    async fn create_tree(
        &self,
        base_tree_sha: &str,
        path: &str,
        blob_sha: &str,
        mode: &str,
    ) -> crate::Result<String> {
        let url = format!("https://api.github.com/repos/{}/{}/git/trees", self.owner, self.repo);

        let body = serde_json::json!({
            "base_tree": base_tree_sha,
            "tree": [{
                "path": path,
                "mode": mode,
                "type": "blob",
                "sha": blob_sha
            }]
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::generic(format!("Failed to create tree: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::generic(format!("Failed to create tree: {}", response.status())));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Error::generic(format!("Failed to parse response: {}", e)))?;

        json["sha"]
            .as_str()
            .ok_or_else(|| Error::generic("Missing SHA in response"))?
            .to_string()
            .pipe(Ok)
    }

    async fn create_tree_delete(&self, base_tree_sha: &str, path: &str) -> crate::Result<String> {
        let url = format!("https://api.github.com/repos/{}/{}/git/trees", self.owner, self.repo);

        let body = serde_json::json!({
            "base_tree": base_tree_sha,
            "tree": [{
                "path": path,
                "mode": "100644",
                "type": "blob",
                "sha": null
            }]
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::generic(format!("Failed to create tree: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::generic(format!("Failed to create tree: {}", response.status())));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Error::generic(format!("Failed to parse response: {}", e)))?;

        json["sha"]
            .as_str()
            .ok_or_else(|| Error::generic("Missing SHA in response"))?
            .to_string()
            .pipe(Ok)
    }

    async fn create_commit(
        &self,
        parent_sha: &str,
        tree_sha: &str,
        message: &str,
    ) -> crate::Result<String> {
        let url = format!("https://api.github.com/repos/{}/{}/git/commits", self.owner, self.repo);

        let body = serde_json::json!({
            "message": message,
            "tree": tree_sha,
            "parents": [parent_sha]
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::generic(format!("Failed to create commit: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::generic(format!("Failed to create commit: {}", response.status())));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Error::generic(format!("Failed to parse response: {}", e)))?;

        json["sha"]
            .as_str()
            .ok_or_else(|| Error::generic("Missing SHA in response"))?
            .to_string()
            .pipe(Ok)
    }

    async fn update_branch_ref(&self, branch: &str, sha: &str) -> crate::Result<()> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/git/refs/heads/{}",
            self.owner, self.repo, branch
        );

        let body = serde_json::json!({
            "sha": sha,
            "force": false
        });

        let response = self
            .client
            .patch(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::generic(format!("Failed to update branch: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::generic(format!("Failed to update branch: {}", response.status())));
        }

        Ok(())
    }

    async fn create_pull_request(
        &self,
        request: &PRRequest,
        head_sha: &str,
    ) -> crate::Result<PRResult> {
        let url = format!("https://api.github.com/repos/{}/{}/pulls", self.owner, self.repo);

        let body = serde_json::json!({
            "title": request.title,
            "body": request.body,
            "head": request.branch,
            "base": self.base_branch
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::generic(format!("Failed to create PR: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::generic(format!(
                "Failed to create PR: {} - {}",
                status, error_text
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Error::generic(format!("Failed to parse response: {}", e)))?;

        Ok(PRResult {
            number: json["number"].as_u64().ok_or_else(|| Error::generic("Missing PR number"))?,
            url: json["html_url"]
                .as_str()
                .ok_or_else(|| Error::generic("Missing PR URL"))?
                .to_string(),
            branch: request.branch.clone(),
            title: request.title.clone(),
        })
    }

    async fn add_labels(&self, pr_number: u64, labels: &[String]) -> crate::Result<()> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/issues/{}/labels",
            self.owner, self.repo, pr_number
        );

        let body = serde_json::json!({
            "labels": labels
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::generic(format!("Failed to add labels: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::generic(format!("Failed to add labels: {}", response.status())));
        }

        Ok(())
    }

    async fn request_reviewers(&self, pr_number: u64, reviewers: &[String]) -> crate::Result<()> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls/{}/requested_reviewers",
            self.owner, self.repo, pr_number
        );

        let body = serde_json::json!({
            "reviewers": reviewers
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::generic(format!("Failed to request reviewers: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::generic(format!(
                "Failed to request reviewers: {}",
                response.status()
            )));
        }

        Ok(())
    }
}

// Helper trait for pipe operator
trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
    {
        f(self)
    }
}

impl<T> Pipe for T {}
