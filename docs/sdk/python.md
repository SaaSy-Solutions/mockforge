# MockForge Python SDK

Build MockForge plugins in Python that run as remote services.

## Installation

```bash
pip install mockforge-plugin

# With FastAPI support (recommended)
pip install mockforge-plugin[fastapi]
```

## Overview

The Python SDK allows you to build plugins that run as standalone HTTP services. MockForge communicates with your plugin over HTTP, which means you can:

- Use any Python library (unlike WASM plugins)
- Access databases, APIs, and file systems
- Run anywhere Python runs (Docker, serverless, etc.)

## Quick Start

```python
from mockforge_plugin import RemotePlugin, PluginContext, AuthCredentials, AuthResult

class MyAuthPlugin(RemotePlugin):
    async def authenticate(
        self, ctx: PluginContext, creds: AuthCredentials
    ) -> AuthResult:
        if creds.token == "valid-token":
            return AuthResult(
                authenticated=True,
                user_id="user123",
                claims={"role": "admin"}
            )
        return AuthResult(authenticated=False, user_id="", claims={})

if __name__ == "__main__":
    plugin = MyAuthPlugin(name="My Auth Plugin", version="1.0.0")
    plugin.run(host="0.0.0.0", port=8080)
```

## Plugin Types

### Authentication Plugin

Validate credentials and extract user information.

```python
from mockforge_plugin import (
    RemotePlugin, PluginContext, AuthCredentials, AuthResult
)
import jwt
import httpx

class JWTAuthPlugin(RemotePlugin):
    def __init__(self, jwks_url: str):
        super().__init__(name="JWT Auth", version="1.0.0")
        self.jwks_url = jwks_url
        self.jwks_client = jwt.PyJWKClient(jwks_url)

    async def authenticate(
        self, ctx: PluginContext, creds: AuthCredentials
    ) -> AuthResult:
        try:
            # Get signing key from JWKS
            signing_key = self.jwks_client.get_signing_key_from_jwt(creds.token)

            # Decode and validate JWT
            payload = jwt.decode(
                creds.token,
                signing_key.key,
                algorithms=["RS256"],
                audience="my-api"
            )

            return AuthResult(
                authenticated=True,
                user_id=payload["sub"],
                claims={
                    "email": payload.get("email"),
                    "roles": payload.get("roles", []),
                    "exp": payload.get("exp")
                }
            )
        except jwt.InvalidTokenError as e:
            return AuthResult(
                authenticated=False,
                user_id="",
                claims={"error": str(e)}
            )

if __name__ == "__main__":
    plugin = JWTAuthPlugin("https://auth.example.com/.well-known/jwks.json")
    plugin.run(port=8080)
```

### Template Function Plugin

Add custom Handlebars helpers.

```python
from mockforge_plugin import (
    RemotePlugin, ResolutionContext, TemplateFunction, FunctionParameter
)
import hashlib
import hmac
import base64

class CryptoPlugin(RemotePlugin):
    async def execute_template_function(
        self, function_name: str, args: list, ctx: ResolutionContext
    ):
        match function_name:
            case "sha256":
                return hashlib.sha256(str(args[0]).encode()).hexdigest()

            case "hmac_sha256":
                key = args[0].encode()
                message = args[1].encode()
                return hmac.new(key, message, hashlib.sha256).hexdigest()

            case "base64_encode":
                return base64.b64encode(str(args[0]).encode()).decode()

            case "base64_decode":
                return base64.b64decode(args[0]).decode()

            case _:
                raise ValueError(f"Unknown function: {function_name}")

    def get_functions(self) -> list:
        return [
            TemplateFunction(
                name="sha256",
                description="SHA256 hash of input",
                parameters=[
                    FunctionParameter("input", "string", True, "Value to hash")
                ],
                return_type="string"
            ),
            TemplateFunction(
                name="hmac_sha256",
                description="HMAC-SHA256 signature",
                parameters=[
                    FunctionParameter("key", "string", True, "Secret key"),
                    FunctionParameter("message", "string", True, "Message to sign")
                ],
                return_type="string"
            ),
            TemplateFunction(
                name="base64_encode",
                description="Base64 encode",
                parameters=[
                    FunctionParameter("input", "string", True, "Value to encode")
                ],
                return_type="string"
            ),
            TemplateFunction(
                name="base64_decode",
                description="Base64 decode",
                parameters=[
                    FunctionParameter("input", "string", True, "Value to decode")
                ],
                return_type="string"
            ),
        ]

if __name__ == "__main__":
    plugin = CryptoPlugin(name="Crypto Functions", version="1.0.0")
    plugin.run(port=8080)
```

