//! CRUD Flow support for load testing
//!
//! This module provides functionality to automatically detect and configure
//! CRUD (Create, Read, Update, Delete) flows from OpenAPI specifications,
//! enabling sequential testing with response chaining.

use crate::error::{BenchError, Result};
use crate::spec_parser::ApiOperation;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Field extraction configuration - supports both simple field names and aliased extraction
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ExtractField {
    /// The field name to extract from the response
    pub field: String,
    /// The name to store it as (defaults to field name if not specified)
    /// Note: Deserialization accepts "as" via custom Deserialize impl, but serializes as "store_as"
    pub store_as: String,
}

impl ExtractField {
    /// Create a new extract field with the same name for field and storage
    pub fn simple(field: String) -> Self {
        Self {
            store_as: field.clone(),
            field,
        }
    }

    /// Create a new extract field with an alias
    pub fn aliased(field: String, store_as: String) -> Self {
        Self { field, store_as }
    }
}

impl<'de> Deserialize<'de> for ExtractField {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};

        struct ExtractFieldVisitor;

        impl<'de> Visitor<'de> for ExtractFieldVisitor {
            type Value = ExtractField;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string or an object with 'field' and optional 'as' keys")
            }

            // Handle simple string: "uuid"
            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ExtractField::simple(value.to_string()))
            }

            // Handle object: {field: "uuid", as: "pool_uuid"}
            fn visit_map<M>(self, mut map: M) -> std::result::Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut field: Option<String> = None;
                let mut store_as: Option<String> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "field" => {
                            field = Some(map.next_value()?);
                        }
                        "as" => {
                            store_as = Some(map.next_value()?);
                        }
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                let field = field.ok_or_else(|| de::Error::missing_field("field"))?;
                let store_as = store_as.unwrap_or_else(|| field.clone());

                Ok(ExtractField { field, store_as })
            }
        }

        deserializer.deserialize_any(ExtractFieldVisitor)
    }
}

/// A single step in a CRUD flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowStep {
    /// The operation identifier (e.g., "POST /users" or operation_id)
    pub operation: String,
    /// Fields to extract from the response (for subsequent steps)
    /// Supports both simple strings and objects with aliases:
    /// - Simple: "uuid" (extracts uuid, stores as uuid)
    /// - Aliased: {field: "uuid", as: "pool_uuid"} (extracts uuid, stores as pool_uuid)
    #[serde(default)]
    pub extract: Vec<ExtractField>,
    /// Mapping of path/body variables to extracted values
    /// Key: variable name in request, Value: extracted field name
    #[serde(default)]
    pub use_values: HashMap<String, String>,
    /// Optional description for this step
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl FlowStep {
    /// Create a new flow step
    pub fn new(operation: String) -> Self {
        Self {
            operation,
            extract: Vec::new(),
            use_values: HashMap::new(),
            description: None,
        }
    }

    /// Add fields to extract from response (simple field names)
    pub fn with_extract(mut self, fields: Vec<String>) -> Self {
        self.extract = fields.into_iter().map(ExtractField::simple).collect();
        self
    }

    /// Add fields to extract with aliases
    pub fn with_extract_fields(mut self, fields: Vec<ExtractField>) -> Self {
        self.extract = fields;
        self
    }

    /// Add value mappings for this step
    pub fn with_values(mut self, values: HashMap<String, String>) -> Self {
        self.use_values = values;
        self
    }

    /// Add a description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
}

/// A complete CRUD flow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrudFlow {
    /// Name of this flow
    pub name: String,
    /// Base path for this resource (e.g., "/users")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_path: Option<String>,
    /// Ordered list of steps in the flow
    pub steps: Vec<FlowStep>,
}

impl CrudFlow {
    /// Create a new CRUD flow with a name
    pub fn new(name: String) -> Self {
        Self {
            name,
            base_path: None,
            steps: Vec::new(),
        }
    }

    /// Set the base path
    pub fn with_base_path(mut self, path: String) -> Self {
        self.base_path = Some(path);
        self
    }

    /// Add a step to the flow
    pub fn add_step(&mut self, step: FlowStep) {
        self.steps.push(step);
    }

