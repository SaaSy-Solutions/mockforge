# AI Generator Implementation Guide

This document explains how to use the new AI generator feature that integrates the RAG engine with OpenAPI route handlers.

## Overview

The AI generator feature allows MockForge to generate dynamic, intelligent responses using LLMs (Large Language Models) based on request context. This is implemented using a dependency injection pattern to avoid circular dependencies between crates.

## Architecture

### Components

1. **`AiGenerator` Trait** (`mockforge-core`)
   - Defines the interface for AI generation
   - Located in `crates/mockforge-core/src/openapi/response.rs`
   - Allows different implementations without coupling to specific LLM providers

2. **`RagAiGenerator` Implementation** (`mockforge-http`)
   - Implements `AiGenerator` using the RAG engine from `mockforge-data`
   - Located in `crates/mockforge-http/src/rag_ai_generator.rs`
   - Handles LLM communication and response parsing

3. **Integration Point** (`mockforge-core`)
   - `ResponseGenerator::generate_ai_response()` accepts an optional `AiGenerator`
   - Falls back to placeholder response if no generator is provided
   - Called from `OpenApiRoute::mock_response_with_status_async()`

## Usage

### 1. Configure AI in OpenAPI Specification

Add the `x-mockforge-ai` extension to your OpenAPI operations:

```yaml
paths:
  /users/{id}:
    get:
      operationId: getUser
      x-mockforge-ai:
        enabled: true
        mode: intelligent
        prompt: |
          Generate a realistic user profile for user ID {{path.id}}.
          The user should have:
          - A name
          - Email address matching the pattern user{{path.id}}@example.com
          - Age between 18 and 80
          - A job title
          Return as JSON with fields: id, name, email, age, jobTitle
        temperature: 0.7
        max_tokens: 500
```

### 2. Set Up Environment Variables

Configure the LLM provider:

```bash
# Required
export MOCKFORGE_AI_PROVIDER=openai  # or anthropic, ollama, openai-compatible
export MOCKFORGE_AI_API_KEY=your-api-key

# Optional (with defaults shown)
export MOCKFORGE_AI_MODEL=gpt-3.5-turbo
export MOCKFORGE_AI_ENDPOINT=https://api.openai.com/v1/chat/completions
export MOCKFORGE_AI_TEMPERATURE=0.7
export MOCKFORGE_AI_MAX_TOKENS=1024
```

### 3. Create and Use the AI Generator in Your HTTP Handler

```rust
use mockforge_http::rag_ai_generator::RagAiGenerator;
use mockforge_core::openapi::route::OpenApiRoute;
use mockforge_core::ai_response::RequestContext;

// Create the AI generator (typically during server startup)
let ai_generator = RagAiGenerator::from_env()?;

// When handling a request for an OpenAPI route
let context = RequestContext::new("GET".to_string(), "/users/123".to_string())
    .with_path_params(path_params);

// Call the route handler with the AI generator
let (status_code, response_body) = route
    .mock_response_with_status_async(&context, Some(&ai_generator))
    .await;
```

### 4. Alternative: Create Generator with Explicit Configuration

```rust
use mockforge_data::rag::{RagConfig, LlmProvider};
use mockforge_http::rag_ai_generator::RagAiGenerator;

let rag_config = RagConfig {
    provider: LlmProvider::OpenAI,
    api_key: Some("your-api-key".to_string()),
    model: "gpt-4".to_string(),
    api_endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
    temperature: 0.7,
    max_tokens: 1024,
    ..Default::default()
};

let ai_generator = RagAiGenerator::new(rag_config)?;
```

## Supported LLM Providers

### OpenAI

```bash
export MOCKFORGE_AI_PROVIDER=openai
export MOCKFORGE_AI_API_KEY=sk-...
export MOCKFORGE_AI_MODEL=gpt-4
```

### Anthropic Claude

```bash
export MOCKFORGE_AI_PROVIDER=anthropic
export MOCKFORGE_AI_API_KEY=sk-ant-...
export MOCKFORGE_AI_MODEL=claude-3-opus-20240229
export MOCKFORGE_AI_ENDPOINT=https://api.anthropic.com/v1/messages
```