### Response Generator Plugin

Generate dynamic responses.

```python
from mockforge_plugin import (
    RemotePlugin, PluginContext, ResponseRequest, ResponseData
)
import json
from faker import Faker

class FakeDataPlugin(RemotePlugin):
    def __init__(self):
        super().__init__(name="Fake Data Generator", version="1.0.0")
        self.fake = Faker()

    async def generate_response(
        self, ctx: PluginContext, req: ResponseRequest
    ) -> ResponseData:
        # Parse path to determine what to generate
        path_parts = req.path.strip("/").split("/")

        if path_parts[0] == "users":
            count = int(path_parts[1]) if len(path_parts) > 1 else 1
            users = [self._generate_user() for _ in range(count)]
            body = json.dumps(users if count > 1 else users[0])

        elif path_parts[0] == "orders":
            count = int(path_parts[1]) if len(path_parts) > 1 else 1
            orders = [self._generate_order() for _ in range(count)]
            body = json.dumps(orders if count > 1 else orders[0])

        else:
            body = json.dumps({"error": "Unknown resource"})
            return ResponseData(
                status_code=404,
                headers={"Content-Type": "application/json"},
                body=body.encode(),
                content_type="application/json"
            )

        return ResponseData(
            status_code=200,
            headers={"Content-Type": "application/json"},
            body=body.encode(),
            content_type="application/json"
        )

    def _generate_user(self):
        return {
            "id": self.fake.uuid4(),
            "name": self.fake.name(),
            "email": self.fake.email(),
            "phone": self.fake.phone_number(),
            "address": {
                "street": self.fake.street_address(),
                "city": self.fake.city(),
                "country": self.fake.country()
            },
            "created_at": self.fake.iso8601()
        }

    def _generate_order(self):
        return {
            "id": f"ORD-{self.fake.random_number(digits=8)}",
            "customer_id": self.fake.uuid4(),
            "items": [
                {
                    "sku": self.fake.ean13(),
                    "name": self.fake.catch_phrase(),
                    "quantity": self.fake.random_int(1, 5),
                    "price": float(self.fake.pricetag().replace("$", "").replace(",", ""))
                }
                for _ in range(self.fake.random_int(1, 5))
            ],
            "status": self.fake.random_element(["pending", "shipped", "delivered"]),
            "created_at": self.fake.iso8601()
        }

if __name__ == "__main__":
    plugin = FakeDataPlugin()
    plugin.run(port=8080)
```

### Data Source Plugin

Connect external data sources.

```python
from mockforge_plugin import (
    RemotePlugin, PluginContext, DataQuery, DataResult, ColumnInfo
)
import asyncpg

class PostgresPlugin(RemotePlugin):
    def __init__(self, database_url: str):
        super().__init__(name="PostgreSQL Data Source", version="1.0.0")
        self.database_url = database_url
        self.pool = None

    async def _ensure_pool(self):
        if self.pool is None:
            self.pool = await asyncpg.create_pool(self.database_url)

    async def query_datasource(
        self, query: DataQuery, ctx: PluginContext
    ) -> DataResult:
        await self._ensure_pool()

        # Execute query with parameters
        params = list(query.parameters.values())
        rows = await self.pool.fetch(query.query, *params)

        if not rows:
            return DataResult(columns=[], rows=[])

        # Build column info from first row
        columns = [
            ColumnInfo(name=key, data_type=self._get_type(value))
            for key, value in rows[0].items()
        ]

        # Convert rows to dicts
        data = [dict(row) for row in rows]

        return DataResult(columns=columns, rows=data)

    def _get_type(self, value) -> str:
        if isinstance(value, int):
            return "integer"
        elif isinstance(value, float):
            return "float"
        elif isinstance(value, bool):
            return "boolean"
        elif isinstance(value, (list, dict)):
            return "json"
        else:
            return "string"

if __name__ == "__main__":
    import os
    plugin = PostgresPlugin(os.environ["DATABASE_URL"])
    plugin.run(port=8080)
```

## Configuration

### Plugin Capabilities

Declare what resources your plugin needs:

```python
from mockforge_plugin import (
    RemotePlugin, PluginCapabilities, NetworkCapabilities,
    FilesystemCapabilities, ResourceLimits
)

class MyPlugin(RemotePlugin):
    def get_capabilities(self) -> PluginCapabilities:
        return PluginCapabilities(
            network=NetworkCapabilities(
                allow_http_outbound=True,
                allowed_hosts=["api.example.com", "auth.example.com"]
            ),
            filesystem=FilesystemCapabilities(
                allow_read=True,
                allow_write=False,
                allowed_paths=["/app/config", "/app/data"]
            ),
            resources=ResourceLimits(
                max_memory_bytes=50 * 1024 * 1024,  # 50MB
                max_cpu_time_ms=5000  # 5 seconds
            )
        )
```

