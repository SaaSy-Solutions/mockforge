# AI-Based Schema Extrapolation

## Overview

MockForge now includes AI-powered API specification suggestion and extrapolation. This feature allows users to provide minimal input—such as a single endpoint example, an API description, or a partial specification—and have MockForge automatically generate a complete, production-ready API specification.

## Features

- **Minimal Input Required**: Start with just one endpoint example or even a text description
- **Multiple Input Formats**:
  - Single endpoint with request/response examples
  - Text description of desired API
  - Partial OpenAPI specification
  - List of endpoint paths
- **Multiple Output Formats**:
  - OpenAPI 3.0 specification
  - MockForge YAML configuration
  - Both formats simultaneously
- **Intelligent Suggestions**: AI analyzes input and suggests:
  - Related CRUD operations
  - Utility endpoints (health, status, metrics)
  - Search, filtering, and pagination
  - Batch operations
  - Realistic request/response schemas
- **Customizable**: Control number of suggestions, domain hints, LLM provider, and more

## Architecture

### Components

1. **`SpecSuggestionEngine`** (`crates/mockforge-core/src/intelligent_behavior/spec_suggestion.rs`)
   - Core engine for AI-powered spec generation
   - Handles prompt construction and LLM interaction
   - Parses and structures LLM responses

2. **`SuggestionInput`** (enum)
   - `Endpoint`: Single endpoint with method, path, and examples
   - `Description`: Natural language API description
   - `PartialSpec`: Partial OpenAPI specification to complete
   - `Paths`: List of endpoint paths to expand

3. **`SuggestionConfig`**
   - Configuration for suggestion generation
   - Includes LLM settings, output format, and domain hints

4. **CLI Command** (`mockforge suggest`)
   - User-facing interface in `crates/mockforge-cli/src/main.rs`
   - Handles file I/O and user interaction

### Data Flow

```
User Input (JSON/Text)
    ↓
Input Parser (detect format)
    ↓
SpecSuggestionEngine
    ↓
Prompt Builder (system + user prompts)
    ↓
LLM Client (OpenAI, Anthropic, Ollama, etc.)
    ↓
Response Parser
    ↓
Output Generator (OpenAPI/MockForge YAML)
    ↓
File Output
```

## Usage

### Basic Examples

#### 1. From Single Endpoint
```bash
mockforge suggest --from examples/ai-suggestions/single-endpoint.json --output api-spec.yaml
```

#### 2. From Description
```bash
mockforge suggest --from-description "A blog API with posts, comments, and user authentication" --output blog-api.yaml
```

#### 3. Generate Both Formats
```bash
mockforge suggest --from example.json --format both --output my-api
# Creates: my-api.openapi.yaml and my-api.mockforge.yaml
```

#### 4. With Domain Hint
```bash
mockforge suggest --from example.json --domain e-commerce --num-suggestions 15 --output ecommerce-api.yaml
```

### Advanced Options

```bash
mockforge suggest \
  --from endpoint.json \
  --format both \
  --output banking-api \
  --num-suggestions 20 \
  --domain fintech \
  --llm-provider anthropic \
  --llm-model claude-3-5-sonnet-20241022 \
  --temperature 0.8 \
  --include-examples
```

### Command Options

| Option | Description | Default |
|--------|-------------|---------|
| `--from <FILE>` | Input JSON file | - |
| `--from-description <TEXT>` | Generate from text description | - |
| `--format <FORMAT>` | Output format: `openapi`, `mockforge`, or `both` | `openapi` |
| `--output <FILE>` | Output file path | - |
| `--num-suggestions <N>` | Number of additional endpoints to suggest | `5` |
| `--include-examples` | Include examples in generated specs | `true` |
| `--domain <DOMAIN>` | API domain hint (e-commerce, fintech, etc.) | - |
| `--llm-provider <PROVIDER>` | LLM provider: `openai`, `anthropic`, `ollama`, `openai-compatible` | `openai` |
| `--llm-model <MODEL>` | LLM model name | Provider default |
| `--llm-endpoint <URL>` | Custom LLM API endpoint | - |
| `--llm-api-key <KEY>` | LLM API key (or use env vars) | - |
| `--temperature <TEMP>` | Generation temperature (0.0-1.0) | `0.7` |
| `--print-json` | Print results as JSON instead of saving | `false` |

## Input Formats

### Single Endpoint Format

```json
{
  "method": "GET",
  "path": "/api/users/{id}",
  "description": "Get a user by ID",
  "request": null,
  "response": {
    "id": "user_123",
    "name": "John Doe",
    "email": "john@example.com"
  }
}
```

### Paths List Format

```json
{
  "paths": [
    "/api/products",
    "/api/products/{id}",
    "/api/categories"
  ]
}
```

### Partial OpenAPI Spec

```json
{
  "openapi": "3.0.0",
  "info": {
    "title": "My API",
    "version": "1.0.0"
  },
  "paths": {
    "/users": {
      "get": {
        "summary": "List users"
      }
    }
  }
}
```

## LLM Provider Configuration

### OpenAI

```bash
export OPENAI_API_KEY="sk-..."
mockforge suggest --from example.json --output spec.yaml
```

Default model: `gpt-4o-mini`

### Anthropic Claude

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
mockforge suggest --from example.json --llm-provider anthropic --output spec.yaml
```

Default model: `claude-3-5-sonnet-20241022`

### Local Ollama

```bash
mockforge suggest --from example.json --llm-provider ollama --llm-model llama3.1 --output spec.yaml
```

### Custom OpenAI-Compatible API

```bash
mockforge suggest \
  --from example.json \
  --llm-provider openai-compatible \
  --llm-endpoint https://your-api.com/v1/chat/completions \
  --llm-api-key your-key \
  --output spec.yaml
