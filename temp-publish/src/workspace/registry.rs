//! Workspace registry and management
//!
//! This module provides the WorkspaceRegistry for managing multiple workspaces,
//! including loading, saving, and organizing workspaces.

use crate::routing::RouteRegistry;
use crate::workspace::core::{EntityId, Environment, Folder, MockRequest, Workspace};
use crate::workspace::request::RequestProcessor;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Workspace registry for managing multiple workspaces
#[derive(Debug, Clone)]
pub struct WorkspaceRegistry {
    /// All workspaces indexed by ID
    workspaces: HashMap<EntityId, Workspace>,
    /// Active workspace ID
    active_workspace_id: Option<EntityId>,
    /// Route registry for all workspace requests
    route_registry: Arc<RwLock<RouteRegistry>>,
    /// Environment registry
    environments: HashMap<EntityId, Environment>,
    /// Request processor for converting requests to routes
    request_processor: RequestProcessor,
}

/// Configuration for workspace registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRegistryConfig {
    /// Maximum number of workspaces allowed
    pub max_workspaces: Option<usize>,
    /// Default workspace name
    pub default_workspace_name: String,
    /// Auto-save interval in seconds
    pub auto_save_interval_seconds: u64,
}

impl Default for WorkspaceRegistryConfig {
    fn default() -> Self {
        Self {
            max_workspaces: None,
            default_workspace_name: "Default Workspace".to_string(),
            auto_save_interval_seconds: 300, // 5 minutes
        }
    }
}

/// Workspace statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStats {
    /// Total number of workspaces
    pub total_workspaces: usize,
    /// Total number of folders across all workspaces
    pub total_folders: usize,
    /// Total number of requests across all workspaces
    pub total_requests: usize,
    /// Total number of responses across all workspaces
    pub total_responses: usize,
    /// Total number of environments
    pub total_environments: usize,
    /// Last modification timestamp
    pub last_modified: DateTime<Utc>,
}

impl WorkspaceRegistry {
    /// Create a new empty workspace registry
    pub fn new() -> Self {
        Self {
            workspaces: HashMap::new(),
            active_workspace_id: None,
            route_registry: Arc::new(RwLock::new(RouteRegistry::new())),
            environments: HashMap::new(),
            request_processor: RequestProcessor::new(),
        }
    }

    /// Create a new workspace registry with configuration
    pub fn with_config(config: WorkspaceRegistryConfig) -> Self {
        let mut registry = Self::new();

        // Create default workspace
        let default_workspace = Workspace::new(config.default_workspace_name);
        let _ = registry.add_workspace(default_workspace);

        registry
    }

    /// Add a workspace to the registry
    pub fn add_workspace(&mut self, workspace: Workspace) -> Result<EntityId, String> {
        // Check max workspaces limit
        if let Some(max) = self.get_config().max_workspaces {
            if self.workspaces.len() >= max {
                return Err(format!("Maximum number of workspaces ({}) exceeded", max));
            }
        }

        let id = workspace.id.clone();
        self.workspaces.insert(id.clone(), workspace);

        // Update route registry
        self.update_route_registry();

        Ok(id)
    }

    /// Get a workspace by ID
    pub fn get_workspace(&self, id: &EntityId) -> Option<&Workspace> {
        self.workspaces.get(id)
    }

    /// Get a mutable workspace by ID
    pub fn get_workspace_mut(&mut self, id: &EntityId) -> Option<&mut Workspace> {
        self.workspaces.get_mut(id)
    }

    /// Remove a workspace from the registry
    pub fn remove_workspace(&mut self, id: &EntityId) -> Result<Workspace, String> {
        if let Some(workspace) = self.workspaces.remove(id) {
            // Update active workspace if necessary
            if self.active_workspace_id.as_ref() == Some(id) {
                self.active_workspace_id = self.workspaces.keys().next().cloned();
            }

            // Update route registry
            self.update_route_registry();

            Ok(workspace)
        } else {
            Err(format!("Workspace with ID {} not found", id))
        }
    }

    /// Get all workspaces
    pub fn get_all_workspaces(&self) -> Vec<&Workspace> {
        self.workspaces.values().collect()
    }

    /// Get all workspaces mutably
    pub fn get_all_workspaces_mut(&mut self) -> Vec<&mut Workspace> {
        self.workspaces.values_mut().collect()
    }

