# AI-Powered API Specification Suggestions

This directory contains examples for using MockForge's AI-powered specification suggestion feature.

## Overview

The `mockforge suggest` command uses AI to generate complete OpenAPI specifications or MockForge configurations from minimal input. It can expand a single endpoint example into a full API design with realistic endpoints, schemas, and documentation.

## Example Files

### 1. Single Endpoint Example (`single-endpoint.json`)

Provide a single GET endpoint, and MockForge will suggest related CRUD operations and additional endpoints.

**Usage:**
```bash
mockforge suggest --from single-endpoint.json --output api-spec.yaml --num-suggestions 10
```

### 2. Endpoint with Request Body (`endpoint-with-request.json`)

Provide a POST endpoint with both request and response examples. MockForge will analyze the data structures and suggest a complete e-commerce API.

**Usage:**
```bash
mockforge suggest --from endpoint-with-request.json --format both --output ecommerce-api --domain e-commerce
```

This creates:
- `ecommerce-api.openapi.yaml` - Full OpenAPI specification
- `ecommerce-api.mockforge.yaml` - MockForge configuration

### 3. Paths Only (`paths-only.json`)

Provide just a list of endpoint paths, and MockForge will infer the resource model and generate appropriate HTTP methods, schemas, and responses.

**Usage:**
```bash
mockforge suggest --from paths-only.json --output suggested-spec.yaml
```

### 4. From Description

No file needed! Just describe your API in natural language.

**Usage:**
```bash
mockforge suggest --from-description "A blog API with posts, comments, tags, and user authentication" --output blog-api.yaml
```

## Command Options

```bash
mockforge suggest [OPTIONS]

Options:
  -f, --from <FILE>                    Input JSON file
      --from-description <TEXT>        Generate from description
      --format <FORMAT>                Output format: openapi, mockforge, or both [default: openapi]
  -o, --output <FILE>                  Output file path
      --num-suggestions <N>            Number of additional endpoints [default: 5]
      --domain <DOMAIN>                API domain hint (e-commerce, social-media, fintech, etc.)
      --llm-provider <PROVIDER>        LLM provider [default: openai]
      --llm-model <MODEL>              LLM model name
      --llm-api-key <KEY>              API key (or set env var)
      --temperature <TEMP>             Temperature for generation [default: 0.7]
      --print-json                     Print results as JSON
```

## Input Formats

### Single Endpoint Format
```json
{
  "method": "GET|POST|PUT|DELETE|PATCH",
  "path": "/api/resource/{id}",
  "description": "Optional description",
  "request": { /* optional request body example */ },
  "response": { /* optional response body example */ }
}
```

### Paths List Format
```json
{
  "paths": [
    "/api/resource1",
    "/api/resource2/{id}"
  ]
}
```

### Partial OpenAPI Spec Format
```json
{
  "openapi": "3.0.0",
  "info": { "title": "My API" },
  "paths": {
    "/users": {
      "get": { /* partial definition */ }
    }
  }
}
```

## LLM Provider Configuration

### OpenAI (default)
```bash
export OPENAI_API_KEY="your-key"
mockforge suggest --from example.json --output spec.yaml
```

### Anthropic Claude
```bash
export ANTHROPIC_API_KEY="your-key"
mockforge suggest --from example.json --llm-provider anthropic --output spec.yaml
```

### Local Ollama
```bash
mockforge suggest --from example.json --llm-provider ollama --llm-model llama3.1 --output spec.yaml
```

### Custom OpenAI-Compatible API
```bash
mockforge suggest --from example.json \
  --llm-provider openai-compatible \
  --llm-endpoint https://your-api.com/v1/chat/completions \
  --llm-api-key your-key \
  --output spec.yaml
```

## Example Workflow

1. **Start with a single endpoint:**
   ```bash
   mockforge suggest --from single-endpoint.json --output full-api.yaml --num-suggestions 15
   ```

2. **Review the suggestions:**
   ```bash
   cat full-api.yaml
   ```

3. **Use the generated spec with MockForge:**
   ```bash
   mockforge serve --spec full-api.yaml --http-port 3000
   ```

4. **Test your mock API:**
   ```bash
   curl http://localhost:3000/api/users/123
   ```

## Tips

- Use `--domain` to help the AI understand your API's context (e.g., `e-commerce`, `fintech`, `healthcare`)
- Higher `--temperature` (0.8-1.0) generates more creative suggestions
- Lower `--temperature` (0.3-0.5) generates more conservative, predictable suggestions
- Use `--format both` to get both OpenAPI and MockForge configs
- The `--print-json` flag is useful for programmatic processing

## Advanced Example

Generate a complete fintech API from a description with high creativity:

```bash
mockforge suggest \
  --from-description "A banking API with accounts, transactions, transfers, bill payments, and account statements" \
  --domain fintech \
  --num-suggestions 20 \
  --temperature 0.9 \
  --format both \
  --output banking-api \
  --llm-provider anthropic \
  --llm-model claude-3-5-sonnet-20241022
```

This creates:
- `banking-api.openapi.yaml` - Complete OpenAPI 3.0 spec
- `banking-api.mockforge.yaml` - MockForge configuration with realistic examples

Then serve it:
```bash
mockforge serve --spec banking-api.mockforge.yaml --http-port 8080
```
