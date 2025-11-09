//! Scenario registry client
//!
//! Client for discovering and downloading scenarios from the registry.
//! Reuses plugin registry infrastructure where possible.

use crate::error::{Result, ScenarioError};
use serde::{Deserialize, Serialize};

/// Scenario registry entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioRegistryEntry {
    /// Scenario name
    pub name: String,

    /// Scenario description
    pub description: String,

    /// Current version
    pub version: String,

    /// All available versions
    pub versions: Vec<ScenarioVersionEntry>,

    /// Author information
    pub author: String,

    /// Author email
    pub author_email: Option<String>,

    /// Scenario tags
    pub tags: Vec<String>,

    /// Scenario category
    pub category: String,

    /// Download count
    pub downloads: u64,

    /// Rating (0.0 - 5.0)
    pub rating: f64,

    /// Total reviews
    pub reviews_count: u32,

    /// List of reviews (optional, may not be included in all API responses)
    #[serde(default)]
    pub reviews: Vec<ScenarioReview>,

    /// Repository URL
    pub repository: Option<String>,

    /// Homepage URL
    pub homepage: Option<String>,

    /// License
    pub license: String,

    /// Created timestamp
    pub created_at: String,

    /// Updated timestamp
    pub updated_at: String,
}

/// Scenario review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioReview {
    /// Review ID
    pub id: String,

    /// Reviewer name/username
    pub reviewer: String,

    /// Reviewer email (optional, may be hidden)
    #[serde(default)]
    pub reviewer_email: Option<String>,

    /// Rating (1-5)
    pub rating: u8,

    /// Review title
    #[serde(default)]
    pub title: Option<String>,

    /// Review text/comment
    pub comment: String,

    /// Review timestamp
    pub created_at: String,

    /// Whether this review was helpful (for other users)
    #[serde(default)]
    pub helpful_count: u32,

    /// Whether the reviewer verified they used the scenario
    #[serde(default)]
    pub verified_purchase: bool,
}

/// Review submission request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioReviewSubmission {
    /// Scenario name
    pub scenario_name: String,

    /// Scenario version (optional)
    pub scenario_version: Option<String>,

    /// Rating (1-5)
    pub rating: u8,

    /// Review title (optional)
    pub title: Option<String>,

    /// Review text/comment
    pub comment: String,

    /// Reviewer name/username
    pub reviewer: String,

    /// Reviewer email (optional)
    pub reviewer_email: Option<String>,

    /// Whether the reviewer verified they used the scenario
    #[serde(default)]
    pub verified_purchase: bool,
}

/// Version-specific entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioVersionEntry {
    /// Version string (semver)
    pub version: String,

    /// Download URL
    pub download_url: String,

    /// SHA-256 checksum
    pub checksum: String,

    /// File size in bytes
    pub size: u64,

    /// Published timestamp
    pub published_at: String,

    /// Yanked (removed from index)
    pub yanked: bool,

    /// Minimum MockForge version required
    pub min_mockforge_version: Option<String>,
}

/// Search query for scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSearchQuery {
    /// Search terms
    pub query: Option<String>,

    /// Filter by category
    pub category: Option<String>,

    /// Filter by tags
    pub tags: Vec<String>,

    /// Sort order
    pub sort: ScenarioSortOrder,

    /// Page number (0-indexed)
    pub page: usize,

    /// Results per page
    pub per_page: usize,
}

impl Default for ScenarioSearchQuery {
    fn default() -> Self {
        Self {
            query: None,
            category: None,
            tags: vec![],
            sort: ScenarioSortOrder::Relevance,
            page: 0,
            per_page: 20,
        }
    }
}

/// Sort order for search results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScenarioSortOrder {
    Relevance,
    Downloads,
    Rating,
    Recent,
    Name,
}

/// Search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSearchResults {
    pub scenarios: Vec<ScenarioRegistryEntry>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

/// Registry client for scenarios
pub struct ScenarioRegistry {
    /// Registry base URL
    base_url: String,

    /// HTTP client
    client: reqwest::Client,

    /// API token (optional)
    token: Option<String>,
}

