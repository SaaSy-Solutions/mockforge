# MockForge Configuration Specification

This document provides the complete specification for MockForge YAML configuration files.

## File Format

MockForge configuration files use YAML format with the `.yaml` or `.yml` extension. The default filename is `mockforge.yaml`.

```yaml
# mockforge.yaml
version: "1.0"

# Server configuration
http:
  port: 3000
  host: "0.0.0.0"

# Endpoint definitions
endpoints:
  - path: "/api/users"
    method: GET
    response:
      body: [{ id: 1, name: "Alice" }]
```

## Configuration Sections

### Server Configuration

#### HTTP Server

```yaml
http:
  port: 3000                      # Port number (default: 3000)
  host: "0.0.0.0"                 # Bind address (default: 0.0.0.0)

  tls:
    enabled: false                # Enable HTTPS
    cert_file: "./certs/cert.pem" # Certificate file path
    key_file: "./certs/key.pem"   # Private key file path
    ca_file: "./certs/ca.pem"     # CA file for client verification
    client_auth: false            # Require client certificates

  cors:
    enabled: true
    allowed_origins:
      - "http://localhost:3000"
      - "https://*.example.com"
    allowed_methods: ["GET", "POST", "PUT", "DELETE"]
    allowed_headers: ["Content-Type", "Authorization"]
    expose_headers: ["X-Request-Id"]
    max_age: 3600
    credentials: true

  rate_limit:
    enabled: false
    requests_per_minute: 60
    burst: 10
```

#### Admin Interface

```yaml
admin:
  enabled: true
  port: 9080
  host: "127.0.0.1"

  api_enabled: true              # Enable REST API
  ui_enabled: true               # Enable web UI

  auth:
    enabled: false
    type: "basic"                # basic, bearer, or api-key
    credentials:
      username: "admin"
      password: "secret"
```

### Protocol Servers

#### WebSocket

```yaml
websocket:
  enabled: true
  port: 8080
  path: "/ws"
  max_connections: 10000
  max_message_size: 65536
  ping_interval: 30

  handlers:
    - path: "/echo"
      type: echo

    - path: "/chat"
      type: pattern
      rules:
        - match: { type: "message" }
          response: { type: "ack" }
```

#### gRPC

```yaml
grpc:
  enabled: true
  port: 50051

  proto_paths:
    - "./protos"

  reflection: true

  tls:
    enabled: false
    cert_path: "./certs/server.crt"
    key_path: "./certs/server.key"

  services:
    - package: "user.v1"
      service: "UserService"
      methods:
        - name: "GetUser"
          response:
            user: { id: "{{request.user_id}}" }
```

#### MQTT

```yaml
mqtt:
  enabled: true
  port: 1883
  host: "0.0.0.0"
  max_connections: 1000
  max_packet_size: 65536
  keep_alive_secs: 60

  tls:
    enabled: false
    port: 8883
    cert_path: "./certs/server.crt"
    key_path: "./certs/server.key"
    ca_path: "./certs/ca.crt"
    client_auth: false

  mocks:
    - topic: "sensors/+/temperature"
      qos: 1
      response:
        payload: '{"value": 23.5}'
```

#### AMQP

```yaml
amqp:
  enabled: true
  port: 5672
  default_vhost: "/"
  default_user: "guest"
  default_password: "guest"

  exchanges:
    - name: "orders"
      type: direct
      durable: true
      bindings:
        - queue: "order-processing"
          routing_key: "new"

  queues:
    - name: "orders"
      durable: true
      arguments:
        x-message-ttl: 60000
```

#### SMTP

```yaml
smtp:
  enabled: true
  port: 2525
  host: "0.0.0.0"
  hostname: "mock-smtp.local"

  auth:
    enabled: false
    users:
      - username: "test"
        password: "test"
```

### Endpoint Definitions