    /// Get all fields that need to be extracted across all steps (returns field names)
    pub fn get_all_extract_fields(&self) -> HashSet<String> {
        self.steps
            .iter()
            .flat_map(|step| step.extract.iter().map(|e| e.field.clone()))
            .collect()
    }

    /// Get all extract field configurations across all steps
    pub fn get_all_extract_configs(&self) -> Vec<&ExtractField> {
        self.steps.iter().flat_map(|step| step.extract.iter()).collect()
    }
}

/// CRUD flow configuration containing multiple flows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrudFlowConfig {
    /// List of CRUD flows
    pub flows: Vec<CrudFlow>,
    /// Default fields to extract if not specified
    #[serde(default)]
    pub default_extract_fields: Vec<String>,
}

impl Default for CrudFlowConfig {
    fn default() -> Self {
        Self {
            flows: Vec::new(),
            default_extract_fields: vec!["id".to_string(), "uuid".to_string()],
        }
    }
}

impl CrudFlowConfig {
    /// Load configuration from a YAML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| BenchError::Other(format!("Failed to read flow config: {}", e)))?;

        Self::from_yaml(&content)
    }

    /// Parse configuration from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        serde_yaml::from_str(yaml)
            .map_err(|e| BenchError::Other(format!("Failed to parse flow config: {}", e)))
    }

    /// Create configuration with a single flow
    pub fn single_flow(flow: CrudFlow) -> Self {
        Self {
            flows: vec![flow],
            ..Default::default()
        }
    }
}

/// Grouped operations by base path for CRUD detection
#[derive(Debug, Clone)]
pub struct ResourceOperations {
    /// Base path (e.g., "/users")
    pub base_path: String,
    /// CREATE operation (typically POST on base path)
    pub create: Option<ApiOperation>,
    /// READ single operation (typically GET on {id} path)
    pub read: Option<ApiOperation>,
    /// UPDATE operation (typically PUT/PATCH on {id} path)
    pub update: Option<ApiOperation>,
    /// DELETE operation (typically DELETE on {id} path)
    pub delete: Option<ApiOperation>,
    /// LIST operation (typically GET on base path)
    pub list: Option<ApiOperation>,
}

impl ResourceOperations {
    /// Create a new resource operations group
    pub fn new(base_path: String) -> Self {
        Self {
            base_path,
            create: None,
            read: None,
            update: None,
            delete: None,
            list: None,
        }
    }

    /// Check if this resource has CRUD operations
    pub fn has_crud_operations(&self) -> bool {
        self.create.is_some()
            && (self.read.is_some() || self.update.is_some() || self.delete.is_some())
    }

    /// Get the ID parameter name from the path (e.g., "{id}", "{userId}")
    pub fn get_id_param_name(&self) -> Option<String> {
        // Look at read, update, or delete operations to find the ID parameter
        let path = self
            .read
            .as_ref()
            .or(self.update.as_ref())
            .or(self.delete.as_ref())
            .map(|op| &op.path)?;

        // Extract parameter name from path like "/users/{id}" or "/users/{userId}"
        extract_id_param_from_path(path)
    }
}

/// Extract the ID parameter name from a path pattern
fn extract_id_param_from_path(path: &str) -> Option<String> {
    // Find the last path segment that looks like {paramName}
    for segment in path.split('/').rev() {
        if segment.starts_with('{') && segment.ends_with('}') {
            return Some(segment[1..segment.len() - 1].to_string());
        }
    }
    None
}

/// Extract the base path from a full path
/// e.g., "/users/{id}" -> "/users"
fn get_base_path(path: &str) -> String {
    let segments: Vec<&str> = path.split('/').collect();
    let mut base_segments = Vec::new();

    for segment in segments {
        if segment.starts_with('{') {
            break;
        }
        if !segment.is_empty() {
            base_segments.push(segment);
        }
    }

    if base_segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", base_segments.join("/"))
    }
}

/// Check if a path is a "detail" path (has ID parameter)
fn is_detail_path(path: &str) -> bool {
    path.contains('{') && path.contains('}')
}

