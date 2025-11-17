//! Consumer mapping for tracking endpoint → SDK method → consuming app relationships
//!
//! This module provides functionality for mapping API endpoints to SDK methods and
//! tracking which applications consume those methods. This enables consumer-focused
//! drift insights that show which apps will be affected by contract changes.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Type of consuming application
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AppType {
    /// Web application
    Web,
    /// Mobile application (iOS)
    #[serde(rename = "mobile_ios")]
    MobileIos,
    /// Mobile application (Android)
    #[serde(rename = "mobile_android")]
    MobileAndroid,
    /// Internal tool or service
    InternalTool,
    /// CLI tool
    Cli,
    /// Other/unknown
    Other,
}

impl std::fmt::Display for AppType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppType::Web => write!(f, "Web App"),
            AppType::MobileIos => write!(f, "Mobile App (iOS)"),
            AppType::MobileAndroid => write!(f, "Mobile App (Android)"),
            AppType::InternalTool => write!(f, "Internal Tool"),
            AppType::Cli => write!(f, "CLI Tool"),
            AppType::Other => write!(f, "Other"),
        }
    }
}

/// A consuming application that uses SDK methods
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ConsumingApp {
    /// Unique identifier for the app
    pub app_id: String,
    /// Human-readable name
    pub app_name: String,
    /// Type of application
    pub app_type: AppType,
    /// Optional repository URL
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository_url: Option<String>,
    /// Timestamp when this app was last updated
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<i64>,
    /// Optional description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// An SDK method that calls an endpoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SDKMethod {
    /// SDK name (e.g., "typescript-sdk", "python-sdk")
    pub sdk_name: String,
    /// Method name (e.g., "getUser", "createOrder")
    pub method_name: String,
    /// List of consuming apps that use this SDK method
    #[serde(default)]
    pub consuming_apps: Vec<ConsumingApp>,
}

/// Mapping from endpoint to SDK methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerMapping {
    /// Endpoint path (e.g., "/api/users")
    pub endpoint: String,
    /// HTTP method (e.g., "GET", "POST")
    pub method: String,
    /// SDK methods that call this endpoint
    #[serde(default)]
    pub sdk_methods: Vec<SDKMethod>,
    /// Timestamp when this mapping was created
    #[serde(default)]
    pub created_at: i64,
    /// Timestamp when this mapping was last updated
    #[serde(default)]
    pub updated_at: i64,
}

/// Impact analysis result showing which consumers are affected by drift
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerImpact {
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// SDK methods that are affected
    pub affected_sdk_methods: Vec<SDKMethod>,
    /// Applications that are affected
    pub affected_apps: Vec<ConsumingApp>,
    /// Human-readable impact summary
    pub impact_summary: String,
}

/// Registry for managing consumer mappings
#[derive(Debug, Clone)]
pub struct ConsumerMappingRegistry {
    /// Mappings keyed by "{method} {endpoint}"
    mappings: HashMap<String, ConsumerMapping>,
}

impl ConsumerMappingRegistry {
    /// Create a new consumer mapping registry
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
        }
    }

    /// Add or update a consumer mapping
    pub fn add_mapping(&mut self, mapping: ConsumerMapping) {
        let key = format!("{} {}", mapping.method, mapping.endpoint);
        self.mappings.insert(key, mapping);
    }

    /// Get mapping for a specific endpoint
    pub fn get_mapping(&self, endpoint: &str, method: &str) -> Option<&ConsumerMapping> {
        let key = format!("{} {}", method, endpoint);
        self.mappings.get(&key)
    }

    /// List all mappings
    pub fn list_mappings(&self) -> Vec<&ConsumerMapping> {
        self.mappings.values().collect()
    }

    /// Remove a mapping
    pub fn remove_mapping(&mut self, endpoint: &str, method: &str) -> Option<ConsumerMapping> {
        let key = format!("{} {}", method, endpoint);
        self.mappings.remove(&key)
    }

    /// Add an SDK method to an endpoint mapping
    pub fn add_sdk_method(&mut self, endpoint: &str, method: &str, sdk_method: SDKMethod) {
        let key = format!("{} {}", method, endpoint);
        let mapping = self.mappings.entry(key).or_insert_with(|| ConsumerMapping {
            endpoint: endpoint.to_string(),
            method: method.to_string(),
            sdk_methods: Vec::new(),
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        });

        // Check if SDK method already exists
        if let Some(existing) = mapping
            .sdk_methods
            .iter_mut()
            .find(|m| m.sdk_name == sdk_method.sdk_name && m.method_name == sdk_method.method_name)
        {
            // Merge consuming apps (avoid duplicates)
            let mut existing_app_ids: HashSet<String> =
                existing.consuming_apps.iter().map(|a| a.app_id.clone()).collect();

            for app in sdk_method.consuming_apps {
                if !existing_app_ids.contains(&app.app_id) {
                    existing.consuming_apps.push(app);
                    existing_app_ids.insert(existing.consuming_apps.last().unwrap().app_id.clone());
                }
            }
        } else {
            mapping.sdk_methods.push(sdk_method);
        }

        mapping.updated_at = chrono::Utc::now().timestamp();
    }

    /// Add a consuming app to an SDK method
    pub fn add_consuming_app(
        &mut self,
        endpoint: &str,
        method: &str,
        sdk_name: &str,
        method_name: &str,
        app: ConsumingApp,
    ) {
        let key = format!("{} {}", method, endpoint);
        if let Some(mapping) = self.mappings.get_mut(&key) {
            if let Some(sdk_method) = mapping
                .sdk_methods
                .iter_mut()
                .find(|m| m.sdk_name == sdk_name && m.method_name == method_name)
            {
                // Check if app already exists
                if !sdk_method.consuming_apps.iter().any(|a| a.app_id == app.app_id) {
                    sdk_method.consuming_apps.push(app);
                    mapping.updated_at = chrono::Utc::now().timestamp();
                }
            }
        }
    }
}