    /// Set the active workspace
    pub fn set_active_workspace(&mut self, id: EntityId) -> Result<(), String> {
        if self.workspaces.contains_key(&id) {
            self.active_workspace_id = Some(id);
            Ok(())
        } else {
            Err(format!("Workspace with ID {} not found", id))
        }
    }

    /// Get the active workspace
    pub fn get_active_workspace(&self) -> Option<&Workspace> {
        self.active_workspace_id.as_ref().and_then(|id| self.workspaces.get(id))
    }

    /// Get the active workspace mutably
    pub fn get_active_workspace_mut(&mut self) -> Option<&mut Workspace> {
        self.active_workspace_id.as_ref().and_then(|id| self.workspaces.get_mut(id))
    }

    /// Add an environment to the registry
    pub fn add_environment(&mut self, environment: Environment) -> EntityId {
        let id = environment.id.clone();
        self.environments.insert(id.clone(), environment);
        id
    }

    /// Get an environment by ID
    pub fn get_environment(&self, id: &EntityId) -> Option<&Environment> {
        self.environments.get(id)
    }

    /// Get the active environment
    pub fn get_active_environment(&self) -> Option<&Environment> {
        self.environments.values().find(|env| env.active)
    }

    /// Set the active environment
    pub fn set_active_environment(&mut self, id: EntityId) -> Result<(), String> {
        if self.environments.contains_key(&id) {
            // Deactivate all environments and activate the selected one
            for (env_id, env) in self.environments.iter_mut() {
                env.active = *env_id == id;
            }
            Ok(())
        } else {
            Err(format!("Environment with ID {} not found", id))
        }
    }

    /// Get workspace statistics
    pub fn get_stats(&self) -> WorkspaceStats {
        let total_folders = self.workspaces.values().map(|w| w.folders.len()).sum::<usize>();

        let total_requests = self.workspaces.values().map(|w| w.requests.len()).sum::<usize>();

        let total_responses = self
            .workspaces
            .values()
            .map(|w| w.requests.iter().map(|r| r.responses.len()).sum::<usize>())
            .sum::<usize>();

        WorkspaceStats {
            total_workspaces: self.workspaces.len(),
            total_folders,
            total_requests,
            total_responses,
            total_environments: self.environments.len(),
            last_modified: Utc::now(),
        }
    }

    /// Update the route registry with all workspace requests
    fn update_route_registry(&mut self) {
        if let Ok(mut route_registry) = self.route_registry.write() {
            route_registry.clear();

            for workspace in self.workspaces.values() {
                // Add root requests
                for request in &workspace.requests {
                    if request.enabled {
                        if let Some(_response) = request.active_response() {
                            if let Ok(route) =
                                self.request_processor.create_route_from_request(request)
                            {
                                let _ = route_registry.add_route(route);
                            }
                        }
                    }
                }

                // Add folder requests recursively
                self.add_folder_requests_to_registry(&mut route_registry, &workspace.folders);
            }
        }
    }

    /// Recursively add folder requests to the route registry
    fn add_folder_requests_to_registry(
        &self,
        route_registry: &mut RouteRegistry,
        folders: &[Folder],
    ) {
        for folder in folders {
            // Add folder requests
            for request in &folder.requests {
                if request.enabled {
                    if let Some(_response) = request.active_response() {
                        if let Ok(route) = self.request_processor.create_route_from_request(request)
                        {
                            let _ = route_registry.add_route(route);
                        }
                    }
                }
            }

            // Add subfolder requests
            self.add_folder_requests_to_registry(route_registry, &folder.folders);
        }
    }

    /// Get the route registry
    pub fn get_route_registry(&self) -> &Arc<RwLock<RouteRegistry>> {
        &self.route_registry
    }

    /// Get the configuration (placeholder implementation)
    pub fn get_config(&self) -> WorkspaceRegistryConfig {
        WorkspaceRegistryConfig::default()
    }

    /// Find a request by ID across all workspaces
    pub fn find_request(&self, request_id: &EntityId) -> Option<&MockRequest> {
        for workspace in self.workspaces.values() {
            // Check root requests
            if let Some(request) = workspace.requests.iter().find(|r| &r.id == request_id) {
                return Some(request);
            }

            // Check folder requests
            if let Some(request) = self.find_request_in_folder(&workspace.folders, request_id) {
                return Some(request);
            }
        }

        None
    }