```yaml
endpoints:
  # Basic endpoint
  - path: "/api/users"
    method: GET
    response:
      status: 200
      headers:
        Content-Type: "application/json"
      body:
        - { id: 1, name: "Alice" }
        - { id: 2, name: "Bob" }

  # Path parameters
  - path: "/api/users/{id}"
    method: GET
    response:
      body:
        id: "{{pathParams.id}}"
        name: "User {{pathParams.id}}"

  # Query parameters
  - path: "/api/search"
    method: GET
    response:
      body:
        query: "{{queryParams.q}}"
        page: "{{queryParams.page | default: 1}}"

  # Request body access
  - path: "/api/users"
    method: POST
    response:
      status: 201
      body:
        id: "{{uuid}}"
        name: "{{body.name}}"
        email: "{{body.email}}"

  # Headers access
  - path: "/api/protected"
    method: GET
    response:
      body:
        authenticated: true
        user: "{{headers.X-User-Id}}"

  # Multiple responses (round-robin)
  - path: "/api/flaky"
    method: GET
    responses:
      - status: 200
        body: { success: true }
        weight: 8
      - status: 500
        body: { error: "Server Error" }
        weight: 2

  # Conditional responses
  - path: "/api/feature"
    method: GET
    rules:
      - when:
          headers:
            X-Beta-User: "true"
        response:
          body: { feature: "beta" }
      - response:
          body: { feature: "stable" }

  # Latency simulation
  - path: "/api/slow"
    method: GET
    latency:
      min_ms: 100
      max_ms: 500
    response:
      body: { result: "delayed" }

  # Proxy to upstream
  - path: "/api/external/*"
    method: "*"
    proxy:
      upstream: "https://api.example.com"
      strip_prefix: "/api/external"
      add_headers:
        X-Forwarded-By: "mockforge"
```

### OpenAPI Integration

```yaml
openapi:
  spec: "./api.yaml"              # OpenAPI spec file

  validation:
    request: "warn"               # off, warn, or strict
    response: false               # Validate responses
    aggregate_errors: false       # Collect all errors vs fail fast

  coverage:
    enabled: true
    output: "./coverage"

  generation:
    enabled: true
    realistic: true               # Use AI for realistic data
    examples_first: true          # Prefer spec examples
```

### Data Sources

```yaml
datasources:
  # JSON file
  - id: "users"
    type: "json"
    path: "./data/users.json"

  # CSV file
  - id: "products"
    type: "csv"
    path: "./data/products.csv"
    delimiter: ","
    headers: true

  # SQLite database
  - id: "orders"
    type: "sqlite"
    path: "./data/orders.db"

  # Remote plugin
  - id: "postgres"
    type: "plugin"
    plugin_url: "http://localhost:8080"
```

### Fixtures

```yaml
fixtures:
  directory: "./fixtures"

  sets:
    - name: "happy-path"
      files:
        - "users.json"
        - "orders.json"

    - name: "error-states"
      files:
        - "error-responses.json"
```

### Plugins

```yaml
plugins:
  wasm:
    - id: "auth-jwt"
      path: "./plugins/auth-jwt.wasm"
      type: "auth"
      config:
        jwks_url: "https://auth.example.com/.well-known/jwks.json"

  remote:
    - id: "fake-data"
      url: "http://localhost:8080"
      type: "response"
      timeout_ms: 5000
      retries: 3
```

### Chaos Engineering

```yaml
chaos:
  enabled: false

  latency:
    enabled: true
    min_ms: 50
    max_ms: 200
    distribution: "normal"        # uniform, normal, or pareto

  errors:
    enabled: true
    rate: 0.05                    # 5% error rate
    codes:
      - 500
      - 502
      - 503

  packet_loss:
    enabled: false
    rate: 0.01

  bandwidth:
    enabled: false
    max_bytes_per_sec: 1048576    # 1 MB/s
```

### Observability

```yaml
metrics:
  enabled: true
  path: "/__mockforge/metrics"
  format: "prometheus"

tracing:
  enabled: false
  exporter: "jaeger"              # jaeger, zipkin, or otlp
  endpoint: "http://localhost:14268/api/traces"
  sample_rate: 1.0

logging:
  level: "info"                   # trace, debug, info, warn, error
  format: "json"                  # text or json
  output: "stdout"                # stdout, stderr, or file path
```

### Performance

