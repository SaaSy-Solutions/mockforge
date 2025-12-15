//! Workspace and folder organization for MockForge requests
//!
//! This module has been refactored into sub-modules for better organization:
//! - core: Core workspace and folder structures
//! - registry: Workspace registry and management
//! - environment: Environment configuration and management
//! - sync: Synchronization functionality
//! - request: Mock request handling and processing

// Re-export sub-modules for backward compatibility
pub mod core;
pub mod environment;
pub mod mock_environment;
pub mod promotion_trait;
pub mod rbac;
pub mod registry;
pub mod request;
pub mod scenario_promotion;
pub mod sync;
pub mod template_application;

// Re-export commonly used types
pub use environment::*;
pub use mock_environment::*;
pub use rbac::*;
pub use registry::*;
pub use request::*;
pub use scenario_promotion::*;
pub use sync::*;
pub use template_application::*;

// Legacy imports for compatibility
use crate::config::AuthConfig;
use crate::encryption::AutoEncryptionConfig;
use crate::fidelity::FidelityScore;
use crate::reality::RealityLevel;
use crate::routing::{HttpMethod, Route, RouteRegistry};
use crate::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FolderInheritanceConfig {
    /// Headers to be inherited by child requests (if not overridden)
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Authentication configuration for inheritance
    pub auth: Option<AuthConfig>,
}

/// Folder represents a hierarchical grouping within a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    /// Unique identifier
    pub id: EntityId,
    /// Human-readable name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Parent folder ID (None if root folder)
    pub parent_id: Option<EntityId>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
    /// Associated tags
    pub tags: Vec<String>,
    /// Inheritance configuration for this folder
    #[serde(default)]
    pub inheritance: FolderInheritanceConfig,
    /// Child folders
    pub folders: Vec<Folder>,
    /// Requests in this folder
    pub requests: Vec<MockRequest>,
}

/// Mock request definition with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockRequest {
    /// Unique identifier
    pub id: EntityId,
    /// Human-readable name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// HTTP method
    pub method: HttpMethod,
    /// Request path
    pub path: String,
    /// HTTP headers
    pub headers: HashMap<String, String>,
    /// Query parameters
    pub query_params: HashMap<String, String>,
    /// Request body template
    pub body: Option<String>,
    /// Expected response
    pub response: MockResponse,
    /// History of actual request executions
    pub response_history: Vec<ResponseHistoryEntry>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
    /// Associated tags
    pub tags: Vec<String>,
    /// Authentication configuration
    pub auth: Option<AuthConfig>,
    /// Priority for route matching (higher = more specific)
    pub priority: i32,
}

/// Mock response definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockResponse {
    /// HTTP status code
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body template
    pub body: Option<String>,
    /// Content type
    pub content_type: Option<String>,
    /// Response delay in milliseconds
    pub delay_ms: Option<u64>,
}

/// Response history entry for tracking actual request executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseHistoryEntry {
    /// Unique execution ID
    pub id: String,
    /// Execution timestamp
    pub executed_at: DateTime<Utc>,
    /// Actual request method used
    pub request_method: HttpMethod,
    /// Actual request path used
    pub request_path: String,
    /// Request headers sent
    pub request_headers: HashMap<String, String>,
    /// Request body sent
    pub request_body: Option<String>,
    /// Response status code received
    pub response_status_code: u16,
    /// Response headers received
    pub response_headers: HashMap<String, String>,
    /// Response body received
    pub response_body: Option<String>,
    /// Response time in milliseconds
    pub response_time_ms: u64,
    /// Response size in bytes
    pub response_size_bytes: u64,
    /// Error message if execution failed
    pub error_message: Option<String>,
}

/// Represents a color for environment visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentColor {
    /// Hex color code (e.g., "#FF5733")
    pub hex: String,
    /// Optional color name for accessibility
    pub name: Option<String>,
}

/// Environment for managing variable collections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    /// Unique identifier
    pub id: EntityId,
    /// Human-readable name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Color for visual distinction in UI
    pub color: Option<EnvironmentColor>,
    /// Environment variables
    pub variables: HashMap<String, String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
    /// Display order for UI sorting (lower numbers appear first)
    #[serde(default)]
    pub order: i32,
    /// Whether this environment can be shared/synced
    #[serde(default)]
    pub sharable: bool,
}

/// Directory sync configuration for a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Enable directory syncing for this workspace
    pub enabled: bool,
    /// Target directory for sync (relative or absolute path)
    pub target_directory: Option<String>,
    /// Directory structure to use (flat, nested, grouped)
    pub directory_structure: SyncDirectoryStructure,
    /// Auto-sync direction (one-way workspace→directory, bidirectional, or manual)
    pub sync_direction: SyncDirection,
    /// Whether to include metadata files
    pub include_metadata: bool,
    /// Filesystem monitoring enabled for real-time sync
    pub realtime_monitoring: bool,
    /// Custom filename pattern for exported files
    pub filename_pattern: String,
    /// Regular expression for excluding workspaces/requests
    pub exclude_pattern: Option<String>,
    /// Force overwrite existing files during sync
    pub force_overwrite: bool,
    /// Last sync timestamp
    pub last_sync: Option<DateTime<Utc>>,
}

/// Directory structure options for sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncDirectoryStructure {
    /// All workspaces in flat structure: workspace-name.yaml
    Flat,
    /// Nested by workspace: workspaces/{name}/workspace.yaml + requests/
    Nested,
    /// Grouped by type: requests/, responses/, metadata/
    Grouped,
}

/// Sync direction options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncDirection {
    /// Manual sync only (one-off operations)
    Manual,
    /// One-way: workspace changes sync silently to directory
    WorkspaceToDirectory,
    /// Bidirectional: changes in either direction trigger sync
    Bidirectional,
}

