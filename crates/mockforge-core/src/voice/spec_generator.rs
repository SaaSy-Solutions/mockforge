//! OpenAPI spec generator from parsed voice commands
//!
//! This module generates OpenAPI 3.0 specifications from parsed voice commands
//! using the existing OpenApiSpec infrastructure.

use crate::openapi::OpenApiSpec;
use crate::Result;
use openapiv3::*;
use serde_json::Value;

use super::command_parser::{EndpointRequirement, ModelRequirement, ParsedCommand};

/// Voice spec generator that creates OpenAPI specs from parsed commands
pub struct VoiceSpecGenerator;

impl VoiceSpecGenerator {
    /// Create a new voice spec generator
    pub fn new() -> Self {
        Self
    }

    /// Generate OpenAPI spec from a parsed command
    pub async fn generate_spec(&self, parsed: &ParsedCommand) -> Result<OpenApiSpec> {
        // Create base OpenAPI structure
        let mut spec = OpenAPI {
            openapi: "3.0.3".to_string(),
            info: Info {
                title: parsed.title.clone(),
                version: "1.0.0".to_string(),
                description: Some(parsed.description.clone()),
                ..Default::default()
            },
            paths: Paths {
                paths: indexmap::IndexMap::new(),
                ..Default::default()
            },
            components: Some(Components {
                schemas: indexmap::IndexMap::new(),
                ..Default::default()
            }),
            ..Default::default()
        };

        // Generate schemas from models
        if let Some(ref mut components) = spec.components {
            for model in &parsed.models {
                let schema = self.model_to_schema(model);
                components.schemas.insert(model.name.clone(), ReferenceOr::Item(schema));
            }
        }

        // Generate paths from endpoints
        for endpoint in &parsed.endpoints {
            self.add_endpoint_to_spec(&mut spec, endpoint, &parsed.models)?;
        }

        // Convert to OpenApiSpec
        let spec_json = serde_json::to_value(&spec)?;
        OpenApiSpec::from_json(spec_json)
    }

    /// Generate OpenAPI spec by merging with existing spec (for conversational mode)
    pub async fn merge_spec(
        &self,
        existing: &OpenApiSpec,
        parsed: &ParsedCommand,
    ) -> Result<OpenApiSpec> {
        // Start with existing spec
        let mut spec_json = serde_json::to_value(&existing.spec)?;

        // Add new endpoints
        for endpoint in &parsed.endpoints {
            self.add_endpoint_to_json(&mut spec_json, endpoint, &parsed.models)?;
        }

        // Add new models/schemas
        if let Some(components) = spec_json.get_mut("components") {
            if let Some(schemas) = components.get_mut("schemas") {
                for model in &parsed.models {
                    let schema = self.model_to_schema(model);
                    let schema_value = serde_json::to_value(&schema)?;
                    schemas[model.name.clone()] = schema_value;
                }
            }
        }

        OpenApiSpec::from_json(spec_json)
    }

