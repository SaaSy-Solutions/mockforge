//! Consumer mapping for tracking endpoint → SDK method → consuming app relationships
//!
//! This module provides functionality for mapping API endpoints to SDK methods and
//! tracking which applications consume those methods. This enables consumer-focused
//! drift insights that show which apps will be affected by contract changes.
//!
//! The data types (`AppType`, `ConsumingApp`, `SDKMethod`, `ConsumerMapping`,
//! `ConsumerImpact`) live in `mockforge-foundation::contract_drift_types` so
//! consumers do not need to depend on deprecated core modules. The registry and
//! analyzer logic stays here because it does not have cross-crate consumers yet.

pub use mockforge_foundation::contract_drift_types::{
    AppType, ConsumerImpact, ConsumerMapping, ConsumingApp, SDKMethod,
};
use std::collections::{HashMap, HashSet};

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
    /// Returns a `ConsumerImpact` showing which SDK methods and apps are affected.
    /// Works with both HTTP endpoints (e.g., "/api/users", "GET") and protocol-specific
    /// operation IDs (e.g., "service.method", "grpc" for gRPC or "topic", "mqtt" for MQTT).
    ///
    /// This function tries multiple lookup strategies to find consumer mappings:
    /// 1. Direct lookup with the provided endpoint and method
    /// 2. For gRPC: tries "service.method" with method="grpc", service-level matching, and wildcard patterns
    /// 3. For WebSocket: tries message_type with method="websocket", and also tries "topic:message_type" format
    /// 4. For MQTT/Kafka: tries topic with protocol as method
    /// 5. Also tries using operation_id directly if provided and different from endpoint
    pub fn analyze_impact(&self, endpoint: &str, method: &str) -> Option<ConsumerImpact> {
        self.analyze_impact_with_operation_id(endpoint, method, None)
    }

    /// Analyze impact with an optional operation_id for more flexible matching
    ///
    /// This is useful when the operation_id format differs from the endpoint format,
    /// which can happen with protocol-specific contracts.
    pub fn analyze_impact_with_operation_id(
        &self,
        endpoint: &str,
        method: &str,
        operation_id: Option<&str>,
    ) -> Option<ConsumerImpact> {
        // Try direct lookup first
        let mut mapping = self.registry.get_mapping(endpoint, method);

        // If operation_id is provided and different from endpoint, try it
        if mapping.is_none() {
            if let Some(op_id) = operation_id {
                if op_id != endpoint {
                    mapping = self.registry.get_mapping(op_id, method);
                }
            }
        }

        // If not found and this looks like a protocol-specific operation, try alternative formats
        if mapping.is_none() {
            // For gRPC: endpoint might be "service.method", try with method="grpc"
            if method == "grpc"
                || (method.contains("grpc")
                    && (endpoint.contains('.')
                        || operation_id.map(|id| id.contains('.')).unwrap_or(false)))
            {
                // Try exact match first with endpoint
                mapping = self.registry.get_mapping(endpoint, "grpc");

                // Try with operation_id if different
                if mapping.is_none() {
                    if let Some(op_id) = operation_id {
                        if op_id != endpoint {
                            mapping = self.registry.get_mapping(op_id, "grpc");
                        }
                    }
                }

                // Try service-level matching (e.g., "user.UserService.*" matches "user.UserService.GetUser")
                if mapping.is_none() {
                    let lookup_key = operation_id.unwrap_or(endpoint);
                    if lookup_key.contains('.') {
                        // Try service-level wildcard pattern
                        let parts: Vec<&str> = lookup_key.split('.').collect();
                        if parts.len() >= 2 {
                            // Try "service.*" pattern
                            let service_pattern =
                                format!("{}.*", parts[0..parts.len() - 1].join("."));
                            mapping = self.try_wildcard_match(&service_pattern, "grpc");

                            // If still not found, try just service name
                            if mapping.is_none() {
                                let service_name = parts[0..parts.len() - 1].join(".");
                                mapping = self.registry.get_mapping(&service_name, "grpc");
                            }
                        }
                    }
                }
            }
            // For WebSocket: endpoint might be message_type, try with method="websocket"
            else if method == "websocket" || method.contains("websocket") {
                // Try exact match first
                mapping = self.registry.get_mapping(endpoint, "websocket");

                // Try with operation_id if different
                if mapping.is_none() {
                    if let Some(op_id) = operation_id {
                        if op_id != endpoint {
                            mapping = self.registry.get_mapping(op_id, "websocket");
                        }
                    }
                }

                // If endpoint contains ':', try extracting message_type (format: "topic:message_type")
                if mapping.is_none() && endpoint.contains(':') {
                    if let Some(message_type) = endpoint.split(':').nth(1) {
                        mapping = self.registry.get_mapping(message_type, "websocket");
                    }
                }

                // Try with operation_id if it contains ':'
                if mapping.is_none() {
                    if let Some(op_id) = operation_id {
                        if op_id.contains(':') {
                            if let Some(message_type) = op_id.split(':').nth(1) {
                                mapping = self.registry.get_mapping(message_type, "websocket");
                            }
                        }
                    }
                }
            }
            // For MQTT/Kafka: endpoint is topic, try with protocol as method
            else if method == "mqtt"
                || method == "kafka"
                || method.contains("mqtt")
                || method.contains("kafka")
            {
                let protocol = if method.contains("mqtt") {
                    "mqtt"
                } else {
                    "kafka"
                };
                mapping = self.registry.get_mapping(endpoint, protocol);

                // Try with operation_id if different
                if mapping.is_none() {
                    if let Some(op_id) = operation_id {
                        if op_id != endpoint {
                            mapping = self.registry.get_mapping(op_id, protocol);
                        }
                    }
                }

                // Try wildcard matching for topic patterns (e.g., "devices/+/telemetry" matches "devices/device1/telemetry")
                if mapping.is_none() {
                    let lookup_key = operation_id.unwrap_or(endpoint);
                    mapping = self.try_topic_wildcard_match(lookup_key, protocol);
                }
            }
        }

        let mapping = mapping?;

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

    /// Try to find a mapping using wildcard pattern matching
    ///
    /// Supports patterns like "service.*" or "user.UserService.*"
    fn try_wildcard_match(&self, pattern: &str, method: &str) -> Option<&ConsumerMapping> {
        // Remove trailing ".*" from pattern
        let base_pattern = pattern.strip_suffix(".*").unwrap_or(pattern);

        // Try to find mappings that start with the base pattern
        for (key, mapping) in &self.registry.mappings {
            if key.starts_with(&format!("{} ", method)) {
                let endpoint = &mapping.endpoint;
                if endpoint.starts_with(base_pattern) {
                    return Some(mapping);
                }
            }
        }

        None
    }

    /// Try to find a mapping using topic wildcard matching
    ///
    /// Supports MQTT-style wildcards like "devices/+/telemetry" or "devices/#"
    fn try_topic_wildcard_match(&self, topic: &str, method: &str) -> Option<&ConsumerMapping> {
        // Try to find mappings that match the topic pattern
        for (key, mapping) in &self.registry.mappings {
            if key.starts_with(&format!("{} ", method)) {
                let pattern = &mapping.endpoint;

                // Check if pattern matches topic using MQTT wildcard rules
                if self.matches_topic_pattern(topic, pattern) {
                    return Some(mapping);
                }
            }
        }

        None
    }

    /// Check if a topic matches a pattern with MQTT wildcards
    ///
    /// Supports:
    /// - `+` matches a single level (e.g., "devices/+/telemetry" matches "devices/device1/telemetry")
    /// - `#` matches multiple levels (e.g., "devices/#" matches "devices/device1/telemetry")
    fn matches_topic_pattern(&self, topic: &str, pattern: &str) -> bool {
        // Exact match
        if topic == pattern {
            return true;
        }

        // Handle wildcards
        if pattern.contains('+') || pattern.contains('#') {
            let topic_parts: Vec<&str> = topic.split('/').collect();
            let pattern_parts: Vec<&str> = pattern.split('/').collect();

            // Handle multi-level wildcard at the end
            if let Some(base_pattern) = pattern.strip_suffix("/#") {
                return topic.starts_with(base_pattern);
            }

            // Handle single-level wildcards
            if topic_parts.len() == pattern_parts.len() {
                for (topic_part, pattern_part) in topic_parts.iter().zip(pattern_parts.iter()) {
                    if *pattern_part != "+" && *pattern_part != "#" && topic_part != pattern_part {
                        return false;
                    }
                }
                return true;
            }
        }

        false
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
