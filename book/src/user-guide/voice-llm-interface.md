# Voice + LLM Interface

The Voice + LLM Interface allows you to create mock APIs conversationally using natural language commands, powered by LLM interpretation. Generate OpenAPI specifications and mock APIs from voice or text commands.

## Overview

The Voice + LLM Interface provides:

- **Voice Command Parsing**: Use natural language to describe APIs
- **OpenAPI Generation**: Automatically generate OpenAPI 3.0 specifications
- **Conversational Mode**: Multi-turn interactions for complex APIs
- **Single-Shot Mode**: Complete API generation in one command
- **CLI and Web UI**: Use from command line or web interface

## Quick Start

### CLI Usage

#### Single-Shot Mode

Create a complete API in one command:

```bash
# Create API from text command
mockforge voice create \
  --command "Create a user management API with endpoints for listing users, getting a user by ID, creating users, and updating users" \
  --output api.yaml

# Or use interactive input
mockforge voice create
# Enter your command when prompted
```

#### Conversational Mode

Build APIs through conversation:

```bash
# Start interactive conversation
mockforge voice interactive

# Example conversation:
# > Create a user management API
# > Add an endpoint to get user by email
# > Add authentication to all endpoints
# > Show me the spec
# > done
```

### Web UI Usage

1. Navigate to **Voice** page in Admin UI
2. Click microphone or type your command
3. View generated OpenAPI spec
4. Download or use the spec

## Features

### Natural Language Commands

Describe your API in plain English:

```
Create a REST API for an e-commerce store with:
- Product catalog with categories
- Shopping cart management
- Order processing
- User authentication
```

### OpenAPI Generation

Automatically generates complete OpenAPI 3.0 specifications:

```yaml
openapi: 3.0.0
info:
  title: E-commerce Store API
  version: 1.0.0
paths:
  /products:
    get:
      summary: List products
      responses:
        '200':
          description: List of products
  /cart:
    post:
      summary: Add item to cart
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              properties:
                product_id:
                  type: integer
                quantity:
                  type: integer
```

### Conversational Mode

Build complex APIs through multiple interactions:

```
> Create a blog API
✓ Created blog API with posts endpoint

> Add comments to posts
✓ Added comments endpoint with post_id relationship

> Add user authentication
✓ Added authentication to all endpoints

> Show me the spec
[Displays generated OpenAPI spec]

> done
✓ Saved to blog-api.yaml
```

### Single-Shot Mode

Generate complete APIs in one command:

```bash
mockforge voice create \
  --command "Create a task management API with CRUD operations for tasks, projects, and users" \
  --output task-api.yaml
```

## CLI Commands

### Create (Single-Shot)

```bash
mockforge voice create \
  --command "<description>" \
  --output <file> \
  --format yaml \
  --ai-provider ollama \
  --ai-model llama3.2
```

**Options:**
- `--command`: Natural language description of API
- `--output`: Output file path (default: `generated-api.yaml`)
- `--format`: Output format (`yaml` or `json`)
- `--ai-provider`: LLM provider (`ollama`, `openai`, `anthropic`)
- `--ai-model`: Model name (e.g., `llama3.2`, `gpt-3.5-turbo`)

### Interactive (Conversational)

```bash
mockforge voice interactive \
  --ai-provider ollama \
  --ai-model llama3.2
```

**Special Commands:**
- `help` - Show available commands
- `show spec` - Display current OpenAPI spec
- `save <file>` - Save spec to file
- `done` - Exit and save
- `exit` - Exit without saving

## Web UI

### Voice Input

Use Web Speech API for voice input:

1. Click microphone button
2. Speak your command
3. View real-time transcript
4. See generated spec

### Text Input

Type commands directly:

1. Enter command in text field
2. Click "Generate" or press Enter
3. View generated spec
4. Download or use spec

### Command History

View last 10 commands:

- Click on history item to reuse
- Edit before regenerating
- Save successful commands

## Configuration

### AI Provider Configuration

```yaml
voice:
  enabled: true
  ai_provider: "ollama"  # or "openai", "anthropic"
  ai_model: "llama3.2"
  ai_base_url: "http://localhost:11434"  # For Ollama
  ai_api_key: "${AI_API_KEY}"  # For OpenAI/Anthropic
```

### CLI Configuration

```bash
# Set AI provider via environment
export MOCKFORGE_VOICE_AI_PROVIDER=ollama
export MOCKFORGE_VOICE_AI_MODEL=llama3.2
export MOCKFORGE_VOICE_AI_BASE_URL=http://localhost:11434

# Or use OpenAI
export MOCKFORGE_VOICE_AI_PROVIDER=openai
export MOCKFORGE_VOICE_AI_MODEL=gpt-3.5-turbo
export MOCKFORGE_VOICE_AI_API_KEY=sk-...
```

## API Endpoints

### Process Voice Command

```http
POST /api/v2/voice/process
Content-Type: application/json

{
  "command": "Create a user management API",
  "mode": "single_shot",  # or "conversational"
  "conversation_id": null  # For conversational mode
}
```

**Response:**
```json
{
  "success": true,
  "spec": {
    "openapi": "3.0.0",
    "info": {...},
    "paths": {...}
  },
  "conversation_id": "uuid"  # For conversational mode
}
```

### Continue Conversation

```http
POST /api/v2/voice/process
Content-Type: application/json

{
  "command": "Add authentication",
  "mode": "conversational",
  "conversation_id": "uuid"
}
```

## Use Cases

### Rapid Prototyping

Quickly create API prototypes:

```bash
mockforge voice create \
  --command "Create a simple todo API with CRUD operations" \
  --output todo-api.yaml
```

### API Design

Design APIs by describing them:

```bash
mockforge voice interactive

# > Create a social media API
# > Add posts, comments, and likes
# > Add user profiles
# > Show me the spec
```

### Learning

Learn OpenAPI by example:

```bash
# Generate spec
mockforge voice create --command "..."

# Review generated spec
cat generated-api.yaml
```

## Best Practices

1. **Be Specific**: Provide clear, detailed descriptions
2. **Iterate**: Use conversational mode for complex APIs
3. **Review Generated Specs**: Always review and validate generated specs
4. **Use Local LLMs**: Use Ollama for faster, free generation
5. **Save Good Examples**: Save successful commands for reuse

## Troubleshooting

### Command Not Understood

- Be more specific in your description
- Break complex APIs into smaller parts
- Use conversational mode for clarification

### Spec Generation Fails

- Check AI provider is accessible
- Verify API key is set (for OpenAI/Anthropic)
- Review server logs for errors

### Voice Input Not Working

- Check browser permissions for microphone
- Verify Web Speech API is supported
- Use text input as fallback

## Related Documentation

- [Generative Schema Mode](generative-schema.md) - JSON-based API generation
- [OpenAPI Integration](http-mocking/openapi.md) - Working with OpenAPI specs
- [Configuration Guide](../configuration/files.md) - Complete configuration reference

