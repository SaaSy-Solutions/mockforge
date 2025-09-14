# Advanced Data Synthesis

MockForge provides sophisticated data synthesis capabilities that go beyond simple random data generation. The advanced data synthesis system combines intelligent field inference, deterministic seeding, relationship-aware generation, and cross-endpoint validation to create realistic, coherent, and reproducible test data.

## Overview

The advanced data synthesis system consists of four main components:

1. **Smart Mock Generator** - Intelligent field-based mock data generation with deterministic seeding
2. **Schema Graph Extraction** - Automatic discovery of relationships from protobuf schemas  
3. **RAG-Driven Synthesis** - Domain-aware data generation using Retrieval-Augmented Generation
4. **Validation Framework** - Cross-endpoint consistency and integrity validation

These components work together to provide enterprise-grade test data generation that maintains referential integrity across your entire gRPC service ecosystem.

## Smart Mock Generator

The Smart Mock Generator provides intelligent mock data generation based on field names, types, and patterns. It automatically detects the intent behind field names and generates appropriate realistic data.

### Field Name Intelligence

The generator automatically infers appropriate data types based on field names:

| Field Pattern | Generated Data Type | Example Values |
|---------------|-------------------|----------------|
| `email`, `email_address` | Realistic email addresses | `user@example.com`, `alice.smith@company.org` |
| `phone`, `mobile`, `phone_number` | Formatted phone numbers | `+1-555-0123`, `(555) 123-4567` |
| `id`, `user_id`, `order_id` | Sequential or UUID-based IDs | `user_001`, `550e8400-e29b-41d4-a716-446655440000` |
| `name`, `first_name`, `last_name` | Realistic names | `John Doe`, `Alice`, `Johnson` |
| `created_at`, `updated_at`, `timestamp` | ISO timestamps | `2023-10-15T14:30:00Z` |
| `latitude`, `longitude` | Geographic coordinates | `40.7128`, `-74.0060` |
| `url`, `website` | Valid URLs | `https://example.com` |
| `token`, `api_key` | Security tokens | `sk_live_4eC39HqLyjWDarjtT1zdp7dc` |

### Deterministic Generation

For reproducible test fixtures, the Smart Mock Generator supports deterministic seeding:

```rust
use mockforge_grpc::reflection::smart_mock_generator::{SmartMockGenerator, SmartMockConfig};

// Create a deterministic generator with a fixed seed
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

This ensures that your tests produce consistent results across different runs and environments.

## Schema Graph Extraction

The schema graph extraction system analyzes your protobuf definitions to automatically discover relationships and foreign key patterns between entities.

### Foreign Key Detection

The system uses naming conventions to detect foreign key relationships:

```protobuf
message Order {
  string id = 1;
  string user_id = 2;     // → Detected as foreign key to User
  string customer_ref = 3; // → Detected as reference to Customer  
  int64 timestamp = 4;
}

message User {
  string id = 1;          // → Detected as primary key
  string name = 2;
  string email = 3;
}
```

**Common Foreign Key Patterns:**
- `user_id` → references `User` entity
- `orderId` → references `Order` entity  
- `customer_ref` → references `Customer` entity

### Relationship Types

The system identifies various relationship types:

- **Foreign Key**: Direct ID references (`user_id` → `User`)
- **Embedded**: Nested message types within other messages
- **One-to-Many**: Repeated field relationships
- **Composition**: Ownership relationships between entities

## RAG-Driven Data Synthesis

RAG (Retrieval-Augmented Generation) enables context-aware data generation using domain knowledge from documentation, examples, and business rules.

### Configuration

```yaml
grpc:
  data_synthesis:
    rag:
      enabled: true
      api_endpoint: "https://api.openai.com/v1/chat/completions"
      model: "gpt-3.5-turbo" 
      embedding_model: "text-embedding-ada-002"
      similarity_threshold: 0.7
      max_documents: 5
    context_sources:
      - id: "user_docs"
        type: "documentation"
        path: "./docs/user_guide.md"
        weight: 1.0
      - id: "examples"
        type: "examples"
        path: "./examples/sample_data.json" 
        weight: 0.8
```

### Business Rule Extraction

The RAG system automatically extracts business rules from your documentation:

- **Email Validation**: "Email fields must follow valid email format"
- **Phone Formatting**: "Phone numbers should be in international format" 
- **ID Requirements**: "User IDs must be alphanumeric and 8 characters long"
- **Relationship Constraints**: "Orders must reference valid existing users"

### Domain-Aware Generation

Instead of generic random data, RAG generates contextually appropriate values:

```protobuf
message User {
  string role = 1; // Context: "admin", "user", "moderator" 
  string department = 2; // Context: "engineering", "marketing", "sales"
  string location = 3; // Context: "San Francisco", "New York", "London"
}
```

## Cross-Endpoint Validation

The validation framework ensures data coherence across different endpoints and validates referential integrity.

### Validation Rules

The framework supports multiple types of validation rules:

**Built-in Validations:**
- Foreign key existence validation
- Field format validation (email, phone, URL)
- Range validation for numeric fields  
- Unique constraint validation

**Custom Validation Rules:**
```yaml
grpc:
  data_synthesis:
    validation:
      enabled: true
      strict_mode: false
      custom_rules:
        - name: "email_format"
          applies_to: ["User", "Customer"]
          fields: ["email"]
          type: "format"
          pattern: "^[^@\\s]+@[^@\\s]+\\.[^@\\s]+$"
          error: "Invalid email format"
        - name: "age_range" 
          applies_to: ["User"]
          fields: ["age"]
          type: "range"
          min: 0
          max: 120
          error: "Age must be between 0 and 120"