/// Workspace-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Base URL for all requests in this workspace
    pub base_url: Option<String>,
    /// Default headers for all requests
    pub default_headers: HashMap<String, String>,
    /// Authentication configuration
    pub auth: Option<AuthConfig>,
    /// Global environment (always available)
    pub global_environment: Environment,
    /// Sub-environments (switchable)
    pub environments: Vec<Environment>,
    /// Currently active environment ID (None means only global)
    pub active_environment_id: Option<EntityId>,
    /// Mock environment manager for dev/test/prod environments
    /// Manages environment-specific overrides for reality levels, chaos profiles, and drift budgets
    #[serde(default)]
    pub mock_environments: MockEnvironmentManager,
    /// Directory sync configuration
    pub sync: SyncConfig,
    /// Automatic encryption configuration
    #[serde(default)]
    pub auto_encryption: AutoEncryptionConfig,
    /// Reality level for this workspace (1-5)
    /// Controls the realism of mock behavior (chaos, latency, MockAI)
    /// This is the default reality level; can be overridden per environment
    #[serde(default)]
    pub reality_level: Option<RealityLevel>,
    /// AI mode for this workspace
    /// Controls how AI-generated artifacts are used at runtime
    #[serde(default)]
    pub ai_mode: Option<crate::ai_studio::config::AiMode>,
    /// Current fidelity score for this workspace
    /// Measures how close the mock is to the real upstream
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fidelity_score: Option<FidelityScore>,
}

/// Workspace registry for managing multiple workspaces
#[derive(Debug, Clone)]
pub struct WorkspaceRegistry {
    workspaces: HashMap<EntityId, Workspace>,
    active_workspace: Option<EntityId>,
}

impl Workspace {
    /// Create a new workspace
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        let workspace_id = Uuid::new_v4().to_string();
        let mut workspace = Self {
            id: workspace_id.clone(),
            name,
            description: None,
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
            config: WorkspaceConfig::default(),
            folders: Vec::new(),
            requests: Vec::new(),
            order: 0, // Default order will be updated when added to registry
        };

        // Initialize default mock environments (dev/test/prod)
        workspace.initialize_default_mock_environments();