impl Default for ConsumerMappingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Analyzer for determining consumer impact from drift
#[derive(Debug)]
pub struct ConsumerImpactAnalyzer {
    registry: ConsumerMappingRegistry,
}

impl ConsumerImpactAnalyzer {
    /// Create a new consumer impact analyzer
    pub fn new(registry: ConsumerMappingRegistry) -> Self {
        Self { registry }
    }

    /// Analyze impact of drift on a specific endpoint
    ///
    /// Returns a `ConsumerImpact` showing which SDK methods and apps are affected
    pub fn analyze_impact(&self, endpoint: &str, method: &str) -> Option<ConsumerImpact> {
        let mapping = self.registry.get_mapping(endpoint, method)?;

        // Collect all affected SDK methods and apps
        let affected_sdk_methods = mapping.sdk_methods.clone();
        let mut affected_apps: Vec<ConsumingApp> = Vec::new();
        let mut seen_app_ids: HashSet<String> = HashSet::new();

        for sdk_method in &affected_sdk_methods {
            for app in &sdk_method.consuming_apps {
                if !seen_app_ids.contains(&app.app_id) {
                    affected_apps.push(app.clone());
                    seen_app_ids.insert(app.app_id.clone());
                }
            }
        }

        // Generate impact summary
        let app_names: Vec<String> =
            affected_apps.iter().map(|app| app.app_type.to_string()).collect();

        let impact_summary = if app_names.is_empty() {
            format!("No known consumers for {} {}", method, endpoint)
        } else {
            format!("This change may break: {}", app_names.join(", "))
        };

        Some(ConsumerImpact {
            endpoint: endpoint.to_string(),
            method: method.to_string(),
            affected_sdk_methods,
            affected_apps,
            impact_summary,
        })
    }

    /// Analyze impact for multiple endpoints
    pub fn analyze_impact_multiple(&self, endpoints: &[(String, String)]) -> Vec<ConsumerImpact> {
        endpoints
            .iter()
            .filter_map(|(endpoint, method)| self.analyze_impact(endpoint, method))
            .collect()
    }

    /// Get the registry
    pub fn registry(&self) -> &ConsumerMappingRegistry {
        &self.registry
    }

    /// Get mutable access to the registry
    pub fn registry_mut(&mut self) -> &mut ConsumerMappingRegistry {
        &mut self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consumer_mapping_registry() {
        let mut registry = ConsumerMappingRegistry::new();

        let app1 = ConsumingApp {
            app_id: "web-app-1".to_string(),
            app_name: "Web App".to_string(),
            app_type: AppType::Web,
            repository_url: Some("https://github.com/org/web-app".to_string()),
            last_updated: Some(chrono::Utc::now().timestamp()),
            description: None,
        };

        let sdk_method = SDKMethod {
            sdk_name: "typescript-sdk".to_string(),
            method_name: "getUser".to_string(),
            consuming_apps: vec![app1],
        };

        registry.add_sdk_method("/api/users", "GET", sdk_method);

        let mapping = registry.get_mapping("/api/users", "GET");
        assert!(mapping.is_some());
        assert_eq!(mapping.unwrap().sdk_methods.len(), 1);
    }

    #[test]
    fn test_consumer_impact_analyzer() {
        let mut registry = ConsumerMappingRegistry::new();

        let app1 = ConsumingApp {
            app_id: "web-app-1".to_string(),
            app_name: "Web App".to_string(),
            app_type: AppType::Web,
            repository_url: None,
            last_updated: None,
            description: None,
        };

        let app2 = ConsumingApp {
            app_id: "mobile-android-1".to_string(),
            app_name: "Mobile App".to_string(),
            app_type: AppType::MobileAndroid,
            repository_url: None,
            last_updated: None,
            description: None,
        };

        let sdk_method = SDKMethod {
            sdk_name: "typescript-sdk".to_string(),
            method_name: "getUser".to_string(),
            consuming_apps: vec![app1, app2],
        };

        registry.add_sdk_method("/api/users", "GET", sdk_method);

        let analyzer = ConsumerImpactAnalyzer::new(registry);
        let impact = analyzer.analyze_impact("/api/users", "GET");

        assert!(impact.is_some());
        let impact = impact.unwrap();
        assert_eq!(impact.affected_apps.len(), 2);
        assert!(impact.impact_summary.contains("Web App"));
        assert!(impact.impact_summary.contains("Mobile App (Android)"));
    }
}
