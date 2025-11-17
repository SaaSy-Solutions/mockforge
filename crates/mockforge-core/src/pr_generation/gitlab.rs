//! GitLab PR (Merge Request) client
//!
//! This module provides functionality for creating merge requests on GitLab.

use crate::pr_generation::types::{PRFileChange, PRFileChangeType, PRRequest, PRResult};
use crate::Error;
use reqwest::Client;

/// GitLab PR client
#[derive(Debug, Clone)]
pub struct GitLabPRClient {
    owner: String,
    repo: String,
    token: String,
    base_branch: String,
    client: Client,
    api_url: String,
}

impl GitLabPRClient {
    /// Create a new GitLab PR client
    pub fn new(owner: String, repo: String, token: String, base_branch: String) -> Self {
        Self {
            owner,
            repo,
            token,
            base_branch,
            client: Client::new(),
            api_url: "https://gitlab.com/api/v4".to_string(),
        }
    }

    /// Create a merge request (PR)
    pub async fn create_pr(&self, request: PRRequest) -> crate::Result<PRResult> {
        // GitLab API uses project ID or path
        let project_path = format!("{}/{}", self.owner, self.repo);

        // Step 1: Create branch
        self.create_branch(&request.branch).await?;

        // Step 2: Commit file changes
        for file_change in &request.files {
            match file_change.change_type {
                PRFileChangeType::Create | PRFileChangeType::Update => {
                    self.commit_file(&request.branch, file_change).await?;
                }
                PRFileChangeType::Delete => {
                    self.delete_file(&request.branch, file_change).await?;
                }
            }
        }

        // Step 3: Create merge request
        let mr = self.create_merge_request(&request, &project_path).await?;

        // Step 4: Add labels if any
        if !request.labels.is_empty() {
            self.add_labels(mr.number, &request.labels, &project_path).await?;
        }

        Ok(mr)
    }

    async fn create_branch(&self, branch: &str) -> crate::Result<()> {
        let project_path = format!("{}/{}", self.owner, self.repo);
        let url = format!(
            "{}/projects/{}/repository/branches",
            self.api_url,
            urlencoding::encode(&project_path)
        );

        let body = serde_json::json!({
            "branch": branch,
            "ref": self.base_branch
        });

        let response = self
            .client
            .post(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::generic(format!("Failed to create branch: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            // Branch might already exist, which is okay
            if !error_text.contains("already exists") {
                return Err(Error::generic(format!(
                    "Failed to create branch: {} - {}",
                    status, error_text
                )));
            }
        }

        Ok(())
    }

    async fn commit_file(&self, branch: &str, file_change: &PRFileChange) -> crate::Result<()> {
        let project_path = format!("{}/{}", self.owner, self.repo);
        let url = format!(
            "{}/projects/{}/repository/files/{}",
            self.api_url,
            urlencoding::encode(&project_path),
            urlencoding::encode(&file_change.path)
        );

        let content = base64::encode(&file_change.content);

        let body = serde_json::json!({
            "branch": branch,
            "content": content,
            "encoding": "base64",
            "commit_message": format!("Update {}", file_change.path)
        });

        let response = self
            .client
            .put(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::generic(format!("Failed to commit file: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            // Try creating the file if it doesn't exist
            if status == 404 {
                return self.create_file(branch, file_change).await;
            }

            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::generic(format!(
                "Failed to commit file: {} - {}",
                status, error_text
            )));
        }

        Ok(())
    }

    async fn create_file(&self, branch: &str, file_change: &PRFileChange) -> crate::Result<()> {
        let project_path = format!("{}/{}", self.owner, self.repo);
        let url = format!(
            "{}/projects/{}/repository/files/{}",
            self.api_url,
            urlencoding::encode(&project_path),
            urlencoding::encode(&file_change.path)
        );

        let content = base64::encode(&file_change.content);

        let body = serde_json::json!({
            "branch": branch,
            "content": content,
            "encoding": "base64",
            "commit_message": format!("Create {}", file_change.path)
        });

        let response = self
            .client
            .post(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::generic(format!("Failed to create file: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::generic(format!(
                "Failed to create file: {} - {}",
                status, error_text
            )));
        }

        Ok(())
    }

    async fn delete_file(&self, branch: &str, file_change: &PRFileChange) -> crate::Result<()> {
        let project_path = format!("{}/{}", self.owner, self.repo);
        let url = format!(
            "{}/projects/{}/repository/files/{}",
            self.api_url,
            urlencoding::encode(&project_path),
            urlencoding::encode(&file_change.path)
        );

        let body = serde_json::json!({
            "branch": branch,
            "commit_message": format!("Delete {}", file_change.path)
        });

        let response = self
            .client
            .delete(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::generic(format!("Failed to delete file: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::generic(format!(
                "Failed to delete file: {} - {}",
                status, error_text
            )));
        }

        Ok(())
    }

    async fn create_merge_request(
        &self,
        request: &PRRequest,
        project_path: &str,
    ) -> crate::Result<PRResult> {
        let url = format!(
            "{}/projects/{}/merge_requests",
            self.api_url,
            urlencoding::encode(project_path)
        );

        let body = serde_json::json!({
            "source_branch": request.branch,
            "target_branch": self.base_branch,
            "title": request.title,
            "description": request.body
        });

        let response = self
            .client
            .post(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::generic(format!("Failed to create MR: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::generic(format!(
                "Failed to create MR: {} - {}",
                status, error_text
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Error::generic(format!("Failed to parse response: {}", e)))?;

        Ok(PRResult {
            number: json["iid"].as_u64().ok_or_else(|| Error::generic("Missing MR number"))?,
            url: json["web_url"]
                .as_str()
                .ok_or_else(|| Error::generic("Missing MR URL"))?
                .to_string(),
            branch: request.branch.clone(),
            title: request.title.clone(),
        })
    }

    async fn add_labels(
        &self,
        mr_number: u64,
        labels: &[String],
        project_path: &str,
    ) -> crate::Result<()> {
        let url = format!(
            "{}/projects/{}/merge_requests/{}",
            self.api_url,
            urlencoding::encode(project_path),
            mr_number
        );

        let body = serde_json::json!({
            "add_labels": labels.join(",")
        });

        let response = self
            .client
            .put(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::generic(format!("Failed to add labels: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(Error::generic(format!("Failed to add labels: {}", status)));
        }

        Ok(())
    }
}
