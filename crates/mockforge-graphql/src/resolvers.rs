//! GraphQL resolvers for mock data generation

use async_graphql::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Mock resolver for GraphQL fields
#[derive(Debug, Clone)]
pub struct MockResolver {
    pub field_name: String,
    pub field_type: String,
    pub mock_data: Value,
    pub generator: Option<MockDataGenerator>,
}

/// Data generator for dynamic mock data
#[derive(Debug, Clone)]
pub struct MockDataGenerator {
    pub generator_type: GeneratorType,
    pub config: HashMap<String, serde_json::Value>,
}

/// Types of data generators
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GeneratorType {
    /// Generate random strings
    String { min_length: usize, max_length: usize },
    /// Generate random integers
    Int { min: i64, max: i64 },
    /// Generate random floats
    Float { min: f64, max: f64 },
    /// Generate UUIDs
    Uuid,
    /// Generate email addresses
    Email,
    /// Generate names
    Name,
    /// Generate timestamps
    Timestamp,
    /// Generate from a list of values
    FromList { values: Vec<serde_json::Value> },
    /// Generate nested objects
    Object { fields: HashMap<String, Box<GeneratorType>> },
    /// Generate arrays
    Array { item_generator: Box<GeneratorType>, min_items: usize, max_items: usize },
}

/// Registry for managing resolvers
pub struct ResolverRegistry {
    resolvers: HashMap<String, HashMap<String, MockResolver>>,
}

impl ResolverRegistry {
    /// Create a new resolver registry
    pub fn new() -> Self {
        Self {
            resolvers: HashMap::new(),
        }
    }

    /// Register a resolver for a specific type and field
    pub fn register_resolver(&mut self, type_name: &str, resolver: MockResolver) {
        self.resolvers
            .entry(type_name.to_string())
            .or_insert_with(HashMap::new)
            .insert(resolver.field_name.clone(), resolver);
    }

    /// Get a resolver for a specific type and field
    pub fn get_resolver(&self, type_name: &str, field_name: &str) -> Option<&MockResolver> {
        self.resolvers
            .get(type_name)
            .and_then(|type_resolvers| type_resolvers.get(field_name))
    }

    /// Generate mock data for a field
    pub async fn generate_mock_data(&self, type_name: &str, field_name: &str) -> Value {
        if let Some(resolver) = self.get_resolver(type_name, field_name) {
            if let Some(generator) = &resolver.generator {
                return generator.generate().await;
            }
            return resolver.mock_data.clone();
        }

        // Default mock data generation based on field name patterns
        Self::generate_default_mock_data(field_name).await
    }

    /// Generate default mock data based on field name patterns
    async fn generate_default_mock_data(field_name: &str) -> Value {
        match field_name.to_lowercase().as_str() {
            "id" => Value::String(mockforge_core::templating::expand_str("{{uuid}}")),
            "name" | "title" => Value::String(mockforge_core::templating::expand_str("{{faker.name}}")),
            "email" => Value::String(mockforge_core::templating::expand_str("{{faker.email}}")),
            "description" | "content" => Value::String(mockforge_core::templating::expand_str("{{faker.sentence}}")),
            "age" | "count" | "quantity" => Value::Number((rand::random::<u32>() % 100).into()),
            "price" | "amount" => Value::Number((rand::random::<f64>() * 1000.0).into()),
            "active" | "enabled" | "is_active" => Value::Boolean(rand::random::<bool>()),
            "created_at" | "updated_at" => Value::String(mockforge_core::templating::expand_str("{{now}}")),
            _ => Value::String(mockforge_core::templating::expand_str(&format!("{{{{faker.word}}}}"))),
        }
    }

    /// Create common resolvers for standard GraphQL types
    pub fn create_common_resolvers() -> Self {
        let mut registry = Self::new();

        // User type resolvers
        registry.register_resolver("User", MockResolver {
            field_name: "id".to_string(),
            field_type: "ID!".to_string(),
            mock_data: Value::Null,
            generator: Some(MockDataGenerator {
                generator_type: GeneratorType::Uuid,
                config: HashMap::new(),
            }),
        });

        registry.register_resolver("User", MockResolver {
            field_name: "name".to_string(),
            field_type: "String!".to_string(),
            mock_data: Value::Null,
            generator: Some(MockDataGenerator {
                generator_type: GeneratorType::Name,
                config: HashMap::new(),
            }),
        });

        registry.register_resolver("User", MockResolver {
            field_name: "email".to_string(),
            field_type: "String!".to_string(),
            mock_data: Value::Null,
            generator: Some(MockDataGenerator {
                generator_type: GeneratorType::Email,
                config: HashMap::new(),
            }),
        });

        registry.register_resolver("User", MockResolver {
            field_name: "createdAt".to_string(),
            field_type: "String!".to_string(),
            mock_data: Value::Null,
            generator: Some(MockDataGenerator {
                generator_type: GeneratorType::Timestamp,
                config: HashMap::new(),
            }),
        });

        registry
    }
}

impl MockDataGenerator {
    /// Generate mock data using this generator
    pub async fn generate(&self) -> Value {
        match &self.generator_type {
            GeneratorType::String { min_length, max_length } => {
                let length = rand::random::<usize>() % (max_length - min_length) + min_length;
                let s: String = (0..length).map(|_| rand::random::<char>()).collect();
                Value::String(s)
            }
            GeneratorType::Int { min, max } => {
                let num = rand::random::<i64>() % (max - min) + min;
                Value::Number(num.into())
            }
            GeneratorType::Float { min, max } => {
                let num = rand::random::<f64>() * (max - min) + min;
                Value::Number(serde_json::Number::from_f64(num).unwrap_or(0.into()))
            }
            GeneratorType::Uuid => {
                Value::String(mockforge_core::templating::expand_str("{{uuid}}"))
            }
            GeneratorType::Email => {
                Value::String(mockforge_core::templating::expand_str("{{faker.email}}"))
            }
            GeneratorType::Name => {
                Value::String(mockforge_core::templating::expand_str("{{faker.name}}"))
            }
            GeneratorType::Timestamp => {
                Value::String(mockforge_core::templating::expand_str("{{now}}"))
            }
            GeneratorType::FromList { values } => {
                if values.is_empty() {
                    Value::Null
                } else {
                    let index = rand::random::<usize>() % values.len();
                    serde_json::from_value(values[index].clone()).unwrap_or(Value::Null)
                }
            }
            GeneratorType::Object { fields } => {
                let mut obj = async_graphql::Value::Object(async_graphql::IndexMap::new());
                for (field_name, generator) in fields {
                    let value = generator.generate().await;
                    if let Value::Object(ref mut map) = obj {
                        map.insert(async_graphql::Name::new(field_name), value);
                    }
                }
                obj
            }
            GeneratorType::Array { item_generator, min_items, max_items } => {
                let count = rand::random::<usize>() % (max_items - min_items) + min_items;
                let mut items = Vec::new();
                for _ in 0..count {
                    items.push(item_generator.generate().await);
                }
                Value::List(items)
            }
        }
    }
}

impl Default for ResolverRegistry {
    fn default() -> Self {
        Self::new()
    }
}
