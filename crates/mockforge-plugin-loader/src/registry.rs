//! Plugin registry for managing loaded plugins
//!
//! This module provides the plugin registry that tracks loaded plugins,
//! manages their lifecycle, and provides access to plugin instances.

use super::*;
use mockforge_plugin_core::{PluginHealth, PluginId, PluginInstance, PluginVersion};
use std::collections::HashMap;
use std::collections::HashSet;

/// Plugin registry for managing loaded plugins
pub struct PluginRegistry {
    /// Registered plugins
    plugins: HashMap<PluginId, PluginInstance>,
    /// Plugin load order (for dependency resolution)
    load_order: Vec<PluginId>,
    /// Registry statistics
    stats: RegistryStats,
}

// Implement Send + Sync for PluginRegistry
unsafe impl Send for PluginRegistry {}
unsafe impl Sync for PluginRegistry {}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            load_order: Vec::new(),
            stats: RegistryStats::default(),
        }
    }

    /// Add a plugin to the registry
    pub fn add_plugin(&mut self, plugin: PluginInstance) -> LoaderResult<()> {
        let plugin_id = plugin.id.clone();

        // Check if plugin already exists
        if self.plugins.contains_key(&plugin_id) {
            return Err(PluginLoaderError::already_loaded(plugin_id));
        }

        // Validate plugin dependencies
        self.validate_dependencies(&plugin)?;

        // Add plugin
        self.plugins.insert(plugin_id.clone(), plugin);
        self.load_order.push(plugin_id);

        // Update statistics
        self.stats.total_plugins += 1;
        self.stats.last_updated = chrono::Utc::now();

        Ok(())
    }

    /// Remove a plugin from the registry
    pub fn remove_plugin(&mut self, plugin_id: &PluginId) -> LoaderResult<PluginInstance> {
        // Check if plugin exists
        if !self.plugins.contains_key(plugin_id) {
            return Err(PluginLoaderError::not_found(plugin_id.clone()));
        }

        // Check if other plugins depend on this one
        self.check_reverse_dependencies(plugin_id)?;

        // Remove from load order
        self.load_order.retain(|id| id != plugin_id);

        // Remove plugin
        let plugin = self.plugins.remove(plugin_id).unwrap();

        // Update statistics
        self.stats.total_plugins -= 1;
        self.stats.last_updated = chrono::Utc::now();

        Ok(plugin)
    }

    /// Get a plugin by ID
    pub fn get_plugin(&self, plugin_id: &PluginId) -> Option<&PluginInstance> {
        self.plugins.get(plugin_id)
    }

    /// Get a mutable reference to a plugin
    pub fn get_plugin_mut(&mut self, plugin_id: &PluginId) -> Option<&mut PluginInstance> {
        self.plugins.get_mut(plugin_id)
    }

    /// Check if a plugin is registered
    pub fn has_plugin(&self, plugin_id: &PluginId) -> bool {
        self.plugins.contains_key(plugin_id)
    }

    /// List all registered plugins
    pub fn list_plugins(&self) -> Vec<PluginId> {
        self.plugins.keys().cloned().collect()
    }

    /// Get plugin health status
    pub fn get_plugin_health(&self, plugin_id: &PluginId) -> LoaderResult<PluginHealth> {
        let plugin = self
            .get_plugin(plugin_id)
            .ok_or_else(|| PluginLoaderError::not_found(plugin_id.clone()))?;

        Ok(plugin.health.clone())
    }

    /// Get registry statistics
    pub fn get_stats(&self) -> &RegistryStats {
        &self.stats
    }

    /// Check if a version requirement is compatible with a plugin version
    pub fn is_version_compatible(&self, requirement: &str, version: &PluginVersion) -> bool {
        // Simple version compatibility check
        // For now, just check exact match or caret ranges
        if requirement.starts_with('^') {
            // Caret range: ^1.0.0 matches 1.x.x
            let req_version = requirement.strip_prefix('^').unwrap();
            let req_parts: Vec<&str> = req_version.split('.').collect();
            let ver_parts: Vec<u32> = vec![version.major, version.minor, version.patch];

            if req_parts.len() >= 1 && req_parts[0].parse::<u32>().unwrap_or(0) == ver_parts[0] {
                return true;
            }
        } else {
            // Exact match
            return requirement == &version.to_string();
        }
        false
    }

    /// Find plugins by capability
    pub fn find_plugins_by_capability(&self, capability: &str) -> Vec<&PluginInstance> {
        self.plugins
            .values()
            .filter(|plugin| plugin.manifest.capabilities.contains(&capability.to_string()))
            .collect()
    }

    /// Get plugins in dependency order
    pub fn get_plugins_in_dependency_order(&self) -> Vec<&PluginInstance> {
        self.load_order.iter().filter_map(|id| self.plugins.get(id)).collect()
    }

    /// Validate plugin dependencies
    fn validate_dependencies(&self, plugin: &PluginInstance) -> LoaderResult<()> {
        for (dep_id, _dep_version) in &plugin.manifest.dependencies {
            // Check if dependency is loaded
            if !self.has_plugin(dep_id) {
                return Err(PluginLoaderError::dependency(format!(
                    "Required dependency {} not found",
                    dep_id.0
                )));
            }

            // Check version compatibility (simplified for now)
            if let Some(_loaded_plugin) = self.get_plugin(dep_id) {
                // For now, just check that the loaded plugin exists
                // Version compatibility checking can be added later
            }
        }

        Ok(())
    }

    /// Check reverse dependencies (plugins that depend on the one being removed)
    fn check_reverse_dependencies(&self, plugin_id: &PluginId) -> LoaderResult<()> {
        for (id, plugin) in &self.plugins {
            if id == plugin_id {
                continue; // Skip the plugin being removed
            }

            if plugin.manifest.dependencies.contains_key(plugin_id) {
                return Err(PluginLoaderError::dependency(format!(
                    "Cannot remove plugin {}: required by plugin {}",
                    plugin_id.0, id.0
                )));
            }
        }

        Ok(())
    }

    /// Get plugin dependency graph
    pub fn get_dependency_graph(&self) -> HashMap<PluginId, Vec<PluginId>> {
        let mut graph = HashMap::new();

        for (plugin_id, plugin) in &self.plugins {
            let mut deps = Vec::new();
            for dep_id in plugin.manifest.dependencies.keys() {
                if self.has_plugin(dep_id) {
                    deps.push(dep_id.clone());
                }
            }
            graph.insert(plugin_id.clone(), deps);
        }

        graph
    }

    /// Perform topological sort for plugin initialization
    pub fn get_initialization_order(&self) -> LoaderResult<Vec<PluginId>> {
        let graph = self.get_dependency_graph();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();
        let mut order = Vec::new();

        fn visit(
            plugin_id: &PluginId,
            graph: &HashMap<PluginId, Vec<PluginId>>,
            visited: &mut HashSet<PluginId>,
            visiting: &mut HashSet<PluginId>,
            order: &mut Vec<PluginId>,
        ) -> LoaderResult<()> {
            if visited.contains(plugin_id) {
                return Ok(());
            }

            if visiting.contains(plugin_id) {
                return Err(PluginLoaderError::dependency(format!(
                    "Circular dependency detected involving plugin {}",
                    plugin_id
                )));
            }

            visiting.insert(plugin_id.clone());

            if let Some(deps) = graph.get(plugin_id) {
                for dep in deps {
                    visit(dep, graph, visited, visiting, order)?;
                }
            }

            visiting.remove(plugin_id);
            visited.insert(plugin_id.clone());
            order.push(plugin_id.clone());

            Ok(())
        }

        for plugin_id in self.plugins.keys() {
            if !visited.contains(plugin_id) {
                visit(plugin_id, &graph, &mut visited, &mut visiting, &mut order)?;
            }
        }

        Ok(order)
    }

    /// Clear all plugins from registry
    pub fn clear(&mut self) {
        self.plugins.clear();
        self.load_order.clear();
        self.stats = RegistryStats::default();
    }

    /// Get registry health status
    pub fn health_status(&self) -> RegistryHealth {
        let mut healthy_plugins = 0;
        let mut unhealthy_plugins = 0;

        for plugin in self.plugins.values() {
            if plugin.health.healthy {
                healthy_plugins += 1;
            } else {
                unhealthy_plugins += 1;
            }
        }

        RegistryHealth {
            total_plugins: self.plugins.len(),
            healthy_plugins,
            unhealthy_plugins,
            last_updated: self.stats.last_updated,
        }
    }
}