```yaml
performance:
  compression:
    enabled: true
    algorithms:
      - gzip
      - deflate
      - brotli
    min_size: 1024

  connection_pool:
    max_connections: 1000
    idle_timeout_secs: 60

  workers:
    count: 0                      # 0 = auto (num CPUs)

  circuit_breaker:
    enabled: false
    failure_threshold: 5
    reset_timeout_secs: 30
```

### Hot Reload

```yaml
hot_reload:
  enabled: true
  check_interval_secs: 5
  debounce_delay_ms: 100

  watch_paths:
    - "./mockforge.yaml"
    - "./endpoints/"
    - "./fixtures/"

  graceful_reload: true
  validate_before_reload: true
  rollback_on_failure: true
```

### Secrets

```yaml
secrets:
  provider: "env"                 # env, vault, aws, azure, gcp

  vault:
    address: "https://vault.example.com"
    auth_method: "token"          # token, kubernetes, or approle
    token: "${VAULT_TOKEN}"
    secret_path: "secret/data/mockforge"

  mappings:
    JWT_SECRET: "auth/jwt-secret"
    API_KEY: "api/key"
```

## Template Syntax

MockForge uses Handlebars-style templates for dynamic values.

### Built-in Helpers

```yaml
response:
  body:
    # UUID generation
    id: "{{uuid}}"

    # Timestamps
    created_at: "{{now}}"
    created_iso: "{{now 'iso'}}"
    created_unix: "{{now 'unix'}}"

    # Random values
    random_int: "{{random_int 1 100}}"
    random_float: "{{random_float 0 1}}"
    random_bool: "{{random_bool}}"
    random_string: "{{random_string 10}}"

    # Fake data
    name: "{{fake 'name'}}"
    email: "{{fake 'email'}}"
    phone: "{{fake 'phone'}}"
    address: "{{fake 'address'}}"

    # Request access
    method: "{{request.method}}"
    path: "{{request.path}}"
    header_auth: "{{headers.Authorization}}"
    query_page: "{{queryParams.page}}"
    body_name: "{{body.name}}"
    path_id: "{{pathParams.id}}"

    # Conditionals
    message: "{{#if body.premium}}Premium user{{else}}Standard user{{/if}}"

    # Loops
    items: |
      {{#each body.items}}
      - {{this.name}}: {{this.price}}
      {{/each}}

    # Math
    total: "{{math body.price '*' body.quantity}}"
    discounted: "{{math body.price '*' 0.9}}"

    # String operations
    upper: "{{upper body.name}}"
    lower: "{{lower body.name}}"
    trim: "{{trim body.input}}"

    # JSON operations
    json_path: "{{jsonPath body '$.items[0].name'}}"
    json_stringify: "{{json body.data}}"

    # Environment
    env_value: "{{env 'API_KEY'}}"

    # Base64
    encoded: "{{base64_encode body.data}}"
    decoded: "{{base64_decode body.encoded}}"

    # Hashing
    hash: "{{sha256 body.password}}"
```

## Environment Variable Override

All configuration values can be overridden via environment variables:

```bash
# Pattern: MOCKFORGE_<SECTION>_<KEY>
MOCKFORGE_HTTP_PORT=8080
MOCKFORGE_HTTP_HOST=0.0.0.0
MOCKFORGE_ADMIN_ENABLED=true
MOCKFORGE_CHAOS_ENABLED=true
```

See [ENVIRONMENT_VARIABLES.md](./ENVIRONMENT_VARIABLES.md) for complete reference.

## Configuration Inheritance

Include other configuration files:

```yaml
# mockforge.yaml
includes:
  - "./endpoints/users.yaml"
  - "./endpoints/orders.yaml"
  - "./fixtures/defaults.yaml"
```

## Validation

Validate configuration:

```bash
# Check syntax
mockforge config validate

# Show resolved configuration
mockforge config show

# List all environment variables
mockforge config list-env-vars
```

## See Also

- [Environment Variables Reference](./ENVIRONMENT_VARIABLES.md)
- [Protocol Guides](./protocols/README.md)
- [Plugin Development](./plugins/development-guide.md)