    /// Convert a model requirement to an OpenAPI schema
    fn model_to_schema(&self, model: &ModelRequirement) -> Schema {
        let mut properties = indexmap::IndexMap::new();
        let mut required = Vec::new();

        for field in &model.fields {
            let schema_data = SchemaData {
                title: Some(field.name.clone()),
                description: Some(field.description.clone()),
                ..Default::default()
            };

            let schema_kind = match field.r#type.as_str() {
                "string" => SchemaKind::Type(Type::String(StringType::default())),
                "number" => SchemaKind::Type(Type::Number(NumberType {
                    format: VariantOrUnknownOrEmpty::Empty,
                    minimum: None,
                    maximum: None,
                    exclusive_minimum: false,
                    exclusive_maximum: false,
                    multiple_of: None,
                    enumeration: vec![],
                })),
                "integer" => SchemaKind::Type(Type::Integer(IntegerType {
                    format: VariantOrUnknownOrEmpty::Empty,
                    minimum: None,
                    maximum: None,
                    exclusive_minimum: false,
                    exclusive_maximum: false,
                    multiple_of: None,
                    enumeration: vec![],
                })),
                "boolean" => SchemaKind::Type(Type::Boolean(BooleanType {
                    enumeration: vec![],
                })),
                "array" => SchemaKind::Type(Type::Array(ArrayType {
                    items: Some(ReferenceOr::Item(Box::new(Schema {
                        schema_data: SchemaData::default(),
                        schema_kind: SchemaKind::Type(Type::String(StringType::default())),
                    }))),
                    min_items: None,
                    max_items: None,
                    unique_items: false,
                })),
                "object" => SchemaKind::Type(Type::Object(ObjectType {
                    properties: indexmap::IndexMap::new(),
                    required: vec![],
                    additional_properties: None,
                    ..Default::default()
                })),
                _ => SchemaKind::Type(Type::String(StringType::default())),
            };

            properties.insert(
                field.name.clone(),
                ReferenceOr::Item(Box::new(Schema {
                    schema_data,
                    schema_kind,
                })),
            );

            if field.required {
                required.push(field.name.clone());
            }
        }

