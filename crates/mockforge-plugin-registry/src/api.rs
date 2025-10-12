//! Registry API client

use crate::{
    RegistryConfig, RegistryEntry, RegistryError, Result, SearchQuery, SearchResults, VersionEntry,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Registry API client
pub struct RegistryClient {
    config: RegistryConfig,
    client: reqwest::Client,
}

impl RegistryClient {
    /// Create a new registry client
    pub fn new(config: RegistryConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .build()
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        Ok(Self { config, client })
    }

    /// Search for plugins
    pub async fn search(&self, query: SearchQuery) -> Result<SearchResults> {
        let url = format!("{}/api/v1/plugins/search", self.config.url);
        let response = self
            .client
            .post(&url)
            .json(&query)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RegistryError::Network(format!("Search failed: {}", response.status())));
        }

        let results = response.json().await.map_err(|e| RegistryError::Network(e.to_string()))?;

        Ok(results)
    }

    /// Get plugin details
    pub async fn get_plugin(&self, name: &str) -> Result<RegistryEntry> {
        let url = format!("{}/api/v1/plugins/{}", self.config.url, name);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(RegistryError::PluginNotFound(name.to_string()));
        }

        if !response.status().is_success() {
            return Err(RegistryError::Network(format!(
                "Get plugin failed: {}",
                response.status()
            )));
        }

        let entry = response.json().await.map_err(|e| RegistryError::Network(e.to_string()))?;

        Ok(entry)
    }

    /// Get specific version of a plugin
    pub async fn get_version(&self, name: &str, version: &str) -> Result<VersionEntry> {
        let url = format!("{}/api/v1/plugins/{}/versions/{}", self.config.url, name, version);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RegistryError::InvalidVersion(version.to_string()));
        }

        let entry = response.json().await.map_err(|e| RegistryError::Network(e.to_string()))?;

        Ok(entry)
    }

    /// Publish a new plugin version
    pub async fn publish(&self, manifest: PublishRequest) -> Result<PublishResponse> {
        let token = self.config.token.as_ref().ok_or(RegistryError::AuthRequired)?;

        let url = format!("{}/api/v1/plugins/publish", self.config.url);
        let response = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&manifest)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(RegistryError::AuthRequired);
        }

        if response.status() == reqwest::StatusCode::FORBIDDEN {
            return Err(RegistryError::PermissionDenied);
        }

        if !response.status().is_success() {
            return Err(RegistryError::Network(format!("Publish failed: {}", response.status())));
        }

        let result = response.json().await.map_err(|e| RegistryError::Network(e.to_string()))?;

        Ok(result)
    }

    /// Download plugin binary
    pub async fn download(&self, url: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RegistryError::Network(format!("Download failed: {}", response.status())));
        }

        let bytes = response.bytes().await.map_err(|e| RegistryError::Network(e.to_string()))?;

        Ok(bytes.to_vec())
    }

    /// Yank a plugin version (remove from index)
    pub async fn yank(&self, name: &str, version: &str) -> Result<()> {
        let token = self.config.token.as_ref().ok_or(RegistryError::AuthRequired)?;

        let url = format!("{}/api/v1/plugins/{}/versions/{}/yank", self.config.url, name, version);
        let response = self
            .client
            .delete(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RegistryError::Network(format!("Yank failed: {}", response.status())));
        }

        Ok(())
    }
}

/// Publish request payload
#[derive(Debug, Serialize, Deserialize)]
pub struct PublishRequest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: crate::AuthorInfo,
    pub license: String,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub tags: Vec<String>,
    pub category: crate::PluginCategory,
    pub checksum: String,
    pub size: u64,
    pub min_mockforge_version: Option<String>,
}

/// Publish response
#[derive(Debug, Serialize, Deserialize)]
pub struct PublishResponse {
    pub success: bool,
    pub upload_url: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let config = RegistryConfig::default();
        let client = RegistryClient::new(config);
        assert!(client.is_ok());
    }
}
