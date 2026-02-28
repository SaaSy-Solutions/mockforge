//! Template Library System
//!
//! Provides a system for managing, versioning, and sharing templates.
//! Supports:
//! - Shared template storage
//! - Template versioning
//! - Template marketplace/registry
//! - Template discovery and installation

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Template metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMetadata {
    /// Template ID (unique identifier)
    pub id: String,
    /// Template name
    pub name: String,
    /// Template description
    pub description: Option<String>,
    /// Template version (semver format)
    pub version: String,
    /// Template author
    pub author: Option<String>,
    /// Template tags for categorization
    pub tags: Vec<String>,
    /// Template category (e.g., "user", "payment", "auth")
    pub category: Option<String>,
    /// Template content (the actual template string)
    pub content: String,
    /// Example usage
    pub example: Option<String>,
    /// Dependencies (other template IDs this template depends on)
    pub dependencies: Vec<String>,
    /// Creation timestamp
    pub created_at: Option<String>,
    /// Last updated timestamp
    pub updated_at: Option<String>,
}

/// Template version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVersion {
    /// Version string (semver)
    pub version: String,
    /// Template content for this version
    pub content: String,
    /// Changelog entry for this version
    pub changelog: Option<String>,
    /// Whether this is a pre-release version
    pub prerelease: bool,
    /// Release date
    pub released_at: String,
}

/// Template library entry (can have multiple versions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateLibraryEntry {
    /// Template ID
    pub id: String,
    /// Template name
    pub name: String,
    /// Template description
    pub description: Option<String>,
    /// Template author
    pub author: Option<String>,
    /// Template tags
    pub tags: Vec<String>,
    /// Template category
    pub category: Option<String>,
    /// Available versions
    pub versions: Vec<TemplateVersion>,
    /// Latest version
    pub latest_version: String,
    /// Dependencies
    pub dependencies: Vec<String>,
    /// Example usage
    pub example: Option<String>,
    /// Creation timestamp
    pub created_at: Option<String>,
    /// Last updated timestamp
    pub updated_at: Option<String>,
}

/// Template library registry
pub struct TemplateLibrary {
    /// Local storage directory
    storage_dir: PathBuf,
    /// In-memory cache of templates
    templates: HashMap<String, TemplateLibraryEntry>,
}

impl TemplateLibrary {
    /// Create a new template library
    pub fn new(storage_dir: impl AsRef<Path>) -> Result<Self> {
        let storage_dir = storage_dir.as_ref().to_path_buf();

        // Create storage directory if it doesn't exist
        std::fs::create_dir_all(&storage_dir).map_err(|e| {
            Error::generic(format!(
                "Failed to create template library directory {}: {}",
                storage_dir.display(),
                e
            ))
        })?;

        let mut library = Self {
            storage_dir,
            templates: HashMap::new(),
        };

        // Load existing templates
        library.load_templates()?;

        Ok(library)
    }

    /// Load templates from storage
    fn load_templates(&mut self) -> Result<()> {
        let templates_dir = self.storage_dir.join("templates");

        if !templates_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(&templates_dir)
            .map_err(|e| Error::generic(format!("Failed to read templates directory: {}", e)))?
        {
            let entry = entry
                .map_err(|e| Error::generic(format!("Failed to read directory entry: {}", e)))?;

            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.load_template_file(&path) {
                    Ok(Some(template)) => {
                        let id = template.id.clone();
                        self.templates.insert(id, template);
                    }
                    Ok(None) => {
                        // File doesn't contain a valid template, skip
                    }
                    Err(e) => {
                        warn!("Failed to load template from {}: {}", path.display(), e);
                    }
                }
            }
        }

