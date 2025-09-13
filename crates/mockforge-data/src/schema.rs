//! Schema definitions for data generation

use crate::faker::EnhancedFaker;
use mockforge_core::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Field definition for data generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    /// Field name
    pub name: String,
    /// Field type
    pub field_type: String,
    /// Whether the field is required
    pub required: bool,
    /// Default value (optional)
    pub default: Option<Value>,
    /// Additional constraints
    pub constraints: HashMap<String, Value>,
    /// Faker template (optional)
    pub faker_template: Option<String>,
    /// Field description for RAG
    pub description: Option<String>,
}

impl FieldDefinition {
    /// Create a new field definition
    pub fn new(name: String, field_type: String) -> Self {
        Self {
            name,
            field_type,
            required: true,
            default: None,
            constraints: HashMap::new(),
            faker_template: None,
            description: None,
        }
    }

    /// Mark field as optional
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    /// Set default value
    pub fn with_default(mut self, default: Value) -> Self {
        self.default = Some(default);
        self
    }

    /// Add a constraint
    pub fn with_constraint(mut self, key: String, value: Value) -> Self {
        self.constraints.insert(key, value);
        self
    }

    /// Set faker template
    pub fn with_faker_template(mut self, template: String) -> Self {
        self.faker_template = Some(template);
        self
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Generate a value for this field
    pub fn generate_value(&self, faker: &mut EnhancedFaker) -> Value {
        // Use faker template if provided
        if let Some(template) = &self.faker_template {
            return faker.generate_by_type(template);
        }

        // Use default value if available and field is not required
        if !self.required {
            if let Some(default) = &self.default {
                return default.clone();
            }
        }

        // Generate based on field type
        faker.generate_by_type(&self.field_type)
    }

    /// Validate a generated value against constraints
    pub fn validate_value(&self, value: &Value) -> Result<()> {
        // Check required constraint
        if self.required && value.is_null() {
            return Err(Error::generic(format!("Required field '{}' is null", self.name)));
        }

        // Check type constraints - use field_type as primary, fall back to constraints
        let expected_type = self
            .constraints
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.field_type);

        let actual_type = match value {
            Value::String(_) => "string",
            Value::Number(_) => match expected_type {
                "integer" => "integer",
                _ => "number",
            },
            Value::Bool(_) => "boolean",
            Value::Object(_) => "object",
            Value::Array(_) => "array",
            Value::Null => "null",
        };

        // Normalize expected type for comparison
        let normalized_expected = match expected_type {
            "integer" => "integer",
            "number" => "number",
            "string" => "string",
            "boolean" => "boolean",
            "object" => "object",
            "array" => "array",
            _ => expected_type,
        };

        if normalized_expected != actual_type
            && !(normalized_expected == "number" && actual_type == "integer")
        {
            return Err(Error::generic(format!(
                "Field '{}' type mismatch: expected {}, got {}",
                self.name, normalized_expected, actual_type
            )));
        }

        // Check min/max constraints for numbers
        if let Value::Number(num) = value {
            if let Some(min_val) = self.constraints.get("minimum") {
                if let Some(min_num) = min_val.as_f64() {
                    if num.as_f64().unwrap_or(0.0) < min_num {
                        return Err(Error::generic(format!(
                            "Field '{}' value {} is less than minimum {}",
                            self.name, num, min_num
                        )));
                    }
                }
            }

            if let Some(max_val) = self.constraints.get("maximum") {
                if let Some(max_num) = max_val.as_f64() {
                    if num.as_f64().unwrap_or(0.0) > max_num {
                        return Err(Error::generic(format!(
                            "Field '{}' value {} is greater than maximum {}",
                            self.name, num, max_num
                        )));
                    }
                }
            }
        }

        // Check string constraints
        if let Value::String(s) = value {
            if let Some(min_len) = self.constraints.get("minLength") {
                if let Some(min_len_num) = min_len.as_u64() {
                    if s.len() < min_len_num as usize {
                        return Err(Error::generic(format!(
                            "Field '{}' length {} is less than minimum {}",
                            self.name,
                            s.len(),
                            min_len_num
                        )));
                    }
                }
            }

            if let Some(max_len) = self.constraints.get("maxLength") {
                if let Some(max_len_num) = max_len.as_u64() {
                    if s.len() > max_len_num as usize {
                        return Err(Error::generic(format!(
                            "Field '{}' length {} is greater than maximum {}",
                            self.name,
                            s.len(),
                            max_len_num
                        )));
                    }
                }
            }
        }

        Ok(())
    }
}