```

## Implementation Details

### Prompt Engineering

The system uses carefully crafted prompts that instruct the LLM to:

1. **Analyze** the input to understand the API's purpose and domain
2. **Design** RESTful endpoints following best practices
3. **Generate** realistic schemas with proper data types
4. **Include** appropriate HTTP methods, status codes, and error handling
5. **Consider** pagination, filtering, search, and batch operations
6. **Maintain** consistency in naming conventions and response structures

### System Prompt Structure

```
You are an expert API architect and specification designer.

Generate {format} with these principles:
- RESTful best practices
- Consistent naming and structures
- Complete schemas with validation
- Realistic and practical designs
- Security considerations

Suggest {num_suggestions} additional endpoints considering:
- CRUD operations
- Utility endpoints (health, metrics)
- Related resources
- Filtering, pagination, search
- Batch operations
```

### Response Parsing

The LLM returns structured JSON:

```json
{
  "detected_domain": "e-commerce",
  "endpoints": [
    {
      "method": "GET",
      "path": "/api/products",
      "description": "List all products with pagination",
      "parameters": [...],
      "response_schema": {...},
      "reasoning": "Essential for browsing products in an e-commerce system"
    }
  ],
  "openapi_spec": {...},
  "mockforge_config": {...}
}
```

### Error Handling

The system handles:
- Invalid input formats
- Missing required fields
- LLM API failures with retries
- Partial or malformed LLM responses
- File I/O errors

## Code Structure

### Main Files

```
crates/
├── mockforge-core/
│   └── src/
│       └── intelligent_behavior/
│           ├── spec_suggestion.rs    # Core suggestion engine
│           ├── llm_client.rs         # LLM provider integration
│           ├── config.rs             # Configuration types
│           └── mod.rs                # Module exports
└── mockforge-cli/
    └── src/
        └── main.rs                   # CLI command and handler
```

### Key Types

```rust
pub struct SpecSuggestionEngine {
    llm_client: LlmClient,
    config: SuggestionConfig,
}

pub enum SuggestionInput {
    Endpoint { method, path, request, response, description },
    Description { text },
    PartialSpec { spec },
    Paths { paths },
}

pub enum OutputFormat {
    OpenAPI,
    MockForge,
    Both,
}

pub struct SuggestionResult {
    openapi_spec: Option<Value>,
    mockforge_config: Option<Value>,
    suggestions: Vec<EndpointSuggestion>,
    metadata: SuggestionMetadata,
}
```

## Testing

### Unit Tests

```bash
cargo test --package mockforge-core intelligent_behavior::spec_suggestion
```

### Integration Tests

```bash
# Test with real LLM (requires API key)
export OPENAI_API_KEY="..."
mockforge suggest --from examples/ai-suggestions/single-endpoint.json --output test-output.yaml
```

### Example Test Cases

1. Single endpoint expansion
2. Description-based generation
3. Partial spec completion
4. Paths-only expansion
5. Different domains (e-commerce, fintech, social-media)
6. Different output formats
7. Various LLM providers

## Future Enhancements

### Planned Features

1. **Schema Validation**: Validate generated specs against OpenAPI schema
2. **Interactive Mode**: Ask user questions to refine suggestions
3. **Learning from Examples**: Learn from existing specs in the project
4. **Consistency Checking**: Ensure suggested endpoints match existing patterns
5. **Versioning Support**: Generate multiple API versions
6. **Documentation Generation**: Create comprehensive API documentation
7. **Test Generation**: Generate test cases for suggested endpoints
8. **GraphQL Support**: Extend to GraphQL schema generation
9. **gRPC Support**: Generate .proto files from descriptions

### Potential Improvements

- **Caching**: Cache LLM responses for similar inputs
- **Incremental Suggestions**: Add to existing specs without regenerating
- **Templates**: Pre-built templates for common API patterns
- **Validation Rules**: Enforce organization-specific conventions
- **Multi-language Support**: Generate clients in various languages
- **Collaboration**: Share and refine suggestions with teams

## Troubleshooting

### Common Issues

**Issue**: `No endpoints in LLM response`
- **Solution**: Increase temperature or try a different model

**Issue**: `API key not found`
- **Solution**: Set environment variable or use `--llm-api-key`

**Issue**: `Unable to detect input type`
- **Solution**: Ensure JSON has required fields (`method` for endpoint, `paths` for paths list)

**Issue**: `LLM timeout`
- **Solution**: Reduce `--num-suggestions` or increase timeout in config

## Performance Considerations

- **LLM Latency**: Typical response time is 5-30 seconds depending on provider and model
- **Cost**: Each suggestion uses ~2000-4000 tokens (input + output)
- **Rate Limits**: Respect provider rate limits; consider caching results
- **Model Selection**:
  - Fast/cheap: `gpt-4o-mini`, `claude-3-haiku`
  - Quality: `gpt-4`, `claude-3-5-sonnet`
  - Local: `ollama` with `llama3.1`

## Security

- **API Keys**: Never commit API keys; use environment variables
- **Input Validation**: All inputs are validated before processing
- **Output Sanitization**: Generated specs are validated before saving
- **LLM Safety**: System prompts discourage generation of harmful content

## Contributing

To contribute to this feature:

1. Review the code in `crates/mockforge-core/src/intelligent_behavior/spec_suggestion.rs`
2. Add tests for new input formats or output types
3. Update documentation for new features
4. Follow existing prompt engineering patterns
5. Test with multiple LLM providers

## Examples in Production

See the `examples/ai-suggestions/` directory for:
- Single endpoint examples
- Description-based examples
- Partial spec examples
- Domain-specific examples
- Multi-format output examples

## License

This feature is part of MockForge and follows the same license terms (MIT OR Apache-2.0).