        workspace
    }

    /// Initialize default mock environments for this workspace
    /// This is called when creating a new workspace or when loading an existing one
    /// that doesn't have mock environments configured
    pub fn initialize_default_mock_environments(&mut self) {
        // Only initialize if mock_environments is empty or has no workspace_id set
        if self.config.mock_environments.workspace_id.is_empty()
            || self.config.mock_environments.environments.is_empty()
        {
            // Update workspace_id if needed
            if self.config.mock_environments.workspace_id.is_empty() {
                self.config.mock_environments.workspace_id = self.id.clone();
            }

            // Create default dev environment if it doesn't exist
            if !self
                .config
                .mock_environments
                .environments
                .contains_key(&MockEnvironmentName::Dev)
            {
                let dev_env = MockEnvironment::new(self.id.clone(), MockEnvironmentName::Dev);
                self.config.mock_environments.add_environment(dev_env);
            }

            // Create default test environment if it doesn't exist
            if !self
                .config
                .mock_environments
                .environments
                .contains_key(&MockEnvironmentName::Test)
            {
                let test_env = MockEnvironment::new(self.id.clone(), MockEnvironmentName::Test);
                self.config.mock_environments.add_environment(test_env);
            }

            // Create default prod environment if it doesn't exist
            if !self
                .config
                .mock_environments
                .environments
                .contains_key(&MockEnvironmentName::Prod)
            {
                let prod_env = MockEnvironment::new(self.id.clone(), MockEnvironmentName::Prod);
                self.config.mock_environments.add_environment(prod_env);
            }

            // Set dev as the default active environment if none is set
            if self.config.mock_environments.active_environment.is_none() {
                let _ =
                    self.config.mock_environments.set_active_environment(MockEnvironmentName::Dev);
            }
        }
    }

    /// Add a folder to this workspace
    pub fn add_folder(&mut self, name: String) -> Result<EntityId> {
        let folder = Folder::new(name);
        let id = folder.id.clone();
        self.folders.push(folder);
        self.updated_at = Utc::now();
        Ok(id)
    }

    /// Create a new environment
    pub fn create_environment(
        &mut self,
        name: String,
        description: Option<String>,
    ) -> Result<EntityId> {
        // Check if environment name already exists
        if self.config.environments.iter().any(|env| env.name == name) {
            return Err(Error::generic(format!("Environment with name '{}' already exists", name)));
        }

        let mut environment = Environment::new(name);
        environment.description = description;

        // Set order to the end of the list
        environment.order = self.config.environments.len() as i32;

        let id = environment.id.clone();

        self.config.environments.push(environment);
        self.updated_at = Utc::now();
        Ok(id)
    }

    /// Get all environments (global + sub-environments)
    pub fn get_environments(&self) -> Vec<&Environment> {
        let mut all_envs = vec![&self.config.global_environment];
        all_envs.extend(self.config.environments.iter());
        all_envs
    }

    /// Get environment by ID
    pub fn get_environment(&self, id: &str) -> Option<&Environment> {
        if self.config.global_environment.id == id {
            Some(&self.config.global_environment)
        } else {
            self.config.environments.iter().find(|env| env.id == id)
        }
    }

    /// Get environment by ID (mutable)
    pub fn get_environment_mut(&mut self, id: &str) -> Option<&mut Environment> {
        if self.config.global_environment.id == id {
            Some(&mut self.config.global_environment)
        } else {
            self.config.environments.iter_mut().find(|env| env.id == id)
        }
    }

    /// Set active environment
    pub fn set_active_environment(&mut self, environment_id: Option<String>) -> Result<()> {
        if let Some(ref id) = environment_id {
            if self.get_environment(id).is_none() {
                return Err(Error::generic(format!("Environment with ID '{}' not found", id)));
            }
        }
        self.config.active_environment_id = environment_id;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Get active environment (returns global if no sub-environment is active)
    pub fn get_active_environment(&self) -> &Environment {
        if let Some(ref active_id) = self.config.active_environment_id {
            self.get_environment(active_id).unwrap_or(&self.config.global_environment)
        } else {
            &self.config.global_environment
        }
    }

    /// Get active environment ID
    pub fn get_active_environment_id(&self) -> Option<&str> {
        self.config.active_environment_id.as_deref()
    }

    /// Get variable value from current active environment
    pub fn get_variable(&self, key: &str) -> Option<&String> {
        // First check active environment, then global environment
        let active_env = self.get_active_environment();
        active_env.get_variable(key).or_else(|| {
            if active_env.id != self.config.global_environment.id {
                self.config.global_environment.get_variable(key)
            } else {
                None
            }
        })
    }

    /// Get all variables from current active environment context
    pub fn get_all_variables(&self) -> HashMap<String, String> {
        let mut variables = HashMap::new();

        // Start with global environment variables
        variables.extend(self.config.global_environment.variables.clone());

        // Override with active environment variables if different from global
        let active_env = self.get_active_environment();
        if active_env.id != self.config.global_environment.id {
            variables.extend(active_env.variables.clone());
        }

        variables
    }

    /// Delete an environment
    pub fn delete_environment(&mut self, id: &str) -> Result<()> {
        if id == self.config.global_environment.id {
            return Err(Error::generic("Cannot delete global environment".to_string()));
        }

        let position = self.config.environments.iter().position(|env| env.id == id);
        if let Some(pos) = position {
            self.config.environments.remove(pos);

            // Clear active environment if it was deleted
            if self.config.active_environment_id.as_deref() == Some(id) {
                self.config.active_environment_id = None;
            }

            self.updated_at = Utc::now();
            Ok(())
        } else {
            Err(Error::generic(format!("Environment with ID '{}' not found", id)))
        }
    }

    /// Update the order of environments
    pub fn update_environments_order(&mut self, environment_ids: Vec<String>) -> Result<()> {
        // Validate that all provided IDs exist
        for env_id in &environment_ids {
            if !self.config.environments.iter().any(|env| env.id == *env_id) {
                return Err(Error::generic(format!("Environment with ID '{}' not found", env_id)));
            }
        }

        // Update order for each environment
        for (index, env_id) in environment_ids.iter().enumerate() {
            if let Some(env) = self.config.environments.iter_mut().find(|env| env.id == *env_id) {
                env.order = index as i32;
                env.updated_at = Utc::now();
            }
        }

        self.updated_at = Utc::now();
        Ok(())
    }

    /// Get environments sorted by order
    pub fn get_environments_ordered(&self) -> Vec<&Environment> {
        let mut all_envs = vec![&self.config.global_environment];
        all_envs.extend(self.config.environments.iter());
        all_envs.sort_by_key(|env| env.order);
        all_envs
    }

    /// Get the mock environment manager
    pub fn get_mock_environments(&self) -> &MockEnvironmentManager {
        &self.config.mock_environments
    }

    /// Get mutable access to the mock environment manager
    pub fn get_mock_environments_mut(&mut self) -> &mut MockEnvironmentManager {
        &mut self.config.mock_environments
    }

    /// Get a specific mock environment by name
    pub fn get_mock_environment(&self, name: MockEnvironmentName) -> Option<&MockEnvironment> {
        self.config.mock_environments.get_environment(name)
    }

    /// Get the active mock environment
    pub fn get_active_mock_environment(&self) -> Option<&MockEnvironment> {
        self.config.mock_environments.get_active_environment()
    }

    /// Set the active mock environment
    pub fn set_active_mock_environment(&mut self, name: MockEnvironmentName) -> Result<()> {
        self.config.mock_environments.set_active_environment(name)?;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// List all mock environments
    pub fn list_mock_environments(&self) -> Vec<&MockEnvironment> {
        self.config.mock_environments.list_environments()
    }

    /// Add or update a mock environment configuration
    pub fn set_mock_environment_config(
        &mut self,
        name: MockEnvironmentName,
        reality_config: Option<crate::reality::RealityConfig>,
        chaos_config: Option<crate::chaos_utilities::ChaosConfig>,
        drift_budget_config: Option<crate::contract_drift::DriftBudgetConfig>,
    ) -> Result<()> {
        // Get or create the environment
        let env = if let Some(existing) = self.config.mock_environments.get_environment(name) {
            MockEnvironment::with_configs(
                existing.workspace_id.clone(),
                name,
                reality_config,
                chaos_config,
                drift_budget_config,
            )
        } else {
            MockEnvironment::with_configs(
                self.id.clone(),
                name,
                reality_config,
                chaos_config,
                drift_budget_config,
            )
        };

        self.config.mock_environments.add_environment(env);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Configure directory sync for this workspace
    pub fn configure_sync(&mut self, config: SyncConfig) -> Result<()> {
        self.config.sync = config;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Enable directory sync with default settings
    pub fn enable_sync(&mut self, target_directory: String) -> Result<()> {
        self.config.sync.enabled = true;
        self.config.sync.target_directory = Some(target_directory);
        self.config.sync.realtime_monitoring = true; // Enable realtime monitoring by default
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Disable directory sync
    pub fn disable_sync(&mut self) -> Result<()> {
        self.config.sync.enabled = false;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Get sync configuration
    pub fn get_sync_config(&self) -> &SyncConfig {
        &self.config.sync
    }

    /// Check if sync is enabled
    pub fn is_sync_enabled(&self) -> bool {
        self.config.sync.enabled
    }

    /// Get the target sync directory
    pub fn get_sync_directory(&self) -> Option<&str> {
        self.config.sync.target_directory.as_deref()
    }

    /// Set sync directory
    pub fn set_sync_directory(&mut self, directory: Option<String>) -> Result<()> {
        self.config.sync.target_directory = directory;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Set sync direction
    pub fn set_sync_direction(&mut self, direction: SyncDirection) -> Result<()> {
        self.config.sync.sync_direction = direction;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Get sync direction
    pub fn get_sync_direction(&self) -> &SyncDirection {
        &self.config.sync.sync_direction
    }

    /// Enable/disable real-time monitoring
    pub fn set_realtime_monitoring(&mut self, enabled: bool) -> Result<()> {
        self.config.sync.realtime_monitoring = enabled;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Check if real-time monitoring is enabled
    pub fn is_realtime_monitoring_enabled(&self) -> bool {
        self.config.sync.realtime_monitoring && self.config.sync.enabled
    }

    /// Create filtered copy for directory sync (removes sensitive environments and non-sharable environments)
    pub fn to_filtered_for_sync(&self) -> Workspace {
        let mut filtered = self.clone();

        // Remove sensitive environment variables before sync
        // This implementation filters out common sensitive keys
        filtered.config.global_environment.variables =
            self.filter_sensitive_variables(&self.config.global_environment.variables);

        // Filter out non-sharable environments
        filtered.config.environments = filtered
            .config
            .environments
            .into_iter()
            .filter(|env| env.sharable)
            .map(|mut env| {
                env.variables = self.filter_sensitive_variables(&env.variables);
                env
            })
            .collect();

        filtered
    }

    /// Filter out sensitive environment variables
    fn filter_sensitive_variables(
        &self,
        variables: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let sensitive_keys = [
            // Common sensitive keys that should not be synced
            "password",
            "secret",
            "key",
            "token",
            "credential",
            "api_key",
            "apikey",
            "api_secret",
            "db_password",
            "database_password",
            "aws_secret_key",
            "aws_session_token",
            "private_key",
            "authorization",
            "auth_token",
            "access_token",
            "refresh_token",
            "cookie",
            "session",
            "csrf",
            "jwt",
            "bearer",
        ];

        variables
            .iter()
            .filter(|(key, _)| {
                let key_lower = key.to_lowercase();
                !sensitive_keys.iter().any(|sensitive| key_lower.contains(sensitive))
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Check if this workspace should be included in directory sync
    pub fn should_sync(&self) -> bool {
        self.config.sync.enabled && self.config.sync.target_directory.is_some()
    }

    /// Get the filename for this workspace in directory sync
    pub fn get_sync_filename(&self) -> String {
        // Apply filename pattern, default to {name}
        let pattern = &self.config.sync.filename_pattern;

        // Simple pattern replacement - {name} → workspace name, {id} → workspace id
        let filename = pattern
            .replace("{name}", &sanitize_filename(&self.name))
            .replace("{id}", &self.id);

        if filename.ends_with(".yaml") || filename.ends_with(".yml") {
            filename
        } else {
            format!("{}.yaml", filename)
        }
    }
}

/// Helper function to sanitize filenames for cross-platform compatibility
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c if c.is_whitespace() => '-',
            c => c,
        })
        .collect::<String>()
        .to_lowercase()
}

impl Folder {
    /// Create a new folder
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description: None,
            parent_id: None,
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
            inheritance: FolderInheritanceConfig::default(),
            folders: Vec::new(),
            requests: Vec::new(),
        }
    }

    /// Add a subfolder
    pub fn add_folder(&mut self, name: String) -> Result<EntityId> {
        let mut folder = Folder::new(name);
        folder.parent_id = Some(self.id.clone());
        let id = folder.id.clone();
        self.folders.push(folder);
        self.updated_at = Utc::now();
        Ok(id)
    }

    /// Add a request to this folder
    pub fn add_request(&mut self, request: MockRequest) -> Result<EntityId> {
        let id = request.id.clone();
        self.requests.push(request);
        self.updated_at = Utc::now();
        Ok(id)
    }

    /// Find a folder by ID recursively
    pub fn find_folder(&self, id: &str) -> Option<&Folder> {
        for folder in &self.folders {
            if folder.id == id {
                return Some(folder);
            }
            if let Some(found) = folder.find_folder(id) {
                return Some(found);
            }
        }
        None
    }

    /// Find a folder by ID recursively (mutable)
    pub fn find_folder_mut(&mut self, id: &str) -> Option<&mut Folder> {
        for folder in &mut self.folders {
            if folder.id == id {
                return Some(folder);
            }
            if let Some(found) = folder.find_folder_mut(id) {
                return Some(found);
            }
        }
        None
    }

    /// Find a request by ID recursively
    pub fn find_request(&self, id: &str) -> Option<&MockRequest> {
        // Check this folder's requests
        for request in &self.requests {
            if request.id == id {
                return Some(request);
            }
        }

        // Check subfolders
        for folder in &self.folders {
            if let Some(found) = folder.find_request(id) {
                return Some(found);
            }
        }
        None
    }

    /// Get all routes from this folder and subfolders
    pub fn get_routes(&self, workspace_id: &str) -> Vec<Route> {
        let mut routes = Vec::new();

        // Add this folder's requests
        for request in &self.requests {
            routes.push(
                Route::new(request.method.clone(), request.path.clone())
                    .with_priority(request.priority)
                    .with_metadata("request_id".to_string(), serde_json::json!(request.id))
                    .with_metadata("folder_id".to_string(), serde_json::json!(self.id))
                    .with_metadata("workspace_id".to_string(), serde_json::json!(workspace_id)),
            );
        }

        // Add routes from subfolders
        for folder in &self.folders {
            routes.extend(folder.get_routes(workspace_id));
        }

        routes
    }
}

impl Workspace {
    /// Find a folder by ID recursively
    pub fn find_folder(&self, id: &str) -> Option<&Folder> {
        for folder in &self.folders {
            if folder.id == id {
                return Some(folder);
            }
            if let Some(found) = folder.find_folder(id) {
                return Some(found);
            }
        }
        None
    }

    /// Find a folder by ID recursively (mutable)
    pub fn find_folder_mut(&mut self, id: &str) -> Option<&mut Folder> {
        for folder in &mut self.folders {
            if folder.id == id {
                return Some(folder);
            }
            if let Some(found) = folder.find_folder_mut(id) {
                return Some(found);
            }
        }
        None
    }

    /// Add a request to this workspace
    pub fn add_request(&mut self, request: MockRequest) -> Result<EntityId> {
        let id = request.id.clone();
        self.requests.push(request);
        self.updated_at = Utc::now();
        Ok(id)
    }

    /// Get all routes from this workspace
    pub fn get_routes(&self) -> Vec<Route> {
        let mut routes = Vec::new();

        // Add workspace-level requests
        for request in &self.requests {
            routes.push(
                Route::new(request.method.clone(), request.path.clone())
                    .with_priority(request.priority)
                    .with_metadata("request_id".to_string(), serde_json::json!(request.id))
                    .with_metadata("workspace_id".to_string(), serde_json::json!(self.id)),
            );
        }

        // Add routes from folders
        for folder in &self.folders {
            routes.extend(folder.get_routes(&self.id));
        }

        routes
    }

    /// Get effective authentication for a request at the given path
    pub fn get_effective_auth<'a>(&'a self, folder_path: &[&'a Folder]) -> Option<&'a AuthConfig> {
        // Check folder inheritance (higher priority)
        for folder in folder_path.iter().rev() {
            if let Some(auth) = &folder.inheritance.auth {
                return Some(auth);
            }
        }

        // Fall back to workspace auth
        self.config.auth.as_ref()
    }

    /// Get merged headers for a request at the given path
    pub fn get_effective_headers(&self, folder_path: &[&Folder]) -> HashMap<String, String> {
        let mut effective_headers = HashMap::new();

        // Start with workspace headers (lowest priority)
        for (key, value) in &self.config.default_headers {
            effective_headers.insert(key.clone(), value.clone());
        }

        // Add folder headers (higher priority) in order from parent to child
        for folder in folder_path {
            for (key, value) in &folder.inheritance.headers {
                effective_headers.insert(key.clone(), value.clone());
            }
        }

        effective_headers
    }
}

impl Folder {
    /// Get the inheritance path from this folder to root
    pub fn get_inheritance_path<'a>(&'a self, workspace: &'a Workspace) -> Vec<&'a Folder> {
        let mut path = Vec::new();
        let mut current = Some(self);

        while let Some(folder) = current {
            path.push(folder);
            current =
                folder.parent_id.as_ref().and_then(|parent_id| workspace.find_folder(parent_id));
        }

        path.reverse(); // Root first
        path
    }
}

impl MockRequest {
    /// Apply inheritance to this request, returning headers and auth from the hierarchy
    pub fn apply_inheritance(
        &mut self,
        effective_headers: HashMap<String, String>,
        effective_auth: Option<&AuthConfig>,
    ) {
        // Merge headers - request headers override inherited ones
        for (key, value) in effective_headers {
            self.headers.entry(key).or_insert(value);
        }

        // For authentication - store it as a tag or custom field for use by the handler
        // This will be used by the request processing middleware
        if let Some(auth) = effective_auth {
            self.auth = Some(auth.clone());
        }
    }

    /// Create inherited request with merged headers and auth
    pub fn create_inherited_request(
        mut self,
        workspace: &Workspace,
        folder_path: &[&Folder],
    ) -> Self {
        let effective_headers = workspace.get_effective_headers(folder_path);
        let effective_auth = workspace.get_effective_auth(folder_path);

        self.apply_inheritance(effective_headers, effective_auth);
        self
    }

    /// Create a new mock request
    pub fn new(method: HttpMethod, path: String, name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description: None,
            method,
            path,
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: None,
            response: MockResponse::default(),
            response_history: Vec::new(),
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
            auth: None,
            priority: 0,
        }
    }

    /// Set the response for this request
    pub fn with_response(mut self, response: MockResponse) -> Self {
        self.response = response;
        self
    }

    /// Add a header
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    /// Add a query parameter
    pub fn with_query_param(mut self, key: String, value: String) -> Self {
        self.query_params.insert(key, value);
        self
    }

    /// Set request body
    pub fn with_body(mut self, body: String) -> Self {
        self.body = Some(body);
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }

    /// Add a response history entry
    pub fn add_response_history(&mut self, entry: ResponseHistoryEntry) {
        self.response_history.push(entry);
        // Keep only last 100 history entries to prevent unbounded growth
        if self.response_history.len() > 100 {
            self.response_history.remove(0);
        }
        // Sort by execution time (newest first)
        self.response_history.sort_by(|a, b| b.executed_at.cmp(&a.executed_at));
    }

    /// Get response history (sorted by execution time, newest first)
    pub fn get_response_history(&self) -> &[ResponseHistoryEntry] {
        &self.response_history
    }
}

impl Default for MockResponse {
    fn default() -> Self {
        Self {
            status_code: 200,
            headers: HashMap::new(),
            body: Some("{}".to_string()),
            content_type: Some("application/json".to_string()),
            delay_ms: None,
        }
    }
}

impl Environment {
    /// Create a new environment
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description: None,
            color: None,
            variables: HashMap::new(),
            created_at: now,
            updated_at: now,
            order: 0,        // Default order will be updated when added to workspace
            sharable: false, // Default to not sharable
        }
    }

    /// Create a new global environment
    pub fn new_global() -> Self {
        let mut env = Self::new("Global".to_string());
        env.description =
            Some("Global environment variables available in all contexts".to_string());
        env
    }

    /// Add or update a variable
    pub fn set_variable(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
        self.updated_at = Utc::now();
    }

    /// Remove a variable
    pub fn remove_variable(&mut self, key: &str) -> bool {
        let removed = self.variables.remove(key).is_some();
        if removed {
            self.updated_at = Utc::now();
        }
        removed
    }

    /// Get a variable value
    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }

    /// Set the environment color
    pub fn set_color(&mut self, color: EnvironmentColor) {
        self.color = Some(color);
        self.updated_at = Utc::now();
    }
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            target_directory: None,
            directory_structure: SyncDirectoryStructure::Nested,
            sync_direction: SyncDirection::Manual,
            include_metadata: true,
            realtime_monitoring: false,
            filename_pattern: "{name}".to_string(),
            exclude_pattern: None,
            force_overwrite: false,
            last_sync: None,
        }
    }
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            base_url: None,
            default_headers: HashMap::new(),
            auth: None,
            global_environment: Environment::new_global(),
            environments: Vec::new(),
            active_environment_id: None,
            mock_environments: MockEnvironmentManager::default(),
            sync: SyncConfig::default(),
            auto_encryption: AutoEncryptionConfig::default(),
            reality_level: None,
            fidelity_score: None,
            ai_mode: None,
        }
    }
}