        info!("Loaded {} template(s) from library", self.templates.len());
        Ok(())
    }

    /// Load a template from a file
    fn load_template_file(&self, path: &Path) -> Result<Option<TemplateLibraryEntry>> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            Error::generic(format!("Failed to read template file {}: {}", path.display(), e))
        })?;

        let template: TemplateLibraryEntry = serde_json::from_str(&content).map_err(|e| {
            Error::generic(format!("Failed to parse template file {}: {}", path.display(), e))
        })?;

        Ok(Some(template))
    }

    /// Register a template in the library
    pub fn register_template(&mut self, metadata: TemplateMetadata) -> Result<()> {
        let template_id = metadata.id.clone();

        // Check if template already exists
        let entry = if let Some(existing) = self.templates.get_mut(&template_id) {
            // Add new version to existing template
            let version = TemplateVersion {
                version: metadata.version.clone(),
                content: metadata.content.clone(),
                changelog: None,
                prerelease: false,
                released_at: chrono::Utc::now().to_rfc3339(),
            };

            existing.versions.push(version);
            existing.versions.sort_by(|a, b| {
                // Simple version comparison (could use semver crate for better comparison)
                b.version.cmp(&a.version)
            });
            existing.latest_version = metadata.version.clone();
            existing.updated_at = Some(chrono::Utc::now().to_rfc3339());

            existing.clone()
        } else {
            // Create new template entry
            let version = TemplateVersion {
                version: metadata.version.clone(),
                content: metadata.content.clone(),
                changelog: None,
                prerelease: false,
                released_at: chrono::Utc::now().to_rfc3339(),
            };

            TemplateLibraryEntry {
                id: metadata.id.clone(),
                name: metadata.name.clone(),
                description: metadata.description.clone(),
                author: metadata.author.clone(),
                tags: metadata.tags.clone(),
                category: metadata.category.clone(),
                versions: vec![version],
                latest_version: metadata.version.clone(),
                dependencies: metadata.dependencies.clone(),
                example: metadata.example.clone(),
                created_at: Some(chrono::Utc::now().to_rfc3339()),
                updated_at: Some(chrono::Utc::now().to_rfc3339()),
            }
        };

        // Save to disk
        self.save_template(&entry)?;

        // Update in-memory cache
        self.templates.insert(template_id, entry);

        Ok(())
    }

    /// Save a template to disk
    fn save_template(&self, template: &TemplateLibraryEntry) -> Result<()> {
        let templates_dir = self.storage_dir.join("templates");
        std::fs::create_dir_all(&templates_dir)
            .map_err(|e| Error::generic(format!("Failed to create templates directory: {}", e)))?;

        let file_path = templates_dir.join(format!("{}.json", template.id));
        let json = serde_json::to_string_pretty(template)
            .map_err(|e| Error::generic(format!("Failed to serialize template: {}", e)))?;

        std::fs::write(&file_path, json)
            .map_err(|e| Error::generic(format!("Failed to write template file: {}", e)))?;

        debug!("Saved template {} to {}", template.id, file_path.display());
        Ok(())
    }

    /// Get a template by ID
    pub fn get_template(&self, id: &str) -> Option<&TemplateLibraryEntry> {
        self.templates.get(id)
    }

    /// Get a specific version of a template
    pub fn get_template_version(&self, id: &str, version: &str) -> Option<String> {
        self.templates
            .get(id)
            .and_then(|entry| entry.versions.iter().find(|v| v.version == version))
            .map(|v| v.content.clone())
    }

    /// Get the latest version of a template
    pub fn get_latest_template(&self, id: &str) -> Option<String> {
        self.templates.get(id).map(|entry| {
            entry.versions.first().map(|v| v.content.clone()).unwrap_or_else(|| {
                // Fallback to latest_version field
                self.get_template_version(id, &entry.latest_version).unwrap_or_default()
            })
        })
    }

    /// List all templates
    pub fn list_templates(&self) -> Vec<&TemplateLibraryEntry> {
        self.templates.values().collect()
    }

    /// Search templates by query
    pub fn search_templates(&self, query: &str) -> Vec<&TemplateLibraryEntry> {
        let query_lower = query.to_lowercase();

        self.templates
            .values()
            .filter(|template| {
                template.name.to_lowercase().contains(&query_lower)
                    || template
                        .description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                    || template.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
                    || template
                        .category
                        .as_ref()
                        .map(|c| c.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .collect()
    }

    /// Search templates by category
    pub fn templates_by_category(&self, category: &str) -> Vec<&TemplateLibraryEntry> {
        self.templates
            .values()
            .filter(|template| {
                template
                    .category
                    .as_ref()
                    .map(|c| c.eq_ignore_ascii_case(category))
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Remove a template
    pub fn remove_template(&mut self, id: &str) -> Result<()> {
        if self.templates.remove(id).is_some() {
            let file_path = self.storage_dir.join("templates").join(format!("{}.json", id));
            if file_path.exists() {
                std::fs::remove_file(&file_path).map_err(|e| {
                    Error::generic(format!("Failed to remove template file: {}", e))
                })?;
            }
            info!("Removed template: {}", id);
        }
        Ok(())
    }

    /// Remove a specific version of a template
    pub fn remove_template_version(&mut self, id: &str, version: &str) -> Result<()> {
        if let Some(template) = self.templates.get_mut(id) {
            template.versions.retain(|v| v.version != version);

            if template.versions.is_empty() {
                // Remove entire template if no versions left
                self.remove_template(id)?;
            } else {
                // Update latest version
                template.versions.sort_by(|a, b| b.version.cmp(&a.version));
                template.latest_version =
                    template.versions.first().map(|v| v.version.clone()).unwrap_or_default();
                template.updated_at = Some(chrono::Utc::now().to_rfc3339());

                // Clone template to avoid borrow checker issues
                let template_clone = template.clone();
                let _ = template; // Explicitly drop mutable borrow

                // Save updated template
                self.save_template(&template_clone)?;
            }
        }
        Ok(())
    }

    /// Get storage directory
    pub fn storage_dir(&self) -> &Path {
        &self.storage_dir
    }
}

/// Template marketplace/registry (for remote templates)
pub struct TemplateMarketplace {
    /// Registry URL
    registry_url: String,
    /// Authentication token (optional)
    auth_token: Option<String>,
}

impl TemplateMarketplace {
    /// Create a new template marketplace client
    pub fn new(registry_url: String, auth_token: Option<String>) -> Self {
        Self {
            registry_url,
            auth_token,
        }
    }

    /// Search for templates in the marketplace
    pub async fn search(&self, query: &str) -> Result<Vec<TemplateLibraryEntry>> {
        let encoded_query = urlencoding::encode(query);
        let url = format!("{}/api/templates/search?q={}", self.registry_url, encoded_query);

        let mut request = reqwest::Client::new().get(&url);
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| Error::generic(format!("Failed to search marketplace: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::generic(format!(
                "Marketplace search failed with status: {}",
                response.status()
            )));
        }

        let templates: Vec<TemplateLibraryEntry> = response
            .json()
            .await
            .map_err(|e| Error::generic(format!("Failed to parse marketplace response: {}", e)))?;

        Ok(templates)
    }

    /// Get a template from the marketplace
    pub async fn get_template(
        &self,
        id: &str,
        version: Option<&str>,
    ) -> Result<TemplateLibraryEntry> {
        let url = if let Some(version) = version {
            format!("{}/api/templates/{}/{}", self.registry_url, id, version)
        } else {
            format!("{}/api/templates/{}", self.registry_url, id)
        };

        let mut request = reqwest::Client::new().get(&url);
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request.send().await.map_err(|e| {
            Error::generic(format!("Failed to fetch template from marketplace: {}", e))
        })?;

        if !response.status().is_success() {
            return Err(Error::generic(format!("Failed to fetch template: {}", response.status())));
        }

        let template: TemplateLibraryEntry = response
            .json()
            .await
            .map_err(|e| Error::generic(format!("Failed to parse template: {}", e)))?;

        Ok(template)
    }

    /// List featured/popular templates
    pub async fn list_featured(&self) -> Result<Vec<TemplateLibraryEntry>> {
        let url = format!("{}/api/templates/featured", self.registry_url);

        let mut request = reqwest::Client::new().get(&url);
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| Error::generic(format!("Failed to fetch featured templates: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::generic(format!(
                "Failed to fetch featured templates: {}",
                response.status()
            )));
        }

        let templates: Vec<TemplateLibraryEntry> = response
            .json()
            .await
            .map_err(|e| Error::generic(format!("Failed to parse featured templates: {}", e)))?;

        Ok(templates)
    }

    /// List templates by category
    pub async fn list_by_category(&self, category: &str) -> Result<Vec<TemplateLibraryEntry>> {
        let encoded_category = urlencoding::encode(category);
        let url = format!("{}/api/templates/category/{}", self.registry_url, encoded_category);

        let mut request = reqwest::Client::new().get(&url);
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| Error::generic(format!("Failed to fetch templates by category: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::generic(format!(
                "Failed to fetch templates by category: {}",
                response.status()
            )));
        }

        let templates: Vec<TemplateLibraryEntry> = response
            .json()
            .await
            .map_err(|e| Error::generic(format!("Failed to parse templates: {}", e)))?;

        Ok(templates)
    }
}

