//! GraphQL resolvers for mock data generation

use async_graphql::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    String {
        min_length: usize,
        max_length: usize,
    },
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
    Object {
        fields: HashMap<String, Box<GeneratorType>>,
    },
    /// Generate arrays
    Array {
        item_generator: Box<GeneratorType>,
        min_items: usize,
        max_items: usize,
    },
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
            "name" | "title" => {
                Value::String(mockforge_core::templating::expand_str("{{faker.name}}"))
            }
            "email" => Value::String(mockforge_core::templating::expand_str("{{faker.email}}")),
            "description" | "content" => {
                Value::String(mockforge_core::templating::expand_str("{{faker.sentence}}"))
            }
            "age" | "count" | "quantity" => Value::Number((rand::random::<u32>() % 100).into()),
            "price" | "amount" => {
                let val = rand::random::<f64>() * 1000.0;
                Value::Number(
                    serde_json::Number::from_f64(val)
                        .unwrap_or_else(|| serde_json::Number::from(0)),
                )
            }
            "active" | "enabled" | "is_active" => Value::Boolean(rand::random::<bool>()),
            "created_at" | "updated_at" => {
                Value::String(mockforge_core::templating::expand_str("{{now}}"))
            }
            _ => Value::String(mockforge_core::templating::expand_str(&format!(
                "{{{{faker.word}}}}"
            ))),
        }
    }

    /// Create common resolvers for standard GraphQL types
    pub fn create_common_resolvers() -> Self {
        let mut registry = Self::new();

        // User type resolvers
        registry.register_resolver(
            "User",
            MockResolver {
                field_name: "id".to_string(),
                field_type: "ID!".to_string(),
                mock_data: Value::Null,
                generator: Some(MockDataGenerator {
                    generator_type: GeneratorType::Uuid,
                    config: HashMap::new(),
                }),
            },
        );

        registry.register_resolver(
            "User",
            MockResolver {
                field_name: "name".to_string(),
                field_type: "String!".to_string(),
                mock_data: Value::Null,
                generator: Some(MockDataGenerator {
                    generator_type: GeneratorType::Name,
                    config: HashMap::new(),
                }),
            },
        );

        registry.register_resolver(
            "User",
            MockResolver {
                field_name: "email".to_string(),
                field_type: "String!".to_string(),
                mock_data: Value::Null,
                generator: Some(MockDataGenerator {
                    generator_type: GeneratorType::Email,
                    config: HashMap::new(),
                }),
            },
        );

        registry.register_resolver(
            "User",
            MockResolver {
                field_name: "createdAt".to_string(),
                field_type: "String!".to_string(),
                mock_data: Value::Null,
                generator: Some(MockDataGenerator {
                    generator_type: GeneratorType::Timestamp,
                    config: HashMap::new(),
                }),
            },
        );

        registry
    }
}