impl WorkspaceRegistry {
    /// Create a new workspace registry
    pub fn new() -> Self {
        Self {
            workspaces: HashMap::new(),
            active_workspace: None,
        }
    }

    /// Add a workspace
    pub fn add_workspace(&mut self, mut workspace: Workspace) -> Result<EntityId> {
        let id = workspace.id.clone();

        // Set order to the end of the list if not already set
        if workspace.order == 0 && !self.workspaces.is_empty() {
            workspace.order = self.workspaces.len() as i32;
        }

        self.workspaces.insert(id.clone(), workspace);
        Ok(id)
    }

    /// Get a workspace by ID
    pub fn get_workspace(&self, id: &str) -> Option<&Workspace> {
        self.workspaces.get(id)
    }

    /// Get a workspace by ID (mutable)
    pub fn get_workspace_mut(&mut self, id: &str) -> Option<&mut Workspace> {
        self.workspaces.get_mut(id)
    }

    /// Remove a workspace
    pub fn remove_workspace(&mut self, id: &str) -> Result<()> {
        if self.workspaces.remove(id).is_some() {
            // Clear active workspace if it was removed
            if self.active_workspace.as_deref() == Some(id) {
                self.active_workspace = None;
            }
            Ok(())
        } else {
            Err(Error::generic(format!("Workspace with ID '{}' not found", id)))
        }
    }