### Custom FastAPI Routes

Add additional endpoints to your plugin:

```python
class MyPlugin(RemotePlugin):
    def __init__(self):
        super().__init__()

        # Add custom routes
        @self.app.get("/custom/status")
        async def custom_status():
            return {"custom": "data", "status": "ok"}

        @self.app.post("/custom/webhook")
        async def custom_webhook(data: dict):
            # Process webhook
            return {"received": True}
```

## Deployment

### Docker

```dockerfile
FROM python:3.11-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY plugin.py .

EXPOSE 8080

CMD ["python", "plugin.py"]
```

```yaml
# docker-compose.yaml
services:
  auth-plugin:
    build: .
    ports:
      - "8080:8080"
    environment:
      - JWKS_URL=https://auth.example.com/.well-known/jwks.json
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mockforge-auth-plugin
spec:
  replicas: 2
  selector:
    matchLabels:
      app: auth-plugin
  template:
    metadata:
      labels:
        app: auth-plugin
    spec:
      containers:
        - name: plugin
          image: myregistry/auth-plugin:1.0.0
          ports:
            - containerPort: 8080
          env:
            - name: JWKS_URL
              value: "https://auth.example.com/.well-known/jwks.json"
          livenessProbe:
            httpGet:
              path: /health
              port: 8080
          readinessProbe:
            httpGet:
              path: /health
              port: 8080
---
apiVersion: v1
kind: Service
metadata:
  name: auth-plugin
spec:
  selector:
    app: auth-plugin
  ports:
    - port: 8080
```

### MockForge Configuration

Register your remote plugin in MockForge:

```yaml
# mockforge.yaml
plugins:
  remote:
    - id: "jwt-auth"
      type: "auth"
      url: "http://auth-plugin:8080"
      health_check_interval: 30
      timeout_ms: 5000
      retries: 3

    - id: "fake-data"
      type: "response"
      url: "http://fake-data-plugin:8080"

    - id: "postgres-source"
      type: "datasource"
      url: "http://postgres-plugin:8080"
```

## Testing

```python
import pytest
from httpx import AsyncClient
from plugin import MyAuthPlugin

@pytest.fixture
def plugin():
    return MyAuthPlugin()

@pytest.fixture
async def client(plugin):
    async with AsyncClient(app=plugin.app, base_url="http://test") as client:
        yield client

@pytest.mark.asyncio
async def test_health_check(client):
    response = await client.get("/health")
    assert response.status_code == 200
    assert response.json()["status"] == "healthy"

@pytest.mark.asyncio
async def test_authenticate_valid_token(client):
    response = await client.post("/plugin/authenticate", json={
        "context": {
            "method": "GET",
            "uri": "/api/users",
            "headers": {}
        },
        "credentials": {
            "type": "bearer",
            "token": "valid-token"
        }
    })

    assert response.status_code == 200
    data = response.json()
    assert data["success"] is True
    assert data["result"]["authenticated"] is True

@pytest.mark.asyncio
async def test_authenticate_invalid_token(client):
    response = await client.post("/plugin/authenticate", json={
        "context": {
            "method": "GET",
            "uri": "/api/users",
            "headers": {}
        },
        "credentials": {
            "type": "bearer",
            "token": "invalid"
        }
    })

    assert response.status_code == 200
    data = response.json()
    assert data["result"]["authenticated"] is False
```

## Error Handling

```python
from mockforge_plugin import RemotePlugin, AuthResult
import logging

logger = logging.getLogger(__name__)

class RobustPlugin(RemotePlugin):
    async def authenticate(self, ctx, creds) -> AuthResult:
        try:
            # Attempt authentication
            result = await self._validate_token(creds.token)
            return AuthResult(
                authenticated=result.valid,
                user_id=result.user_id,
                claims=result.claims
            )
        except ConnectionError as e:
            logger.error(f"Connection error during auth: {e}")
            # Fail open or closed based on policy
            return AuthResult(
                authenticated=False,
                user_id="",
                claims={"error": "service_unavailable"}
            )
        except Exception as e:
            logger.exception("Unexpected error during authentication")
            raise  # Let FastAPI handle it
```

## See Also

- [SDK Overview](./README.md)
- [Node.js SDK](./nodejs.md)
- [Go SDK](./go.md)
- [Plugin Development Guide](../plugins/development-guide.md)
