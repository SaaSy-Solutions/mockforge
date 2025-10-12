# Advanced Data Synthesis in MockForge

MockForge provides sophisticated data synthesis capabilities that go far beyond simple random data generation. The advanced data synthesis system combines intelligent field inference, deterministic seeding, relationship-aware generation, and cross-endpoint validation to create realistic, coherent, and reproducible test data.

## Overview

The advanced data synthesis system consists of four main components:

1. **SmartMockGenerator** - Intelligent field-based mock data generation
2. **Schema Graph Extraction** - Relationship discovery from protobuf schemas
3. **RAG-Driven Synthesis** - Domain-aware data generation using Retrieval-Augmented Generation
4. **Validation Framework** - Cross-endpoint consistency and integrity validation

## SmartMockGenerator

The `SmartMockGenerator` provides intelligent mock data generation based on field names, types, and context. It supports deterministic seeding for reproducible test fixtures.

### Basic Usage

```rust
use mockforge_grpc::reflection::smart_mock_generator::{SmartMockGenerator, SmartMockConfig};

// Create a basic generator
let config = SmartMockConfig::default();
let mut generator = SmartMockGenerator::new(config);

// Generate data for a specific field
let value = generator.generate_value_for_field(&field_descriptor, "UserService", "GetUser", 0);
```

### Deterministic Generation

For reproducible test fixtures, use deterministic seeding:

```rust
// Create a deterministic generator
let mut generator = SmartMockGenerator::new_with_seed(
    SmartMockConfig::default(),
    12345 // seed value
);

// Generate reproducible data
let uuid1 = generator.generate_uuid();
let email = generator.generate_random_string(10);

// Reset to regenerate same data
generator.reset();
let uuid2 = generator.generate_uuid(); // Same as uuid1
```

### Configuration Options

```rust
let config = SmartMockConfig {
    field_name_inference: true,     // Enable intelligent field inference
    use_faker: true,               // Use realistic fake data
    field_overrides: HashMap::from([
        ("user_id".to_string(), "fixed_user_123".to_string())
    ]),
    service_profiles: HashMap::new(), // Service-specific configurations
    max_depth: 5,                   // Maximum recursion depth
    seed: Some(42),                 // Seed for deterministic generation
    deterministic: true,            // Enable deterministic mode
};
```

### Field Name Inference

The generator automatically infers appropriate data types based on field names:

- **Email fields**: `email`, `email_address` → realistic email addresses
- **Phone fields**: `phone`, `mobile`, `phone_number` → formatted phone numbers
- **ID fields**: `id`, `user_id`, `order_id` → sequential or UUID-based IDs
- **Name fields**: `name`, `first_name`, `last_name` → realistic names
- **Date/Time fields**: `created_at`, `updated_at`, `timestamp` → ISO timestamps
- **Geographic fields**: `latitude`, `longitude`, `address` → location data
- **Technical fields**: `url`, `token`, `hash`, `uuid` → appropriate formats

## Schema Graph Extraction

The schema graph extraction system analyzes protobuf definitions to discover relationships and foreign key patterns.

### Usage

```rust
use mockforge_grpc::reflection::schema_graph::ProtoSchemaGraphExtractor;
use prost_reflect::DescriptorPool;

let extractor = ProtoSchemaGraphExtractor::new();
let schema_graph = extractor.extract_from_proto(&descriptor_pool)?;

// Examine discovered relationships
for relationship in &schema_graph.relationships {
    println!("Found relationship: {} -> {} via {}",
        relationship.from_entity,
        relationship.to_entity,
        relationship.field_name
    );
}

// Check foreign key mappings
for (entity, mappings) in &schema_graph.foreign_keys {
    for mapping in mappings {
        println!("Foreign key: {}.{} -> {}",
            entity, mapping.field_name, mapping.target_entity
        );
    }
}
```

### Foreign Key Detection

The system automatically detects foreign key relationships using naming conventions:

- `user_id` → references `User` entity
- `orderId` → references `Order` entity
- `customer_ref` → references `Customer` entity

Detection confidence scores help identify the most likely relationships.

### Relationship Types

The system identifies various relationship types:

- **ForeignKey**: Direct ID references (`user_id` → `User`)
- **Embedded**: Nested message types
- **OneToMany**: Repeated field relationships
- **Composition**: Ownership relationships

## RAG-Driven Data Synthesis

The RAG (Retrieval-Augmented Generation) system uses domain knowledge to generate contextually appropriate data.

### Configuration