/// Schema definition for data generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDefinition {
    /// Schema name
    pub name: String,
    /// Schema description
    pub description: Option<String>,
    /// Field definitions
    pub fields: Vec<FieldDefinition>,
    /// Relationships to other schemas
    pub relationships: HashMap<String, Relationship>,
    /// Additional metadata
    pub metadata: HashMap<String, Value>,
}

impl SchemaDefinition {
    /// Create a new schema definition
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            fields: Vec::new(),
            relationships: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a field to the schema
    pub fn with_field(mut self, field: FieldDefinition) -> Self {
        self.fields.push(field);
        self
    }

    /// Add multiple fields to the schema
    pub fn with_fields(mut self, fields: Vec<FieldDefinition>) -> Self {
        self.fields.extend(fields);
        self
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Add a relationship
    pub fn with_relationship(mut self, name: String, relationship: Relationship) -> Self {
        self.relationships.insert(name, relationship);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Generate a single row of data
    pub fn generate_row(&self, faker: &mut EnhancedFaker) -> Result<Value> {
        let mut row = serde_json::Map::new();

        for field in &self.fields {
            let value = field.generate_value(faker);
            field.validate_value(&value)?;
            row.insert(field.name.clone(), value);
        }

        Ok(Value::Object(row))
    }

    /// Get field by name
    pub fn get_field(&self, name: &str) -> Option<&FieldDefinition> {
        self.fields.iter().find(|field| field.name == name)
    }

    /// Create schema from JSON Schema
    pub fn from_json_schema(json_schema: &Value) -> Result<Self> {
        let title = json_schema
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("GeneratedSchema")
            .to_string();

        let description =
            json_schema.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());

        let mut schema = Self::new(title);
        if let Some(desc) = description {
            schema = schema.with_description(desc);
        }

        if let Some(properties) = json_schema.get("properties") {
            if let Some(props_obj) = properties.as_object() {
                for (name, prop_def) in props_obj {
                    let field_type = extract_type_from_json_schema(prop_def);
                    let mut field = FieldDefinition::new(name.clone(), field_type);

                    // Check if required
                    if let Some(required) = json_schema.get("required") {
                        if let Some(required_arr) = required.as_array() {
                            let is_required = required_arr.iter().any(|v| v.as_str() == Some(name));
                            if !is_required {
                                field = field.optional();
                            }
                        }
                    }

                    // Add description
                    if let Some(desc) = prop_def.get("description").and_then(|v| v.as_str()) {
                        field = field.with_description(desc.to_string());
                    }

                    // Add constraints
                    if let Some(minimum) = prop_def.get("minimum") {
                        field = field.with_constraint("minimum".to_string(), minimum.clone());
                    }
                    if let Some(maximum) = prop_def.get("maximum") {
                        field = field.with_constraint("maximum".to_string(), maximum.clone());
                    }
                    if let Some(min_length) = prop_def.get("minLength") {
                        field = field.with_constraint("minLength".to_string(), min_length.clone());
                    }
                    if let Some(max_length) = prop_def.get("maxLength") {
                        field = field.with_constraint("maxLength".to_string(), max_length.clone());
                    }

                    schema = schema.with_field(field);
                }
            }
        }

        Ok(schema)
    }

    /// Create schema from OpenAPI spec
    pub fn from_openapi_spec(openapi_spec: &Value) -> Result<Self> {
        // Validate that it's a valid OpenAPI spec
        if !openapi_spec.is_object() {
            return Err(Error::generic("OpenAPI spec must be a JSON object"));
        }

        let spec = openapi_spec.as_object().unwrap();

        // Extract API title
        let title = spec
            .get("info")
            .and_then(|info| info.get("title"))
            .and_then(|title| title.as_str())
            .unwrap_or("OpenAPI Generated Schema")
            .to_string();

        // Extract description
        let description = spec
            .get("info")
            .and_then(|info| info.get("description"))
            .and_then(|desc| desc.as_str())
            .map(|s| s.to_string());

        let mut schema = Self::new(title);
        if let Some(desc) = description {
            schema = schema.with_description(desc);
        }

        // Parse paths and extract schemas
        if let Some(paths) = spec.get("paths").and_then(|p| p.as_object()) {
            for (path, path_item) in paths {
                if let Some(path_obj) = path_item.as_object() {
                    // Extract schemas from all operations on this path
                    for (method, operation) in path_obj {
                        if let Some(op_obj) = operation.as_object() {
                            // Extract request body schema
                            if let Some(request_body) = op_obj.get("requestBody") {
                                if let Some(rb_obj) = request_body.as_object() {
                                    if let Some(content) = rb_obj.get("content") {
                                        if let Some(json_content) = content.get("application/json")
                                        {
                                            if let Some(schema_obj) = json_content.get("schema") {
                                                let field_name = format!(
                                                    "{}_{}_request",
                                                    path.replace("/", "_").trim_start_matches("_"),
                                                    method
                                                );
                                                if let Some(field) =
                                                    Self::create_field_from_openapi_schema(
                                                        &field_name,
                                                        schema_obj,
                                                    )
                                                {
                                                    schema = schema.with_field(field);
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Extract response schemas
                            if let Some(responses) = op_obj.get("responses") {
                                if let Some(resp_obj) = responses.as_object() {
                                    // Focus on success responses (200, 201, etc.)
                                    for (status_code, response) in resp_obj {
                                        if status_code == "200"
                                            || status_code == "201"
                                            || status_code.starts_with("2")
                                        {
                                            if let Some(resp_obj) = response.as_object() {
                                                if let Some(content) = resp_obj.get("content") {
                                                    if let Some(json_content) =
                                                        content.get("application/json")
                                                    {
                                                        if let Some(schema_obj) =
                                                            json_content.get("schema")
                                                        {
                                                            let field_name = format!(
                                                                "{}_{}_response_{}",
                                                                path.replace("/", "_")
                                                                    .trim_start_matches("_"),
                                                                method,
                                                                status_code
                                                            );
                                                            if let Some(field) = Self::create_field_from_openapi_schema(&field_name, schema_obj) {
                                                                schema = schema.with_field(field);
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
            }
        }

        // Also extract component schemas if they exist
        if let Some(components) = spec.get("components") {
            if let Some(comp_obj) = components.as_object() {
                if let Some(schemas) = comp_obj.get("schemas") {
                    if let Some(schema_obj) = schemas.as_object() {
                        for (name, schema_def) in schema_obj {
                            if let Some(field) =
                                Self::create_field_from_openapi_schema(name, schema_def)
                            {
                                schema = schema.with_field(field);
                            }
                        }
                    }
                }
            }
        }

        Ok(schema)
    }

    /// Create a field definition from an OpenAPI schema
    fn create_field_from_openapi_schema(name: &str, schema: &Value) -> Option<FieldDefinition> {
        if !schema.is_object() {
            return None;
        }

        let schema_obj = schema.as_object().unwrap();

        // Determine field type
        let field_type = if let Some(type_val) = schema_obj.get("type") {
            if let Some(type_str) = type_val.as_str() {
                match type_str {
                    "string" => "string".to_string(),
                    "number" => "float".to_string(),
                    "integer" => "int".to_string(),
                    "boolean" => "boolean".to_string(),
                    "object" => "object".to_string(),
                    "array" => "array".to_string(),
                    _ => "string".to_string(),
                }
            } else {
                "string".to_string()
            }
        } else {
            "string".to_string()
        };

        let mut field = FieldDefinition::new(name.to_string(), field_type);

        // Set description
        if let Some(desc) = schema_obj.get("description").and_then(|d| d.as_str()) {
            field = field.with_description(desc.to_string());
        }

        // Mark as required if not explicitly optional
        if let Some(required) = schema_obj.get("required") {
            if let Some(required_arr) = required.as_array() {
                if !required_arr.iter().any(|v| v.as_str() == Some(name)) {
                    field = field.optional();
                }
            }
        }

        // Add constraints
        if let Some(minimum) = schema_obj.get("minimum") {
            field = field.with_constraint("minimum".to_string(), minimum.clone());
        }
        if let Some(maximum) = schema_obj.get("maximum") {
            field = field.with_constraint("maximum".to_string(), maximum.clone());
        }
        if let Some(min_length) = schema_obj.get("minLength") {
            field = field.with_constraint("minLength".to_string(), min_length.clone());
        }
        if let Some(max_length) = schema_obj.get("maxLength") {
            field = field.with_constraint("maxLength".to_string(), max_length.clone());
        }

        // Handle enum values
        if let Some(enum_vals) = schema_obj.get("enum") {
            if let Some(_enum_arr) = enum_vals.as_array() {
                field = field.with_constraint("enum".to_string(), enum_vals.clone());
            }
        }

        Some(field)
    }
}

/// Relationship definition between schemas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Target schema name
    pub target_schema: String,
    /// Relationship type
    pub relationship_type: RelationshipType,
    /// Foreign key field name
    pub foreign_key: String,
    /// Whether this is a required relationship
    pub required: bool,
}

impl Relationship {
    /// Create a new relationship
    pub fn new(
        target_schema: String,
        relationship_type: RelationshipType,
        foreign_key: String,
    ) -> Self {
        Self {
            target_schema,
            relationship_type,
            foreign_key,
            required: true,
        }
    }

    /// Mark relationship as optional
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }
}

/// Type of relationship between schemas
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RelationshipType {
    /// One-to-one relationship
    OneToOne,
    /// One-to-many relationship
    OneToMany,
    /// Many-to-one relationship
    ManyToOne,
    /// Many-to-many relationship
    ManyToMany,
}

/// Extract type from JSON Schema property definition
fn extract_type_from_json_schema(prop_def: &Value) -> String {
    if let Some(type_val) = prop_def.get("type") {
        if let Some(type_str) = type_val.as_str() {
            return match type_str {
                "string" => "string".to_string(),
                "number" => "float".to_string(),
                "integer" => "int".to_string(),
                "boolean" => "boolean".to_string(),
                "object" => "object".to_string(),
                "array" => "array".to_string(),
                "null" => "null".to_string(),
                _ => "string".to_string(),
            };
        }
    }

    // Default to string if type is not specified
    "string".to_string()
}

/// Common schema templates
pub mod templates {
    use super::*;

    /// Create a user schema
    pub fn user_schema() -> SchemaDefinition {
        SchemaDefinition::new("User".to_string())
            .with_description("User account information".to_string())
            .with_fields(vec![
                FieldDefinition::new("id".to_string(), "uuid".to_string()),
                FieldDefinition::new("email".to_string(), "email".to_string()),
                FieldDefinition::new("name".to_string(), "name".to_string()),
                FieldDefinition::new("created_at".to_string(), "date".to_string()),
                FieldDefinition::new("active".to_string(), "boolean".to_string()),
            ])
    }

    /// Create a product schema
    pub fn product_schema() -> SchemaDefinition {
        SchemaDefinition::new("Product".to_string())
            .with_description("Product catalog item".to_string())
            .with_fields(vec![
                FieldDefinition::new("id".to_string(), "uuid".to_string()),
                FieldDefinition::new("name".to_string(), "string".to_string()),
                FieldDefinition::new("description".to_string(), "paragraph".to_string()),
                FieldDefinition::new("price".to_string(), "float".to_string())
                    .with_constraint("minimum".to_string(), Value::Number(0.into())),
                FieldDefinition::new("category".to_string(), "string".to_string()),
                FieldDefinition::new("in_stock".to_string(), "boolean".to_string()),
            ])
    }

    /// Create an order schema with relationship to user
    pub fn order_schema() -> SchemaDefinition {
        SchemaDefinition::new("Order".to_string())
            .with_description("Customer order".to_string())
            .with_fields(vec![
                FieldDefinition::new("id".to_string(), "uuid".to_string()),
                FieldDefinition::new("user_id".to_string(), "uuid".to_string()),
                FieldDefinition::new("total_amount".to_string(), "float".to_string())
                    .with_constraint("minimum".to_string(), Value::Number(0.into())),
                FieldDefinition::new("status".to_string(), "string".to_string()),
                FieldDefinition::new("created_at".to_string(), "date".to_string()),
            ])
            .with_relationship(
                "user".to_string(),
                Relationship::new(
                    "User".to_string(),
                    RelationshipType::ManyToOne,
                    "user_id".to_string(),
                ),
            )
    }
}