        Schema {
            schema_data: SchemaData {
                title: Some(model.name.clone()),
                ..Default::default()
            },
            schema_kind: SchemaKind::Type(Type::Object(ObjectType {
                properties,
                required,
                additional_properties: None,
                ..Default::default()
            })),
        }
    }

    /// Add an endpoint to the OpenAPI spec
    fn add_endpoint_to_spec(
        &self,
        spec: &mut OpenAPI,
        endpoint: &EndpointRequirement,
        models: &[ModelRequirement],
    ) -> Result<()> {
        // Get or create path item
        let path_item = spec
            .paths
            .paths
            .entry(endpoint.path.clone())
            .or_insert_with(|| ReferenceOr::Item(PathItem::default()));

        let path_item = match path_item {
            ReferenceOr::Item(item) => item,
            ReferenceOr::Reference { .. } => {
                return Err(crate::Error::generic("Path reference not supported"));
            }
        };

        // Create operation
        let mut operation = Operation {
            summary: Some(endpoint.description.clone()),
            description: Some(endpoint.description.clone()),
            ..Default::default()
        };

        // Add request body if present
        if let Some(ref request_body) = endpoint.request_body {
            operation.request_body = Some(ReferenceOr::Item(RequestBody {
                description: None,
                content: {
                    let mut content = indexmap::IndexMap::new();
                    let schema = if let Some(ref schema) = request_body.schema {
                        self.json_value_to_schema(schema)
                    } else {
                        // Default to object schema
                        Schema {
                            schema_data: SchemaData::default(),
                            schema_kind: SchemaKind::Type(Type::Object(ObjectType {
                                properties: indexmap::IndexMap::new(),
                                required: vec![],
                                additional_properties: None,
                                ..Default::default()
                            })),
                        }
                    };

                    content.insert(
                        "application/json".to_string(),
                        MediaType {
                            schema: Some(ReferenceOr::Item(schema)),
                            ..Default::default()
                        },
                    );
                    content
                },
                required: !request_body.required.is_empty(),
                extensions: indexmap::IndexMap::new(),
            }));
        }

        // Add response
        if let Some(ref response) = endpoint.response {
            let _status_code = response.status.to_string();
            let is_array = response.is_array;
            let schema = if let Some(ref schema_value) = response.schema {
                self.json_value_to_schema(schema_value)
            } else if is_array {
                // Array response - try to infer from endpoint path
                let item_schema = self.infer_schema_from_path(&endpoint.path, models);
                Schema {
                    schema_data: SchemaData::default(),
                    schema_kind: SchemaKind::Type(Type::Array(ArrayType {
                        items: Some(ReferenceOr::Item(Box::new(item_schema))),
                        min_items: None,
                        max_items: None,
                        unique_items: false,
                    })),
                }
            } else {
                // Single object response
                self.infer_schema_from_path(&endpoint.path, models)
            };

            operation.responses = Responses {
                responses: {
                    let mut responses = indexmap::IndexMap::new();
                    let status =
                        StatusCode::Code(response.status.to_string().parse::<u16>().unwrap_or(200));
                    responses.insert(
                        status,
                        ReferenceOr::Item(Response {
                            description: format!("{} response", endpoint.method),
                            content: {
                                let mut content = indexmap::IndexMap::new();
                                content.insert(
                                    "application/json".to_string(),
                                    MediaType {
                                        schema: Some(ReferenceOr::Item(schema)),
                                        ..Default::default()
                                    },
                                );
                                content
                            },
                            ..Default::default()
                        }),
                    );
                    responses
                },
                ..Default::default()
            };
        } else {
            // Default 200 response
            operation.responses = Responses {
                responses: {
                    let mut responses = indexmap::IndexMap::new();
                    responses.insert(
                        StatusCode::Code(200),
                        ReferenceOr::Item(Response {
                            description: "Success".to_string(),
                            ..Default::default()
                        }),
                    );
                    responses
                },
                ..Default::default()
            };
        }

        // Add operation to path item based on method
        match endpoint.method.to_uppercase().as_str() {
            "GET" => path_item.get = Some(operation),
            "POST" => path_item.post = Some(operation),
            "PUT" => path_item.put = Some(operation),
            "DELETE" => path_item.delete = Some(operation),
            "PATCH" => path_item.patch = Some(operation),
            _ => {
                return Err(crate::Error::generic(format!(
                    "Unsupported HTTP method: {}",
                    endpoint.method
                )));
            }
        }

        Ok(())
    }

    /// Add endpoint to JSON spec (for merging)
    fn add_endpoint_to_json(
        &self,
        spec_json: &mut Value,
        endpoint: &EndpointRequirement,
        models: &[ModelRequirement],
    ) -> Result<()> {
        let paths = spec_json
            .get_mut("paths")
            .and_then(|p| p.as_object_mut())
            .ok_or_else(|| crate::Error::generic("Invalid spec JSON structure"))?;

        let path_item = paths
            .entry(endpoint.path.clone())
            .or_insert_with(|| Value::Object(serde_json::Map::new()));

        let path_obj = path_item
            .as_object_mut()
            .ok_or_else(|| crate::Error::generic("Invalid path item"))?;

        // Create operation object
        let mut operation = serde_json::Map::new();
        operation.insert("summary".to_string(), Value::String(endpoint.description.clone()));
        operation.insert("description".to_string(), Value::String(endpoint.description.clone()));

        // Add request body if present
        if let Some(ref request_body) = endpoint.request_body {
            let mut req_body = serde_json::Map::new();
            if let Some(ref schema) = request_body.schema {
                req_body.insert("content".to_string(), {
                    let mut content = serde_json::Map::new();
                    content.insert(
                        "application/json".to_string(),
                        Value::Object({
                            let mut media_type = serde_json::Map::new();
                            media_type.insert("schema".to_string(), schema.clone());
                            media_type
                        }),
                    );
                    Value::Object(content)
                });
            }
            operation.insert("requestBody".to_string(), Value::Object(req_body));
        }

        // Add response
        let mut responses = serde_json::Map::new();
        let status_code = endpoint
            .response
            .as_ref()
            .map(|r| r.status.to_string())
            .unwrap_or_else(|| "200".to_string());

        let mut response_obj = serde_json::Map::new();
        response_obj.insert("description".to_string(), Value::String("Success".to_string()));

        if endpoint.response.as_ref().map(|r| r.is_array).unwrap_or(false) {
            let schema = self.infer_schema_from_path(&endpoint.path, models);
            let schema_value = serde_json::to_value(&schema)?;
            response_obj.insert(
                "content".to_string(),
                Value::Object({
                    let mut content = serde_json::Map::new();
                    content.insert(
                        "application/json".to_string(),
                        Value::Object({
                            let mut media_type = serde_json::Map::new();
                            media_type.insert(
                                "schema".to_string(),
                                Value::Object({
                                    let mut array_schema = serde_json::Map::new();
                                    array_schema.insert(
                                        "type".to_string(),
                                        Value::String("array".to_string()),
                                    );
                                    array_schema.insert("items".to_string(), schema_value);
                                    array_schema
                                }),
                            );
                            media_type
                        }),
                    );
                    content
                }),
            );
        }

        responses.insert(status_code, Value::Object(response_obj));
        operation.insert("responses".to_string(), Value::Object(responses));

        // Add to path item
        path_obj.insert(endpoint.method.to_lowercase(), Value::Object(operation));

        Ok(())
    }

    /// Infer schema from endpoint path (e.g., /api/products -> Product model)
    fn infer_schema_from_path(&self, path: &str, models: &[ModelRequirement]) -> Schema {
        // Try to find a model that matches the path
        // e.g., /api/products -> Product model
        let path_lower = path.to_lowercase();
        for model in models {
            let model_lower = model.name.to_lowercase();
            if path_lower.contains(&model_lower) {
                return self.model_to_schema(model);
            }
        }

        // Default to generic object schema
        Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::Type(Type::Object(ObjectType {
                properties: indexmap::IndexMap::new(),
                ..Default::default()
            })),
        }
    }

    /// Convert JSON value to OpenAPI schema
    fn json_value_to_schema(&self, value: &Value) -> Schema {
        match value {
            Value::Object(obj) => {
                let mut properties = indexmap::IndexMap::new();
                for (key, val) in obj {
                    properties.insert(
                        key.clone(),
                        ReferenceOr::Item(Box::new(self.json_value_to_schema(val))),
                    );
                }
                Schema {
                    schema_data: SchemaData::default(),
                    schema_kind: SchemaKind::Type(Type::Object(ObjectType {
                        properties,
                        required: vec![],
                        additional_properties: None,
                        ..Default::default()
                    })),
                }
            }
            Value::Array(arr) => {
                let item_schema = if arr.is_empty() {
                    Schema {
                        schema_data: SchemaData::default(),
                        schema_kind: SchemaKind::Type(Type::String(StringType::default())),
                    }
                } else {
                    self.json_value_to_schema(&arr[0])
                };
                Schema {
                    schema_data: SchemaData::default(),
                    schema_kind: SchemaKind::Type(Type::Array(ArrayType {
                        items: Some(ReferenceOr::Item(Box::new(item_schema))),
                        min_items: None,
                        max_items: None,
                        unique_items: false,
                    })),
                }
            }
            Value::String(_) => Schema {
                schema_data: SchemaData::default(),
                schema_kind: SchemaKind::Type(Type::String(StringType::default())),
            },
            Value::Number(n) => {
                if n.is_i64() {
                    Schema {
                        schema_data: SchemaData::default(),
                        schema_kind: SchemaKind::Type(Type::Integer(IntegerType {
                            format: VariantOrUnknownOrEmpty::Empty,
                            minimum: None,
                            maximum: None,
                            exclusive_minimum: false,
                            exclusive_maximum: false,
                            multiple_of: None,
                            enumeration: vec![],
                        })),
                    }
                } else {
                    Schema {
                        schema_data: SchemaData::default(),
                        schema_kind: SchemaKind::Type(Type::Number(NumberType {
                            format: VariantOrUnknownOrEmpty::Empty,
                            minimum: None,
                            maximum: None,
                            exclusive_minimum: false,
                            exclusive_maximum: false,
                            multiple_of: None,
                            enumeration: vec![],
                        })),
                    }
                }
            }
            Value::Bool(_) => Schema {
                schema_data: SchemaData::default(),
                schema_kind: SchemaKind::Type(Type::Boolean(BooleanType {
                    enumeration: vec![],
                })),
            },
            Value::Null => Schema {
                schema_data: SchemaData::default(),
                schema_kind: SchemaKind::Type(Type::String(StringType::default())),
            },
        }
    }
}