```rust
use mockforge_grpc::reflection::rag_synthesis::{RagDataSynthesizer, RagSynthesisConfig};

let config = RagSynthesisConfig {
    enabled: true,
    rag_config: Some(RagSynthesisRagConfig {
        api_endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
        api_key: Some("your-api-key".to_string()),
        model: "gpt-3.5-turbo".to_string(),
        embedding_model: "text-embedding-ada-002".to_string(),
        similarity_threshold: 0.7,
        max_documents: 5,
    }),
    context_sources: vec![
        ContextSource {
            id: "user_docs".to_string(),
            source_type: ContextSourceType::Documentation,
            path: "./docs/user_guide.md".to_string(),
            weight: 1.0,
            required: false,
        }
    ],
    prompt_templates: HashMap::new(),
    max_context_length: 2000,
    cache_contexts: true,
};

let mut synthesizer = RagDataSynthesizer::new(config);
```

### Business Rule Extraction

The RAG system automatically extracts business rules from documentation:

```rust
// Generate context for an entity
let context = synthesizer.generate_entity_context("User").await?;

// Check extracted business rules
for rule in &context.business_rules {
    println!("Rule: {} applies to {:?}", rule.description, rule.applies_to_fields);
}

// Use context-aware field generation
let field_value = synthesizer.synthesize_field_data("User", "email", "string").await?;
```

### Context Sources

Configure various sources of domain knowledge:

- **Documentation**: API docs, specifications
- **Examples**: Sample data files
- **Business Rules**: Constraint definitions
- **Glossary**: Domain terminology
- **Knowledge Base**: External documentation

## Validation Framework

The validation framework ensures data coherence across different endpoints and validates referential integrity.

### Basic Setup

```rust
use mockforge_grpc::reflection::validation_framework::{
    ValidationFramework, ValidationConfig, GeneratedEntity
};

let config = ValidationConfig {
    enabled: true,
    strict_mode: false,  // Don't fail on warnings
    max_validation_depth: 3,
    custom_rules: vec![],
    cache_results: true,
};

let mut validator = ValidationFramework::new(config);
```

### Entity Registration

Register generated entities for validation:

```rust
let entity = GeneratedEntity {
    entity_type: "User".to_string(),
    primary_key: Some("user_123".to_string()),
    field_values: HashMap::from([
        ("id".to_string(), "user_123".to_string()),
        ("name".to_string(), "John Doe".to_string()),
        ("email".to_string(), "john@example.com".to_string()),
    ]),
    endpoint: "/users".to_string(),
    generated_at: SystemTime::now(),
};

validator.register_entity(entity)?;
```

### Custom Validation Rules

Define custom validation rules for specific business logic:

```rust
let email_rule = CustomValidationRule {
    name: "email_format".to_string(),
    applies_to_entities: vec!["User".to_string()],
    validates_fields: vec!["email".to_string()],
    rule_type: ValidationRuleType::FieldFormat,
    parameters: HashMap::from([
        ("pattern".to_string(), r"^[^@\s]+@[^@\s]+\.[^@\s]+$".to_string())
    ]),
    error_message: "Invalid email format".to_string(),
};

let age_rule = CustomValidationRule {
    name: "age_range".to_string(),
    applies_to_entities: vec!["User".to_string()],
    validates_fields: vec!["age".to_string()],
    rule_type: ValidationRuleType::Range,
    parameters: HashMap::from([
        ("min".to_string(), "0".to_string()),
        ("max".to_string(), "120".to_string()),
    ]),
    error_message: "Age must be between 0 and 120".to_string(),
};
```

### Running Validation

```rust
// Run comprehensive validation
let result = validator.validate_all_entities();

if !result.is_valid {
    println!("Validation failed with {} errors:", result.errors.len());
    for error in &result.errors {
        println!("  {}: {}", error.entity_name, error.message);
        if let Some(fix) = &error.suggested_fix {
            println!("    Suggested fix: {}", fix);
        }
    }
}

// Check warnings
for warning in &result.warnings {
    println!("Warning: {}", warning.message);
}

// Get validation statistics
let stats = validator.get_statistics();
println!("Validated {} entities across {} types",
    stats.total_entities, stats.entity_type_count);
```

## End-to-End Workflow

Here's a complete example showing how all components work together:

