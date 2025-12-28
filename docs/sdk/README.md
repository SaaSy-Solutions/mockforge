# MockForge SDK Documentation

MockForge provides SDKs for multiple languages to integrate with your testing workflows and extend functionality.

## Available SDKs

| SDK | Use Case | Installation |
|-----|----------|--------------|
| [Node.js](./nodejs.md) | Embed mock servers in JS/TS tests | `npm install @mockforge/sdk` |
| [Python](./python.md) | Build remote plugins | `pip install mockforge-plugin` |
| [Go](./go.md) | Build WASM plugins with TinyGo | `go get github.com/mockforge/sdk/go` |
| [.NET](./dotnet.md) | Embed in C# tests | `dotnet add package MockForge.Sdk` |

## SDK Types

### 1. Testing SDKs

Embed MockForge servers directly in your test suites:

```typescript
// Node.js example
import { MockServer } from '@mockforge/sdk';

const server = await MockServer.start();
await server.stubResponse('GET', '/api/users/1', { id: 1, name: 'Test' });
// Run tests against server.url()
await server.stop();
```

### 2. Plugin SDKs

Extend MockForge with custom logic:

```python
# Python remote plugin
from mockforge_plugin import RemotePlugin, AuthResult

class MyPlugin(RemotePlugin):
    async def authenticate(self, ctx, creds):
        return AuthResult(authenticated=True, user_id="user1")
```

```go
// Go WASM plugin
func (p *MyPlugin) Authenticate(ctx *PluginContext, creds *AuthCredentials) (*AuthResult, error) {
    return &AuthResult{Authenticated: true, UserID: "user1"}, nil
}
```

## Quick Start by Use Case

### Testing HTTP APIs

```typescript
// Jest / Vitest
import { MockServer } from '@mockforge/sdk';

describe('User API', () => {
  let server: MockServer;

  beforeAll(async () => {
    server = await MockServer.start({ port: 0 });
  });

  afterAll(async () => {
    await server.stop();
  });

  it('fetches user data', async () => {
    await server.stubResponse('GET', '/users/1', {
      id: 1,
      name: 'Alice',
      email: 'alice@example.com'
    });

    const res = await fetch(`${server.url()}/users/1`);
    const user = await res.json();

    expect(user.name).toBe('Alice');
  });
});
```

### Custom Authentication

```python
# Python plugin for OAuth validation
from mockforge_plugin import RemotePlugin, PluginContext, AuthCredentials, AuthResult
import requests

class OAuthPlugin(RemotePlugin):
    async def authenticate(self, ctx: PluginContext, creds: AuthCredentials) -> AuthResult:
        # Validate token with OAuth provider
        response = requests.post(
            "https://oauth.example.com/introspect",
            data={"token": creds.token}
        )

        if response.ok and response.json().get("active"):
            return AuthResult(
                authenticated=True,
                user_id=response.json()["sub"],
                claims=response.json()
            )

        return AuthResult(authenticated=False, user_id="")

if __name__ == "__main__":
    OAuthPlugin().run(port=8080)
```

### Custom Template Functions

```go
// Go plugin for custom Handlebars helpers
package main

import "github.com/mockforge/sdk/go/mockforge"

type HashPlugin struct{}

func (p *HashPlugin) ExecuteFunction(name string, args []interface{}, ctx *mockforge.ResolutionContext) (interface{}, error) {
    switch name {
    case "sha256":
        return hashSHA256(args[0].(string)), nil
    case "bcrypt":
        return hashBcrypt(args[0].(string), args[1].(int)), nil
    }
    return nil, fmt.Errorf("unknown function: %s", name)
}

func (p *HashPlugin) GetFunctions() []mockforge.TemplateFunction {
    return []mockforge.TemplateFunction{
        {Name: "sha256", Description: "SHA256 hash"},
        {Name: "bcrypt", Description: "Bcrypt hash with cost"},
    }
}
```

### Custom Data Sources

```python
# Python plugin for PostgreSQL data source
from mockforge_plugin import RemotePlugin, DataQuery, DataResult, ColumnInfo
import asyncpg

class PostgresPlugin(RemotePlugin):
    def __init__(self):
        super().__init__()
        self.pool = None

    async def query_datasource(self, query: DataQuery, ctx) -> DataResult:
        if not self.pool:
            self.pool = await asyncpg.create_pool("postgresql://...")

        rows = await self.pool.fetch(query.query, *query.parameters.values())

        columns = [ColumnInfo(name=k, data_type="text") for k in rows[0].keys()]
        return DataResult(columns=columns, rows=[dict(r) for r in rows])
```

## Common Patterns

### Contract Testing

```typescript
import { MockServer } from '@mockforge/sdk';
import SwaggerParser from '@apidevtools/swagger-parser';

describe('API Contract', () => {
  it('matches OpenAPI spec', async () => {
    const server = await MockServer.start({
      openApiSpec: './api.yaml'
    });

    // Request validation is automatic
    const res = await fetch(`${server.url()}/users`, {
      method: 'POST',
      body: JSON.stringify({ invalid: 'data' })
    });

    expect(res.status).toBe(400); // Validation error
    await server.stop();
  });
});
```

### Request Verification

```typescript
const server = await MockServer.start();

await server.stubResponse('POST', '/orders', { id: 'order-1' });

// Make requests
await fetch(`${server.url()}/orders`, { method: 'POST', body: '{"item":"A"}' });
await fetch(`${server.url()}/orders`, { method: 'POST', body: '{"item":"B"}' });

// Verify calls
const result = await server.verify(
  { method: 'POST', path: '/orders' },
  { type: 'exactly', value: 2 }
);
expect(result.matched).toBe(true);

// Verify sequence
await server.verifySequence([
  { method: 'POST', path: '/orders', bodyPattern: '{"item":"A"}' },
  { method: 'POST', path: '/orders', bodyPattern: '{"item":"B"}' }
]);
```

### Dynamic Responses

```typescript
const server = await MockServer.start();

// Stateful counter
let counter = 0;
server.onRequest('GET', '/counter', () => {
  counter++;
  return { count: counter };
});

// Conditional responses
server.onRequest('GET', '/feature', (req) => {
  if (req.headers['x-beta-user']) {
    return { features: ['new-ui', 'dark-mode'] };
  }
  return { features: ['basic'] };
});
```

## See Also

- [Node.js SDK Reference](./nodejs.md)
- [Python SDK Reference](./python.md)
- [Go SDK Reference](./go.md)
- [Plugin Development Guide](../plugins/development-guide.md)
