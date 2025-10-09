# AI-Assisted Response Generation

MockForge v1.1+ supports AI-assisted response generation, allowing you to create dynamic, context-aware mock responses using Large Language Models (LLMs). This feature builds on the existing RAG (Retrieval-Augmented Generation) foundation to enable realistic, varied responses based on request context.

## Overview

Instead of serving static mock responses, MockForge can now generate responses dynamically using AI models like:
- OpenAI GPT (GPT-3.5, GPT-4)
- Anthropic Claude
- OpenAI-compatible endpoints
- Local models via Ollama

## Configuration

### 1. Global RAG Configuration

First, configure your LLM provider in the MockForge config:

```yaml
# mockforge.yml
data:
  rag:
    enabled: true
    provider: openai  # or anthropic, ollama, openai_compatible
    api_key: ${OPENAI_API_KEY}  # Use environment variable
    model: gpt-3.5-turbo
    max_tokens: 1024
    temperature: 0.7
    timeout_secs: 30
```

Or set via environment variables:

```bash
export MOCKFORGE_RAG_ENABLED=true
export MOCKFORGE_RAG_PROVIDER=openai
export OPENAI_API_KEY=sk-...
export MOCKFORGE_RAG_MODEL=gpt-3.5-turbo
```

### 2. Per-Endpoint AI Configuration

Add the `x-mockforge-ai` vendor extension to your OpenAPI specification:

```yaml
paths:
  /chat:
    post:
      summary: Chat endpoint with AI responses
      x-mockforge-ai:
        enabled: true
        mode: intelligent  # intelligent, hybrid, or static
        prompt: |
          You are a helpful support chatbot.
          User said: "{{body.message}}"
          Respond helpfully and professionally in JSON format.
        temperature: 0.7
        max_tokens: 500
        context: "Additional context about your service"
      requestBody:
        content:
          application/json:
            schema:
              type: object
              properties:
                message:
                  type: string
      responses:
        '200':
          description: AI-generated response
```

## Prompt Template Variables

The `prompt` field supports template variables that are replaced with actual request data:

| Variable | Description | Example |
|----------|-------------|---------|
| `{{method}}` | HTTP method | `POST`, `GET` |
| `{{path}}` | Request path | `/users/123` |
| `{{body.field}}` | Request body field | `{{body.message}}` |
| `{{path.param}}` | Path parameter | `{{path.user_id}}` |
| `{{query.param}}` | Query parameter | `{{query.search}}` |
| `{{headers.name}}` | Request header | `{{headers.user-agent}}` |

### Example with Multiple Variables

```yaml
x-mockforge-ai:
  enabled: true
  mode: intelligent
  prompt: |
    Process a {{method}} request to {{path}}.
    User ID: {{path.user_id}}
    Search query: {{query.q}}
    User agent: {{headers.user-agent}}
    Request body: {{body}}

    Generate an appropriate response.
```

## Response Generation Modes

### Static (default)
No AI generation. Uses standard OpenAPI examples or schema-based generation.

```yaml
x-mockforge-ai:
  enabled: false
  mode: static
```

### Intelligent
Fully AI-generated responses based on the prompt template.

```yaml
x-mockforge-ai:
  enabled: true
  mode: intelligent
  prompt: "Generate a realistic user profile response"
```

### Hybrid
Combines static templates with AI enhancement.

```yaml
x-mockforge-ai:
  enabled: true
  mode: hybrid
  prompt: "Enhance this response with realistic details"
```

## Complete Example

Here's a complete example of an AI-powered chatbot endpoint:

```yaml
openapi: 3.0.0
info:
  title: Support Chatbot API
  version: 1.0.0

paths:
  /chat:
    post:
      summary: Send message to support chatbot
      x-mockforge-ai:
        enabled: true
        mode: intelligent
        prompt: |
          You are a customer support agent for "DataFlow Analytics",
          a business intelligence SaaS platform.

          Customer message: "{{body.message}}"
          Customer sentiment: {{body.sentiment}}

          Provide a helpful, professional response that:
          1. Addresses their question directly
          2. Offers next steps or additional help
          3. Maintains a friendly tone

          Respond in JSON format:
          {
            "message": "your response",
            "suggestions": ["action1", "action2"],
            "timestamp": "ISO timestamp"
          }
        temperature: 0.7
        context: |
          Product info: DataFlow Analytics helps companies visualize
          and analyze business data. Common issues: data import,
          dashboard creation, user permissions, API integration.
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              properties:
                message:
                  type: string
                  example: "How do I import CSV data?"
                sentiment:
                  type: string
                  enum: [positive, neutral, negative]
                  example: "neutral"
      responses:
        '200':
          description: Chatbot response
          content:
            application/json:
              schema:
                type: object
                properties:
                  message:
                    type: string
                  suggestions:
                    type: array
                    items:
                      type: string
                  timestamp:
                    type: string
                    format: date-time
```