    /// Find a request in a folder hierarchy
    #[allow(clippy::only_used_in_recursion)]
    fn find_request_in_folder<'a>(
        &self,
        folders: &'a [Folder],
        request_id: &EntityId,
    ) -> Option<&'a MockRequest> {
        for folder in folders {
            // Check folder requests
            if let Some(request) = folder.requests.iter().find(|r| &r.id == request_id) {
                return Some(request);
            }

            // Check subfolders
            if let Some(request) = self.find_request_in_folder(&folder.folders, request_id) {
                return Some(request);
            }
        }

        None
    }

    /// Find a folder by ID across all workspaces
    pub fn find_folder(&self, folder_id: &EntityId) -> Option<&Folder> {
        for workspace in self.workspaces.values() {
            if let Some(folder) = self.find_folder_in_workspace(&workspace.folders, folder_id) {
                return Some(folder);
            }
        }

        None
    }

    /// Find a folder in a workspace hierarchy
    #[allow(clippy::only_used_in_recursion)]
    fn find_folder_in_workspace<'a>(
        &self,
        folders: &'a [Folder],
        folder_id: &EntityId,
    ) -> Option<&'a Folder> {
        for folder in folders {
            if &folder.id == folder_id {
                return Some(folder);
            }

            if let Some(found) = self.find_folder_in_workspace(&folder.folders, folder_id) {
                return Some(found);
            }
        }

        None
    }

    /// Export workspace to JSON
    pub fn export_workspace(&self, workspace_id: &EntityId) -> Result<String, String> {
        if let Some(workspace) = self.workspaces.get(workspace_id) {
            serde_json::to_string_pretty(workspace)
                .map_err(|e| format!("Failed to serialize workspace: {}", e))
        } else {
            Err(format!("Workspace with ID {} not found", workspace_id))
        }
    }

    /// Import workspace from JSON
    pub fn import_workspace(&mut self, json_data: &str) -> Result<EntityId, String> {
        let workspace: Workspace = serde_json::from_str(json_data)
            .map_err(|e| format!("Failed to deserialize workspace: {}", e))?;

        self.add_workspace(workspace)
    }

    /// Search for requests across all workspaces
    pub fn search_requests(&self, query: &str) -> Vec<&MockRequest> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for workspace in self.workspaces.values() {
            // Search root requests
            for request in &workspace.requests {
                if request.name.to_lowercase().contains(&query_lower)
                    || request.url.to_lowercase().contains(&query_lower)
                    || request
                        .description
                        .as_ref()
                        .map(|d| d.to_lowercase())
                        .unwrap_or_default()
                        .contains(&query_lower)
                {
                    results.push(request);
                }
            }

            // Search folder requests
            self.search_requests_in_folders(&workspace.folders, &query_lower, &mut results);
        }

        results
    }

    /// Search for requests in folder hierarchy
    #[allow(clippy::only_used_in_recursion)]
    fn search_requests_in_folders<'a>(
        &self,
        folders: &'a [Folder],
        query: &str,
        results: &mut Vec<&'a MockRequest>,
    ) {
        for folder in folders {
            // Search folder requests
            for request in &folder.requests {
                if request.name.to_lowercase().contains(query)
                    || request.url.to_lowercase().contains(query)
                    || request
                        .description
                        .as_ref()
                        .map(|d| d.to_lowercase())
                        .unwrap_or_default()
                        .contains(query)
                {
                    results.push(request);
                }
            }

            // Search subfolders
            self.search_requests_in_folders(&folder.folders, query, results);
        }
    }

    /// Get requests by tag
    pub fn get_requests_by_tag(&self, tag: &str) -> Vec<&MockRequest> {
        let mut results = Vec::new();

        for workspace in self.workspaces.values() {
            // Check root requests
            for request in &workspace.requests {
                if request.tags.contains(&tag.to_string()) {
                    results.push(request);
                }
            }

            // Check folder requests
            self.get_requests_by_tag_in_folders(&workspace.folders, tag, &mut results);
        }

        results
    }

    /// Get requests by tag in folder hierarchy
    #[allow(clippy::only_used_in_recursion)]
    fn get_requests_by_tag_in_folders<'a>(
        &self,
        folders: &'a [Folder],
        tag: &str,
        results: &mut Vec<&'a MockRequest>,
    ) {
        for folder in folders {
            // Check folder requests
            for request in &folder.requests {
                if request.tags.contains(&tag.to_string()) {
                    results.push(request);
                }
            }

            // Check subfolders
            self.get_requests_by_tag_in_folders(&folder.folders, tag, results);
        }
    }
}

impl Default for WorkspaceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