    /// Set the active workspace
    pub fn set_active_workspace(&mut self, id: Option<String>) -> Result<()> {
        if let Some(ref workspace_id) = id {
            if !self.workspaces.contains_key(workspace_id) {
                return Err(Error::generic(format!(
                    "Workspace with ID '{}' not found",
                    workspace_id
                )));
            }
        }
        self.active_workspace = id;
        Ok(())
    }

    /// Get the active workspace
    pub fn get_active_workspace(&self) -> Option<&Workspace> {
        self.active_workspace.as_ref().and_then(|id| self.workspaces.get(id))
    }

    /// Get the active workspace ID
    pub fn get_active_workspace_id(&self) -> Option<&str> {
        self.active_workspace.as_deref()
    }

    /// Get all workspaces
    pub fn get_workspaces(&self) -> Vec<&Workspace> {
        self.workspaces.values().collect()
    }

    /// Get all workspaces sorted by order
    pub fn get_workspaces_ordered(&self) -> Vec<&Workspace> {
        let mut workspaces: Vec<&Workspace> = self.workspaces.values().collect();
        workspaces.sort_by_key(|w| w.order);
        workspaces
    }

    /// Update the order of workspaces
    pub fn update_workspaces_order(&mut self, workspace_ids: Vec<String>) -> Result<()> {
        // Validate that all provided IDs exist
        for workspace_id in &workspace_ids {
            if !self.workspaces.contains_key(workspace_id) {
                return Err(Error::generic(format!(
                    "Workspace with ID '{}' not found",
                    workspace_id
                )));
            }
        }

        // Update order for each workspace
        for (index, workspace_id) in workspace_ids.iter().enumerate() {
            if let Some(workspace) = self.workspaces.get_mut(workspace_id) {
                workspace.order = index as i32;
                workspace.updated_at = Utc::now();
            }
        }

        Ok(())
    }

