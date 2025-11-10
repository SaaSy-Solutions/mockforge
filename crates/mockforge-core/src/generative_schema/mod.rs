//! Generative Schema Mode - Complete API ecosystem generation from JSON examples
//!
//! This module provides functionality to generate entire API ecosystems from a few
//! example JSON payloads. It automatically infers entity structures, routes, and
//! relationships, creating a complete mock API ready for deployment.
//!
//! # Features
//!
//! - **JSON → API Ecosystem**: Generate complete APIs from JSON payloads
//! - **Auto-Route Generation**: Realistic CRUD mapping with proper HTTP methods
//! - **Entity Relation Inference**: Automatically detect relationships between entities
//! - **One-Click Environment Creation**: Generate and deploy in one command
//! - **Preview/Edit Support**: Review and modify generated schemas before deployment
//! - **Configurable Naming**: Custom naming and pluralization rules
//! - **Reversibility**: Regenerate schema from modified data

pub mod ecosystem_generator;
pub mod entity_inference;
pub mod naming_rules;
pub mod route_generator;
pub mod schema_builder;

pub use ecosystem_generator::{EcosystemGenerationResult, EcosystemGenerator, GenerationOptions};
pub use entity_inference::{EntityDefinition, EntityInference, RelationshipType};
pub use naming_rules::{NamingRules, PluralizationRule};
pub use route_generator::{CrudOperation, RouteDefinition, RouteGenerator};
pub use schema_builder::{SchemaBuilder, SchemaPreview};