/// Template library manager (combines local library and marketplace)
pub struct TemplateLibraryManager {
    /// Local template library
    library: TemplateLibrary,
    /// Marketplace client (optional)
    marketplace: Option<TemplateMarketplace>,
}

impl TemplateLibraryManager {
    /// Create a new template library manager
    pub fn new(storage_dir: impl AsRef<Path>) -> Result<Self> {
        let library = TemplateLibrary::new(storage_dir)?;
        Ok(Self {
            library,
            marketplace: None,
        })
    }

    /// Enable marketplace integration
    pub fn with_marketplace(mut self, registry_url: String, auth_token: Option<String>) -> Self {
        self.marketplace = Some(TemplateMarketplace::new(registry_url, auth_token));
        self
    }

    /// Install a template from marketplace to local library
    pub async fn install_from_marketplace(
        &mut self,
        id: &str,
        version: Option<&str>,
    ) -> Result<()> {
        let marketplace = self
            .marketplace
            .as_ref()
            .ok_or_else(|| Error::generic("Marketplace not configured".to_string()))?;

        let template = marketplace.get_template(id, version).await?;

        // Convert to metadata and register
        let latest_version = template
            .versions
            .first()
            .ok_or_else(|| Error::generic("Template has no versions".to_string()))?;

        let metadata = TemplateMetadata {
            id: template.id.clone(),
            name: template.name.clone(),
            description: template.description.clone(),
            version: latest_version.version.clone(),
            author: template.author.clone(),
            tags: template.tags.clone(),
            category: template.category.clone(),
            content: latest_version.content.clone(),
            example: template.example.clone(),
            dependencies: template.dependencies.clone(),
            created_at: template.created_at.clone(),
            updated_at: template.updated_at.clone(),
        };

        self.library.register_template(metadata)?;
        info!("Installed template {} from marketplace", id);

        Ok(())
    }

