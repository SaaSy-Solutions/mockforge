# HTTP AI Integration Example

This document demonstrates how to integrate AI features with HTTP responses in MockForge.

## Overview

The AI handler integration allows MockResponse objects to use:
1. **Intelligent Mock Generation** - Generate responses from natural language prompts
2. **Data Drift Simulation** - Evolve response data across requests

## MockResponse Structure

MockResponse now includes two optional AI configuration fields:

```rust
pub struct MockResponse {
    // ... existing fields ...

    /// AI-powered intelligent mock generation config
    pub intelligent: Option<serde_json::Value>,

    /// Data drift simulation config
    pub drift: Option<serde_json::Value>,
}
```

## Usage in Request Handlers

### Basic Example

```rust
use mockforge_http::process_response_with_ai;
use serde_json::json;

async fn handle_request(mock_response: &MockResponse) -> Result<Response> {
    // Parse the base response body
    let base_body = serde_json::from_str(&mock_response.body).ok();

    // Apply AI features if configured
    let processed_body = process_response_with_ai(
        base_body,
        mock_response.intelligent.clone(),
        mock_response.drift.clone(),
    ).await?;

    // Return response with AI-processed body
    Ok(Response::builder()
        .status(mock_response.status_code)
        .body(processed_body.to_string())
        .unwrap())
}
```

### YAML Configuration Example

```yaml
responses:
  - name: "Intelligent Customer Response"
    status_code: 200
    body: '{}'  # Base template (optional with intelligent mode)
    intelligent:
      mode: intelligent
      prompt: "Generate realistic customer data for a retail SaaS API"
      schema:
        type: object
        properties:
          id: { type: string }
          name: { type: string }
          email: { type: string }
          tier: { type: string, enum: [bronze, silver, gold] }

  - name: "Drifting Order Response"
    status_code: 200
    body: '{"id": "order-123", "status": "pending", "total": 100.00}'
    drift:
      enabled: true
      request_based: true
      rules:
        - field: status
          strategy: state_machine
          states: [pending, processing, shipped, delivered]
          transitions:
            pending: [[processing, 0.8], [cancelled, 0.2]]
            processing: [[shipped, 0.9], [failed, 0.1]]
            shipped: [[delivered, 1.0]]
```

## Integration with Workspace MockRequests

When processing workspace MockRequest objects:

```rust
use mockforge_http::process_response_with_ai;

async fn execute_workspace_request(request: &MockRequest) -> Result<Response> {
    // Get active response
    let response = request.active_response()
        .ok_or("No active response")?;

    // Parse base body
    let base_body = serde_json::from_str(&response.body).ok();

    // Apply AI processing
    let ai_processed = process_response_with_ai(
        base_body,
        response.intelligent.clone(),
        response.drift.clone(),
    ).await?;

    // Build and return response
    Ok(Response::builder()
        .status(response.status_code)
        .body(ai_processed.to_string())
        .unwrap())
}
```

## Advanced: Direct Handler Usage

For more control, use `AiResponseHandler` directly:

```rust
use mockforge_http::AiResponseHandler;
use mockforge_data::{IntelligentMockConfig, DataDriftConfig, ResponseMode};

async fn advanced_ai_processing() -> Result<serde_json::Value> {
    // Configure intelligent generation
    let intelligent_config = IntelligentMockConfig::new(ResponseMode::Intelligent)
        .with_prompt("Generate realistic user data".to_string());

    // Configure drift
    let drift_rule = DriftRule::new("status".to_string(), DriftStrategy::Linear)
        .with_rate(1.0);
    let drift_config = DataDriftConfig::new().with_rule(drift_rule);

    // Create handler
    let mut handler = AiResponseHandler::new(
        Some(intelligent_config),
        Some(drift_config),
    )?;

    // Generate AI-powered response
    let response = handler.generate_response(None).await?;

    Ok(response)
}
```

## Environment Variables

Configure AI providers via environment variables:

```bash
# For OpenAI
export OPENAI_API_KEY=sk-...

# For Anthropic
export ANTHROPIC_API_KEY=sk-ant-...

# For Ollama (local, free)
export OLLAMA_HOST=http://localhost:11434
export OLLAMA_MODEL=llama2
```

## Testing

Test AI integration with unit tests:

```rust
#[tokio::test]
async fn test_ai_response_generation() {
    let intelligent_config = json!({
        "mode": "intelligent",
        "prompt": "Generate test customer data"
    });

    let result = process_response_with_ai(
        None,
        Some(intelligent_config),
        None,
    ).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.is_object());
}
```

## Performance Considerations

- **Caching**: AI responses are automatically cached to reduce API calls
- **Timeouts**: Configure timeouts via RagConfig
- **Retries**: Automatic retry logic with exponential backoff
- **Cost**: Use Ollama for free local development

## Next Steps

1. See `examples/ai/` for complete YAML configurations
2. Read `docs/AI_DRIVEN_MOCKING.md` for detailed feature documentation
3. Review `AI_FEATURES_STATUS.md` for implementation status

## Status

✅ HTTP AI handler integration complete
✅ MockResponse structure updated with AI fields
✅ Helper functions exported and documented
⏳ WebSocket integration pending
⏳ CLI enhancements pending