impl MockDataGenerator {
    /// Generate mock data using this generator
    pub async fn generate(&self) -> Value {
        match &self.generator_type {
            GeneratorType::String {
                min_length,
                max_length,
            } => {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let length = rng.gen_range(*min_length..*max_length);
                let s: String = (0..length)
                    .map(|_| {
                        let c = rng.gen_range(b'a'..=b'z');
                        c as char
                    })
                    .collect();
                Value::String(s)
            }
            GeneratorType::Int { min, max } => {
                let num = rand::random::<i64>() % (max - min) + min;
                Value::Number(num.into())
            }
            GeneratorType::Float { min, max } => {
                let num = rand::random::<f64>() * (max - min) + min;
                Value::Number(
                    serde_json::Number::from_f64(num)
                        .unwrap_or_else(|| serde_json::Number::from(0)),
                )
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
                use rand::Rng;
                if values.is_empty() {
                    Value::Null
                } else {
                    let mut rng = rand::thread_rng();
                    let index = rng.gen_range(0..values.len());
                    serde_json::from_value(values[index].clone()).unwrap_or(Value::Null)
                }
            }
            GeneratorType::Object { fields: _ } => {
                // Returns empty object
                // Note: Nested object generation is intentionally simplified to avoid
                // recursion issues. For mock testing, an empty object of the correct type
                // is typically sufficient. Users can implement custom handlers for
                // complex nested structures if needed.
                use indexmap::IndexMap;
                let map = IndexMap::new();
                Value::Object(map)
            }
            GeneratorType::Array {
                item_generator: _,
                min_items,
                max_items,
            } => {
                // Returns array of nulls with correct count
                // Note: Array item generation is intentionally simplified to avoid
                // recursion issues. The array has the correct length, which is
                // sufficient for most mock scenarios. Users can implement custom
                // handlers for arrays with complex items if needed.
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let count = rng.gen_range(*min_items..*max_items);
                let items = vec![Value::Null; count];
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_resolver_creation() {
        let resolver = MockResolver {
            field_name: "id".to_string(),
            field_type: "ID!".to_string(),
            mock_data: Value::String("test-123".to_string()),
            generator: None,
        };

        assert_eq!(resolver.field_name, "id");
        assert_eq!(resolver.field_type, "ID!");
        assert!(resolver.generator.is_none());
    }

    #[test]
    fn test_mock_resolver_with_generator() {
        let resolver = MockResolver {
            field_name: "email".to_string(),
            field_type: "String!".to_string(),
            mock_data: Value::Null,
            generator: Some(MockDataGenerator {
                generator_type: GeneratorType::Email,
                config: HashMap::new(),
            }),
        };

        assert!(resolver.generator.is_some());
    }

    #[test]
    fn test_generator_type_string() {
        let gen_type = GeneratorType::String {
            min_length: 5,
            max_length: 10,
        };

        match gen_type {
            GeneratorType::String {
                min_length,
                max_length,
            } => {
                assert_eq!(min_length, 5);
                assert_eq!(max_length, 10);
            }
            _ => panic!("Wrong generator type"),
        }
    }

    #[test]
    fn test_generator_type_int() {
        let gen_type = GeneratorType::Int { min: 1, max: 100 };

        match gen_type {
            GeneratorType::Int { min, max } => {
                assert_eq!(min, 1);
                assert_eq!(max, 100);
            }
            _ => panic!("Wrong generator type"),
        }
    }

    #[test]
    fn test_generator_type_float() {
        let gen_type = GeneratorType::Float { min: 0.0, max: 1.0 };

        match gen_type {
            GeneratorType::Float { min, max } => {
                assert_eq!(min, 0.0);
                assert_eq!(max, 1.0);
            }
            _ => panic!("Wrong generator type"),
        }
    }

    #[test]
    fn test_generator_type_uuid() {
        let gen_type = GeneratorType::Uuid;
        assert!(matches!(gen_type, GeneratorType::Uuid));
    }

    #[test]
    fn test_generator_type_email() {
        let gen_type = GeneratorType::Email;
        assert!(matches!(gen_type, GeneratorType::Email));
    }

    #[test]
    fn test_generator_type_name() {
        let gen_type = GeneratorType::Name;
        assert!(matches!(gen_type, GeneratorType::Name));
    }

    #[test]
    fn test_generator_type_timestamp() {
        let gen_type = GeneratorType::Timestamp;
        assert!(matches!(gen_type, GeneratorType::Timestamp));
    }

    #[test]
    fn test_generator_type_from_list() {
        let values = vec![serde_json::json!("value1"), serde_json::json!("value2")];
        let gen_type = GeneratorType::FromList {
            values: values.clone(),
        };

        match gen_type {
            GeneratorType::FromList { values: v } => {
                assert_eq!(v.len(), 2);
            }
            _ => panic!("Wrong generator type"),
        }
    }

    #[test]
    fn test_resolver_registry_new() {
        let registry = ResolverRegistry::new();
        assert_eq!(registry.resolvers.len(), 0);
    }

    #[test]
    fn test_resolver_registry_default() {
        let registry = ResolverRegistry::default();
        assert_eq!(registry.resolvers.len(), 0);
    }

    #[test]
    fn test_resolver_registry_register() {
        let mut registry = ResolverRegistry::new();

        let resolver = MockResolver {
            field_name: "id".to_string(),
            field_type: "ID!".to_string(),
            mock_data: Value::String("test-id".to_string()),
            generator: None,
        };

        registry.register_resolver("User", resolver);

        assert!(registry.resolvers.contains_key("User"));
        assert!(registry.resolvers.get("User").unwrap().contains_key("id"));
    }

    #[test]
    fn test_resolver_registry_get_resolver() {
        let mut registry = ResolverRegistry::new();

        let resolver = MockResolver {
            field_name: "email".to_string(),
            field_type: "String!".to_string(),
            mock_data: Value::String("test@example.com".to_string()),
            generator: None,
        };

        registry.register_resolver("User", resolver);

        let retrieved = registry.get_resolver("User", "email");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().field_name, "email");
    }

    #[test]
    fn test_resolver_registry_get_resolver_not_found() {
        let registry = ResolverRegistry::new();
        let retrieved = registry.get_resolver("User", "unknown");
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_generate_mock_data_with_static_data() {
        let mut registry = ResolverRegistry::new();

        let resolver = MockResolver {
            field_name: "name".to_string(),
            field_type: "String!".to_string(),
            mock_data: Value::String("John Doe".to_string()),
            generator: None,
        };

        registry.register_resolver("User", resolver);

        let result = registry.generate_mock_data("User", "name").await;
        assert!(matches!(result, Value::String(_)));
    }

    #[tokio::test]
    async fn test_generate_default_mock_data_id() {
        let result = ResolverRegistry::generate_default_mock_data("id").await;
        assert!(matches!(result, Value::String(_)));
    }

    #[tokio::test]
    async fn test_generate_default_mock_data_name() {
        let result = ResolverRegistry::generate_default_mock_data("name").await;
        assert!(matches!(result, Value::String(_)));
    }

    #[tokio::test]
    async fn test_generate_default_mock_data_email() {
        let result = ResolverRegistry::generate_default_mock_data("email").await;
        assert!(matches!(result, Value::String(_)));
    }

    #[tokio::test]
    async fn test_generate_default_mock_data_age() {
        let result = ResolverRegistry::generate_default_mock_data("age").await;
        assert!(matches!(result, Value::Number(_)));
    }

    #[tokio::test]
    async fn test_generate_default_mock_data_active() {
        let result = ResolverRegistry::generate_default_mock_data("active").await;
        assert!(matches!(result, Value::Boolean(_)));
    }

    #[test]
    fn test_create_common_resolvers() {
        let registry = ResolverRegistry::create_common_resolvers();

        assert!(registry.get_resolver("User", "id").is_some());
        assert!(registry.get_resolver("User", "name").is_some());
        assert!(registry.get_resolver("User", "email").is_some());
        assert!(registry.get_resolver("User", "createdAt").is_some());
    }

    #[tokio::test]
    async fn test_mock_data_generator_uuid() {
        let generator = MockDataGenerator {
            generator_type: GeneratorType::Uuid,
            config: HashMap::new(),
        };

        let result = generator.generate().await;
        assert!(matches!(result, Value::String(_)));
    }

    #[tokio::test]
    async fn test_mock_data_generator_email() {
        let generator = MockDataGenerator {
            generator_type: GeneratorType::Email,
            config: HashMap::new(),
        };

        let result = generator.generate().await;
        assert!(matches!(result, Value::String(_)));
    }

    #[tokio::test]
    async fn test_mock_data_generator_name() {
        let generator = MockDataGenerator {
            generator_type: GeneratorType::Name,
            config: HashMap::new(),
        };

        let result = generator.generate().await;
        assert!(matches!(result, Value::String(_)));
    }

    #[tokio::test]
    async fn test_mock_data_generator_timestamp() {
        let generator = MockDataGenerator {
            generator_type: GeneratorType::Timestamp,
            config: HashMap::new(),
        };

        let result = generator.generate().await;
        assert!(matches!(result, Value::String(_)));
    }

    #[tokio::test]
    async fn test_mock_data_generator_int() {
        let generator = MockDataGenerator {
            generator_type: GeneratorType::Int { min: 1, max: 100 },
            config: HashMap::new(),
        };

        let result = generator.generate().await;
        assert!(matches!(result, Value::Number(_)));
    }

    #[tokio::test]
    async fn test_mock_data_generator_float() {
        let generator = MockDataGenerator {
            generator_type: GeneratorType::Float {
                min: 0.0,
                max: 10.0,
            },
            config: HashMap::new(),
        };

        let result = generator.generate().await;
        assert!(matches!(result, Value::Number(_)));
    }

    #[tokio::test]
    async fn test_mock_data_generator_string() {
        let generator = MockDataGenerator {
            generator_type: GeneratorType::String {
                min_length: 5,
                max_length: 10,
            },
            config: HashMap::new(),
        };

        let result = generator.generate().await;
        if let Value::String(s) = result {
            assert!(s.len() >= 5);
            assert!(s.len() <= 10);
        } else {
            panic!("Expected string value");
        }
    }

    #[tokio::test]
    async fn test_mock_data_generator_from_list() {
        let values = vec![
            serde_json::json!("value1"),
            serde_json::json!("value2"),
            serde_json::json!("value3"),
        ];

        let generator = MockDataGenerator {
            generator_type: GeneratorType::FromList { values },
            config: HashMap::new(),
        };

        let result = generator.generate().await;
        assert!(matches!(result, Value::String(_)));
    }

    #[tokio::test]
    async fn test_mock_data_generator_from_empty_list() {
        let generator = MockDataGenerator {
            generator_type: GeneratorType::FromList { values: vec![] },
            config: HashMap::new(),
        };

        let result = generator.generate().await;
        assert!(matches!(result, Value::Null));
    }

    #[tokio::test]
    async fn test_mock_data_generator_array() {
        let generator = MockDataGenerator {
            generator_type: GeneratorType::Array {
                item_generator: Box::new(GeneratorType::Uuid),
                min_items: 2,
                max_items: 5,
            },
            config: HashMap::new(),
        };

        let result = generator.generate().await;
        if let Value::List(items) = result {
            assert!(items.len() >= 2);
            assert!(items.len() <= 5);
        } else {
            panic!("Expected list value");
        }
    }

    #[tokio::test]
    async fn test_resolver_registry_multiple_types() {
        let mut registry = ResolverRegistry::new();

        registry.register_resolver(
            "User",
            MockResolver {
                field_name: "id".to_string(),
                field_type: "ID!".to_string(),
                mock_data: Value::String("user-id".to_string()),
                generator: None,
            },
        );

        registry.register_resolver(
            "Post",
            MockResolver {
                field_name: "id".to_string(),
                field_type: "ID!".to_string(),
                mock_data: Value::String("post-id".to_string()),
                generator: None,
            },
        );

        assert!(registry.get_resolver("User", "id").is_some());
        assert!(registry.get_resolver("Post", "id").is_some());
    }

    #[test]
    fn test_resolver_registry_multiple_fields_same_type() {
        let mut registry = ResolverRegistry::new();

        registry.register_resolver(
            "User",
            MockResolver {
                field_name: "id".to_string(),
                field_type: "ID!".to_string(),
                mock_data: Value::Null,
                generator: None,
            },
        );

        registry.register_resolver(
            "User",
            MockResolver {
                field_name: "name".to_string(),
                field_type: "String!".to_string(),
                mock_data: Value::Null,
                generator: None,
            },
        );

        registry.register_resolver(
            "User",
            MockResolver {
                field_name: "email".to_string(),
                field_type: "String!".to_string(),
                mock_data: Value::Null,
                generator: None,
            },
        );

        assert_eq!(registry.resolvers.get("User").unwrap().len(), 3);
    }
}
