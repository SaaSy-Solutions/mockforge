# Getting Started with MockForge

**Welcome to MockForge!** This guide will get you up and running in minutes. MockForge is a powerful, multi-protocol mocking framework that helps frontend and backend teams work in parallel by providing realistic API mocks.

## Table of Contents

- [What is MockForge?](#what-is-mockforge)
- [Installation](#installation)
- [Quick Start: Your First Mock API](#quick-start-your-first-mock-api)
- [Basic Configuration](#basic-configuration)
- [Next Steps](#next-steps)

## What is MockForge?

MockForge is a comprehensive mocking framework that supports multiple protocols:

- **HTTP/REST APIs** - Mock REST endpoints from OpenAPI/Swagger specs
- **WebSocket** - Simulate real-time connections with replay and interactive modes
- **gRPC** - Mock gRPC services from `.proto` files
- **GraphQL** - Generate mock resolvers from GraphQL schemas

### Why MockForge?

- 🚀 **Fast Setup**: Go from OpenAPI spec to running mock server in seconds
- 🔄 **Multi-Protocol**: Mock HTTP, WebSocket, gRPC, and GraphQL in one tool
- 🎯 **Realistic Data**: Generate intelligent mock data with faker functions and templates
- 🔌 **Extensible**: Plugin system for custom authentication, templates, and data sources
- 📊 **Admin UI**: Visual interface for monitoring and managing mock servers

## Installation

### Prerequisites

MockForge requires one of:
- Rust toolchain (for `cargo install`)
- Docker (for containerized deployment)

### Method 1: Cargo Install (Recommended)

```bash
cargo install mockforge-cli
```

Verify installation:
```bash
mockforge --version
```

### Method 2: Docker

```bash
# Build the Docker image
git clone https://github.com/SaaSy-Solutions/mockforge.git
cd mockforge
docker build -t mockforge .

# Run with default ports
docker run -p 3000:3000 -p 3001:3001 -p 9080:9080 mockforge
```

### Method 3: Build from Source

```bash
git clone https://github.com/SaaSy-Solutions/mockforge.git
cd mockforge
cargo build --release

# Install globally
cargo install --path crates/mockforge-cli
```

**See [Installation Guide](installation.md) for detailed instructions and troubleshooting.**

## Quick Start: Your First Mock API

Let's create a simple mock API in 3 steps:

### Step 1: Create an OpenAPI Specification

Create a file `my-api.yaml`:

```yaml
openapi: 3.0.3
info:
  title: My First API
  version: 1.0.0
paths:
  /users:
    get:
      summary: List users
      responses:
        '200':
          description: Success
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/User'
    post:
      summary: Create user
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/User'
      responses:
        '201':
          description: Created
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/User'
  /users/{id}:
    get:
      summary: Get user by ID
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          description: Success
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/User'
components:
  schemas:
    User:
      type: object
      required:
        - id
        - name
        - email
      properties:
        id:
          type: string
          example: "{{uuid}}"
        name:
          type: string
          example: "John Doe"
        email:
          type: string
          format: email
          example: "john@example.com"
        createdAt:
          type: string
          format: date-time
          example: "{{now}}"
```

### Step 2: Start MockForge with Your Spec

```bash
mockforge serve --spec my-api.yaml --http-port 3000
```

You should see:
```
🚀 MockForge v1.0.0 starting...
📡 HTTP server listening on 0.0.0.0:3000
📋 OpenAPI spec loaded from my-api.yaml
✅ Ready to serve requests at http://localhost:3000
```

### Step 3: Test Your Mock API

Open a new terminal and test your endpoints:

```bash
# List users
curl http://localhost:3000/users

# Get a specific user
curl http://localhost:3000/users/123

# Create a user
curl -X POST http://localhost:3000/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Jane Smith", "email": "jane@example.com"}'
```

**Congratulations!** You have a working mock API! 🎉

### Enable Dynamic Data (Optional)

To get unique data on each request, enable template expansion:

```bash
# Stop the server (Ctrl+C), then restart with templates enabled
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true \
  mockforge serve --spec my-api.yaml --http-port 3000
```

Now `{{uuid}}` and `{{now}}` in your spec will generate unique values!

## Basic Configuration

### Using a Configuration File

Create `mockforge.yaml` for better control:

```yaml
http:
  port: 3000
  openapi_spec: my-api.yaml
  response_template_expand: true
  cors:
    enabled: true
    allowed_origins: ["http://localhost:3000"]

admin:
  enabled: true
  port: 9080

logging:
  level: info
```

Start with the config file:
```bash
mockforge serve --config mockforge.yaml
```

### Environment Variables

All settings can be set via environment variables:

```bash
export MOCKFORGE_HTTP_PORT=3000
export MOCKFORGE_ADMIN_ENABLED=true
export MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true
export MOCKFORGE_LOG_LEVEL=debug

mockforge serve --spec my-api.yaml
```

**See [Configuration Reference](../configuration/files.md) for all options.**

## Common Use Cases

### Frontend Development

Start a mock server and point your frontend to it:

```bash
# Terminal 1: Start mock server
mockforge serve --spec api.json --http-port 3000 --admin

# Terminal 2: Point frontend to mock server
export REACT_APP_API_URL=http://localhost:3000
npm start
```

### API Contract Testing

Test that your API matches the OpenAPI specification:

```bash
mockforge serve --spec api.json \
  --validation enforce \
  --http-port 3000
```

### Team Collaboration

Share mock configurations via Git:

```bash
# Commit your mock config
git add mockforge.yaml
git commit -m "Add user API mocks"

# Team members can use the same mocks
git pull
mockforge serve --config mockforge.yaml
```

## Next Steps

Now that you have MockForge running, explore these resources:

### Tutorials

- [5-Minute API Tutorial](five-minute-api.md) - Build a complete mock API quickly
- [Mock from OpenAPI Spec](../tutorials/mock-openapi-spec.md) - Detailed OpenAPI workflow
- [React + MockForge Workflow](../tutorials/react-workflow.md) - Use MockForge with React apps
- [Vue + MockForge Workflow](../tutorials/vue-workflow.md) - Use MockForge with Vue apps

### User Guides

- [HTTP Mocking](../user-guide/http-mocking.md) - REST API mocking features
- [WebSocket Mocking](../user-guide/websocket-mocking.md) - Real-time connection mocking
- [gRPC Mocking](../user-guide/grpc-mocking.md) - gRPC service mocking
- [Plugin System](../user-guide/plugins.md) - Extend MockForge with plugins

### Reference

- [Configuration Guide](../configuration/files.md) - Complete configuration options
- [FAQ](../reference/faq.md) - Common questions and answers
- [Troubleshooting](../reference/troubleshooting.md) - Solve common issues

### Examples

- [React Demo](../../examples/react-demo/) - Complete React application
- [Vue Demo](../../examples/vue-demo/) - Complete Vue 3 application
- [Example Projects](../../examples/README.md) - All available examples

## Troubleshooting

### Server Won't Start

```bash
# Check if port is in use
lsof -i :3000

# Use a different port
mockforge serve --spec my-api.yaml --http-port 3001
```

### Templates Not Working

Enable template expansion:
```bash
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true mockforge serve --spec my-api.yaml
```

### Need More Help?

- Check the [FAQ](../reference/faq.md)
- Review [Troubleshooting Guide](../reference/troubleshooting.md)
- [Open a GitHub Issue](https://github.com/SaaSy-Solutions/mockforge/issues)

---

**Ready to dive deeper?** Continue to the [5-Minute Tutorial](five-minute-api.md) or explore [all available examples](../../examples/README.md).