/// Registry statistics
#[derive(Debug, Clone, Default)]
pub struct RegistryStats {
    /// Total number of registered plugins
    pub total_plugins: usize,
    /// Last update timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
    /// Total plugin loads
    pub total_loads: u64,
    /// Total plugin unloads
    pub total_unloads: u64,
}

/// Registry health status
#[derive(Debug, Clone)]
pub struct RegistryHealth {
    /// Total plugins
    pub total_plugins: usize,
    /// Healthy plugins
    pub healthy_plugins: usize,
    /// Unhealthy plugins
    pub unhealthy_plugins: usize,
    /// Last updated
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl RegistryHealth {
    /// Check if registry is healthy
    pub fn is_healthy(&self) -> bool {
        self.unhealthy_plugins == 0
    }

    /// Get health percentage
    pub fn health_percentage(&self) -> f64 {
        if self.total_plugins == 0 {
            100.0
        } else {
            (self.healthy_plugins as f64 / self.total_plugins as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_plugin_core::{PluginMetrics, PluginState};

    #[test]
    fn test_registry_creation() {
        let registry = PluginRegistry::new();
        assert_eq!(registry.list_plugins().len(), 0);
        assert_eq!(registry.get_stats().total_plugins, 0);
    }

    #[test]
    fn test_registry_health() {
        let health = RegistryHealth {
            total_plugins: 10,
            healthy_plugins: 8,
            unhealthy_plugins: 2,
            last_updated: chrono::Utc::now(),
        };

        assert!(!health.is_healthy());
        assert_eq!(health.health_percentage(), 80.0);
    }

    #[test]
    fn test_empty_registry_health() {
        let health = RegistryHealth {
            total_plugins: 0,
            healthy_plugins: 0,
            unhealthy_plugins: 0,
            last_updated: chrono::Utc::now(),
        };

        assert!(health.is_healthy());
        assert_eq!(health.health_percentage(), 100.0);
    }

    #[test]
    fn test_version_compatibility() {
        let registry = PluginRegistry::new();

        // Test exact version match
        let v1 = PluginVersion::new(1, 0, 0);
        assert!(registry.is_version_compatible("1.0.0", &v1));

        // Test caret range (simplified)
        assert!(registry.is_version_compatible("^1.0.0", &v1));

        // Test non-match
        assert!(!registry.is_version_compatible("2.0.0", &v1));
    }

    #[tokio::test]
    async fn test_registry_operations() {
        let mut registry = PluginRegistry::new();

        // Create a test plugin
        let plugin_id = PluginId::new("test-plugin");
        let plugin_info = PluginInfo::new(
            plugin_id.clone(),
            PluginVersion::new(1, 0, 0),
            "Test Plugin",
            "A test plugin",
            PluginAuthor::new("Test Author"),
        );
        let manifest = PluginManifest::new(plugin_info);

        let plugin = PluginInstance {
            id: plugin_id.clone(),
            manifest,
            state: PluginState::Ready,
            health: PluginHealth::healthy("Test plugin".to_string(), PluginMetrics::default()),
        };

        // Test adding plugin
        registry.add_plugin(plugin).unwrap();
        assert_eq!(registry.list_plugins().len(), 1);
        assert!(registry.has_plugin(&plugin_id));

        // Test getting plugin
        assert!(registry.get_plugin(&plugin_id).is_some());

        // Test removing plugin
        let removed = registry.remove_plugin(&plugin_id).unwrap();
        assert_eq!(removed.id, plugin_id);
        assert_eq!(registry.list_plugins().len(), 0);
        assert!(!registry.has_plugin(&plugin_id));
    }

    #[tokio::test]
    async fn test_duplicate_plugin() {
        let mut registry = PluginRegistry::new();

        let plugin_id = PluginId::new("test-plugin");
        let plugin_info = PluginInfo::new(
            plugin_id.clone(),
            PluginVersion::new(1, 0, 0),
            "Test Plugin",
            "A test plugin",
            PluginAuthor::new("Test Author"),
        );
        let manifest = PluginManifest::new(plugin_info.clone());

        let plugin1 = PluginInstance {
            id: plugin_id.clone(),
            manifest: manifest.clone(),
            state: PluginState::Ready,
            health: PluginHealth::healthy("Test plugin".to_string(), PluginMetrics::default()),
        };

        let plugin2 = PluginInstance {
            id: plugin_id.clone(),
            manifest,
            state: PluginState::Ready,
            health: PluginHealth::healthy("Test plugin".to_string(), PluginMetrics::default()),
        };

        // Add first plugin
        registry.add_plugin(plugin1).unwrap();

        // Try to add duplicate
        let result = registry.add_plugin(plugin2);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PluginLoaderError::AlreadyLoaded { .. }));
    }
}