impl ScenarioRegistry {
    /// Create a new registry client
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
            token: None,
        }
    }

    /// Create a new registry client with authentication
    pub fn with_token(base_url: String, token: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
            token: Some(token),
        }
    }

    /// Search for scenarios
    pub async fn search(&self, query: ScenarioSearchQuery) -> Result<ScenarioSearchResults> {
        let url = format!("{}/api/v1/scenarios/search", self.base_url);

        let mut request = self.client.post(&url);
        if let Some(ref token) = self.token {
            request = request.bearer_auth(token);
        }

        let response = request
            .json(&query)
            .send()
            .await
            .map_err(|e| ScenarioError::Network(format!("Search request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(ScenarioError::Network(format!(
                "Search failed with status: {}",
                response.status()
            )));
        }

        let results: ScenarioSearchResults = response.json().await.map_err(|e| {
            ScenarioError::Network(format!("Failed to parse search results: {}", e))
        })?;

        Ok(results)
    }

    /// Get scenario by name
    pub async fn get_scenario(&self, name: &str) -> Result<ScenarioRegistryEntry> {
        let url = format!("{}/api/v1/scenarios/{}", self.base_url, name);

        let mut request = self.client.get(&url);
        if let Some(ref token) = self.token {
            request = request.bearer_auth(token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| ScenarioError::Network(format!("Get scenario request failed: {}", e)))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(ScenarioError::NotFound(format!("Scenario '{}' not found", name)));
        }

        if !response.status().is_success() {
            return Err(ScenarioError::Network(format!(
                "Get scenario failed with status: {}",
                response.status()
            )));
        }

        let entry: ScenarioRegistryEntry = response.json().await.map_err(|e| {
            ScenarioError::Network(format!("Failed to parse scenario entry: {}", e))
        })?;

        Ok(entry)
    }

    /// Get scenario version
    pub async fn get_version(&self, name: &str, version: &str) -> Result<ScenarioVersionEntry> {
        let url = format!("{}/api/v1/scenarios/{}/versions/{}", self.base_url, name, version);

        let mut request = self.client.get(&url);
        if let Some(ref token) = self.token {
            request = request.bearer_auth(token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| ScenarioError::Network(format!("Get version request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(ScenarioError::InvalidVersion(format!(
                "Version {}@{} not found or invalid",
                name, version
            )));
        }

        let entry: ScenarioVersionEntry = response
            .json()
            .await
            .map_err(|e| ScenarioError::Network(format!("Failed to parse version entry: {}", e)))?;

        Ok(entry)
    }

    /// Download scenario package
    pub async fn download(
        &self,
        download_url: &str,
        expected_checksum: Option<&str>,
    ) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(download_url)
            .send()
            .await
            .map_err(|e| ScenarioError::Network(format!("Download failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(ScenarioError::Network(format!(
                "Download failed with status: {}",
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| ScenarioError::Network(format!("Failed to read download: {}", e)))?;

        let data = bytes.to_vec();

        // Verify checksum if provided
        if let Some(checksum) = expected_checksum {
            use ring::digest::{Context, SHA256};
            let mut context = Context::new(&SHA256);
            context.update(&data);
            let digest = context.finish();
            let calculated = hex::encode(digest.as_ref());

            if calculated != checksum {
                return Err(ScenarioError::ChecksumMismatch {
                    expected: checksum.to_string(),
                    actual: calculated,
                });
            }
        }

        Ok(data)
    }

    /// Publish a scenario to the registry
    pub async fn publish(
        &self,
        publish_request: ScenarioPublishRequest,
    ) -> Result<ScenarioPublishResponse> {
        let token = self.token.as_ref().ok_or_else(|| ScenarioError::AuthRequired)?;

        let url = format!("{}/api/v1/scenarios/publish", self.base_url);

        let response = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&publish_request)
            .send()
            .await
            .map_err(|e| ScenarioError::Network(format!("Publish request failed: {}", e)))?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ScenarioError::AuthRequired);
        }

        if response.status() == reqwest::StatusCode::FORBIDDEN {
            return Err(ScenarioError::PermissionDenied);
        }

        if !response.status().is_success() {
            return Err(ScenarioError::Network(format!(
                "Publish failed with status: {}",
                response.status()
            )));
        }

        let result: ScenarioPublishResponse = response.json().await.map_err(|e| {
            ScenarioError::Network(format!("Failed to parse publish response: {}", e))
        })?;

        Ok(result)
    }

    /// Submit a review for a scenario
    pub async fn submit_review(&self, review: ScenarioReviewSubmission) -> Result<ScenarioReview> {
        let url = format!("{}/api/v1/scenarios/{}/reviews", self.base_url, review.scenario_name);

        let mut request = self.client.post(&url);
        if let Some(ref token) = self.token {
            request = request.bearer_auth(token);
        }

        let response =
            request.json(&review).send().await.map_err(|e| {
                ScenarioError::Network(format!("Submit review request failed: {}", e))
            })?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ScenarioError::AuthRequired);
        }

        if !response.status().is_success() {
            return Err(ScenarioError::Network(format!(
                "Submit review failed with status: {}",
                response.status()
            )));
        }

        let submitted_review: ScenarioReview = response.json().await.map_err(|e| {
            ScenarioError::Network(format!("Failed to parse review response: {}", e))
        })?;

        Ok(submitted_review)
    }

    /// Get reviews for a scenario
    pub async fn get_reviews(
        &self,
        name: &str,
        page: Option<usize>,
        per_page: Option<usize>,
    ) -> Result<Vec<ScenarioReview>> {
        let page = page.unwrap_or(0);
        let per_page = per_page.unwrap_or(20);
        let url = format!(
            "{}/api/v1/scenarios/{}/reviews?page={}&per_page={}",
            self.base_url, name, page, per_page
        );

        let mut request = self.client.get(&url);
        if let Some(ref token) = self.token {
            request = request.bearer_auth(token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| ScenarioError::Network(format!("Get reviews request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(ScenarioError::Network(format!(
                "Get reviews failed with status: {}",
                response.status()
            )));
        }

        let reviews: Vec<ScenarioReview> = response
            .json()
            .await
            .map_err(|e| ScenarioError::Network(format!("Failed to parse reviews: {}", e)))?;

        Ok(reviews)
    }
}

/// Publish request for scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioPublishRequest {
    /// Scenario manifest
    pub manifest: String, // JSON string of ScenarioManifest

    /// Package archive (base64 encoded)
    pub package: String,

    /// Package checksum (SHA-256)
    pub checksum: String,

    /// Package size in bytes
    pub size: u64,
}

/// Publish response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioPublishResponse {
    /// Published scenario name
    pub name: String,

    /// Published version
    pub version: String,

    /// Download URL
    pub download_url: String,

    /// Published timestamp
    pub published_at: String,
}

/// Registry client (type alias for convenience)
pub type RegistryClient = ScenarioRegistry;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_default() {
        let query = ScenarioSearchQuery::default();
        assert_eq!(query.page, 0);
        assert_eq!(query.per_page, 20);
    }
}