    /// Get all routes from all workspaces
    pub fn get_all_routes(&self) -> Vec<Route> {
        let mut all_routes = Vec::new();
        for workspace in self.workspaces.values() {
            all_routes.extend(workspace.get_routes());
        }
        all_routes
    }

    /// Create a route registry from all workspaces
    pub fn create_route_registry(&self) -> Result<RouteRegistry> {
        let mut registry = RouteRegistry::new();
        let routes = self.get_all_routes();

        for route in routes {
            registry.add_http_route(route)?;
        }

        Ok(registry)
    }
}

impl Default for WorkspaceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ApiKeyConfig;

    #[test]
    fn test_workspace_creation() {
        let workspace = Workspace::new("Test Workspace".to_string());
        assert_eq!(workspace.name, "Test Workspace");
        assert!(!workspace.id.is_empty());
        assert!(workspace.folders.is_empty());
        assert!(workspace.requests.is_empty());
    }

    #[test]
    fn test_folder_creation() {
        let folder = Folder::new("Test Folder".to_string());
        assert_eq!(folder.name, "Test Folder");
        assert!(!folder.id.is_empty());
        assert!(folder.folders.is_empty());
        assert!(folder.requests.is_empty());
    }

    #[test]
    fn test_request_creation() {
        let request =
            MockRequest::new(HttpMethod::GET, "/test".to_string(), "Test Request".to_string());
        assert_eq!(request.name, "Test Request");
        assert_eq!(request.method, HttpMethod::GET);
        assert_eq!(request.path, "/test");
        assert_eq!(request.response.status_code, 200);
    }

    #[test]
    fn test_workspace_hierarchy() {
        let mut workspace = Workspace::new("Test Workspace".to_string());

        // Add folder
        let folder_id = workspace.add_folder("Test Folder".to_string()).unwrap();
        assert_eq!(workspace.folders.len(), 1);

        // Add request to workspace
        let request =
            MockRequest::new(HttpMethod::GET, "/test".to_string(), "Test Request".to_string());
        workspace.add_request(request).unwrap();
        assert_eq!(workspace.requests.len(), 1);

        // Add request to folder
        let folder = workspace.find_folder_mut(&folder_id).unwrap();
        let folder_request = MockRequest::new(
            HttpMethod::POST,
            "/folder-test".to_string(),
            "Folder Request".to_string(),
        );
        folder.add_request(folder_request).unwrap();
        assert_eq!(folder.requests.len(), 1);
    }

    #[test]
    fn test_workspace_registry() {
        let mut registry = WorkspaceRegistry::new();

        let workspace = Workspace::new("Test Workspace".to_string());
        let workspace_id = registry.add_workspace(workspace).unwrap();

        // Set as active
        registry.set_active_workspace(Some(workspace_id.clone())).unwrap();
        assert!(registry.get_active_workspace().is_some());

        // Get workspace
        let retrieved = registry.get_workspace(&workspace_id).unwrap();
        assert_eq!(retrieved.name, "Test Workspace");

        // Remove workspace
        registry.remove_workspace(&workspace_id).unwrap();
        assert!(registry.get_workspace(&workspace_id).is_none());
    }

    #[test]
    fn test_inheritance_header_priority() {
        let mut workspace = Workspace::new("Test Workspace".to_string());
        workspace
            .config
            .default_headers
            .insert("X-Common".to_string(), "workspace-value".to_string());
        workspace
            .config
            .default_headers
            .insert("X-Workspace-Only".to_string(), "workspace-only-value".to_string());

        // Add folder with inheritance
        let mut folder = Folder::new("Test Folder".to_string());
        folder
            .inheritance
            .headers
            .insert("X-Common".to_string(), "folder-value".to_string());
        folder
            .inheritance
            .headers
            .insert("X-Folder-Only".to_string(), "folder-only-value".to_string());

        // Test single folder
        let folder_path = vec![&folder];
        let effective_headers = workspace.get_effective_headers(&folder_path);

        assert_eq!(effective_headers.get("X-Common").unwrap(), "folder-value"); // Folder overrides workspace
        assert_eq!(effective_headers.get("X-Workspace-Only").unwrap(), "workspace-only-value"); // Workspace value preserved
        assert_eq!(effective_headers.get("X-Folder-Only").unwrap(), "folder-only-value");
        // Folder value added
    }

    #[test]
    fn test_inheritance_request_headers_override() {
        let mut workspace = Workspace::new("Test Workspace".to_string());
        workspace
            .config
            .default_headers
            .insert("Authorization".to_string(), "Bearer workspace-token".to_string());

        let folder_path = vec![];
        let effective_headers = workspace.get_effective_headers(&folder_path);
        let mut request = MockRequest::new(
            crate::routing::HttpMethod::GET,
            "/test".to_string(),
            "Test Request".to_string(),
        );

        // Request headers should override inherited ones
        request
            .headers
            .insert("Authorization".to_string(), "Bearer request-token".to_string());

        // Apply inheritance - request headers should take priority
        request.apply_inheritance(effective_headers, None);

        assert_eq!(request.headers.get("Authorization").unwrap(), "Bearer request-token");
    }

    #[test]
    fn test_inheritance_nested_folders() {
        let mut workspace = Workspace::new("Test Workspace".to_string());
        workspace
            .config
            .default_headers
            .insert("X-Level".to_string(), "workspace".to_string());

        // Parent folder
        let mut parent_folder = Folder::new("Parent Folder".to_string());
        parent_folder
            .inheritance
            .headers
            .insert("X-Level".to_string(), "parent".to_string());
        parent_folder
            .inheritance
            .headers
            .insert("X-Parent-Only".to_string(), "parent-value".to_string());

        // Child folder
        let mut child_folder = Folder::new("Child Folder".to_string());
        child_folder
            .inheritance
            .headers
            .insert("X-Level".to_string(), "child".to_string());
        child_folder
            .inheritance
            .headers
            .insert("X-Child-Only".to_string(), "child-value".to_string());

        // Parent-to-child hierarchy
        let folder_path = vec![&parent_folder, &child_folder];
        let effective_headers = workspace.get_effective_headers(&folder_path);

        // Child should override parent which overrides workspace
        assert_eq!(effective_headers.get("X-Level").unwrap(), "child");
        assert_eq!(effective_headers.get("X-Parent-Only").unwrap(), "parent-value");
        assert_eq!(effective_headers.get("X-Child-Only").unwrap(), "child-value");
    }

    #[test]
    fn test_inheritance_auth_from_folder() {
        // Create workspace without auth
        let workspace = Workspace::new("Test Workspace".to_string());

        // Create folder with auth
        let mut folder = Folder::new("Test Folder".to_string());
        let auth = AuthConfig {
            require_auth: true,
            api_key: Some(ApiKeyConfig {
                header_name: "X-API-Key".to_string(),
                query_name: Some("api_key".to_string()),
                keys: vec!["folder-key".to_string()],
            }),
            ..Default::default()
        };
        folder.inheritance.auth = Some(auth);

        let folder_path = vec![&folder];
        let effective_auth = workspace.get_effective_auth(&folder_path);

        assert!(effective_auth.is_some());
        let auth_config = effective_auth.unwrap();
        assert!(auth_config.require_auth);
        let api_key_config = auth_config.api_key.as_ref().unwrap();
        assert_eq!(api_key_config.keys, vec!["folder-key".to_string()]);
    }

    #[test]
    fn test_folder_inheritance_config_default() {
        let config = FolderInheritanceConfig::default();
        assert!(config.headers.is_empty());
        assert!(config.auth.is_none());
    }

    #[test]
    fn test_mock_response_default() {
        let response = MockResponse::default();
        assert_eq!(response.status_code, 200);
        assert!(response.headers.is_empty());
    }

    #[test]
    fn test_mock_response_serialization() {
        let mut response = MockResponse::default();
        response.status_code = 404;
        response.body = Some("Not Found".to_string());
        response.headers.insert("Content-Type".to_string(), "application/json".to_string());
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("404"));
    }

    #[test]
    fn test_response_history_entry_creation() {
        let entry = ResponseHistoryEntry {
            id: "exec-123".to_string(),
            executed_at: Utc::now(),
            request_method: HttpMethod::GET,
            request_path: "/api/test".to_string(),
            request_headers: HashMap::new(),
            request_body: None,
            response_status_code: 200,
            response_headers: HashMap::new(),
            response_body: Some("{}".to_string()),
            response_time_ms: 150,
            response_size_bytes: 2,
            error_message: None,
        };

        assert_eq!(entry.response_status_code, 200);
        assert_eq!(entry.response_time_ms, 150);
        assert_eq!(entry.id, "exec-123");
    }

    #[test]
    fn test_environment_color_creation() {
        let color = EnvironmentColor {
            hex: "#FF8040".to_string(),
            name: Some("Orange".to_string()),
        };
        assert_eq!(color.hex, "#FF8040");
        assert_eq!(color.name, Some("Orange".to_string()));
    }

    #[test]
    fn test_environment_color_serialization() {
        let color = EnvironmentColor {
            hex: "#FF0000".to_string(),
            name: None,
        };
        let json = serde_json::to_string(&color).unwrap();
        assert!(json.contains("#FF0000"));
    }

    #[test]
    fn test_sync_config_default() {
        // Use the SyncConfig from workspace.rs (not from sync module)
        // This is the one used in WorkspaceConfig
        let config = super::SyncConfig::default();
        assert!(!config.enabled);
        // Just verify it can be created
        let _ = config;
    }

    #[test]
    fn test_sync_directory_structure_serialization() {
        let structures = vec![
            SyncDirectoryStructure::Flat,
            SyncDirectoryStructure::Nested,
            SyncDirectoryStructure::Grouped,
        ];

        for structure in structures {
            let json = serde_json::to_string(&structure).unwrap();
            assert!(!json.is_empty());
            // Just verify it can be deserialized
            let _deserialized: SyncDirectoryStructure = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_sync_direction_serialization() {
        let directions = vec![
            SyncDirection::Manual,
            SyncDirection::WorkspaceToDirectory,
            SyncDirection::Bidirectional,
        ];

        for direction in directions {
            let json = serde_json::to_string(&direction).unwrap();
            assert!(!json.is_empty());
            // Just verify it can be deserialized
            let _deserialized: SyncDirection = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_workspace_config_default() {
        let config = WorkspaceConfig::default();
        assert!(config.base_url.is_none());
        assert!(config.default_headers.is_empty());
        assert!(config.auth.is_none());
        assert!(config.environments.is_empty());
        assert!(config.active_environment_id.is_none());
    }

    #[test]
    fn test_workspace_registry_new() {
        let registry = WorkspaceRegistry::new();
        assert!(registry.get_workspaces().is_empty());
        assert!(registry.get_active_workspace().is_none());
    }

    #[test]
    fn test_workspace_registry_get_active_workspace_id() {
        let mut registry = WorkspaceRegistry::new();
        let workspace = Workspace::new("Test".to_string());
        let id = registry.add_workspace(workspace).unwrap();
        registry.set_active_workspace(Some(id.clone())).unwrap();
        
        assert_eq!(registry.get_active_workspace_id(), Some(id.as_str()));
    }

    #[test]
    fn test_workspace_registry_get_workspaces_ordered() {
        let mut registry = WorkspaceRegistry::new();
        let mut ws1 = Workspace::new("First".to_string());
        ws1.order = 2;
        let mut ws2 = Workspace::new("Second".to_string());
        ws2.order = 1;
        
        registry.add_workspace(ws1).unwrap();
        registry.add_workspace(ws2).unwrap();
        
        let ordered = registry.get_workspaces_ordered();
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].name, "Second"); // Lower order first
        assert_eq!(ordered[1].name, "First");
    }

    #[test]
    fn test_workspace_registry_update_workspaces_order() {
        let mut registry = WorkspaceRegistry::new();
        let id1 = registry.add_workspace(Workspace::new("First".to_string())).unwrap();
        let id2 = registry.add_workspace(Workspace::new("Second".to_string())).unwrap();
        
        registry.update_workspaces_order(vec![id2.clone(), id1.clone()]).unwrap();
        
        let ordered = registry.get_workspaces_ordered();
        assert_eq!(ordered[0].id, id2);
        assert_eq!(ordered[1].id, id1);
    }

    #[test]
    fn test_workspace_registry_update_workspaces_order_invalid_id() {
        let mut registry = WorkspaceRegistry::new();
        let id1 = registry.add_workspace(Workspace::new("First".to_string())).unwrap();
        
        let result = registry.update_workspaces_order(vec![id1, "invalid-id".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_workspace_registry_set_active_workspace_invalid() {
        let mut registry = WorkspaceRegistry::new();
        let result = registry.set_active_workspace(Some("invalid-id".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_workspace_registry_remove_active_workspace() {
        let mut registry = WorkspaceRegistry::new();
        let id = registry.add_workspace(Workspace::new("Test".to_string())).unwrap();
        registry.set_active_workspace(Some(id.clone())).unwrap();
        registry.remove_workspace(&id).unwrap();
        
        assert!(registry.get_active_workspace().is_none());
    }

    #[test]
    fn test_workspace_clone() {
        let workspace1 = Workspace::new("Test Workspace".to_string());
        let workspace2 = workspace1.clone();
        assert_eq!(workspace1.name, workspace2.name);
        assert_eq!(workspace1.id, workspace2.id);
    }

    #[test]
    fn test_workspace_debug() {
        let workspace = Workspace::new("Debug Test".to_string());
        let debug_str = format!("{:?}", workspace);
        assert!(debug_str.contains("Workspace"));
    }

    #[test]
    fn test_workspace_serialization() {
        let workspace = Workspace::new("Serialization Test".to_string());
        let json = serde_json::to_string(&workspace).unwrap();
        assert!(json.contains("Serialization Test"));
    }

    #[test]
    fn test_folder_clone() {
        let folder1 = Folder::new("Test Folder".to_string());
        let folder2 = folder1.clone();
        assert_eq!(folder1.name, folder2.name);
        assert_eq!(folder1.id, folder2.id);
    }

    #[test]
    fn test_folder_debug() {
        let folder = Folder::new("Debug Folder".to_string());
        let debug_str = format!("{:?}", folder);
        assert!(debug_str.contains("Folder"));
    }

    #[test]
    fn test_folder_serialization() {
        let folder = Folder::new("Serialization Folder".to_string());
        let json = serde_json::to_string(&folder).unwrap();
        assert!(json.contains("Serialization Folder"));
    }

    #[test]
    fn test_folder_inheritance_config_clone() {
        let mut config1 = FolderInheritanceConfig::default();
        config1.headers.insert("X-Test".to_string(), "value".to_string());
        let config2 = config1.clone();
        assert_eq!(config1.headers, config2.headers);
    }

    #[test]
    fn test_folder_inheritance_config_debug() {
        let config = FolderInheritanceConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("FolderInheritanceConfig"));
    }

    #[test]
    fn test_workspace_config_clone() {
        let mut config1 = WorkspaceConfig::default();
        config1.base_url = Some("https://api.example.com".to_string());
        let config2 = config1.clone();
        assert_eq!(config1.base_url, config2.base_url);
    }

    #[test]
    fn test_workspace_config_debug() {
        let config = WorkspaceConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("WorkspaceConfig"));
    }

    #[test]
    fn test_workspace_registry_clone() {
        let mut registry1 = WorkspaceRegistry::new();
        let id = registry1.add_workspace(Workspace::new("Test".to_string())).unwrap();
        let registry2 = registry1.clone();
        assert!(registry2.get_workspace(&id).is_some());
    }

    #[test]
    fn test_workspace_registry_debug() {
        let registry = WorkspaceRegistry::new();
        let debug_str = format!("{:?}", registry);
        assert!(debug_str.contains("WorkspaceRegistry"));
    }
}
