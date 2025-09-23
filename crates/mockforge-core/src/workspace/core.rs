//! Core workspace and folder structures
//!
//! This module provides the fundamental data structures for workspaces, folders,
//! and mock requests, including their properties and basic operations.

use crate::{Result, Error, routing::HttpMethod};
use crate::config::AuthConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Unique identifier for workspace entities
pub type EntityId = String;

/// Workspace represents a top-level organizational unit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// Unique identifier
    pub id: EntityId,
    /// Human-readable name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
    /// Associated tags for filtering and organization
    pub tags: Vec<String>,
    /// Configuration specific to this workspace
    pub config: WorkspaceConfig,
    /// Root folders in this workspace
    pub folders: Vec<Folder>,
    /// Root requests (not in any folder)
    pub requests: Vec<MockRequest>,
    /// Display order for UI sorting (lower numbers appear first)
    #[serde(default)]
    pub order: i32,
}

/// Configuration for request inheritance at folder level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderInheritanceConfig {
    /// Headers to be inherited by child requests (if not overridden)
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Authentication configuration for inheritance
    pub auth: Option<AuthConfig>,
}

/// Folder for organizing requests hierarchically
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    /// Unique identifier
    pub id: EntityId,
    /// Human-readable name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Parent folder ID (None for root folders)
    pub parent_id: Option<EntityId>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
    /// Child folders
    pub folders: Vec<Folder>,
    /// Requests in this folder
    pub requests: Vec<MockRequest>,
    /// Configuration for inheritance to child requests
    pub inheritance: FolderInheritanceConfig,
    /// Display order for UI sorting
    #[serde(default)]
    pub order: i32,
    /// Whether this folder is expanded in the UI
    #[serde(default = "default_true")]
    pub expanded: bool,
    /// Whether this folder is collapsed in the UI
    #[serde(default)]
    pub collapsed: bool,
}

/// Mock request representing an API endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockRequest {
    /// Unique identifier
    pub id: EntityId,
    /// Human-readable name
    pub name: String,
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub method: HttpMethod,
    /// Request URL/path
    pub url: String,
    /// Optional description
    pub description: Option<String>,
    /// Parent folder ID (None for root requests)
    pub folder_id: Option<EntityId>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
    /// Request headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Query parameters
    #[serde(default)]
    pub query_params: HashMap<String, String>,
    /// Request body (JSON, XML, form data, etc.)
    pub body: Option<String>,
    /// Authentication configuration
    pub auth: Option<AuthConfig>,
    /// Associated mock responses
    pub responses: Vec<MockResponse>,
    /// Display order for UI sorting
    #[serde(default)]
    pub order: i32,
    /// Tags for filtering and organization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Whether this request is enabled/active
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Mock response for a request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockResponse {
    /// Unique identifier
    pub id: EntityId,
    /// HTTP status code
    pub status_code: u16,
    /// Response name (e.g., "Success", "Not Found", etc.)
    pub name: String,
    /// Response body
    pub body: String,
    /// Response headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Response delay in milliseconds
    #[serde(default)]
    pub delay: u64,
    /// Whether this response is active/selected
    #[serde(default)]
    pub active: bool,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
    /// Associated history entries
    pub history: Vec<ResponseHistoryEntry>,
}

/// History entry for response usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseHistoryEntry {
    /// Timestamp when this response was used
    pub timestamp: DateTime<Utc>,
    /// Request that triggered this response
    pub request_id: EntityId,
    /// Response duration in milliseconds
    pub duration_ms: u64,
}

/// Color for environment identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentColor {
    /// Red component (0-255)
    pub red: u8,
    /// Green component (0-255)
    pub green: u8,
    /// Blue component (0-255)
    pub blue: u8,
    /// Alpha component (0-255)
    #[serde(default = "default_alpha")]
    pub alpha: u8,
}

impl EnvironmentColor {
    /// Create a new color with RGB values
    pub fn new(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 255,
        }
    }

    /// Create a color from a hex string (e.g., "#ff0000")
    pub fn from_hex(hex: &str) -> Result<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return Err(Error::generic("Invalid hex color format"));
        }

        let red = u8::from_str_radix(&hex[0..2], 16)
            .map_err(|_| Error::generic("Invalid hex color format"))?;
        let green = u8::from_str_radix(&hex[2..4], 16)
            .map_err(|_| Error::generic("Invalid hex color format"))?;
        let blue = u8::from_str_radix(&hex[4..6], 16)
            .map_err(|_| Error::generic("Invalid hex color format"))?;

        Ok(Self::new(red, green, blue))
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.red, self.green, self.blue)
    }
}

/// Environment configuration for variable substitution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    /// Unique identifier
    pub id: EntityId,
    /// Environment name
    pub name: String,
    /// Environment variables
    pub variables: HashMap<String, String>,
    /// Color for UI identification
    pub color: EnvironmentColor,
    /// Whether this environment is active
    #[serde(default)]
    pub active: bool,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
}

/// Workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Base URL for requests in this workspace
    pub base_url: Option<String>,
    /// Default authentication
    pub auth: Option<AuthConfig>,
    /// Default headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// Whether SSL verification is enabled
    #[serde(default = "default_true")]
    pub ssl_verify: bool,
    /// Proxy configuration
    pub proxy: Option<String>,
}

/// Default timeout value (30 seconds)
fn default_timeout() -> u64 {
    30
}

/// Default boolean true value
fn default_true() -> bool {
    true
}

/// Default alpha value (255)
fn default_alpha() -> u8 {
    255
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            base_url: None,
            auth: None,
            headers: HashMap::new(),
            timeout_seconds: default_timeout(),
            ssl_verify: default_true(),
            proxy: None,
        }
    }
}

impl Workspace {
    /// Create a new workspace
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        Self {
            id,
            name,
            description: None,
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
            config: WorkspaceConfig::default(),
            folders: Vec::new(),
            requests: Vec::new(),
            order: 0,
        }
    }

    /// Update the workspace's last modification time
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Add a folder to the workspace
    pub fn add_folder(&mut self, mut folder: Folder) {
        folder.parent_id = None;
        folder.touch();
        self.folders.push(folder);
        self.touch();
    }

    /// Add a request to the workspace
    pub fn add_request(&mut self, mut request: MockRequest) {
        request.folder_id = None;
        request.touch();
        self.requests.push(request);
        self.touch();
    }
}

impl Folder {
    /// Create a new folder
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        Self {
            id,
            name,
            description: None,
            parent_id: None,
            created_at: now,
            updated_at: now,
            folders: Vec::new(),
            requests: Vec::new(),
            inheritance: FolderInheritanceConfig {
                headers: HashMap::new(),
                auth: None,
            },
            order: 0,
            expanded: true,
            collapsed: false,
        }
    }

    /// Update the folder's last modification time
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Add a subfolder
    pub fn add_folder(&mut self, mut folder: Folder) {
        folder.parent_id = Some(self.id.clone());
        folder.touch();
        self.folders.push(folder);
        self.touch();
    }

    /// Add a request to the folder
    pub fn add_request(&mut self, mut request: MockRequest) {
        request.folder_id = Some(self.id.clone());
        request.touch();
        self.requests.push(request);
        self.touch();
    }

    /// Get inherited headers from parent folders
    pub fn get_inherited_headers(&self, all_folders: &[Folder]) -> HashMap<String, String> {
        let mut headers = HashMap::new();

        // Recursively get headers from parent folders
        if let Some(parent_id) = &self.parent_id {
            if let Some(parent) = all_folders.iter().find(|f| f.id == *parent_id) {
                headers = parent.get_inherited_headers(all_folders);
            }
        }

        // Merge current folder's inheritance headers, overriding parent headers
        for (key, value) in &self.inheritance.headers {
            headers.insert(key.clone(), value.clone());
        }

        headers
    }
}

impl MockRequest {
    /// Create a new request
    pub fn new(name: String, method: HttpMethod, url: String) -> Self {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        Self {
            id,
            name,
            method,
            url,
            description: None,
            folder_id: None,
            created_at: now,
            updated_at: now,
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: None,
            auth: None,
            responses: Vec::new(),
            order: 0,
            tags: Vec::new(),
            enabled: true,
        }
    }

    /// Update the request's last modification time
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Add a response to the request
    pub fn add_response(&mut self, mut response: MockResponse) {
        response.active = self.responses.is_empty(); // Make first response active
        response.touch();
        self.responses.push(response);
        self.touch();
    }

    /// Get the active response
    pub fn active_response(&self) -> Option<&MockResponse> {
        self.responses.iter().find(|r| r.active)
    }

    /// Get the active response mutably
    pub fn active_response_mut(&mut self) -> Option<&mut MockResponse> {
        self.responses.iter_mut().find(|r| r.active)
    }
}

impl MockResponse {
    /// Create a new response
    pub fn new(status_code: u16, name: String, body: String) -> Self {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        Self {
            id,
            status_code,
            name,
            body,
            headers: HashMap::new(),
            delay: 0,
            active: false,
            created_at: now,
            updated_at: now,
            history: Vec::new(),
        }
    }

    /// Update the response's last modification time
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Record a response usage in history
    pub fn record_usage(&mut self, request_id: EntityId, duration_ms: u64) {
        self.history.push(ResponseHistoryEntry {
            timestamp: Utc::now(),
            request_id,
            duration_ms,
        });
        self.touch();
    }
}

impl Environment {
    /// Create a new environment
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        Self {
            id,
            name,
            variables: HashMap::new(),
            color: EnvironmentColor::new(64, 128, 255), // Default blue color
            active: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Update the environment's last modification time
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Set a variable in the environment
    pub fn set_variable(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
        self.touch();
    }

    /// Remove a variable from the environment
    pub fn remove_variable(&mut self, key: &str) -> Option<String> {
        let result = self.variables.remove(key);
        if result.is_some() {
            self.touch();
        }
        result
    }

    /// Get a variable from the environment
    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }
}