    /// Get local library reference
    pub fn library(&self) -> &TemplateLibrary {
        &self.library
    }

    /// Get mutable local library reference
    pub fn library_mut(&mut self) -> &mut TemplateLibrary {
        &mut self.library
    }

    /// Get marketplace reference
    pub fn marketplace(&self) -> Option<&TemplateMarketplace> {
        self.marketplace.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_template_metadata() {
        let metadata = TemplateMetadata {
            id: "user-profile".to_string(),
            name: "User Profile Template".to_string(),
            description: Some("Template for user profile data".to_string()),
            version: "1.0.0".to_string(),
            author: Some("Test Author".to_string()),
            tags: vec!["user".to_string(), "profile".to_string()],
            category: Some("user".to_string()),
            content: "{{faker.name}} - {{faker.email}}".to_string(),
            example: Some("John Doe - john@example.com".to_string()),
            dependencies: Vec::new(),
            created_at: None,
            updated_at: None,
        };

        assert_eq!(metadata.id, "user-profile");
        assert_eq!(metadata.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_template_library() {
        let temp_dir = TempDir::new().unwrap();
        let library = TemplateLibrary::new(temp_dir.path()).unwrap();

        let metadata = TemplateMetadata {
            id: "test-template".to_string(),
            name: "Test Template".to_string(),
            description: None,
            version: "1.0.0".to_string(),
            author: None,
            tags: Vec::new(),
            category: None,
            content: "{{uuid}}".to_string(),
            example: None,
            dependencies: Vec::new(),
            created_at: None,
            updated_at: None,
        };

        let mut library = library;
        library.register_template(metadata).unwrap();

        let template = library.get_template("test-template");
        assert!(template.is_some());
        assert_eq!(template.unwrap().name, "Test Template");
    }
}