/// Auto-detect CRUD flows from a list of operations
pub struct CrudFlowDetector;

impl CrudFlowDetector {
    /// Detect CRUD flows from API operations
    ///
    /// Groups operations by base path and identifies CRUD patterns.
    pub fn detect_flows(operations: &[ApiOperation]) -> Vec<CrudFlow> {
        // Group operations by base path
        let mut resources: HashMap<String, ResourceOperations> = HashMap::new();

        for op in operations {
            let base_path = get_base_path(&op.path);
            let is_detail = is_detail_path(&op.path);
            let method = op.method.to_lowercase();

            let resource = resources
                .entry(base_path.clone())
                .or_insert_with(|| ResourceOperations::new(base_path));

            match (method.as_str(), is_detail) {
                ("post", false) => resource.create = Some(op.clone()),
                ("get", false) => resource.list = Some(op.clone()),
                ("get", true) => resource.read = Some(op.clone()),
                ("put", true) | ("patch", true) => resource.update = Some(op.clone()),
                ("delete", true) => resource.delete = Some(op.clone()),
                _ => {}
            }
        }

        // Build flows from resources that have CRUD operations
        resources
            .into_values()
            .filter(|r| r.has_crud_operations())
            .map(|r| Self::build_flow_from_resource(&r))
            .collect()
    }

    /// Build a CRUD flow from a resource's operations
    fn build_flow_from_resource(resource: &ResourceOperations) -> CrudFlow {
        let name = resource.base_path.trim_start_matches('/').replace('/', "_").to_string();

        let mut flow =
            CrudFlow::new(format!("{} CRUD", name)).with_base_path(resource.base_path.clone());

        let id_param = resource.get_id_param_name().unwrap_or_else(|| "id".to_string());

        // Step 1: CREATE (POST) - extract ID
        if let Some(create_op) = &resource.create {
            let step =
                FlowStep::new(format!("{} {}", create_op.method.to_uppercase(), create_op.path))
                    .with_extract(vec!["id".to_string(), "uuid".to_string()])
                    .with_description("Create resource".to_string());
            flow.add_step(step);
        }

        // Step 2: READ (GET) - verify creation
        if let Some(read_op) = &resource.read {
            let mut values = HashMap::new();
            values.insert(id_param.clone(), "id".to_string());

            let step = FlowStep::new(format!("{} {}", read_op.method.to_uppercase(), read_op.path))
                .with_values(values)
                .with_description("Read created resource".to_string());
            flow.add_step(step);
        }

        // Step 3: UPDATE (PUT/PATCH) - modify resource
        if let Some(update_op) = &resource.update {
            let mut values = HashMap::new();
            values.insert(id_param.clone(), "id".to_string());

            let step =
                FlowStep::new(format!("{} {}", update_op.method.to_uppercase(), update_op.path))
                    .with_values(values)
                    .with_description("Update resource".to_string());
            flow.add_step(step);
        }

        // Step 4: READ again (GET) - verify update
        if let Some(read_op) = &resource.read {
            let mut values = HashMap::new();
            values.insert(id_param.clone(), "id".to_string());

            let step = FlowStep::new(format!("{} {}", read_op.method.to_uppercase(), read_op.path))
                .with_values(values)
                .with_description("Verify update".to_string());
            flow.add_step(step);
        }

        // Step 5: DELETE - cleanup
        if let Some(delete_op) = &resource.delete {
            let mut values = HashMap::new();
            values.insert(id_param.clone(), "id".to_string());

            let step =
                FlowStep::new(format!("{} {}", delete_op.method.to_uppercase(), delete_op.path))
                    .with_values(values)
                    .with_description("Delete resource".to_string());
            flow.add_step(step);
        }

        flow
    }

    /// Merge auto-detected flows with user-provided configuration
    ///
    /// User configuration takes precedence over auto-detected flows.
    pub fn merge_with_config(detected: Vec<CrudFlow>, config: &CrudFlowConfig) -> Vec<CrudFlow> {
        if !config.flows.is_empty() {
            // If user provided flows, use those
            config.flows.clone()
        } else {
            // Otherwise use auto-detected flows
            detected
        }
    }
}