```rust
use mockforge_grpc::reflection::{
    smart_mock_generator::{SmartMockGenerator, SmartMockConfig},
    schema_graph::ProtoSchemaGraphExtractor,
    rag_synthesis::{RagDataSynthesizer, RagSynthesisConfig},
    validation_framework::{ValidationFramework, ValidationConfig, GeneratedEntity},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create deterministic generator
    let mut generator = SmartMockGenerator::new_with_seed(
        SmartMockConfig::default(),
        42
    );

    // 2. Extract schema relationships
    let extractor = ProtoSchemaGraphExtractor::new();
    let schema_graph = extractor.extract_from_proto(&descriptor_pool)?;

    // 3. Setup RAG synthesizer
    let rag_config = RagSynthesisConfig::default();
    let mut rag_synthesizer = RagDataSynthesizer::new(rag_config);
    rag_synthesizer.set_schema_graph(schema_graph.clone());

    // 4. Setup validation framework
    let mut validator = ValidationFramework::new(ValidationConfig::default());
    validator.set_schema_graph(schema_graph);

    // 5. Generate coherent test data
    let user_entity = GeneratedEntity {
        entity_type: "User".to_string(),
        primary_key: Some("user_1".to_string()),
        field_values: HashMap::from([
            ("id".to_string(), "user_1".to_string()),
            ("name".to_string(), "John Doe".to_string()),
            ("email".to_string(), "john@example.com".to_string()),
        ]),
        endpoint: "/users".to_string(),
        generated_at: SystemTime::now(),
    };

    let order_entity = GeneratedEntity {
        entity_type: "Order".to_string(),
        primary_key: Some("order_1".to_string()),
        field_values: HashMap::from([
            ("id".to_string(), "order_1".to_string()),
            ("user_id".to_string(), "user_1".to_string()), // References user above
            ("total".to_string(), "99.99".to_string()),
        ]),
        endpoint: "/orders".to_string(),
        generated_at: SystemTime::now(),
    };

    // 6. Register and validate
    validator.register_entity(user_entity)?;
    validator.register_entity(order_entity)?;

    let result = validator.validate_all_entities();

    if result.is_valid {
        println!("✅ Generated coherent test data with {} entities",
            validator.get_statistics().total_entities);
    } else {
        println!("❌ Validation failed: {:?}", result.errors);
    }

    Ok(())
}
```

## Best Practices

### 1. Deterministic Testing
- Always use seeded generators for test suites
- Reset generators between test cases for consistency
- Use fixed seeds in CI/CD pipelines

### 2. Schema Design
- Use consistent naming conventions for foreign keys
- Document relationships in protobuf comments
- Consider using field options for validation hints

### 3. Validation Rules
- Start with lenient validation and gradually tighten rules
- Use warnings for potential issues, errors for critical problems
- Provide helpful error messages and suggested fixes

### 4. Performance Optimization
- Enable caching for repeated operations
- Batch entity registration when possible
- Use indexed lookups for foreign key validation

### 5. RAG Integration
- Provide high-quality domain documentation
- Use specific, actionable prompt templates
- Monitor API costs and implement caching

## Troubleshooting

### Common Issues

**Deterministic Generation Not Working**
```rust
// Ensure both seed and deterministic flag are set
let config = SmartMockConfig {
    seed: Some(42),
    deterministic: true,  // Must be true
    ..Default::default()
};
```

**Foreign Keys Not Detected**
- Check field naming conventions (`user_id`, `userId`, `user_ref`)
- Verify entity names match protobuf message names
- Use schema graph debugging to inspect extracted relationships

**Validation Errors**
- Check that referenced entities are registered before referencing entities
- Verify primary key values match foreign key references
- Review custom validation rule patterns

**RAG Not Working**
- Verify API credentials and endpoints
- Check context source file paths
- Monitor API rate limits and quotas

## Configuration Reference

### SmartMockConfig
```rust
pub struct SmartMockConfig {
    pub field_name_inference: bool,     // Enable intelligent field inference
    pub use_faker: bool,               // Use realistic fake data
    pub field_overrides: HashMap<String, String>, // Field-specific overrides
    pub service_profiles: HashMap<String, ServiceProfile>, // Service configs
    pub max_depth: usize,              // Maximum recursion depth
    pub seed: Option<u64>,             // Deterministic seed
    pub deterministic: bool,           // Enable deterministic mode
}
```

### ValidationConfig
```rust
pub struct ValidationConfig {
    pub enabled: bool,                 // Enable validation
    pub strict_mode: bool,             // Fail on warnings
    pub max_validation_depth: usize,   // Validation recursion limit
    pub custom_rules: Vec<CustomValidationRule>, // Custom rules
    pub cache_results: bool,           // Cache validation results
}
```

### RagSynthesisConfig
```rust
pub struct RagSynthesisConfig {
    pub enabled: bool,                 // Enable RAG synthesis
    pub rag_config: Option<RagSynthesisRagConfig>, // RAG API config
    pub context_sources: Vec<ContextSource>, // Knowledge sources
    pub prompt_templates: HashMap<String, PromptTemplate>, // Templates
    pub max_context_length: usize,     // Context window size
    pub cache_contexts: bool,          // Cache generated contexts
}
```

This advanced data synthesis system provides the foundation for generating realistic, coherent, and validated test data that maintains referential integrity across your entire gRPC service ecosystem.
