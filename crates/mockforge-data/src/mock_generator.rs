//! Enhanced Mock Data Generator for OpenAPI Specifications
//!
//! This module provides comprehensive mock data generation capabilities that go beyond
//! the basic schema generator, offering intelligent data generation based on OpenAPI
//! specifications with type safety and realistic data patterns.

use crate::consistency::ConsistencyStore;
use crate::domains::Domain;
use crate::faker::EnhancedFaker;
use crate::persona::PersonaRegistry;
use crate::persona_backstory::BackstoryGenerator;
use crate::persona_templates::PersonaTemplateRegistry;
use crate::schema::{FieldDefinition, SchemaDefinition};
use crate::{Error, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Configuration for mock data generation
#[derive(Debug, Clone)]
pub struct MockGeneratorConfig {
    /// Whether to use realistic data patterns
    pub realistic_mode: bool,
    /// Default array size for generated arrays
    pub default_array_size: usize,
    /// Maximum array size for generated arrays
    pub max_array_size: usize,
    /// Whether to include optional fields
    pub include_optional_fields: bool,
    /// Custom field mappings for specific field names
    pub field_mappings: HashMap<String, String>,
    /// Whether to validate generated data against schemas
    pub validate_generated_data: bool,
    /// Whether to generate backstories for personas
    pub enable_backstories: bool,
}

impl Default for MockGeneratorConfig {
    fn default() -> Self {
        Self {
            realistic_mode: true,
            default_array_size: 3,
            max_array_size: 10,
            include_optional_fields: true,
            field_mappings: HashMap::new(),
            validate_generated_data: true,
            enable_backstories: false,
        }
    }
}

impl MockGeneratorConfig {
    /// Create a new configuration with realistic defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable realistic data generation
    pub fn realistic_mode(mut self, enabled: bool) -> Self {
        self.realistic_mode = enabled;
        self
    }

    /// Set default array size
    pub fn default_array_size(mut self, size: usize) -> Self {
        self.default_array_size = size;
        self
    }

    /// Set maximum array size
    pub fn max_array_size(mut self, size: usize) -> Self {
        self.max_array_size = size;
        self
    }

    /// Control whether to include optional fields
    pub fn include_optional_fields(mut self, include: bool) -> Self {
        self.include_optional_fields = include;
        self
    }

    /// Add a custom field mapping
    pub fn field_mapping(mut self, field_name: String, faker_type: String) -> Self {
        self.field_mappings.insert(field_name, faker_type);
        self
    }

    /// Enable/disable data validation
    pub fn validate_generated_data(mut self, validate: bool) -> Self {
        self.validate_generated_data = validate;
        self
    }

    /// Enable/disable backstory generation for personas
    pub fn enable_backstories(mut self, enable: bool) -> Self {
        self.enable_backstories = enable;
        self
    }
}

/// Enhanced mock data generator with intelligent schema analysis
#[derive(Debug)]
pub struct MockDataGenerator {
    /// Configuration for the generator
    config: MockGeneratorConfig,
    /// Enhanced faker instance
    faker: EnhancedFaker,
    /// Field name patterns for intelligent mapping
    field_patterns: HashMap<String, String>,
    /// Persona registry for consistent persona-based generation
    persona_registry: Option<Arc<PersonaRegistry>>,
    /// Consistency store for entity ID → persona mappings
    consistency_store: Option<Arc<ConsistencyStore>>,
    /// Active domain for persona-based generation
    active_domain: Option<Domain>,
}

impl MockDataGenerator {
    /// Create a new mock data generator with default configuration
    pub fn new() -> Self {
        Self::with_config(MockGeneratorConfig::new())
    }

    /// Create a new mock data generator with custom configuration
    pub fn with_config(config: MockGeneratorConfig) -> Self {
        let mut generator = Self {
            config,
            faker: EnhancedFaker::new(),
            field_patterns: Self::create_field_patterns(),
            persona_registry: None,
            consistency_store: None,
            active_domain: None,
        };

        // Initialize with common schema patterns
        generator.initialize_common_schemas();
        generator
    }

    /// Create a new mock data generator with persona support
    pub fn with_persona_support(config: MockGeneratorConfig, domain: Option<Domain>) -> Self {
        let persona_registry = Arc::new(PersonaRegistry::new());
        let consistency_store =
            Arc::new(ConsistencyStore::with_registry_and_domain(persona_registry.clone(), domain));

        let mut generator = Self {
            config,
            faker: EnhancedFaker::new(),
            field_patterns: Self::create_field_patterns(),
            persona_registry: Some(persona_registry),
            consistency_store: Some(consistency_store),
            active_domain: domain,
        };

        // Initialize with common schema patterns
        generator.initialize_common_schemas();
        generator
    }

    /// Set the active domain for persona-based generation
    pub fn set_active_domain(&mut self, domain: Option<Domain>) {
        self.active_domain = domain;
        // Note: ConsistencyStore doesn't have a setter for default domain,
        // so we just update the active_domain field which is used when generating values
    }

    /// Get the persona registry
    pub fn persona_registry(&self) -> Option<&Arc<PersonaRegistry>> {
        self.persona_registry.as_ref()
    }

    /// Get the consistency store
    pub fn consistency_store(&self) -> Option<&Arc<ConsistencyStore>> {
        self.consistency_store.as_ref()
    }

    /// Generate mock data from an OpenAPI specification
    pub fn generate_from_openapi_spec(&mut self, spec: &Value) -> Result<MockDataResult> {
        info!("Generating mock data from OpenAPI specification");

        // Parse the OpenAPI spec info
        let spec_info = self.parse_openapi_spec_info(spec)?;

        // Extract all schemas from the spec
        let schemas = self.extract_schemas_from_spec(spec)?;

        // Generate mock data for each schema
        let mut generated_data = HashMap::new();
        let mut warnings = Vec::new();

        for (schema_name, schema_def) in schemas {
            debug!("Generating data for schema: {}", schema_name);

            match self.generate_schema_data(&schema_def) {
                Ok(data) => {
                    generated_data.insert(schema_name, data);
                }
                Err(e) => {
                    let warning =
                        format!("Failed to generate data for schema '{}': {}", schema_name, e);
                    warn!("{}", warning);
                    warnings.push(warning);
                }
            }
        }

        // Generate mock responses for each endpoint
        // Parse paths directly from the JSON spec since parse_openapi_spec doesn't parse them
        let mut mock_responses = HashMap::new();
        if let Some(paths) = spec.get("paths") {
            if let Some(paths_obj) = paths.as_object() {
                for (path, path_item) in paths_obj {
                    if let Some(path_obj) = path_item.as_object() {
                        for (method, operation) in path_obj {
                            if let Some(op_obj) = operation.as_object() {
                                let endpoint_key = format!("{} {}", method.to_uppercase(), path);

                                // Extract response schema from the operation
                                if let Some(responses) = op_obj.get("responses") {
                                    if let Some(resp_obj) = responses.as_object() {
                                        // Look for 200, 201, or any 2xx response
                                        let mut response_schema = None;

                                        // Try 200 first
                                        if let Some(response) = resp_obj.get("200") {
                                            response_schema = self
                                                .extract_response_schema_from_json(response)
                                                .ok()
                                                .flatten();
                                        }

                                        // Try 201 if 200 not found
                                        if response_schema.is_none() {
                                            if let Some(response) = resp_obj.get("201") {
                                                response_schema = self
                                                    .extract_response_schema_from_json(response)
                                                    .ok()
                                                    .flatten();
                                            }
                                        }

                                        // Try any 2xx if still not found
                                        if response_schema.is_none() {
                                            for (status_code, response) in resp_obj {
                                                if let Ok(code) = status_code.parse::<u16>() {
                                                    if (200..300).contains(&code) {
                                                        if let Some(schema) = self
                                                            .extract_response_schema_from_json(
                                                                response,
                                                            )
                                                            .ok()
                                                            .flatten()
                                                        {
                                                            response_schema = Some(schema);
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        // Generate mock response if we found a schema
                                        if let Some(schema) = response_schema {
                                            // Resolve $ref if present
                                            let resolved_schema = if let Some(ref_path) =
                                                schema.get("$ref").and_then(|r| r.as_str())
                                            {
                                                self.resolve_schema_ref(spec, ref_path)?
                                            } else {
                                                Some(schema)
                                            };

                                            if let Some(resolved) = resolved_schema {
                                                if let Ok(mock_data) =
                                                    self.generate_from_json_schema(&resolved)
                                                {
                                                    mock_responses.insert(
                                                        endpoint_key,
                                                        MockResponse {
                                                            status: 200,
                                                            headers: HashMap::new(),
                                                            body: mock_data,
                                                        },
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(MockDataResult {
            schemas: generated_data,
            responses: mock_responses,
            warnings,
            spec_info,
        })
    }

    /// Generate mock data from a JSON Schema
    pub fn generate_from_json_schema(&mut self, schema: &Value) -> Result<Value> {
        debug!("Generating mock data from JSON Schema");

        // Convert JSON Schema to our internal schema format
        let schema_def = SchemaDefinition::from_json_schema(schema)?;

        // Generate data using our enhanced generator
        self.generate_schema_data(&schema_def)
    }

    /// Generate mock data for a specific schema definition
    fn generate_schema_data(&mut self, schema: &SchemaDefinition) -> Result<Value> {
        let mut object = serde_json::Map::new();

        for field in &schema.fields {
            // Skip optional fields if configured to do so
            if !field.required && !self.config.include_optional_fields {
                continue;
            }

            // Determine the best faker type for this field
            let faker_type = self.determine_faker_type(field);

            // Generate the value
            let value = self.generate_field_value(field, &faker_type)?;

            // Validate the generated value if configured
            if self.config.validate_generated_data {
                field.validate_value(&value)?;
            }

            object.insert(field.name.clone(), value);
        }

        Ok(Value::Object(object))
    }

    /// Extract schema from an OpenAPI response (JSON format)
    fn extract_response_schema_from_json(&self, response: &Value) -> Result<Option<Value>> {
        // Check for content -> application/json -> schema
        if let Some(content) = response.get("content") {
            if let Some(json_content) = content.get("application/json") {
                if let Some(schema) = json_content.get("schema") {
                    // Handle $ref references
                    if let Some(ref_path) = schema.get("$ref").and_then(|r| r.as_str()) {
                        // Extract schema name from $ref (e.g., "#/components/schemas/User" -> "User")
                        if let Some(schema_name) = ref_path.split('/').next_back() {
                            // We'll need to resolve this from components, but for now return the ref
                            // The caller should handle resolving from components
                            return Ok(Some(json!({
                                "$ref": ref_path,
                                "schema_name": schema_name
                            })));
                        }
                    }
                    return Ok(Some(schema.clone()));
                }
            }
        }
        Ok(None)
    }

    /// Resolve a $ref reference to an actual schema
    fn resolve_schema_ref(&self, spec: &Value, ref_path: &str) -> Result<Option<Value>> {
        // Handle #/components/schemas/Name format
        if ref_path.starts_with("#/components/schemas/") {
            let schema_name = ref_path.strip_prefix("#/components/schemas/").unwrap();
            if let Some(components) = spec.get("components") {
                if let Some(schemas) = components.get("schemas") {
                    if let Some(schema) = schemas.get(schema_name) {
                        return Ok(Some(schema.clone()));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Determine the best faker type for a field based on its name and type
    fn determine_faker_type(&self, field: &FieldDefinition) -> String {
        let field_name = field.name.to_lowercase();

        // Check custom field mappings first
        if let Some(mapped_type) = self.config.field_mappings.get(&field_name) {
            return mapped_type.clone();
        }

        // Use field name patterns for intelligent mapping
        // Find the longest matching pattern to prioritize more specific matches
        // Also prioritize certain patterns (like "email" over "address")
        let mut best_match: Option<(&String, &String)> = None;
        let priority_patterns = ["email", "mail"]; // Patterns that should take precedence

        for (pattern, faker_type) in &self.field_patterns {
            if field_name.contains(pattern) {
                // Check if this is a priority pattern
                let is_priority = priority_patterns.contains(&pattern.as_str());

                if let Some((best_pattern, _best_faker_type)) = best_match {
                    let best_is_priority = priority_patterns.contains(&best_pattern.as_str());

                    // Priority patterns always win, or longer patterns win
                    if is_priority && !best_is_priority {
                        best_match = Some((pattern, faker_type));
                    } else if !is_priority && best_is_priority {
                        // Keep the priority match
                    } else if pattern.len() > best_pattern.len() {
                        best_match = Some((pattern, faker_type));
                    }
                } else {
                    best_match = Some((pattern, faker_type));
                }
            }
        }

        if let Some((_, faker_type)) = best_match {
            return faker_type.clone();
        }

        // Fall back to field type
        field.field_type.clone()
    }

    /// Generate a value for a specific field
    fn generate_field_value(&mut self, field: &FieldDefinition, faker_type: &str) -> Result<Value> {
        // Note: Automatic persona-based generation from field names would require
        // entity ID values from request context (path params, query params, body).
        // For now, use explicit generate_with_persona() for persona-based generation.
        // Automatic detection can be enhanced in the future when request context is available.

        // Use faker template if provided
        if let Some(template) = &field.faker_template {
            return Ok(self.faker.generate_by_type(template));
        }

        // Handle array generation specially
        if field.field_type == "array" {
            return self.generate_array_value(field);
        }

        // Handle nested object generation specially
        if field.field_type == "object" && field.constraints.contains_key("properties") {
            return self.generate_object_value(field);
        }

        // Generate based on determined faker type
        let value = self.faker.generate_by_type(faker_type);

        // Apply constraints if present
        self.apply_constraints(&value, field)
    }

    /// Generate an array value for a field
    fn generate_array_value(&mut self, field: &FieldDefinition) -> Result<Value> {
        // Determine array size from constraints or use defaults
        let min_items =
            field.constraints.get("minItems").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let max_items = field
            .constraints
            .get("maxItems")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.config.max_array_size as u64) as usize;

        // Use default array size if no constraints
        let array_size = if min_items > 0 || max_items < self.config.max_array_size {
            // Use a size within the constraints
            let size = if min_items > 0 {
                min_items.max(self.config.default_array_size)
            } else {
                self.config.default_array_size
            };
            size.min(max_items.max(min_items))
        } else {
            self.config.default_array_size
        };

        // Generate array of items
        let mut array = Vec::new();

        // Check if we have a full items schema (for objects, nested arrays, etc.)
        if let Some(items_schema) = field.constraints.get("itemsSchema") {
            // Generate items from the schema recursively
            let items_schema_def = SchemaDefinition::from_json_schema(items_schema)?;
            for _ in 0..array_size {
                let item = self.generate_schema_data(&items_schema_def)?;
                array.push(item);
            }
        } else {
            // Simple type - use faker
            let items_type =
                field.constraints.get("itemsType").and_then(|v| v.as_str()).unwrap_or("string");

            for _ in 0..array_size {
                let item = self.faker.generate_by_type(items_type);
                array.push(item);
            }
        }

        Ok(Value::Array(array))
    }

    /// Generate an object value for a field with nested properties
    fn generate_object_value(&mut self, field: &FieldDefinition) -> Result<Value> {
        // Get nested properties from constraints
        let properties = field
            .constraints
            .get("properties")
            .ok_or_else(|| Error::generic("Object field missing properties constraint"))?;

        // Get required fields if present
        let required_fields: Vec<String> = field
            .constraints
            .get("required")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        // Create a nested schema from the properties
        let nested_schema = SchemaDefinition::from_json_schema(&json!({
            "type": "object",
            "properties": properties,
            "required": required_fields
        }))?;

        // Generate the nested object recursively
        self.generate_schema_data(&nested_schema)
    }

    /// Generate data with explicit persona support
    ///
    /// Generates data for a schema using a specific entity ID and domain.
    /// This ensures the same entity ID always generates the same data pattern.
    /// If backstories are enabled, automatically generates backstories for personas.
    pub fn generate_with_persona(
        &mut self,
        entity_id: &str,
        domain: Domain,
        schema: &SchemaDefinition,
    ) -> Result<Value> {
        // Ensure consistency store is available
        let store = self.consistency_store.as_ref().ok_or_else(|| {
            Error::generic("Persona support not enabled. Use with_persona_support() to create generator with persona support.")
        })?;

        // Generate backstory if enabled
        if self.config.enable_backstories {
            self.ensure_persona_backstory(store, entity_id, domain)?;
        }

        let mut object = serde_json::Map::new();

        for field in &schema.fields {
            // Skip optional fields if configured to do so
            if !field.required && !self.config.include_optional_fields {
                continue;
            }

            // Determine the best faker type for this field
            let faker_type = self.determine_faker_type(field);

            // Generate value using persona-based generation
            let value = match store.generate_consistent_value(entity_id, &faker_type, Some(domain))
            {
                Ok(v) => v,
                Err(_) => {
                    // Fallback to regular generation
                    self.faker.generate_by_type(&faker_type)
                }
            };

            // Validate the generated value if configured
            if self.config.validate_generated_data {
                field.validate_value(&value)?;
            }

            object.insert(field.name.clone(), value);
        }

        Ok(Value::Object(object))
    }

    /// Ensure a persona has a backstory, generating one if needed
    ///
    /// Checks if the persona has a backstory, and if not, generates one
    /// using the PersonaTemplateRegistry and BackstoryGenerator.
    fn ensure_persona_backstory(
        &self,
        store: &ConsistencyStore,
        entity_id: &str,
        domain: Domain,
    ) -> Result<()> {
        let persona_registry = store.persona_registry();
        let persona = store.get_entity_persona(entity_id, Some(domain));

        // If persona already has a backstory, no need to generate
        if persona.has_backstory() {
            return Ok(());
        }

        // Generate traits using template if persona doesn't have traits
        let mut persona_mut = persona.clone();
        if persona_mut.traits.is_empty() {
            let template_registry = PersonaTemplateRegistry::new();
            template_registry.apply_template_to_persona(&mut persona_mut)?;
        }

        // Generate backstory using BackstoryGenerator
        let backstory_generator = BackstoryGenerator::new();
        match backstory_generator.generate_backstory(&persona_mut) {
            Ok(backstory) => {
                // Update persona in registry with traits and backstory
                let mut traits = HashMap::new();
                for (key, value) in &persona_mut.traits {
                    traits.insert(key.clone(), value.clone());
                }

                // Update traits first
                if !traits.is_empty() {
                    persona_registry.update_persona(entity_id, traits)?;
                }

                // Update backstory
                persona_registry.update_persona_backstory(entity_id, backstory)?;
            }
            Err(e) => {
                warn!("Failed to generate backstory for persona {}: {}", entity_id, e);
            }
        }

        Ok(())
    }

    /// Apply constraints to a generated value
    fn apply_constraints(&mut self, value: &Value, field: &FieldDefinition) -> Result<Value> {
        let mut constrained_value = value.clone();

        // Apply numeric constraints
        if let Value::Number(num) = value {
            // Check if field type is integer to preserve integer type when applying constraints
            let is_integer_field = field.field_type == "int" || field.field_type == "integer";

            if let Some(minimum) = field.constraints.get("minimum") {
                if let Some(min_val) = minimum.as_f64() {
                    if num.as_f64().unwrap_or(0.0) < min_val {
                        // Preserve integer type if field is integer
                        if is_integer_field {
                            constrained_value = json!(min_val as i64);
                        } else {
                            constrained_value = json!(min_val);
                        }
                    }
                }
            }

            if let Some(maximum) = field.constraints.get("maximum") {
                if let Some(max_val) = maximum.as_f64() {
                    if num.as_f64().unwrap_or(0.0) > max_val {
                        // Preserve integer type if field is integer
                        if is_integer_field {
                            constrained_value = json!(max_val as i64);
                        } else {
                            constrained_value = json!(max_val);
                        }
                    }
                }
            }
        }

        // Apply string constraints
        if let Value::String(s) = value {
            let mut constrained_string = s.clone();

            // Apply min/max length constraints
            if let Some(min_length) = field.constraints.get("minLength") {
                if let Some(min_len) = min_length.as_u64() {
                    if constrained_string.len() < min_len as usize {
                        // Pad with random characters
                        let padding_needed = min_len as usize - constrained_string.len();
                        let padding = self.faker.string(padding_needed);
                        constrained_string = format!("{}{}", constrained_string, padding);
                    }
                }
            }

            if let Some(max_length) = field.constraints.get("maxLength") {
                if let Some(max_len) = max_length.as_u64() {
                    if constrained_string.len() > max_len as usize {
                        constrained_string.truncate(max_len as usize);
                    }
                }
            }

            constrained_value = json!(constrained_string);
        }

        // Apply enum constraints
        if let Some(enum_values) = field.constraints.get("enum") {
            if let Some(enum_array) = enum_values.as_array() {
                if !enum_array.is_empty() {
                    if let Some(random_value) = self.faker.random_element(enum_array) {
                        constrained_value = random_value.clone();
                    }
                }
            }
        }

        Ok(constrained_value)
    }

    /// Parse OpenAPI specification info section
    fn parse_openapi_spec_info(&self, spec: &Value) -> Result<OpenApiInfo> {
        let spec_obj = spec
            .as_object()
            .ok_or_else(|| Error::generic("Invalid OpenAPI specification"))?;

        let info = spec_obj
            .get("info")
            .ok_or_else(|| Error::generic("Missing 'info' section in OpenAPI spec"))?;

        let title = info.get("title").and_then(|t| t.as_str()).unwrap_or("Unknown API").to_string();

        let version = info.get("version").and_then(|v| v.as_str()).unwrap_or("1.0.0").to_string();

        let description = info.get("description").and_then(|d| d.as_str()).map(|s| s.to_string());

        Ok(OpenApiInfo {
            title,
            version,
            description,
        })
    }

    /// Extract schemas from OpenAPI specification
    fn extract_schemas_from_spec(
        &mut self,
        spec: &Value,
    ) -> Result<HashMap<String, SchemaDefinition>> {
        let mut schemas = HashMap::new();

        // Extract component schemas
        if let Some(components) = spec.get("components") {
            if let Some(schemas_section) = components.get("schemas") {
                if let Some(schema_obj) = schemas_section.as_object() {
                    for (name, schema_def) in schema_obj {
                        let schema = SchemaDefinition::from_json_schema(schema_def)?;
                        schemas.insert(name.clone(), schema);
                    }
                }
            }
        }

        // Extract schemas from paths
        if let Some(paths) = spec.get("paths") {
            if let Some(paths_obj) = paths.as_object() {
                for (path, path_item) in paths_obj {
                    if let Some(path_obj) = path_item.as_object() {
                        for (method, operation) in path_obj {
                            if let Some(op_obj) = operation.as_object() {
                                // Extract request body schemas
                                if let Some(request_body) = op_obj.get("requestBody") {
                                    if let Some(content) = request_body.get("content") {
                                        if let Some(json_content) = content.get("application/json")
                                        {
                                            if let Some(schema) = json_content.get("schema") {
                                                let schema_name = format!(
                                                    "{}_{}_request",
                                                    path.replace("/", "_").trim_start_matches("_"),
                                                    method
                                                );
                                                let schema_def =
                                                    SchemaDefinition::from_json_schema(schema)?;
                                                schemas.insert(schema_name, schema_def);
                                            }
                                        }
                                    }
                                }

                                // Extract response schemas
                                if let Some(responses) = op_obj.get("responses") {
                                    if let Some(resp_obj) = responses.as_object() {
                                        for (status_code, response) in resp_obj {
                                            if let Some(content) = response.get("content") {
                                                if let Some(json_content) =
                                                    content.get("application/json")
                                                {
                                                    if let Some(schema) = json_content.get("schema")
                                                    {
                                                        let schema_name = format!(
                                                            "{}_{}_response_{}",
                                                            path.replace("/", "_")
                                                                .trim_start_matches("_"),
                                                            method,
                                                            status_code
                                                        );
                                                        let schema_def =
                                                            SchemaDefinition::from_json_schema(
                                                                schema,
                                                            )?;
                                                        schemas.insert(schema_name, schema_def);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(schemas)
    }

    /// Create field name patterns for intelligent mapping
    fn create_field_patterns() -> HashMap<String, String> {
        let mut patterns = HashMap::new();

        // Email patterns
        patterns.insert("email".to_string(), "email".to_string());
        patterns.insert("mail".to_string(), "email".to_string());

        // Name patterns
        patterns.insert("name".to_string(), "name".to_string());
        patterns.insert("firstname".to_string(), "name".to_string());
        patterns.insert("lastname".to_string(), "name".to_string());
        patterns.insert("username".to_string(), "name".to_string());

        // Phone patterns
        patterns.insert("phone".to_string(), "phone".to_string());
        patterns.insert("mobile".to_string(), "phone".to_string());
        patterns.insert("telephone".to_string(), "phone".to_string());

        // Address patterns
        patterns.insert("address".to_string(), "address".to_string());
        patterns.insert("street".to_string(), "address".to_string());
        patterns.insert("city".to_string(), "string".to_string());
        patterns.insert("state".to_string(), "string".to_string());
        patterns.insert("zip".to_string(), "string".to_string());
        patterns.insert("postal".to_string(), "string".to_string());

        // Company patterns
        patterns.insert("company".to_string(), "company".to_string());
        patterns.insert("organization".to_string(), "company".to_string());
        patterns.insert("corp".to_string(), "company".to_string());

        // URL patterns
        patterns.insert("url".to_string(), "url".to_string());
        patterns.insert("website".to_string(), "url".to_string());
        patterns.insert("link".to_string(), "url".to_string());

        // Date patterns
        patterns.insert("date".to_string(), "date".to_string());
        patterns.insert("created".to_string(), "date".to_string());
        patterns.insert("updated".to_string(), "date".to_string());
        patterns.insert("timestamp".to_string(), "date".to_string());

        // ID patterns
        patterns.insert("id".to_string(), "uuid".to_string());
        patterns.insert("uuid".to_string(), "uuid".to_string());
        patterns.insert("guid".to_string(), "uuid".to_string());

        // IP patterns
        patterns.insert("ip".to_string(), "ip".to_string());
        patterns.insert("ipv4".to_string(), "ip".to_string());
        patterns.insert("ipv6".to_string(), "ip".to_string());

        patterns
    }

    /// Initialize common schemas
    fn initialize_common_schemas(&mut self) {
        // Add common schema patterns here
        // This could include User, Product, Order, etc.
    }
}

impl Default for MockDataGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of mock data generation
#[derive(Debug, Clone, serde::Serialize)]
pub struct MockDataResult {
    /// Generated data for each schema
    pub schemas: HashMap<String, Value>,
    /// Generated mock responses for each endpoint
    pub responses: HashMap<String, MockResponse>,
    /// Warnings encountered during generation
    pub warnings: Vec<String>,
    /// OpenAPI specification info
    pub spec_info: OpenApiInfo,
}

/// Mock response data
#[derive(Debug, Clone, serde::Serialize)]
pub struct MockResponse {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Value,
}

/// OpenAPI specification info
#[derive(Debug, Clone, serde::Serialize)]
pub struct OpenApiInfo {
    /// API title
    pub title: String,
    /// API version
    pub version: String,
    /// API description
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_generator_config_default() {
        let config = MockGeneratorConfig::default();

        assert!(config.realistic_mode);
        assert_eq!(config.default_array_size, 3);
        assert_eq!(config.max_array_size, 10);
        assert!(config.include_optional_fields);
        assert!(config.validate_generated_data);
    }

    #[test]
    fn test_mock_generator_config_custom() {
        let config = MockGeneratorConfig::new()
            .realistic_mode(false)
            .default_array_size(5)
            .max_array_size(20)
            .include_optional_fields(false)
            .field_mapping("email".to_string(), "email".to_string())
            .validate_generated_data(false);

        assert!(!config.realistic_mode);
        assert_eq!(config.default_array_size, 5);
        assert_eq!(config.max_array_size, 20);
        assert!(!config.include_optional_fields);
        assert!(!config.validate_generated_data);
        assert!(config.field_mappings.contains_key("email"));
    }

    #[test]
    fn test_mock_data_generator_new() {
        let generator = MockDataGenerator::new();

        assert!(generator.config.realistic_mode);
        assert!(!generator.field_patterns.is_empty());
    }

    #[test]
    fn test_mock_data_generator_with_config() {
        let config = MockGeneratorConfig::new().realistic_mode(false).default_array_size(10);

        let generator = MockDataGenerator::with_config(config);

        assert!(!generator.config.realistic_mode);
        assert_eq!(generator.config.default_array_size, 10);
    }

    #[test]
    fn test_determine_faker_type_custom_mapping() {
        let mut config = MockGeneratorConfig::new();
        config.field_mappings.insert("user_email".to_string(), "email".to_string());

        let generator = MockDataGenerator::with_config(config);

        let field = FieldDefinition::new("user_email".to_string(), "string".to_string());
        let faker_type = generator.determine_faker_type(&field);

        assert_eq!(faker_type, "email");
    }

    #[test]
    fn test_determine_faker_type_pattern_matching() {
        let generator = MockDataGenerator::new();

        let field = FieldDefinition::new("email_address".to_string(), "string".to_string());
        let faker_type = generator.determine_faker_type(&field);

        assert_eq!(faker_type, "email");
    }

    #[test]
    fn test_determine_faker_type_fallback() {
        let generator = MockDataGenerator::new();

        let field = FieldDefinition::new("unknown_field".to_string(), "integer".to_string());
        let faker_type = generator.determine_faker_type(&field);

        assert_eq!(faker_type, "integer");
    }

    #[test]
    fn test_field_patterns_creation() {
        let patterns = MockDataGenerator::create_field_patterns();

        assert!(patterns.contains_key("email"));
        assert!(patterns.contains_key("name"));
        assert!(patterns.contains_key("phone"));
        assert!(patterns.contains_key("address"));
        assert!(patterns.contains_key("company"));
        assert!(patterns.contains_key("url"));
        assert!(patterns.contains_key("date"));
        assert!(patterns.contains_key("id"));
        assert!(patterns.contains_key("ip"));
    }

    #[test]
    fn test_generate_from_json_schema_simple() {
        let mut generator = MockDataGenerator::new();

        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer" },
                "email": { "type": "string" }
            },
            "required": ["name", "age"]
        });

        let result = generator.generate_from_json_schema(&schema).unwrap();

        assert!(result.is_object());
        let obj = result.as_object().unwrap();
        assert!(obj.contains_key("name"));
        assert!(obj.contains_key("age"));
        assert!(obj.contains_key("email"));
    }

    #[test]
    fn test_generate_from_json_schema_with_constraints() {
        let mut generator = MockDataGenerator::new();

        let schema = json!({
            "type": "object",
            "properties": {
                "age": {
                    "type": "integer",
                    "minimum": 18,
                    "maximum": 65
                },
                "name": {
                    "type": "string",
                    "minLength": 5,
                    "maxLength": 20
                }
            }
        });

        let result = generator.generate_from_json_schema(&schema).unwrap();

        assert!(result.is_object());
        let obj = result.as_object().unwrap();

        if let Some(age) = obj.get("age") {
            if let Some(age_num) = age.as_i64() {
                assert!(age_num >= 18);
                assert!(age_num <= 65);
            }
        }
    }

    #[test]
    fn test_generate_from_json_schema_with_enum() {
        let mut generator = MockDataGenerator::new();

        let schema = json!({
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["active", "inactive", "pending"]
                }
            }
        });

        let result = generator.generate_from_json_schema(&schema).unwrap();

        assert!(result.is_object());
        let obj = result.as_object().unwrap();

        if let Some(status) = obj.get("status") {
            if let Some(status_str) = status.as_str() {
                assert!(["active", "inactive", "pending"].contains(&status_str));
            }
        }
    }
}