/// Context for CRUD flow execution (tracks extracted values)
#[derive(Debug, Clone, Default)]
pub struct FlowExecutionContext {
    /// Extracted values from previous steps
    /// Key: field name, Value: extracted value
    pub extracted_values: HashMap<String, String>,
    /// Current step index
    pub current_step: usize,
    /// Errors encountered
    pub errors: Vec<String>,
}

impl FlowExecutionContext {
    /// Create a new execution context
    pub fn new() -> Self {
        Self::default()
    }

    /// Store an extracted value
    pub fn store_value(&mut self, key: String, value: String) {
        self.extracted_values.insert(key, value);
    }

    /// Get an extracted value
    pub fn get_value(&self, key: &str) -> Option<&String> {
        self.extracted_values.get(key)
    }

    /// Advance to the next step
    pub fn next_step(&mut self) {
        self.current_step += 1;
    }

    /// Record an error
    pub fn record_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// Check if execution has errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openapiv3::Operation;

    fn create_test_operation(method: &str, path: &str, operation_id: Option<&str>) -> ApiOperation {
        ApiOperation {
            method: method.to_string(),
            path: path.to_string(),
            operation: Operation::default(),
            operation_id: operation_id.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_get_base_path() {
        assert_eq!(get_base_path("/users"), "/users");
        assert_eq!(get_base_path("/users/{id}"), "/users");
        assert_eq!(get_base_path("/api/v1/users/{userId}"), "/api/v1/users");
        assert_eq!(get_base_path("/resources/{resourceId}/items/{itemId}"), "/resources");
        assert_eq!(get_base_path("/"), "/");
    }

    #[test]
    fn test_is_detail_path() {
        assert!(!is_detail_path("/users"));
        assert!(is_detail_path("/users/{id}"));
        assert!(is_detail_path("/users/{userId}"));
        assert!(!is_detail_path("/users/list"));
    }

    #[test]
    fn test_extract_id_param_from_path() {
        assert_eq!(extract_id_param_from_path("/users/{id}"), Some("id".to_string()));
        assert_eq!(extract_id_param_from_path("/users/{userId}"), Some("userId".to_string()));
        assert_eq!(extract_id_param_from_path("/a/{b}/{c}"), Some("c".to_string()));
        assert_eq!(extract_id_param_from_path("/users"), None);
    }

    #[test]
    fn test_crud_flow_detection() {
        let operations = vec![
            create_test_operation("post", "/users", Some("createUser")),
            create_test_operation("get", "/users", Some("listUsers")),
            create_test_operation("get", "/users/{id}", Some("getUser")),
            create_test_operation("put", "/users/{id}", Some("updateUser")),
            create_test_operation("delete", "/users/{id}", Some("deleteUser")),
        ];

        let flows = CrudFlowDetector::detect_flows(&operations);
        assert_eq!(flows.len(), 1);

        let flow = &flows[0];
        assert!(flow.name.contains("users"));
        assert_eq!(flow.steps.len(), 5); // CREATE, READ, UPDATE, READ, DELETE

        // First step should be POST (create)
        assert!(flow.steps[0].operation.starts_with("POST"));
        // Should extract id
        assert!(flow.steps[0].extract.iter().any(|e| e.field == "id"));
    }

    #[test]
    fn test_multiple_resources_detection() {
        let operations = vec![
            // Users resource
            create_test_operation("post", "/users", Some("createUser")),
            create_test_operation("get", "/users/{id}", Some("getUser")),
            create_test_operation("delete", "/users/{id}", Some("deleteUser")),
            // Posts resource
            create_test_operation("post", "/posts", Some("createPost")),
            create_test_operation("get", "/posts/{id}", Some("getPost")),
            create_test_operation("put", "/posts/{id}", Some("updatePost")),
        ];

        let flows = CrudFlowDetector::detect_flows(&operations);
        assert_eq!(flows.len(), 2);
    }

    #[test]
    fn test_no_crud_without_create() {
        let operations = vec![
            // Only read operations - no CREATE
            create_test_operation("get", "/users", Some("listUsers")),
            create_test_operation("get", "/users/{id}", Some("getUser")),
        ];

        let flows = CrudFlowDetector::detect_flows(&operations);
        assert!(flows.is_empty());
    }

    #[test]
    fn test_flow_step_builder() {
        let step = FlowStep::new("POST /users".to_string())
            .with_extract(vec!["id".to_string(), "uuid".to_string()])
            .with_description("Create a new user".to_string());

        assert_eq!(step.operation, "POST /users");
        assert_eq!(step.extract.len(), 2);
        assert_eq!(step.description, Some("Create a new user".to_string()));
    }

    #[test]
    fn test_flow_step_use_values() {
        let mut values = HashMap::new();
        values.insert("id".to_string(), "user_id".to_string());

        let step = FlowStep::new("GET /users/{id}".to_string()).with_values(values);

        assert_eq!(step.use_values.get("id"), Some(&"user_id".to_string()));
    }

    #[test]
    fn test_crud_flow_config_from_yaml() {
        let yaml = r#"
flows:
  - name: "User CRUD"
    base_path: "/users"
    steps:
      - operation: "POST /users"
        extract: ["id"]
        description: "Create user"
      - operation: "GET /users/{id}"
        use_values:
          id: "id"
        description: "Get user"
default_extract_fields:
  - id
  - uuid
"#;

        let config = CrudFlowConfig::from_yaml(yaml).expect("Should parse YAML");
        assert_eq!(config.flows.len(), 1);
        assert_eq!(config.flows[0].name, "User CRUD");
        assert_eq!(config.flows[0].steps.len(), 2);
        assert_eq!(config.default_extract_fields.len(), 2);
    }

    #[test]
    fn test_execution_context() {
        let mut ctx = FlowExecutionContext::new();

        ctx.store_value("id".to_string(), "12345".to_string());
        assert_eq!(ctx.get_value("id"), Some(&"12345".to_string()));

        ctx.next_step();
        assert_eq!(ctx.current_step, 1);

        ctx.record_error("Something went wrong".to_string());
        assert!(ctx.has_errors());
    }

    #[test]
    fn test_resource_operations_has_crud() {
        let mut resource = ResourceOperations::new("/users".to_string());
        assert!(!resource.has_crud_operations());

        resource.create = Some(create_test_operation("post", "/users", Some("createUser")));
        assert!(!resource.has_crud_operations()); // Still needs read/update/delete

        resource.read = Some(create_test_operation("get", "/users/{id}", Some("getUser")));
        assert!(resource.has_crud_operations());
    }

    #[test]
    fn test_get_id_param_name() {
        let mut resource = ResourceOperations::new("/users".to_string());
        resource.read = Some(create_test_operation("get", "/users/{userId}", Some("getUser")));

        assert_eq!(resource.get_id_param_name(), Some("userId".to_string()));
    }

    #[test]
    fn test_merge_with_config_user_provided() {
        let detected = vec![CrudFlow::new("detected_flow".to_string())];

        let mut config = CrudFlowConfig::default();
        config.flows.push(CrudFlow::new("user_flow".to_string()));

        let result = CrudFlowDetector::merge_with_config(detected, &config);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "user_flow");
    }

    #[test]
    fn test_merge_with_config_auto_detected() {
        let detected = vec![CrudFlow::new("detected_flow".to_string())];

        let config = CrudFlowConfig::default();

        let result = CrudFlowDetector::merge_with_config(detected, &config);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "detected_flow");
    }

    #[test]
    fn test_crud_flow_get_all_extract_fields() {
        let mut flow = CrudFlow::new("test".to_string());
        flow.add_step(FlowStep::new("POST /test".to_string()).with_extract(vec!["id".to_string()]));
        flow.add_step(
            FlowStep::new("GET /test/{id}".to_string()).with_extract(vec!["uuid".to_string()]),
        );

        let fields = flow.get_all_extract_fields();
        assert!(fields.contains(&"id".to_string()));
        assert!(fields.contains(&"uuid".to_string()));
        assert_eq!(fields.len(), 2);
    }
}