### Ollama (Local)

```bash
export MOCKFORGE_AI_PROVIDER=ollama
export MOCKFORGE_AI_MODEL=llama2
export MOCKFORGE_AI_ENDPOINT=http://localhost:11434/api/generate
# No API key needed for local Ollama
```

### OpenAI-Compatible APIs

For providers that implement the OpenAI API format (e.g., LocalAI, LM Studio):

```bash
export MOCKFORGE_AI_PROVIDER=openai-compatible
export MOCKFORGE_AI_MODEL=your-model
export MOCKFORGE_AI_ENDPOINT=http://localhost:8080/v1/chat/completions
export MOCKFORGE_AI_API_KEY=optional-key  # If required by your provider
```

## Prompt Template Variables

The `x-mockforge-ai.prompt` field supports template variables:

- `{{method}}` - HTTP method (GET, POST, etc.)
- `{{path}}` - Request path
- `{{path.paramName}}` - Path parameter value
- `{{query.paramName}}` - Query parameter value
- `{{body.fieldName}}` - Request body field value
- `{{headers.headerName}}` - Request header value

Example:

```yaml
prompt: |
  Generate a response for {{method}} {{path}}.
  User requested: {{body.message}}
  Preferred format: {{query.format}}
```

## Response Modes

Configure the `mode` field in `x-mockforge-ai`:

- **`static`**: No AI generation (default OpenAPI response)
- **`intelligent`**: Pure AI generation based on prompt
- **`hybrid`**: Combine static template with AI enhancement (future)

## Error Handling

If AI generation fails (network error, API error, etc.):

1. An error is logged via `tracing`
2. The system falls back to standard OpenAPI response generation
3. The request still completes successfully

## Performance Considerations

- **Caching**: AI responses can be cached based on request parameters (future enhancement)
- **Latency**: LLM calls add 1-5 seconds depending on provider and model
- **Rate Limits**: Respect provider rate limits (implement exponential backoff if needed)
- **Cost**: Each AI-generated response incurs API costs

## Testing

Without AI generator (uses placeholder):

```rust
let (status, body) = route
    .mock_response_with_status_async(&context, None)
    .await;

// Returns placeholder JSON with expanded prompt
```

With AI generator:

```rust
let generator = RagAiGenerator::from_env()?;
let (status, body) = route
    .mock_response_with_status_async(&context, Some(&generator))
    .await;

// Returns actual AI-generated response
```

## Migration Guide

### Before (placeholder implementation)

```rust
// Old code - no AI generator parameter
let (status, body) = route.mock_response_with_status_async(&context).await;
```

### After (with AI support)

```rust
// New code - accepts optional AI generator
let ai_generator = RagAiGenerator::from_env().ok();
let (status, body) = route
    .mock_response_with_status_async(&context, ai_generator.as_ref())
    .await;
```

## Troubleshooting

### "No AI generator provided, returning placeholder"

- Ensure you're passing the `AiGenerator` to `mock_response_with_status_async()`
- Check that environment variables are set correctly
- Verify the AI provider is accessible (network, API key, etc.)

### "AI response generation failed, falling back to standard generation"

- Check logs for specific error message
- Verify API key is valid and has sufficient credits
- Ensure the endpoint URL is correct
- Check network connectivity to the LLM provider

### Response is wrapped in `{"response": "...", "note": "..."}`

- The LLM returned plain text instead of JSON
- Update your prompt to explicitly request JSON format
- Example: "Return the response as valid JSON only, with no additional text"

## Future Enhancements

- [ ] Response caching based on request fingerprint
- [ ] Hybrid mode (template + AI enhancement)
- [ ] Streaming responses for long generations
- [ ] RAG document indexing from OpenAPI examples
- [ ] Custom retry logic and circuit breakers
- [ ] Response validation against JSON schema
- [ ] Multi-turn conversations with context
