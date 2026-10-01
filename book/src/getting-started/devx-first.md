# DevX-First Onboarding

**Pillars:** [DevX]

[DevX] - SDKs, generators, playgrounds, ergonomics

## Start Here If...

You care about **developer experience**. You want easy-to-use SDKs, code generators, interactive playgrounds, and ergonomic tooling that makes working with mocks seamless.

Perfect for:
- Developers who want to integrate mocks into their test suites quickly
- Teams needing client code generation for their APIs
- Developers who prefer interactive playgrounds over configuration files
- Teams wanting plugin-based extensibility

## Quick Start: 5 Minutes

Let's get started with MockForge's developer experience features:

```bash
# Install MockForge CLI
cargo install mockforge-cli

# Or use npm for Node.js SDK
npm install @mockforge-dev/sdk

# Or pip for Python SDK
pip install mockforge-sdk
```

### Option 1: Use the SDK in Your Tests

**Rust:**
```rust
use mockforge_sdk::MockServer;

#[tokio::test]
async fn test_user_api() {
    let mut server = MockServer::new()
        .port(3000)
        .start()
        .await
        .expect("Failed to start server");

    server
        .stub_response("GET", "/api/users/{id}", json!({
            "id": "123",
            "name": "Test User",
            "email": "test@example.com"
        }))
        .await
        .expect("Failed to stub response");

    // Your test code here
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:3000/api/users/123")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    
    server.stop().await.expect("Failed to stop server");
}
```

**Node.js:**
```javascript
const { MockServer } = require('@mockforge-dev/sdk');

describe('User API', () => {
  let server;

  beforeAll(async () => {
    server = new MockServer({ port: 3000 });
    await server.start();
    
    await server.stubResponse('GET', '/api/users/{id}', {
      id: '123',
      name: 'Test User',
      email: 'test@example.com'
    });
  });

  afterAll(async () => {
    await server.stop();
  });

  it('should return user data', async () => {
    const response = await fetch('http://localhost:3000/api/users/123');
    expect(response.status).toBe(200);
    const data = await response.json();
    expect(data.name).toBe('Test User');
  });
});
```

**Python:**
```python
from mockforge_sdk import MockServer

def test_user_api():
    server = MockServer(port=3000)
    server.start()
    
    server.stub_response('GET', '/api/users/{id}', {
        'id': '123',
        'name': 'Test User',
        'email': 'test@example.com'
    })
    
    # Your test code here
    import requests
    response = requests.get('http://localhost:3000/api/users/123')
    assert response.status_code == 200
    
    server.stop()
```

### Option 2: Use the Interactive Playground

The playground is a page in the Admin UI. Start the server with the admin UI:

```bash
mockforge serve --spec openapi.yaml --admin
```

Then open `http://localhost:9080` in your browser and select **Playground** to
send REST and GraphQL requests to your mocks interactively.

### Option 3: Generate Client Code

From an OpenAPI spec:

```bash
# Generate a React client (hooks + TypeScript types)
mockforge client generate --framework react --spec openapi.yaml --output ./src/api --include-types

# Other frameworks: vue, angular, svelte
mockforge client generate --framework vue --spec openapi.yaml --output ./src/api

# List supported frameworks
mockforge client list
```

## Key DevX Features

### 1. Multi-Language SDKs

MockForge provides SDKs for:
- **Rust** - Native SDK with full type safety
- **Node.js/TypeScript** - NPM package with TypeScript definitions
- **Python** - Pip package with async support
- **Go** - Go module with idiomatic API
- **Java** - Maven/Gradle package
- **.NET** - NuGet package

### 2. Client Code Generation

Generate type-safe clients for:
- **TypeScript/JavaScript** - Full type definitions
- **Python** - With async/await support
- **React** - Custom hooks for data fetching
- **Vue** - Composition API composables
- **Angular** - Injectable services
- **Svelte** - Reactive stores

### 3. Interactive Playground

The playground (Admin UI) provides:
- A list of the endpoints your mocks serve
- REST and GraphQL request execution, with GraphQL introspection
- Request history with replay
- Code snippet generation for a request

### 4. CLI Tooling

Comprehensive CLI commands:
- `mockforge serve` - Start mock server
- `mockforge generate` - Generate mock servers from OpenAPI specs
- `mockforge config validate` - Validate configurations
- `mockforge sync` - Bidirectional workspace directory sync

### 5. Plugin System

Extend MockForge with plugins:
- Custom authentication handlers
- Data source plugins
- Response generators
- Template token resolvers

## Next Steps

1. **Explore SDKs**: Choose your language and integrate mocks into your tests
2. **Try the Playground**: Use the Admin UI to send requests to your mocks interactively
3. **Generate Clients**: Create type-safe API clients from your OpenAPI specs
4. **Build Plugins**: Extend MockForge with custom functionality

## Cross-Pillar Exploration

Once you've mastered DevX, explore these complementary pillars:

- **Add realism** → Explore [Reality](reality-first.md) features
- **Add validation** → Explore [Contracts](contracts-first.md) features
- **Enable collaboration** → Explore [Cloud](../docs/PILLARS.md#cloud--registry-orgs-governance-monetization-marketplace) features
- **Enhance with AI** → Explore [AI](ai-first.md) features

## Resources

- [SDK Documentation](../reference/sdk.md)
- [CLI Reference](../reference/cli.md)
- [Plugin Development Guide](../contributing/plugins.md)
- [Client Generation Guide](../guides/client-generation.md)