## Running the Example

1. Start MockForge with AI configuration:

```bash
# Set your API key
export OPENAI_API_KEY=sk-...

# Start the server with the AI-enabled spec
mockforge start --openapi examples/openapi-ai-chatbot.yaml
```

2. Send a test request:

```bash
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{
    "message": "How do I import data from a CSV file?",
    "sentiment": "neutral"
  }'
```

3. Receive an AI-generated response:

```json
{
  "message": "To import data from a CSV file in DataFlow Analytics, navigate to the Data Sources section and click 'Import'. Select CSV as your file type, upload your file, and map the columns to your schema. Would you like detailed step-by-step instructions?",
  "suggestions": [
    "View CSV import tutorial",
    "Check data format requirements",
    "Contact support for assistance"
  ],
  "timestamp": "2025-10-09T10:30:00Z"
}
```

## Best Practices

### 1. Prompt Engineering
- Be specific about the desired response format
- Include relevant context about your domain
- Request JSON output for structured responses
- Test different temperature values (0.6-0.8 typical range)

### 2. Error Handling
MockForge automatically falls back to standard response generation if AI generation fails:
- API key issues
- Rate limiting
- Network errors
- Invalid responses

### 3. Performance
- AI generation adds 1-3 seconds latency (depending on model)
- Use caching (`cache_enabled: true`) for repeated identical requests
- Consider hybrid mode for better performance with AI enhancement

### 4. Cost Management
- Use cheaper models (gpt-3.5-turbo) for development
- Set appropriate `max_tokens` limits
- Use local models (Ollama) for cost-free testing
- Monitor your API usage

## Use Cases

### 1. Support Chatbots
Generate contextual responses based on user questions.

### 2. Dynamic Content Generation
Create varied, realistic content for testing frontend applications.

### 3. Personalized Responses
Generate responses based on user context (ID, role, preferences).

### 4. Multi-language Support
Use AI to generate responses in different languages based on request headers.

### 5. Sentiment-Aware Responses
Adjust response tone based on detected user sentiment.

## Limitations

- **Latency**: AI generation adds 1-3+ seconds per request
- **Cost**: LLM API calls incur costs (except local models)
- **Determinism**: Responses vary even with same input (use lower temperature for consistency)
- **Rate Limits**: Subject to LLM provider rate limits
- **Context Size**: Limited by model's context window

## Troubleshooting

### AI responses not generating

Check:
1. `x-mockforge-ai.enabled` is `true`
2. Global RAG is configured correctly
3. API key is set correctly
4. Network connectivity to LLM provider

View logs for details:
```bash
RUST_LOG=debug mockforge start --openapi spec.yaml
```

### Responses don't match expected format

- Be more explicit in your prompt about JSON structure
- Lower the temperature for more consistent outputs
- Add schema validation in your `x-mockforge-ai` config

### High latency

- Use a faster model (gpt-3.5-turbo vs gpt-4)
- Reduce `max_tokens`
- Consider hybrid mode instead of intelligent
- Use local models for development

## Advanced Configuration

### Custom Model Parameters

```yaml
x-mockforge-ai:
  enabled: true
  mode: intelligent
  prompt: "..."
  temperature: 0.8        # Creativity (0.0-2.0)
  max_tokens: 500         # Response length limit
  cache_enabled: true     # Cache identical requests
  schema:                 # JSON Schema for validation
    type: object
    properties:
      message:
        type: string
      confidence:
        type: number
```

### Provider-Specific Settings

Different providers can be configured globally:

```yaml
data:
  rag:
    provider: anthropic
    api_key: ${ANTHROPIC_API_KEY}
    model: claude-3-sonnet-20240229
    api_endpoint: https://api.anthropic.com/v1
```

For Ollama (local):
```yaml
data:
  rag:
    provider: ollama
    api_endpoint: http://localhost:11434
    model: llama2
```

## Next Steps

- Explore the [example OpenAPI spec](../examples/openapi-ai-chatbot.yaml)
- Read about [RAG configuration](./RAG_CONFIGURATION.md)
- Check the [API reference](https://docs.rs/mockforge-core/latest/mockforge_core/ai_response/)

## Feedback

This is an initial phase feature. Please report issues and suggestions:
- [GitHub Issues](https://github.com/SaaSy-Solutions/mockforge/issues)
- Tag with `ai-response-generation`
