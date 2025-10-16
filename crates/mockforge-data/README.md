# MockForge Data

Synthetic data generation engine with faker primitives and RAG (Retrieval-Augmented Generation).

This crate provides powerful tools for generating realistic test data, including traditional faker-based generation and advanced RAG-powered synthetic data creation. It's designed to work seamlessly with MockForge's mocking framework to create comprehensive test datasets.

## Features

- **Faker Primitives**: Generate realistic fake data (names, emails, addresses, etc.)
- **Schema-Based Generation**: Define data structures and generate conforming datasets
- **RAG Integration**: Use Retrieval-Augmented Generation for contextually aware data synthesis
- **Template Support**: Create complex data structures with variable substitution
- **Multiple Output Formats**: JSON, JSON Lines, YAML, and CSV support
- **Relationship Handling**: Generate data with cross-schema relationships
- **Batch Generation**: Create multiple datasets simultaneously

## Quick Start

### Basic Data Generation

```rust,no_run
use mockforge_data::{DataConfig, DataGenerator, SchemaDefinition};

// Define a simple user schema
let mut schema = SchemaDefinition::new("user".to_string());
schema = schema.with_field(
    mockforge_data::FieldDefinition::new("name".to_string(), "name".to_string())
);
schema = schema.with_field(
    mockforge_data::FieldDefinition::new("email".to_string(), "email".to_string())
);

// Configure generation
let config = DataConfig {
    rows: 100,
    ..Default::default()
};

// Generate data
let mut generator = DataGenerator::new(schema, config)?;
let result = generator.generate().await?;

// Access generated data
println!("Generated {} rows", result.count);
println!("First row: {}", result.data[0]);
```

### Using Faker Directly

```rust,no_run
use mockforge_data::faker::{EnhancedFaker, quick};

// Quick functions for common data
let email = quick::email();
let name = quick::name();
let uuid = quick::uuid();

// Enhanced faker with more options
let mut faker = EnhancedFaker::new();
let address = faker.address();
let phone = faker.phone();
let date = faker.date_iso();
```

### Template-Based Generation

```rust,no_run
use mockforge_data::faker::TemplateFaker;
use serde_json::Value;

let mut faker = TemplateFaker::new()
    .with_variable("user_type".to_string(), Value::String("admin".to_string()));

let result = faker.generate_from_template("User: {{faker.name}} ({{user_type}})");
```

### RAG-Enhanced Generation

```rust,no_run
use mockforge_data::{DataConfig, RagConfig, RagEngine};

// Configure RAG
let rag_config = RagConfig {
    provider: LlmProvider::OpenAI,
    model: "gpt-4".to_string(),
    api_key: Some("your-api-key".to_string()),
    semantic_search_enabled: true,
    ..Default::default()
};

// Create RAG engine
let mut engine = RagEngine::new(rag_config);

// Add context documents
engine.add_document("user_profiles", "Users can be admin, regular, or guest types")?;

// Generate with RAG
let schema = SchemaDefinition::new("user".to_string());
let config = DataConfig {
    rows: 50,
    rag_enabled: true,
    ..Default::default()
};

let result = engine.generate_with_rag(&schema, &config).await?;
```

## Key Modules

### Faker (`faker`)
Enhanced faker utilities for generating realistic fake data:

- **Basic Types**: Strings, numbers, booleans, dates, UUIDs
- **Personal Data**: Names, emails, addresses, phone numbers
- **Business Data**: Company names, URLs, IP addresses
- **Template Support**: Variable substitution with `{{variable}}` syntax
- **Quick Functions**: One-liner access to common generators

### Schema (`schema`)
Define data structures for generation:

- **Field Definitions**: Type-based field specifications
- **Relationships**: Cross-schema foreign key relationships
- **Templates**: Pre-built schemas for common entities (users, products, orders)

### Generator (`generator`)
Core data generation engine:

- **DataGenerator**: Single schema generation with configuration
- **BatchGenerator**: Multi-schema batch processing
- **Relationship Resolution**: Automatic foreign key population
- **Performance**: Optimized for large dataset generation

### RAG (`rag`)
Retrieval-Augmented Generation for intelligent data synthesis:

- **Multiple Providers**: OpenAI, Anthropic, Ollama, OpenAI-compatible APIs
- **Semantic Search**: Vector-based document retrieval
- **Context Integration**: Use existing data as generation context
- **Configurable Models**: Support for various LLM architectures

## Output Formats

Generated data can be exported in multiple formats:

```rust,no_run
use mockforge_data::GenerationResult;

// JSON (default)
let json = result.to_json_string()?;

// JSON Lines
let jsonl = result.to_jsonl_string()?;

// Access raw data
for row in &result.data {
    println!("{}", row);
}
```

## Configuration

### DataConfig
Control generation parameters:

```rust,no_run
let config = DataConfig {
    rows: 1000,                    // Number of rows to generate
    seed: Some(42),               // For reproducible results
    rag_enabled: true,            // Enable RAG generation
    rag_context_length: 2000,     // Max context for RAG
    format: OutputFormat::Json,   // Output format
};
```

### RagConfig
Configure RAG behavior:

```rust,no_run
let rag_config = RagConfig {
    provider: LlmProvider::OpenAI,
    model: "gpt-4".to_string(),
    api_key: Some("sk-...".to_string()),
    temperature: 0.7,
    semantic_search_enabled: true,
    similarity_threshold: 0.8,
    max_chunks: 5,
    ..Default::default()
};
```

## Examples

### Generate Related Data

```rust,no_run
use mockforge_data::generator::utils;

// Generate orders with related users
let results = utils::generate_orders_with_users(100, 50).await?;
let user_data = &results[0];
let order_data = &results[1];
```

### Custom Schema Generation

```rust,no_run
use mockforge_data::generator::utils;

// Generate from field definitions
let result = utils::generate_sample_data(
    "product",
    vec![
        ("id", "uuid"),
        ("name", "string"),
        ("price", "float"),
        ("in_stock", "bool"),
    ],
    200,
).await?;
```

## Integration with MockForge

This crate is designed to work with the broader MockForge ecosystem:

- **MockForge Core**: Use generated data in mock responses
- **MockForge HTTP**: Populate REST API mocks with realistic data
- **MockForge GraphQL**: Generate GraphQL schema-conforming data

## Performance Considerations

- **Memory Usage**: Large datasets are generated in batches
- **RAG Overhead**: Semantic search adds processing time
- **Parallel Generation**: Use `BatchGenerator` for concurrent processing
- **Caching**: RAG engines cache embeddings for performance

## Contributing

See the main [MockForge repository](https://github.com/SaaSy-Solutions/mockforge) for contribution guidelines.

## License

Licensed under MIT OR Apache-2.0