```

### Referential Integrity

The validator automatically checks that:
- Foreign key references point to existing entities
- Required relationships are satisfied
- Cross-service data dependencies are maintained
- Business constraints are enforced

## Configuration

### Environment Variables

```bash
# Enable advanced data synthesis
MOCKFORGE_DATA_SYNTHESIS_ENABLED=true

# Deterministic generation  
MOCKFORGE_DATA_SYNTHESIS_SEED=12345
MOCKFORGE_DATA_SYNTHESIS_DETERMINISTIC=true

# RAG configuration
MOCKFORGE_RAG_ENABLED=true
MOCKFORGE_RAG_API_KEY=your-api-key
MOCKFORGE_RAG_MODEL=gpt-3.5-turbo

# Validation settings
MOCKFORGE_VALIDATION_ENABLED=true
MOCKFORGE_VALIDATION_STRICT_MODE=false
```

### Configuration File

```yaml
grpc:
  port: 50051
  proto_dir: "proto/"
  data_synthesis:
    enabled: true
    smart_generator:
      field_inference: true
      use_faker: true
      deterministic: true
      seed: 42
      max_depth: 5
    rag:
      enabled: true
      api_endpoint: "https://api.openai.com/v1/chat/completions"
      api_key: "${RAG_API_KEY}"
      model: "gpt-3.5-turbo"
      embedding_model: "text-embedding-ada-002"  
      similarity_threshold: 0.7
      max_context_length: 2000
      cache_contexts: true
    validation:
      enabled: true
      strict_mode: false
      max_validation_depth: 3
      cache_results: true
    schema_extraction:
      extract_relationships: true
      detect_foreign_keys: true
      confidence_threshold: 0.8
```

## Example Usage

### Basic Smart Generation

```bash
# Start MockForge with advanced data synthesis
MOCKFORGE_DATA_SYNTHESIS_ENABLED=true \
MOCKFORGE_DATA_SYNTHESIS_SEED=12345 \
mockforge serve --grpc-port 50051
```

### With RAG Enhancement

```bash  
# Start with RAG-powered domain awareness
MOCKFORGE_DATA_SYNTHESIS_ENABLED=true \
MOCKFORGE_RAG_ENABLED=true \
MOCKFORGE_RAG_API_KEY=your-api-key \
MOCKFORGE_VALIDATION_ENABLED=true \
mockforge serve --grpc-port 50051
```

### Testing Deterministic Generation

```bash
# Generate data twice with same seed - should be identical
grpcurl -plaintext -d '{"user_id": "123"}' \
  localhost:50051 com.example.UserService/GetUser

# Reset and call again - will generate same response
grpcurl -plaintext -d '{"user_id": "123"}' \
  localhost:50051 com.example.UserService/GetUser
```

## Best Practices

### Deterministic Testing
- Use fixed seeds in CI/CD pipelines for reproducible tests
- Reset generators between test cases for consistency
- Document seed values used in critical test scenarios

### Schema Design for Synthesis
- Use consistent naming conventions for foreign keys (`user_id`, `customer_ref`)
- Add comments to proto files describing business rules
- Consider field naming that indicates data type (`email_address` vs `contact`)

### RAG Integration
- Provide high-quality domain documentation as context sources
- Use specific, actionable descriptions in documentation
- Monitor API costs and implement appropriate caching

### Validation Strategy
- Start with lenient validation and gradually add stricter rules
- Use warnings for potential issues, errors for critical problems
- Provide helpful error messages with suggested fixes

## Advanced Scenarios

### Multi-Service Data Coherence

When mocking multiple related gRPC services, ensure data coherence:

```bash
# Start user service
MOCKFORGE_DATA_SYNTHESIS_SEED=100 \
mockforge serve --grpc-port 50051 --proto-dir user-proto &

# Start order service with same seed for consistency  
MOCKFORGE_DATA_SYNTHESIS_SEED=100 \
mockforge serve --grpc-port 50052 --proto-dir order-proto &
```

### Custom Field Overrides

Override specific fields with custom values:

```yaml
grpc:
  data_synthesis:
    field_overrides:
      "admin_email": "admin@company.com"
      "api_version": "v2.1"
      "environment": "testing"
```

### Business Rule Templates

Define reusable business rule templates:

```yaml
grpc:
  data_synthesis:
    rule_templates:
      - name: "financial_data"
        applies_to: ["Invoice", "Payment", "Transaction"]
        rules:
          - field_pattern: "*_amount"
            type: "range" 
            min: 0.01
            max: 10000.00
          - field_pattern: "*_currency"
            type: "enum"
            values: ["USD", "EUR", "GBP"]
```

## Troubleshooting

### Common Issues

**Generated data not realistic enough**
- Enable RAG synthesis with domain documentation
- Check field naming conventions for better inference
- Add custom business rules for specific constraints

**Non-deterministic behavior**  
- Ensure `deterministic: true` and provide a `seed` value
- Reset generators between test runs
- Check for external randomness sources

**Validation failures**
- Review foreign key naming conventions
- Ensure referenced entities are generated before referencing ones
- Check custom validation rule patterns

**RAG not working**
- Verify API credentials and endpoints
- Check context source file paths and permissions
- Monitor API rate limits and error responses

### Debug Commands

```bash
# Test data synthesis configuration
mockforge validate-config

# Show detected schema relationships
mockforge analyze-schema --proto-dir proto/

# Test deterministic generation
MOCKFORGE_DATA_SYNTHESIS_DEBUG=true \
mockforge serve --grpc-port 50051
```

Advanced data synthesis transforms MockForge from a simple mocking tool into a comprehensive test data management platform, enabling realistic, consistent, and validated test scenarios across your entire service architecture.