impl Default for VoiceSpecGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::command_parser::{
        EndpointRequirement, FieldRequirement, ModelRequirement, ParsedCommand,
        RequestBodyRequirement, ResponseRequirement,
    };

    #[test]
    fn test_voice_spec_generator_new() {
        let generator = VoiceSpecGenerator::new();
        // Just verify it can be created
        let _ = generator;
    }

    #[test]
    fn test_voice_spec_generator_default() {
        let generator = VoiceSpecGenerator::default();
        // Just verify it can be created
        let _ = generator;
    }

    #[tokio::test]
    async fn test_generate_spec_basic() {
        let generator = VoiceSpecGenerator::new();
        let parsed = ParsedCommand {
            api_type: "test".to_string(),
            title: "Test API".to_string(),
            description: "A test API".to_string(),
            endpoints: vec![],
            models: vec![],
            relationships: vec![],
            sample_counts: std::collections::HashMap::new(),
            flows: vec![],
        };

        let spec = generator.generate_spec(&parsed).await.unwrap();
        assert_eq!(spec.title(), "Test API");
    }

    #[tokio::test]
    async fn test_generate_spec_with_model() {
        let generator = VoiceSpecGenerator::new();
        let model = ModelRequirement {
            name: "Product".to_string(),
            fields: vec![
                FieldRequirement {
                    name: "id".to_string(),
                    r#type: "integer".to_string(),
                    description: "Product ID".to_string(),
                    required: true,
                },
                FieldRequirement {
                    name: "name".to_string(),
                    r#type: "string".to_string(),
                    description: "Product name".to_string(),
                    required: true,
                },
            ],
        };

        let parsed = ParsedCommand {
            api_type: "e-commerce".to_string(),
            title: "Shop API".to_string(),
            description: "E-commerce API".to_string(),
            endpoints: vec![],
            models: vec![model],
            relationships: vec![],
            sample_counts: std::collections::HashMap::new(),
            flows: vec![],
        };

        let spec = generator.generate_spec(&parsed).await.unwrap();
        assert_eq!(spec.title(), "Shop API");
    }

    #[tokio::test]
    async fn test_generate_spec_with_endpoint() {
        let generator = VoiceSpecGenerator::new();
        let endpoint = EndpointRequirement {
            path: "/api/products".to_string(),
            method: "GET".to_string(),
            description: "Get products".to_string(),
            request_body: None,
            response: Some(ResponseRequirement {
                status: 200,
                schema: None,
                is_array: true,
                count: None,
            }),
        };

        let parsed = ParsedCommand {
            api_type: "e-commerce".to_string(),
            title: "Shop API".to_string(),
            description: "E-commerce API".to_string(),
            endpoints: vec![endpoint],
            models: vec![],
            relationships: vec![],
            sample_counts: std::collections::HashMap::new(),
            flows: vec![],
        };

        let spec = generator.generate_spec(&parsed).await.unwrap();
        assert_eq!(spec.title(), "Shop API");
    }

    #[tokio::test]
    async fn test_merge_spec() {
        let generator = VoiceSpecGenerator::new();

        // Create existing spec
        let existing_json = serde_json::json!({
            "openapi": "3.0.3",
            "info": {
                "title": "Existing API",
                "version": "1.0.0"
            },
            "paths": {},
            "components": {
                "schemas": {}
            }
        });
        let existing = OpenApiSpec::from_json(existing_json).unwrap();

        // Create parsed command with new endpoint
        let parsed = ParsedCommand {
            api_type: "test".to_string(),
            title: "New API".to_string(),
            description: "New API description".to_string(),
            endpoints: vec![],
            models: vec![],
            relationships: vec![],
            sample_counts: std::collections::HashMap::new(),
            flows: vec![],
        };

        let merged = generator.merge_spec(&existing, &parsed).await.unwrap();
        assert_eq!(merged.title(), "Existing API"); // Title should remain from existing
    }
}
