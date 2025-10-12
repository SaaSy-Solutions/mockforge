//! # GraphQL Response Generator Plugin for MockForge
//!
//! This plugin generates mock GraphQL responses by analyzing GraphQL queries
//! and generating appropriate mock data based on the requested fields.
//!
//! ## Features
//!
//! - GraphQL query parsing and field analysis
//! - Type-aware mock data generation
//! - Support for nested queries and fragments
//! - Configurable data complexity levels

use graphql_parser::query::{parse_query, Definition, OperationDefinition, Selection};
use mockforge_plugin_core::*;
use rand::{rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLConfig {
    /// Path to GraphQL schema file (optional)
    pub schema_file: Option<String>,
    /// Enable GraphQL introspection
    pub enable_introspection: bool,
    /// Mock data complexity level
    pub mock_data_complexity: String,
}

impl Default for GraphQLConfig {
    fn default() -> Self {
        Self {
            schema_file: None,
            enable_introspection: true,
            mock_data_complexity: "medium".to_string(),
        }
    }
}

/// GraphQL Response Generator Plugin
pub struct GraphQLResponsePlugin {
    config: GraphQLConfig,
}

impl GraphQLResponsePlugin {
    /// Create a new GraphQL response plugin
    pub fn new(config: GraphQLConfig) -> Self {
        Self { config }
    }

    /// Parse GraphQL query and extract requested fields
    fn parse_graphql_query(&self, query: &str) -> Vec<String> {
        let mut fields = Vec::new();

        // Parse the GraphQL query using the proper parser
        match parse_query::<&str>(query) {
            Ok(document) => {
                for definition in document.definitions {
                    if let Definition::Operation(operation) = definition {
                        match operation {
                            OperationDefinition::Query(query_def) => {
                                self.extract_fields_from_selection_set(
                                    &query_def.selection_set,
                                    &mut fields,
                                );
                            }
                            OperationDefinition::Mutation(mutation_def) => {
                                self.extract_fields_from_selection_set(
                                    &mutation_def.selection_set,
                                    &mut fields,
                                );
                            }
                            OperationDefinition::Subscription(subscription_def) => {
                                self.extract_fields_from_selection_set(
                                    &subscription_def.selection_set,
                                    &mut fields,
                                );
                            }
                            OperationDefinition::SelectionSet(selection_set) => {
                                self.extract_fields_from_selection_set(&selection_set, &mut fields);
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // Fallback: extract common GraphQL fields if parsing fails
                if query.contains("user") || query.contains("User") {
                    fields.extend(vec![
                        "id".to_string(),
                        "name".to_string(),
                        "email".to_string(),
                        "createdAt".to_string(),
                    ]);
                }
                if query.contains("product") || query.contains("Product") {
                    fields.extend(vec![
                        "id".to_string(),
                        "name".to_string(),
                        "price".to_string(),
                        "category".to_string(),
                    ]);
                }
                if query.contains("order") || query.contains("Order") {
                    fields.extend(vec![
                        "id".to_string(),
                        "total".to_string(),
                        "status".to_string(),
                        "items".to_string(),
                    ]);
                }
            }
        }

        fields
    }

    /// Recursively extract field names from a selection set
    #[allow(clippy::only_used_in_recursion)]
    fn extract_fields_from_selection_set<'a>(
        &self,
        selection_set: &graphql_parser::query::SelectionSet<'a, &'a str>,
        fields: &mut Vec<String>,
    ) {
        for selection in &selection_set.items {
            match selection {
                Selection::Field(field) => {
                    fields.push(field.name.to_string());
                    // Recursively extract fields from nested selection sets
                    self.extract_fields_from_selection_set(&field.selection_set, fields);
                }
                Selection::FragmentSpread(_) => {
                    // Handle fragment spreads if needed
                }
                Selection::InlineFragment(inline_fragment) => {
                    // Handle inline fragments
                    self.extract_fields_from_selection_set(&inline_fragment.selection_set, fields);
                }
            }
        }
    }

    /// Generate mock data for a field based on its name and type hints
    fn generate_mock_field(&self, field_name: &str) -> serde_json::Value {
        let mut rng = rng();
        match field_name.to_lowercase().as_str() {
            "id" => {
                if field_name.contains("user") || field_name.contains("User") {
                    serde_json::json!(format!("user_{:04X}", rng.random::<u16>()))
                } else if field_name.contains("product") || field_name.contains("Product") {
                    serde_json::json!(format!("prod_{:04X}", rng.random::<u16>()))
                } else if field_name.contains("order") || field_name.contains("Order") {
                    serde_json::json!(format!("order_{:04X}", rng.random::<u16>()))
                } else {
                    serde_json::json!(format!("id_{:04X}", rng.random::<u16>()))
                }
            }

            "name" | "title" => {
                let names = [
                    "Alice Johnson",
                    "Bob Smith",
                    "Charlie Brown",
                    "Diana Prince",
                    "Eve Wilson",
                    "Frank Miller",
                    "Grace Lee",
                    "Henry Ford",
                    "Wireless Headphones",
                    "Smart Watch",
                    "Laptop Computer",
                    "Coffee Maker",
                ];
                serde_json::json!(names[rng.random_range(0..names.len())])
            }

            "email" => {
                let domains = ["gmail.com", "yahoo.com", "hotmail.com", "example.com"];
                let local = format!("user{:03}", rng.random_range(1..999));
                let domain = domains[rng.random_range(0..domains.len())];
                serde_json::json!(format!("{}@{}", local, domain))
            }

            "price" | "amount" | "total" | "cost" => {
                let amount = rng.random_range(10.0..1000.0);
                serde_json::json!(format!("${:.2}", amount))
            }

            "status" => {
                let statuses = ["pending", "processing", "shipped", "delivered", "cancelled"];
                serde_json::json!(statuses[rng.random_range(0..statuses.len())])
            }

            "createdat" | "updatedat" | "timestamp" => {
                serde_json::json!(chrono::Utc::now().to_rfc3339())
            }

            "count" | "quantity" | "stock" => {
                serde_json::json!(rng.random_range(1..100))
            }

            "description" | "bio" | "summary" => {
                let descriptions = [
                    "A high-quality product designed for everyday use.",
                    "Premium features with excellent performance.",
                    "Reliable and durable construction.",
                    "User-friendly interface with advanced capabilities.",
                ];
                serde_json::json!(descriptions[rng.random_range(0..descriptions.len())])
            }

            "category" | "type" | "kind" => {
                let categories = [
                    "Electronics",
                    "Books",
                    "Clothing",
                    "Home",
                    "Sports",
                    "Music",
                ];
                serde_json::json!(categories[rng.random_range(0..categories.len())])
            }

            "url" | "image" | "avatar" => {
                serde_json::json!(format!(
                    "https://example.com/image_{}.jpg",
                    rng.random_range(1..100)
                ))
            }

            "active" | "enabled" | "isactive" => {
                serde_json::json!(rng.random_bool(0.8)) // 80% chance of being true
            }

            _ => {
                // Default mock data based on complexity setting
                match self.config.mock_data_complexity.as_str() {
                    "simple" => serde_json::json!("mock_value"),
                    "medium" => {
                        serde_json::json!(format!(
                            "mock_{}_{}",
                            field_name,
                            rng.random_range(1..100)
                        ))
                    }
                    "complex" => {
                        // Generate more complex mock data
                        let mock_types = ["string", "number", "boolean", "array", "object"];
                        match mock_types[rng.random_range(0..mock_types.len())] {
                            "string" => serde_json::json!(format!("complex_mock_{}", field_name)),
                            "number" => serde_json::json!(rng.random_range(1..1000)),
                            "boolean" => serde_json::json!(rng.random_bool(0.5)),
                            "array" => serde_json::json!([1, 2, 3, rng.random_range(4..10)]),
                            "object" => serde_json::json!({
                                "nested_field": format!("nested_{}", field_name),
                                "value": rng.random_range(1..100)
                            }),
                            _ => serde_json::json!("default_mock"),
                        }
                    }
                    _ => serde_json::json!("mock_value"),
                }
            }
        }
    }

    /// Generate GraphQL response data
    fn generate_graphql_response(&self, query: &str) -> serde_json::Value {
        let fields = self.parse_graphql_query(query);

        if fields.is_empty() {
            // Default response for unparsable queries
            return serde_json::json!({
                "data": {
                    "message": "Mock GraphQL response generated",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }
            });
        }

        // Generate response data
        let mut data = serde_json::Map::new();
        for field in &fields {
            data.insert(field.clone(), self.generate_mock_field(field));
        }

        // Handle nested objects (simplified)
        if fields.contains(&"user".to_string()) || query.contains("user") {
            let mut user_data = serde_json::Map::new();
            user_data.insert("id".to_string(), self.generate_mock_field("id"));
            user_data.insert("name".to_string(), self.generate_mock_field("name"));
            user_data.insert("email".to_string(), self.generate_mock_field("email"));
            data.insert("user".to_string(), serde_json::Value::Object(user_data));
        }

        serde_json::json!({
            "data": serde_json::Value::Object(data)
        })
    }

    /// Check if this is a GraphQL request
    fn is_graphql_request(&self, request: &ResponseRequest) -> bool {
        // Check content type
        if let Some(content_type) = request.headers.get("content-type") {
            if content_type.to_str().unwrap_or("").contains("application/graphql")
                || content_type.to_str().unwrap_or("").contains("application/json")
            {
                return true;
            }
        }

        // Check if body contains GraphQL query
        if let Some(body) = &request.body {
            if let Ok(query_str) = std::str::from_utf8(body) {
                return query_str.contains("query")
                    || query_str.contains("{")
                    || query_str.contains("mutation");
            }
        }

        false
    }
}

#[::async_trait::async_trait]
impl ResponsePlugin for GraphQLResponsePlugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            network: NetworkPermissions {
                allow_http: false,
                allowed_hosts: vec![],
                max_connections: 10,
            },
            filesystem: FilesystemPermissions {
                read_paths: vec![],
                write_paths: vec![],
                allow_temp_files: false,
            },
            resources: ResourceLimits {
                max_memory_bytes: 20 * 1024 * 1024, // 20MB
                max_cpu_percent: 0.5,
                max_execution_time_ms: 200, // 200ms per response
                max_concurrent_executions: 5,
            },
            custom: HashMap::new(),
        }
    }

    async fn initialize(&self, _config: &ResponsePluginConfig) -> Result<()> {
        Ok(())
    }

    async fn can_handle(
        &self,
        _context: &PluginContext,
        request: &ResponseRequest,
        _config: &ResponsePluginConfig,
    ) -> Result<PluginResult<bool>> {
        Ok(PluginResult::success(self.is_graphql_request(request), 0))
    }

    async fn generate_response(
        &self,
        _context: &PluginContext,
        request: &ResponseRequest,
        _config: &ResponsePluginConfig,
    ) -> Result<PluginResult<ResponseData>> {
        if !self.is_graphql_request(request) {
            return Ok(PluginResult::failure("Not a GraphQL request".to_string(), 0));
        }

        // Extract GraphQL query from request body
        let query = if let Some(body) = &request.body {
            if let Ok(query_str) = std::str::from_utf8(body) {
                // Try to extract query from JSON payload
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(query_str) {
                    if let Some(query) = json.get("query").and_then(|q| q.as_str()) {
                        query.to_string()
                    } else {
                        query_str.to_string()
                    }
                } else {
                    query_str.to_string()
                }
            } else {
                return Ok(PluginResult::failure(
                    "Unable to extract query from request body".to_string(),
                    0,
                ));
            }
        } else {
            return Ok(PluginResult::failure(
                "No request body found for GraphQL query".to_string(),
                0,
            ));
        };

        let response_data = self.generate_graphql_response(&query);

        // Convert to ResponseData
        let response = ResponseData {
            status_code: 200,
            headers: HashMap::new(),
            body: serde_json::to_vec(&response_data).unwrap_or_default(),
            content_type: "application/json".to_string(),
            metadata: HashMap::new(),
            cache_control: None,
            custom: HashMap::new(),
        };

        Ok(PluginResult::success(response, 0))
    }

    fn priority(&self) -> i32 {
        0
    }

    fn validate_config(&self, _config: &ResponsePluginConfig) -> Result<()> {
        Ok(())
    }

    fn supported_content_types(&self) -> Vec<String> {
        vec![
            "application/graphql".to_string(),
            "application/json".to_string(),
        ]
    }

    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }
}

