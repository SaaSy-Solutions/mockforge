//! Ecosystem generator - Main entry point
//!
//! Generates complete API ecosystems from JSON payloads with one-click environment creation.

use crate::generative_schema::entity_inference::EntityInference;
use crate::generative_schema::naming_rules::NamingRules;
use crate::generative_schema::route_generator::RouteGenerator;
use crate::generative_schema::schema_builder::SchemaBuilder;
use crate::generative_schema::{EntityDefinition, RouteDefinition, SchemaPreview};
use crate::OpenApiSpec;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

/// Generation options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationOptions {
    /// API title
    pub title: Option<String>,
    /// API version
    pub version: Option<String>,
    /// Naming rules
    pub naming_rules: NamingRules,
    /// Generate CRUD routes
    pub generate_crud: bool,
    /// Output directory
    pub output_dir: Option<PathBuf>,
}

impl Default for GenerationOptions {
    fn default() -> Self {
        Self {
            title: None,
            version: None,
            naming_rules: NamingRules::default(),
            generate_crud: true,
            output_dir: None,
        }
    }
}

/// Ecosystem generation result
#[derive(Debug, Clone)]
pub struct EcosystemGenerationResult {
    /// Generated OpenAPI spec
    pub spec: OpenApiSpec,
    /// Generated OpenAPI spec as JSON
    pub spec_json: Value,
    /// Entity definitions
    pub entities: Vec<EntityDefinition>,
    /// Generated routes
    pub routes: Vec<RouteDefinition>,
    /// Preview information
    pub preview: SchemaPreview,
    /// Output files created
    pub output_files: Vec<PathBuf>,
}

impl Serialize for EcosystemGenerationResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("EcosystemGenerationResult", 6)?;
        state.serialize_field("spec_json", &self.spec_json)?;
        state.serialize_field("entities", &self.entities)?;
        state.serialize_field("routes", &self.routes)?;
        state.serialize_field("preview", &self.preview)?;
        state.serialize_field("output_files", &self.output_files)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for EcosystemGenerationResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct EcosystemGenerationResultVisitor;

        impl<'de> Visitor<'de> for EcosystemGenerationResultVisitor {
            type Value = EcosystemGenerationResult;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct EcosystemGenerationResult")
            }

            fn visit_map<V>(self, mut map: V) -> Result<EcosystemGenerationResult, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut spec_json: Option<Value> = None;
                let mut entities: Option<Vec<EntityDefinition>> = None;
                let mut routes: Option<Vec<RouteDefinition>> = None;
                let mut preview: Option<SchemaPreview> = None;
                let mut output_files: Option<Vec<PathBuf>> = None;

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
                        "preview" => {
                            if preview.is_some() {
                                return Err(de::Error::duplicate_field("preview"));
                            }
                            preview = Some(map.next_value()?);
                        }
                        "output_files" => {
                            if output_files.is_some() {
                                return Err(de::Error::duplicate_field("output_files"));
                            }
                            output_files = Some(map.next_value()?);
                        }
                        _ => {
                            let _ = map.next_value::<de::IgnoredAny>()?;
                        }
                    }
                }

                let spec_json = spec_json.ok_or_else(|| de::Error::missing_field("spec_json"))?;
                let entities = entities.ok_or_else(|| de::Error::missing_field("entities"))?;
                let routes = routes.ok_or_else(|| de::Error::missing_field("routes"))?;
                let preview = preview.ok_or_else(|| de::Error::missing_field("preview"))?;
                let output_files = output_files.unwrap_or_default();

                // Reconstruct OpenApiSpec from JSON
                let spec = OpenApiSpec::from_json(spec_json.clone()).map_err(|e| {
                    de::Error::custom(format!("Failed to parse OpenAPI spec: {}", e))
                })?;

                Ok(EcosystemGenerationResult {
                    spec,
                    spec_json,
                    entities,
                    routes,
                    preview,
                    output_files,
                })
            }
        }

        deserializer.deserialize_map(EcosystemGenerationResultVisitor)
    }
}

/// Ecosystem generator
pub struct EcosystemGenerator;

impl EcosystemGenerator {
    /// Generate complete API ecosystem from JSON payloads
    pub async fn generate_from_json(
        payloads: Vec<Value>,
        options: GenerationOptions,
    ) -> Result<EcosystemGenerationResult, crate::Error> {
        // 1. Infer entities
        let entities = EntityInference::infer_entities(payloads);

        // 2. Generate routes
        let route_generator = RouteGenerator::new(options.naming_rules.clone());
        let mut routes = Vec::new();

        if options.generate_crud {
            for entity in &entities {
                routes.extend(route_generator.generate_crud_routes(&entity.name));
            }
        }

        // 3. Build schema preview
        let preview = SchemaBuilder::create_preview(
            entities.clone(),
            routes.clone(),
            options.title.clone(),
            options.version.clone(),
        )?;

        // 4. Get final spec
        let spec = preview.spec.clone();

        // 5. Convert spec to JSON for serialization
        let spec_json = if let Some(ref raw) = spec.raw_document {
            raw.clone()
        } else {
            serde_json::to_value(&spec.spec).map_err(|e| {
                crate::Error::generic(format!("Failed to serialize OpenAPI spec: {}", e))
            })?
        };

        // 6. Save output files if output directory specified
        let mut output_files = Vec::new();
        if let Some(output_dir) = &options.output_dir {
            std::fs::create_dir_all(output_dir)?;

            // Save OpenAPI spec
            let spec_path = output_dir.join("openapi.json");
            let spec_json_str = serde_json::to_string_pretty(&spec_json)?;
            std::fs::write(&spec_path, spec_json_str)?;
            output_files.push(spec_path);

            // Save entity definitions
            let entities_path = output_dir.join("entities.json");
            let entities_json = serde_json::to_string_pretty(&entities)?;
            std::fs::write(&entities_path, entities_json)?;
            output_files.push(entities_path);
        }

        Ok(EcosystemGenerationResult {
            spec,
            spec_json,
            entities,
            routes,
            preview,
            output_files,
        })
    }

    /// Generate from JSON file
    pub async fn generate_from_file(
        file_path: PathBuf,
        options: GenerationOptions,
    ) -> Result<EcosystemGenerationResult, crate::Error> {
        let content = tokio::fs::read_to_string(&file_path).await?;
        let payloads: Vec<Value> = if file_path.extension().and_then(|s| s.to_str()) == Some("yaml")
            || file_path.extension().and_then(|s| s.to_str()) == Some("yml")
        {
            serde_yaml::from_str(&content)?
        } else {
            serde_json::from_str(&content)?
        };

        Self::generate_from_json(payloads, options).await
    }
}
