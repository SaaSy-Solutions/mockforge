//! Schema builder for preview and editing
//!
//! Provides functionality to build, preview, and edit generated OpenAPI schemas
//! before deployment.

use crate::generative_schema::{EntityDefinition, RouteDefinition};
use crate::OpenApiSpec;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Schema preview with metadata
#[derive(Debug, Clone)]
pub struct SchemaPreview {
    /// Generated OpenAPI spec
    pub spec: OpenApiSpec,
    /// Generated OpenAPI spec as JSON
    pub spec_json: Value,
    /// Entity definitions used
    pub entities: Vec<EntityDefinition>,
    /// Generated routes
    pub routes: Vec<RouteDefinition>,
    /// Generation metadata
    pub metadata: GenerationMetadata,
}

impl Serialize for SchemaPreview {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SchemaPreview", 5)?;
        state.serialize_field("spec_json", &self.spec_json)?;
        state.serialize_field("entities", &self.entities)?;
        state.serialize_field("routes", &self.routes)?;
        state.serialize_field("metadata", &self.metadata)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for SchemaPreview {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct SchemaPreviewVisitor;

        impl<'de> Visitor<'de> for SchemaPreviewVisitor {
            type Value = SchemaPreview;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct SchemaPreview")
            }

            fn visit_map<V>(self, mut map: V) -> Result<SchemaPreview, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut spec_json: Option<Value> = None;
                let mut entities: Option<Vec<EntityDefinition>> = None;
                let mut routes: Option<Vec<RouteDefinition>> = None;
                let mut metadata: Option<GenerationMetadata> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "spec_json" => {
                            if spec_json.is_some() {
                                return Err(de::Error::duplicate_field("spec_json"));
                            }
                            spec_json = Some(map.next_value()?);
                        }
                        "entities" => {
                            if entities.is_some() {
                                return Err(de::Error::duplicate_field("entities"));
                            }
                            entities = Some(map.next_value()?);
                        }
                        "routes" => {
                            if routes.is_some() {
                                return Err(de::Error::duplicate_field("routes"));
                            }
                            routes = Some(map.next_value()?);
                        }
                        "metadata" => {
                            if metadata.is_some() {
                                return Err(de::Error::duplicate_field("metadata"));
                            }
                            metadata = Some(map.next_value()?);
                        }
                        _ => {
                            let _ = map.next_value::<de::IgnoredAny>()?;
                        }
                    }
                }

                let spec_json = spec_json.ok_or_else(|| de::Error::missing_field("spec_json"))?;
                let entities = entities.ok_or_else(|| de::Error::missing_field("entities"))?;
                let routes = routes.ok_or_else(|| de::Error::missing_field("routes"))?;
                let metadata = metadata.ok_or_else(|| de::Error::missing_field("metadata"))?;

                // Reconstruct OpenApiSpec from JSON
                let spec = OpenApiSpec::from_json(spec_json.clone()).map_err(|e| {
                    de::Error::custom(format!("Failed to parse OpenAPI spec: {}", e))
                })?;

                Ok(SchemaPreview {
                    spec,
                    spec_json,
                    entities,
                    routes,
                    metadata,
                })
            }
        }

        deserializer.deserialize_map(SchemaPreviewVisitor)
    }
}

/// Generation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationMetadata {
    /// Number of entities inferred
    pub entity_count: usize,
    /// Number of routes generated
    pub route_count: usize,
    /// Number of relationships detected
    pub relationship_count: usize,
    /// Generation timestamp
    pub generated_at: String,
}

/// Schema builder
pub struct SchemaBuilder;

impl SchemaBuilder {
    /// Build OpenAPI spec from entities and routes
    pub fn build_spec(
        entities: Vec<EntityDefinition>,
        routes: Vec<RouteDefinition>,
        title: Option<String>,
        version: Option<String>,
    ) -> Result<OpenApiSpec, crate::Error> {
        let mut spec_json = json!({
            "openapi": "3.0.0",
            "info": {
                "title": title.unwrap_or_else(|| "Generated API".to_string()),
                "version": version.unwrap_or_else(|| "1.0.0".to_string()),
            },
            "paths": {},
            "components": {
                "schemas": {}
            }
        });

        // Add paths
        if let Some(paths) = spec_json.get_mut("paths").and_then(|p| p.as_object_mut()) {
            for route in &routes {
                let path_entry = paths.entry(route.path.clone()).or_insert_with(|| json!({}));
                if let Some(path_obj) = path_entry.as_object_mut() {
                    let method = route.method.to_lowercase();
                    path_obj.insert(method.clone(), json!({
                        "summary": route.description,
                        "operationId": format!("{}_{}", method, route.entity.to_lowercase()),
                        "responses": {
                            "200": {
                                "description": "Success",
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "$ref": format!("#/components/schemas/{}", route.entity)
                                        }
                                    }
                                }
                            }
                        }
                    }));
                }
            }
        }

        // Add schemas
        if let Some(schemas) = spec_json
            .get_mut("components")
            .and_then(|c| c.as_object_mut())
            .and_then(|c| c.get_mut("schemas"))
            .and_then(|s| s.as_object_mut())
        {
            for entity in &entities {
                schemas.insert(entity.name.clone(), entity.schema.clone());
            }
        }

        OpenApiSpec::from_json(spec_json)
    }

    /// Create preview from entities and routes
    pub fn create_preview(
        entities: Vec<EntityDefinition>,
        routes: Vec<RouteDefinition>,
        title: Option<String>,
        version: Option<String>,
    ) -> Result<SchemaPreview, crate::Error> {
        let spec = Self::build_spec(entities.clone(), routes.clone(), title, version)?;

        let relationship_count = entities.iter().map(|e| e.relationships.len()).sum::<usize>();

        let metadata = GenerationMetadata {
            entity_count: entities.len(),
            route_count: routes.len(),
            relationship_count,
            generated_at: chrono::Utc::now().to_rfc3339(),
        };

        // Convert spec to JSON for serialization
        let spec_json = if let Some(ref raw) = spec.raw_document {
            raw.clone()
        } else {
            serde_json::to_value(&spec.spec).map_err(|e| {
                crate::Error::generic(format!("Failed to serialize OpenAPI spec: {}", e))
            })?
        };

        Ok(SchemaPreview {
            spec,
            spec_json,
            entities,
            routes,
            metadata,
        })
    }
}