/// Plugin factory function
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
#[no_mangle]
pub unsafe extern "C" fn create_response_plugin(
    config_json: *const u8,
    config_len: usize,
) -> *mut GraphQLResponsePlugin {
    let config_bytes = std::slice::from_raw_parts(config_json, config_len);

    let config_str = match std::str::from_utf8(config_bytes) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let config: GraphQLConfig = match serde_json::from_str(config_str) {
        Ok(c) => c,
        Err(_) => return std::ptr::null_mut(),
    };

    let plugin = Box::new(GraphQLResponsePlugin::new(config));
    Box::into_raw(plugin)
}

/// Plugin cleanup function
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
#[no_mangle]
pub unsafe extern "C" fn destroy_response_plugin(plugin: *mut GraphQLResponsePlugin) {
    if !plugin.is_null() {
        let _ = Box::from_raw(plugin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{HeaderMap, HeaderValue, Method};

    #[test]
    fn test_graphql_query_parsing() {
        let config = GraphQLConfig::default();
        let plugin = GraphQLResponsePlugin::new(config);

        // Test simple query
        let query = r#"query { user { id name } }"#;
        let fields = plugin.parse_graphql_query(query);

        // With proper parsing, we should extract all fields including nested ones
        assert!(fields.contains(&"user".to_string()));
        assert!(fields.contains(&"id".to_string()));
        assert!(fields.contains(&"name".to_string()));
    }

    #[test]
    fn test_graphql_parsing_fallback() {
        let config = GraphQLConfig::default();
        let plugin = GraphQLResponsePlugin::new(config);

        // Test invalid query that should fall back to keyword detection
        let query = "some invalid query with user and product";
        let fields = plugin.parse_graphql_query(query);

        // Should fall back to detecting common fields
        assert!(fields.contains(&"id".to_string()));
        assert!(fields.contains(&"name".to_string()));
        assert!(fields.contains(&"email".to_string()));
    }

    #[test]
    fn test_mock_field_generation() {
        let config = GraphQLConfig::default();
        let plugin = GraphQLResponsePlugin::new(config);

        let id_value = plugin.generate_mock_field("id");
        assert!(id_value.is_string());

        let name_value = plugin.generate_mock_field("name");
        assert!(name_value.is_string());

        let price_value = plugin.generate_mock_field("price");
        assert!(price_value.is_string());
        assert!(price_value.as_str().unwrap().starts_with("$"));
    }

    #[test]
    fn test_graphql_response_generation() {
        let config = GraphQLConfig::default();
        let plugin = GraphQLResponsePlugin::new(config);

        let query = r#"query { user { id name email } }"#;
        let response = plugin.generate_graphql_response(query);

        assert!(response.get("data").is_some());
    }

    #[test]
    fn test_graphql_request_detection() {
        let config = GraphQLConfig::default();
        let plugin = GraphQLResponsePlugin::new(config);

        // Test GraphQL content type
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/graphql"));
        let request = mockforge_plugin_core::ResponseRequest {
            method: Method::POST,
            uri: "/graphql".to_string(),
            path: "/graphql".to_string(),
            query_params: HashMap::new(),
            headers: headers.clone(),
            body: None,
            path_params: HashMap::new(),
            client_ip: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            timestamp: chrono::Utc::now(),
            auth_context: None,
            custom: HashMap::new(),
        };
        assert!(plugin.is_graphql_request(&request));

        // Test JSON content type
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        let request = mockforge_plugin_core::ResponseRequest {
            method: Method::POST,
            uri: "/graphql".to_string(),
            path: "/graphql".to_string(),
            query_params: HashMap::new(),
            headers: headers.clone(),
            body: None,
            path_params: HashMap::new(),
            client_ip: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            timestamp: chrono::Utc::now(),
            auth_context: None,
            custom: HashMap::new(),
        };
        assert!(plugin.is_graphql_request(&request));
    }

    #[tokio::test]
    async fn test_response_generation() {
        let config = GraphQLConfig::default();
        let plugin = GraphQLResponsePlugin::new(config);

        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));

        let body = serde_json::json!({
            "query": "query { user { id name email } }"
        });
        let body_bytes = serde_json::to_vec(&body).unwrap();

        let context =
            PluginContext::new(PluginId::new("graphql-response"), PluginVersion::new(1, 0, 0));

        let request = mockforge_plugin_core::ResponseRequest {
            method: Method::POST,
            uri: "/graphql".to_string(),
            path: "/graphql".to_string(),
            query_params: HashMap::new(),
            headers: headers.clone(),
            body: Some(body_bytes),
            path_params: HashMap::new(),
            client_ip: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            timestamp: chrono::Utc::now(),
            auth_context: None,
            custom: HashMap::new(),
        };
        let result = plugin
            .generate_response(&context, &request, &ResponsePluginConfig::default())
            .await;
        assert!(result.is_ok());
        let plugin_result = result.unwrap();
        assert!(plugin_result.success);
        assert!(plugin_result.data.is_some());
    }
}
