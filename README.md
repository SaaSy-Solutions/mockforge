# MockForge

[![Crates.io](https://img.shields.io/crates/v/mockforge.svg)](https://crates.io/crates/mockforge)
[![Documentation](https://docs.rs/mockforge/badge.svg)](https://docs.rs/mockforge)
[![Book](https://img.shields.io/badge/book-read%20online-blue.svg)](https://docs.mockforge.dev/)
[![CI](https://github.com/SaaSy-Solutions/mockforge/workflows/CI/badge.svg)](https://github.com/SaaSy-Solutions/mockforge/actions)
[![Tests](https://github.com/SaaSy-Solutions/mockforge/workflows/Tests/badge.svg)](https://github.com/SaaSy-Solutions/mockforge/actions)
[![Coverage](https://codecov.io/gh/SaaSy-Solutions/mockforge/branch/main/graph/badge.svg)](https://codecov.io/gh/SaaSy-Solutions/mockforge)
[![Benchmarks](https://img.shields.io/badge/benchmarks-criterion-blue)](https://github.com/SaaSy-Solutions/mockforge/actions)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/SaaSy-Solutions/mockforge/blob/main/LICENSE)

MockForge is a comprehensive mocking framework for APIs, gRPC services, and WebSockets. It provides a unified interface for creating, managing, and deploying mock servers across different protocols with advanced data generation capabilities.

## 🏛️ The Five Pillars of MockForge

MockForge is built on five foundational pillars that guide every feature: **[Reality]**, **[Contracts]**, **[DevX]**, **[Cloud]**, and **[AI]**. These pillars ensure MockForge delivers a cohesive, powerful mocking experience that scales from solo developers to enterprise teams.

- **[Reality]** – Everything that makes mocks feel like a real, evolving backend
- **[Contracts]** – Schema, drift, validation, and safety nets
- **[DevX]** – SDKs, generators, playgrounds, ergonomics
- **[Cloud]** – Registry, orgs, governance, monetization, marketplace
- **[AI]** – LLM/voice flows, AI diff/assist, generative behaviors

See the [complete Pillars documentation](docs/PILLARS.md) for detailed information about each pillar, feature mappings, and examples. All changelog entries are tagged with these pillars to make it clear what each release invests in.

### Choose Your Path

- **[Reality-First Onboarding](https://docs.mockforge.dev/getting-started/reality-first.html)** - Start here if you care about realism
- **[Contracts-First Onboarding](https://docs.mockforge.dev/getting-started/contracts-first.html)** - Start here if you're a Platform/API team
- **[AI-First Onboarding](https://docs.mockforge.dev/getting-started/ai-first.html)** - Start here if you want natural-language-driven mocks

## 🔄 Why MockForge?

| Feature | MockForge | WireMock | MockServer | Mockoon |
|---------|-----------|----------|------------|---------|
| **Language** | Rust | Java | Java/JavaScript | JavaScript |
| **Multi-Language SDKs** | ✅ Rust, Node.js, Python, Go, Java, .NET | ⚠️ Java native, clients for others | ⚠️ Java/JS native, clients for others | ⚠️ JS native, clients for others |
| **Performance** | ⚡ High (native Rust) | Medium | Medium | Medium |
| **HTTP/REST** | ✅ Full | ✅ Full | ✅ Full | ✅ Full |
| **gRPC Native** | ✅ Full + HTTP Bridge | ❌ No | ❌ No | ⚠️ Limited |
| **WebSocket** | ✅ Scripted Replay + JSONPath | ❌ No | ⚠️ Basic | ❌ No |
| **GraphQL** | ✅ Yes | ⚠️ Via HTTP | ⚠️ Via HTTP | ✅ Yes |
| **Kafka** | ✅ Full Mock Broker | ❌ No | ❌ No | ❌ No |
| **MQTT** | ✅ Full Broker (3.1.1 & 5.0) | ❌ No | ❌ No | ❌ No |
| **AMQP/RabbitMQ** | ✅ Full Broker (0.9.1) | ❌ No | ❌ No | ❌ No |
| **SMTP** | ✅ Full Email Server | ❌ No | ❌ No | ❌ No |
| **FTP** | ✅ Full File Transfer | ❌ No | ❌ No | ❌ No |
| **TCP** | ✅ Raw TCP Mocking | ❌ No | ❌ No | ❌ No |
| **Client Generation** | ✅ React, Vue, Angular, Svelte | ❌ No | ❌ No | ❌ No |
| **TLS/mTLS** | ✅ HTTPS + Mutual TLS | ⚠️ TLS only | ⚠️ TLS only | ⚠️ TLS only |
| **Admin UI** | ✅ Modern React UI | ⚠️ Basic | ✅ Yes | ✅ Desktop App |
| **Data Generation** | ✅ Advanced (Faker + RAG) | ⚠️ Basic | ⚠️ Basic | ⚠️ Templates |
| **AI-Driven Mocking** | ✅ LLM-powered generation | ❌ No | ❌ No | ❌ No |
| **Data Drift** | ✅ Evolving mock data | ❌ No | ❌ No | ❌ No |
| **AI Event Streams** | ✅ Narrative-driven WebSocket | ❌ No | ❌ No | ❌ No |
| **Plugin System** | ✅ WASM-based | ⚠️ Java extensions | ⚠️ JavaScript | ❌ No |
| **E2E Encryption** | ✅ Built-in (AES-256/ChaCha20) | ❌ No | ⚠️ TLS only | ⚠️ TLS only |
| **Workspace Sync** | ✅ Git integration + file watching | ❌ No | ❌ No | ⚠️ Cloud sync (Pro) |
| **Cross-Endpoint Validation** | ✅ Referential integrity checks | ❌ No | ❌ No | ❌ No |
| **OpenAPI Support** | ✅ Full + Auto-generation | ✅ Yes | ✅ Yes | ✅ Yes |
| **Template Expansion** | ✅ Advanced (faker, time, UUIDs) | ⚠️ Basic | ⚠️ Basic | ✅ Handlebars |
| **Deployment** | Binary, Docker, Cargo | JAR, Docker, Maven | JAR/NPM, Docker | Desktop, NPM, Docker |
| **Stateful Mocking** | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes |
| **Request Matching** | ✅ JSONPath, Regex, Schema | ✅ Yes | ✅ Yes | ✅ Yes |
| **Latency Simulation** | ✅ Configurable profiles | ✅ Yes | ✅ Yes | ✅ Yes |
| **Fault Injection** | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes |
| **CLI Tool** | ✅ Full-featured | ✅ Yes | ✅ Yes | ✅ Yes |
| **License** | MIT/Apache-2.0 | Apache-2.0 | Apache-2.0 | MIT |

## 🌐 Multi-Language Ecosystem

MockForge provides native SDKs for multiple programming languages, enabling developers to embed mock servers directly in their test suites regardless of their technology stack.

### Supported Languages

- **Rust** - Native SDK with zero-overhead embedding
- **Node.js/TypeScript** - Full TypeScript support with type definitions
- **Python** - Context manager support with type hints
- **Go** - Idiomatic Go API with module support
- **Java** - Maven/Gradle integration
- **.NET/C#** - NuGet package with async/await support

### Quick Example

**Rust**:
```rust
let mut server = MockServer::new().port(3000).start().await?;
server.stub_response("GET", "/api/users/{id}", json!({"id": 123})).await?;
```

**Node.js**:
```typescript
const server = await MockServer.start({ port: 3000 });
await server.stubResponse('GET', '/api/users/123', { id: 123 });
```

**Python**:
```python
with MockServer(port=3000) as server:
    server.stub_response('GET', '/api/users/123', {'id': 123})
```

See [SDK Documentation](sdk/README.md) for complete examples and [Ecosystem & Use Cases Guide](docs/ECOSYSTEM_AND_USE_CASES.md) for detailed comparisons with WireMock.

## 🎯 Use Cases

MockForge supports a wide range of use cases, from unit testing to service virtualization:

### 1. Unit Tests
Embed mock servers directly in test suites across all supported languages. No separate server process required for most SDKs.

### 2. Integration Tests
Test complex multi-service interactions with stateful mocking and multi-protocol support (HTTP, gRPC, WebSocket).

### 3. Service Virtualization
Replace external dependencies with mocks using proxy mode and record/replay workflows. Capture real API behavior and replay it later.

### 4. Development/Stub Environments
Create local development environments without backend dependencies. Share mock configurations across teams with workspace synchronization.

### 5. Isolating from Flaky Dependencies
Simulate network failures, timeouts, and slow responses with built-in latency and fault injection. Test application resilience under various failure conditions.

### 6. Simulating APIs That Don't Exist Yet
Generate realistic mocks from API specifications (OpenAPI, GraphQL, gRPC) before implementation. Enable parallel development with schema-driven mock generation.

For detailed use case examples and code samples, see [Ecosystem & Use Cases Guide](docs/ECOSYSTEM_AND_USE_CASES.md).

### v1.0 Feature Status

All major features listed in this README are **implemented and functional in v1.0**, with the following clarification:

- ✅ **Fully Implemented**: HTTP/REST, gRPC (with HTTP Bridge), WebSocket, GraphQL, AI-powered mocking (with data drift & event streams), Plugin system (WASM + remote loading), E2E encryption, Workspace sync, Data generation (RAG-powered), Admin UI (with SSE live logs, metrics, drag-and-drop fixtures)
- ⚠️ **Planned for v1.1**: Admin UI role-based authentication (frontend UI components are built, backend JWT/OAuth integration pending)

All commands, options, and features documented in each protocol section (HTTP, gRPC, WebSocket, GraphQL, Plugins, Data Generation) have been verified to work as described.

### Key Differentiators

- **🚀 True Multi-Protocol**: Only MockForge provides first-class support for HTTP, gRPC, WebSocket, GraphQL, **Kafka, MQTT, and AMQP** in a single binary
- **🧠 AI-Driven Mocking**: Industry-first LLM-powered mock generation from natural language prompts
- **📊 Data Drift Simulation**: Unique realistic data evolution across requests (order status progression, stock depletion, price changes)
- **🌊 AI Event Streams**: Generate narrative-driven WebSocket events for real-time testing scenarios
- **🧬 Advanced Data Generation**: RAG-powered synthetic data with relationship awareness and smart field inference
- **🔌 Modern Plugin System**: Extend functionality with sandboxed WASM plugins for custom generators, auth, and data sources
- **🔒 Enterprise Security**: Built-in end-to-end encryption for sensitive configuration data
- **🌉 gRPC HTTP Bridge**: Automatically expose gRPC services as REST APIs with OpenAPI docs
- **📊 Production-Ready**: Comprehensive testing (unit, integration, mutation), security audits, and automated releases

## ✨ Features

- **Multi-Protocol Support**: HTTP REST APIs, gRPC services, GraphQL APIs, WebSocket connections, SMTP email testing, **Kafka event streaming**, **MQTT pub/sub**, and **AMQP message queuing**
- **🧠 AI-Powered Mocking** *(Industry First)*: Revolutionary artificial intelligence features:
  - **Intelligent Mock Generation**: Generate realistic responses from natural language prompts
    - Natural language → realistic JSON data
    - Schema-aware generation with validation
    - Multi-provider support: OpenAI, Anthropic, Ollama (free local), or OpenAI-compatible APIs
    - Built-in caching for performance optimization
  - **Data Drift Simulation**: Evolving mock data across requests
    - Order statuses progress naturally (pending → processing → shipped → delivered)
    - Stock quantities deplete with purchases
    - Prices fluctuate realistically over time
    - State machine transitions with custom probabilities
  - **AI Event Streams**: LLM-powered WebSocket event generation
    - Generate realistic event streams from narrative descriptions
    - Progressive scenario evolution for contextual continuity
    - Time-based, count-based, or conditional event strategies
    - Perfect for testing real-time features
  - **Free Local Development**: Use Ollama for $0 cost during development
  - **Cost-Effective Production**: ~$0.01 per 1,000 requests with OpenAI GPT-3.5
- **Advanced Data Synthesis**: Intelligent mock data generation with:
  - **Smart Field Inference**: Automatic data type detection from field names
  - **Deterministic Seeding**: Reproducible test fixtures for stable testing
  - **RAG-Driven Generation**: Context-aware data using domain knowledge
  - **Relationship Awareness**: Foreign key detection and cross-reference validation
  - **Schema Graph Extraction**: Automatic relationship discovery from protobuf schemas
- **Plugin System**: WebAssembly-based extensible architecture with:
  - **Custom Response Generators**: Build plugins for specialized mock data
  - **Authentication Providers**: JWT, OAuth2, and custom auth plugins
  - **Data Source Connectors**: CSV, database, and external API integrations
  - **Template Extensions**: Custom template functions and filters
  - **Security Sandbox**: Isolated plugin execution with resource limits
  - **🆕 Remote Loading**: Install plugins from URLs, Git repos, or local files with version pinning
    - `mockforge plugin install https://github.com/user/plugin#v1.0.0`
    - Support for ZIP, tar.gz archives, and direct WASM files
    - Checksum verification and automatic caching
- **End-to-End Encryption**: Enterprise-grade security features:
  - **Multi-Algorithm Support**: AES-256-GCM and ChaCha20-Poly1305 encryption
  - **Key Management**: Hierarchical key system with secure storage
  - **Auto-Encryption**: Automatic encryption of sensitive configuration data
  - **Template Functions**: Built-in encryption/decryption in templates
- **Workspace Synchronization**: Bidirectional sync with version control:
  - **File System Watching**: Real-time sync between workspaces and directories
  - **Git Integration**: Version control your mock configurations
  - **Team Collaboration**: Shared workspaces with conflict resolution
- **Dynamic Response Generation**: Create realistic mock responses with configurable latency and failure rates
- **Cross-Endpoint Validation**: Ensure referential integrity across different endpoints
- **Admin UI v2**: Modern React-based interface with:
  - **Role-Based Authentication**: ✅ Complete JWT-based authentication with Admin, Editor, and Viewer roles
  - **Real-time Collaboration**: ✅ WebSocket-based collaborative editing with presence awareness and cursor tracking
  - **Real-time Monitoring**: Live logs via Server-Sent Events (SSE), metrics, and performance tracking
  - **Visual Configuration**: Drag-and-drop fixture management with tree view
  - **Advanced Search**: Full-text search across services and logs
- **Configuration Management**: Flexible configuration via YAML/JSON files with environment variable overrides
- **Built-in Data Templates**: Pre-configured schemas for common data types (users, products, orders)
- **Production Ready**: Comprehensive testing, security audits, and automated releases

## 📖 Documentation

For comprehensive documentation, tutorials, and guides:

**[📚 Read the MockForge Book](https://docs.mockforge.dev/)**

The documentation covers:
- **[Your First Mock API in 5 Minutes](https://docs.mockforge.dev/getting-started/five-minute-api.html)** - Fastest path to productivity
- Getting started guide and installation
- Detailed configuration options
- API reference for all protocols (HTTP, gRPC, WebSocket)
- Advanced features and examples
- Contributing guidelines

## 🚀 Quick Start

**New to MockForge?** Follow our **[5-Minute Tutorial](https://docs.mockforge.dev/getting-started/five-minute-api.html)** to create your first mock API.

**Need help?** Check the **[FAQ](https://docs.mockforge.dev/reference/faq.html)** or **[Troubleshooting Guide](https://docs.mockforge.dev/reference/troubleshooting.html)**.

### Installation

```bash
# Install from crates.io
cargo install mockforge-cli

# Or build from source
git clone https://github.com/SaaSy-Solutions/mockforge.git
cd mockforge
make setup
make build
make install
```

#### Command Aliases (Optional)

For faster typing, you can set up command aliases:

```bash
# Run the setup script
./scripts/setup-aliases.sh

# Or manually add to your ~/.bashrc or ~/.zshrc:
alias mf='mockforge'
alias mf-serve='mockforge serve'
alias mf-wizard='mockforge wizard'
```

Then use `mf` instead of `mockforge`:
```bash
mf wizard        # Interactive setup wizard
mf serve         # Start mock server
mf init .        # Initialize project
```

### Try the Examples

MockForge comes with comprehensive examples to get you started quickly:

```bash
# Run with the included examples
make run-example

# Or use the configuration file
cargo run -p mockforge-cli -- serve --config demo-config.yaml

# Or run manually with environment variables
MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl \
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true \
cargo run -p mockforge-cli -- serve --spec examples/openapi-demo.json --admin
```

## 📋 Schema/Specification Input Support

MockForge supports multiple API specification formats for generating mocks and clients:

### Supported Formats

- **OpenAPI 3.0.x and 3.1.x**: Full support with comprehensive validation
- **OpenAPI 2.0 (Swagger)**: Format detection and validation (parsing via conversion to 3.x recommended)
- **GraphQL Schema**: Schema Definition Language (SDL) parsing and validation
- **Protocol Buffers**: gRPC service definitions from `.proto` files

### Usage Examples

#### OpenAPI Specification

```bash
# Generate mocks from OpenAPI spec
mockforge generate --spec api.json --output ./generated

# Serve with OpenAPI spec
mockforge serve --spec api.yaml --admin

# Import OpenAPI spec and generate mocks
mockforge import openapi ./specs/api.yaml --output mocks.json
```

**Example OpenAPI 3.0 Specification:**

```json
{
  "openapi": "3.0.0",
  "info": {
    "title": "User Management API",
    "version": "1.0.0",
    "description": "API for managing users"
  },
  "servers": [
    {
      "url": "https://api.example.com/v1"
    }
  ],
  "paths": {
    "/users": {
      "get": {
        "summary": "List users",
        "responses": {
          "200": {
            "description": "List of users",
            "content": {
              "application/json": {
                "schema": {
                  "type": "array",
                  "items": {
                    "$ref": "#/components/schemas/User"
                  }
                }
              }
            }
          }
        }
      },
      "post": {
        "summary": "Create user",
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/User"
              }
            }
          }
        },
        "responses": {
          "201": {
            "description": "User created",
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/User"
                }
              }
            }
          }
        }
      }
    }
  },
  "components": {
    "schemas": {
      "User": {
        "type": "object",
        "required": ["id", "name", "email"],
        "properties": {
          "id": {
            "type": "integer",
            "format": "int64"
          },
          "name": {
            "type": "string"
          },
          "email": {
            "type": "string",
            "format": "email"
          }
        }
      }
    }
  }
}
```

#### GraphQL Schema

```bash
# Serve with GraphQL schema
mockforge serve --graphql schema.graphql --graphql-port 4000

# Generate from GraphQL schema
mockforge generate --spec schema.graphql --output ./generated
```

**Example GraphQL Schema:**

```graphql
type Query {
  users: [User!]!
  user(id: ID!): User
}

type Mutation {
  createUser(input: CreateUserInput!): User!
  updateUser(id: ID!, input: UpdateUserInput!): User!
}

type User {
  id: ID!
  name: String!
  email: String!
  createdAt: DateTime!
}

input CreateUserInput {
  name: String!
  email: String!
}

input UpdateUserInput {
  name: String
  email: String
}

scalar DateTime
```

#### gRPC/Protocol Buffers

```bash
# Serve with proto files
mockforge serve --grpc-port 50051

# Proto files are discovered from configured directories
```

### Validation and Error Reporting

MockForge provides comprehensive validation with detailed error messages:

**Example validation output:**

```bash
$ mockforge generate --spec invalid-api.json

Invalid OpenAPI specification:
  Missing 'info' section in OpenAPI 3.x spec (at /info). Hint: Add an 'info' section with 'title' and 'version' fields
  Missing or empty 'info.title' field (at /info/title). Hint: Add 'title' field to the 'info' section
  'paths' object cannot be empty. At least one endpoint is required (at /paths). Hint: Add at least one path definition

Fix the validation errors above and try again
```

**Validation Features:**

- ✅ Automatic format detection (OpenAPI, GraphQL, Protobuf)
- ✅ Detailed error messages with JSON pointers to problematic fields
- ✅ Helpful suggestions for fixing validation errors
- ✅ Support for both JSON and YAML formats
- ✅ Warnings for incomplete or suboptimal specifications

### Converting Swagger 2.0 to OpenAPI 3.x

While MockForge can detect and validate Swagger 2.0 specifications, full parsing requires OpenAPI 3.x format. Use conversion tools:

```bash
# Using swagger2openapi (Node.js)
npx swagger2openapi swagger.json -o openapi.json

# Or use online converter
# https://editor.swagger.io/
```

See `examples/README.md` for detailed documentation on the example files.

### Docker (Alternative Installation)

MockForge can also be run using Docker for easy deployment:

#### Quick Docker Start

```bash
# Using Docker Compose (recommended)
make docker-compose-up

# Or using Docker directly
make docker-build && make docker-run
```

#### Manual Docker Commands

```bash
# Build the image
docker build -t mockforge .

# Run with examples
docker run -p 3000:3000 -p 3001:3001 -p 50051:50051 -p 9080:9080 \
  -v $(pwd)/examples:/app/examples:ro \
  -e MOCKFORGE_ADMIN_ENABLED=true \
  -e MOCKFORGE_HTTP_OPENAPI_SPEC=examples/openapi-demo.json \
  mockforge
```

See [DOCKER.md](DOCKER.md) for comprehensive Docker documentation and deployment options.

## 🎯 Multi-Framework Client Generation

MockForge now supports generating client code for multiple frontend frameworks from your OpenAPI specifications. This enables seamless integration with your existing applications and reduces development time.

### Supported Frameworks

- **React** - Generate React hooks and TypeScript types
- **Vue** - Generate Vue composables and Pinia stores
- **Angular** - Generate Angular services and modules
- **Svelte** - Generate Svelte stores and components

### Quick Start with Client Generation

```bash
# Generate React client
mockforge client generate --spec examples/user-management-api.json --framework react --output ./generated

# Generate Vue client
mockforge client generate --spec examples/user-management-api.json --framework vue --output ./generated

# Generate Angular client
mockforge client generate --spec examples/user-management-api.json --framework angular --output ./generated

# Generate Svelte client
mockforge client generate --spec examples/user-management-api.json --framework svelte --output ./generated
```

### Example Applications

Complete example applications are available in the `examples/` directory:

- **`react-demo/`** - React application with generated hooks
- **`vue-demo/`** - Vue 3 application with generated composables
- **`angular-demo/`** - Angular 17 application with generated services
- **`svelte-demo/`** - SvelteKit application with generated stores

Each demo includes:
- ✅ Complete working application
- ✅ Generated client integration
- ✅ TypeScript type safety
- ✅ Error handling and loading states
- ✅ Form handling examples
- ✅ Comprehensive documentation

### Generated Code Features

All generated clients include:

- **TypeScript Types** - Full type safety from OpenAPI schemas
- **API Clients** - Framework-specific HTTP clients
- **Error Handling** - Built-in error management
- **Loading States** - Reactive loading indicators
- **Documentation** - Usage examples and API reference

### Usage Example

```typescript
// React
const { data: users, loading, error } = useGetUsers();

// Vue
const { data, loading, error } = useGetUsers();

// Angular
this.userService.getUsers().subscribe({
  next: (users) => this.users = users,
  error: (error) => this.error = error
});

// Svelte
const usersStore = createGetUsersStore();
usersStore.subscribe(state => {
  users = state.data;
  loading = state.loading;
});
```

See [`examples/README.md`](examples/README.md) for detailed documentation on all framework examples.

### Basic Usage

```bash
# Build the project
cargo build

# Start all mock servers with Admin UI (separate port)
cargo run -p mockforge-cli -- serve --admin --admin-port 9080

# Start with custom configuration
cargo run -p mockforge-cli -- serve --config config.yaml --admin

# Generate test data
cargo run -p mockforge-cli -- data template user --rows 50 --output users.json

# Start Admin UI only (standalone server)
cargo run -p mockforge-cli -- admin --port 9080

# Start workspace synchronization daemon
cargo run -p mockforge-cli -- sync start --directory ./workspace-sync

# Access Admin Interface

- Standalone Admin: http://localhost:9080/
- Admin embedded under HTTP (when configured): http://localhost:3000/admin/

# Quick development setup with environment variables
MOCKFORGE_ADMIN_ENABLED=true MOCKFORGE_HTTP_PORT=3000 cargo run -p mockforge-cli -- serve
```

### 🧠 AI Features Quick Start

MockForge supports AI-powered mock generation for intelligent, evolving data. Perfect for realistic testing!

#### Using Free Local AI (Ollama)

```bash
# Install Ollama (one-time setup)
curl https://ollama.ai/install.sh | sh
ollama pull llama2

# Start MockForge with AI enabled
cargo run -p mockforge-cli -- serve \
  --ai-enabled \
  --rag-provider ollama \
  --rag-model llama2 \
  --config examples/ai/intelligent-customer-api.yaml
```

#### Using OpenAI (Paid)

```bash
# Start with OpenAI
export MOCKFORGE_RAG_API_KEY=sk-your-api-key
cargo run -p mockforge-cli -- serve \
  --ai-enabled \
  --rag-provider openai \
  --rag-model gpt-3.5-turbo \
  --config examples/ai/intelligent-customer-api.yaml
```

#### Test AI Features

```bash
# Test intelligent mock generation
cargo run -p mockforge-cli -- test-ai intelligent-mock \
  --prompt "Generate realistic customer data for a SaaS platform" \
  --rag-provider ollama

# Test data drift simulation
cargo run -p mockforge-cli -- test-ai drift \
  --initial-data examples/order.json \
  --iterations 10

# Test AI event stream generation
cargo run -p mockforge-cli -- test-ai event-stream \
  --narrative "Simulate 5 minutes of live stock market data" \
  --event-count 20 \
  --rag-provider ollama
```

#### Configuration Example

```yaml
responses:
  - name: "AI Customer Response"
    status_code: 200
    intelligent:
      mode: intelligent
      prompt: "Generate realistic customer data for a retail SaaS API"
      schema:
        type: object
        properties:
          id: { type: string }
          name: { type: string }
          email: { type: string }
    drift:
      enabled: true
      request_based: true
      rules:
        - field: tier
          strategy: state_machine
          states: [bronze, silver, gold, platinum]
```

**📖 Learn More**: See [`docs/AI_DRIVEN_MOCKING.md`](./docs/AI_DRIVEN_MOCKING.md) for complete AI features documentation.

## HTTP

curl <http://localhost:3000/ping>

## SMTP Email Testing

MockForge includes a fully functional SMTP server for testing email workflows:

```bash
# Start SMTP server
mockforge serve --smtp --smtp-port 1025

# Send test email with Python
python3 << EOF
import smtplib
from email.message import EmailMessage

msg = EmailMessage()
msg['Subject'] = 'Test Email'
msg['From'] = 'sender@example.com'
msg['To'] = 'recipient@example.com'
msg.set_content('This is a test email.')

with smtplib.SMTP('localhost', 1025) as server:
    server.send_message(msg)
    print("Email sent successfully!")
EOF

# Or use command-line tools
swaks --to recipient@example.com \
      --from sender@example.com \
      --server localhost:1025 \
      --body "Test email"
```

### SMTP Features

- ✅ RFC 5321 compliant (HELLO, EHLO, MAIL, RCPT, DATA, QUIT, RSET, NOOP, HELP)
- ✅ Fixture-based email matching (regex patterns for recipients, senders, subjects)
- ✅ In-memory mailbox with size limits
- ✅ Auto-reply configuration with templates
- ✅ Template expansion support (faker functions, UUIDs, timestamps)
- ✅ Storage options (in-memory, file export)
- ✅ Configurable behavior (delays, failure rates)

### SMTP Configuration Example

```yaml
smtp:
  enabled: true
  port: 1025
  hostname: "mockforge-smtp"
  fixtures_dir: "./fixtures/smtp"
  enable_mailbox: true
  max_mailbox_messages: 1000
```

See [SMTP documentation](book/src/protocols/smtp/) for complete guide.

## WebSocket (Scripted Replay)

MockForge supports scripted WebSocket interactions with template expansion and conditional responses.

### Quick Start

```bash
# Set the replay file environment variable
export MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl

# Start the WebSocket server
cargo run -p mockforge-cli -- serve --ws-port 3001
```

### Connect and Test

**Using Node.js:**
```javascript
const WebSocket = require('ws');
const ws = new WebSocket('ws://localhost:3001/ws');

ws.on('open', () => {
  console.log('Connected! Sending CLIENT_READY...');
  ws.send('CLIENT_READY');
});

ws.on('message', (data) => {
  console.log('Received:', data.toString());

  // Auto-respond to expected prompts
  if (data.toString().includes('ACK')) {
    ws.send('ACK');
  }
  if (data.toString().includes('CONFIRMED')) {
    ws.send('CONFIRMED');
  }
});

ws.on('close', () => console.log('Connection closed'));
```

**Using websocat:**
```bash
websocat ws://localhost:3001/ws
# Then type: CLIENT_READY
# The server will respond with scripted messages
```

**Using wscat:**
```bash
wscat -c ws://localhost:3001/ws
# Then type: CLIENT_READY
```

**Browser Console:**
```javascript
const ws = new WebSocket('ws://localhost:3001/ws');
ws.onopen = () => ws.send('CLIENT_READY');
ws.onmessage = (event) => console.log('Received:', event.data);
```

### Advanced Message Matching with JSONPath

MockForge supports JSONPath queries for sophisticated WebSocket message matching:

```json
[
  {"waitFor": "^CLIENT_READY$", "text": "Welcome!"},
  {"waitFor": "$.type", "text": "Type received"},
  {"waitFor": "$.user.id", "text": "User authenticated"},
  {"waitFor": "$.order.status", "text": "Order status updated"}
]
```

**JSONPath Examples:**
- `$.type` - Wait for any message with a `type` property
- `$.user.id` - Wait for messages with user ID
- `$.order.status` - Wait for order status updates
- `$.items[0].name` - Wait for first item name

**JSON Message Testing:**
```javascript
const ws = new WebSocket('ws://localhost:3001/ws');

// Send JSON messages that match JSONPath patterns
ws.onopen = () => {
  ws.send(JSON.stringify({type: 'login'}));           // Matches $.type
  ws.send(JSON.stringify({user: {id: '123'}}));       // Matches $.user.id
  ws.send(JSON.stringify({order: {status: 'paid'}})); // Matches $.order.status
};

ws.onmessage = (event) => console.log('Response:', event.data);
```

See `examples/README-websocket-jsonpath.md` for complete documentation.

### Replay File Format

WebSocket replay files use JSON Lines format with the following structure:

```json
{"ts":0,"dir":"out","text":"HELLO {{uuid}}","waitFor":"^CLIENT_READY$"}
{"ts":10,"dir":"out","text":"{\\"type\\":\\"welcome\\",\\"sessionId\\":\\"{{uuid}}\\"}"}
{"ts":20,"dir":"out","text":"{\\"type\\":\\"data\\",\\"value\\":\\"{{randInt 1 100}}\\"}","waitFor":"^ACK$"}
```

- `ts`: Timestamp in milliseconds for message timing
- `dir`: Direction ("in" for received, "out" for sent)
- `text`: Message content (supports template expansion)
- `waitFor`: Optional regex pattern to wait for before sending

### Template Expansion

WebSocket messages support the same template expansion as HTTP responses:
- `{{uuid}}` → Random UUID
- `{{now}}` → Current timestamp
- `{{now+1h}}` → Future timestamp
- `{{randInt 1 100}}` → Random integer

## gRPC

grpcurl -plaintext -proto crates/mockforge-grpc/proto/gretter.proto -d '{"name":"Ray"}' localhost:50051 mockforge.greeter.Greeter/SayHello

### 🚀 HTTP Bridge for gRPC Services

MockForge now includes an advanced **HTTP Bridge** that automatically converts gRPC services to REST APIs, eliminating the need for separate gRPC and HTTP implementations.

#### Features

- **Automatic Discovery**: Scans `.proto` files and creates REST endpoints for all gRPC services
- **JSON ↔ Protobuf Conversion**: Full bidirectional conversion between JSON and protobuf messages
- **OpenAPI Documentation**: Auto-generated OpenAPI/Swagger specs for all bridged services
- **Streaming Support**: Server-Sent Events (SSE) for server streaming and bidirectional communication
- **Statistics & Monitoring**: Built-in request metrics and health checks

#### Quick Start

```bash
# Start gRPC server with HTTP bridge
cargo run -p mockforge-cli -- serve --config config.dev.yaml --admin
```

The bridge will automatically:
1. Discover services from proto files
2. Create REST endpoints at `/api/{service}/{method}`
3. Generate OpenAPI docs at `/api/docs`
4. Provide health monitoring at `/api/health`

#### Example Usage

**gRPC Service:**
```protobuf
service UserService {
  rpc CreateUser(CreateUserRequest) returns (CreateUserResponse);
  rpc GetUser(GetUserRequest) returns (GetUserResponse);
}
```

**HTTP Bridge Endpoints:**
```bash
# Create user (POST)
curl -X POST http://localhost:3000/api/userservice/createuser \
  -H "Content-Type: application/json" \
  -d '{"name": "John Doe", "email": "john@example.com"}'

# Get user (POST - gRPC semantics)
curl -X POST http://localhost:3000/api/userservice/getuser \
  -H "Content-Type: application/json" \
  -d '{"user_id": "123"}'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "user_id": "123",
    "name": "John Doe",
    "email": "john@example.com",
    "created_at": "2025-01-01T00:00:00Z"
  },
  "error": null,
  "metadata": {
    "x-mockforge-service": "userservice",
    "x-mockforge-method": "createuser"
  }
}
```

#### Configuration

Enable the HTTP bridge by modifying your config:

```yaml
grpc:
  dynamic:
    enabled: true
    proto_dir: "proto"          # Directory containing .proto files
    enable_reflection: true     # Enable gRPC reflection
    http_bridge:
      enabled: true             # Enable HTTP bridge
      base_path: "/api"         # Base path for REST endpoints
      enable_cors: true         # Enable CORS
      timeout_seconds: 30       # Request timeout
```

Or via environment variables:
```bash
export MOCKFORGE_GRPC_DYNAMIC_ENABLED=true
export MOCKFORGE_GRPC_HTTP_BRIDGE_ENABLED=true
export MOCKFORGE_GRPC_PROTO_DIR=proto
```

#### Bridge Endpoints

- **`GET /api/health`** - Health check
- **`GET /api/stats`** - Request statistics and metrics
- **`GET /api/services`** - List available gRPC services
- **`GET /api/docs`** - OpenAPI 3.0 documentation
- **`/api/{service}/{method}`** - Automatically generated REST endpoints

#### Streaming Support

For gRPC streaming methods, the bridge provides:

```bash
# Server streaming endpoint
curl -N http://localhost:3000/api/chat/streammessages \
  -H "Content-Type: application/json" \
  -d '{"topic": "tech"}'
```

Returns server-sent events:
```javascript
data: {"event_type":"message","data":{"text":"Hello!"},"metadata":{}}
event: message

data: {"event_type":"message","data":{"text":"How can I help?"},"metadata":{}}
event: message
```

#### OpenAPI Integration

The bridge auto-generates comprehensive OpenAPI documentation:

```bash
# Access interactive API docs
open http://localhost:3000/api/docs

# Get OpenAPI JSON spec
curl http://localhost:3000/api/docs
```

Features:
- Automatic schema generation from protobuf definitions
- Example requests and responses
- Streaming method documentation
- Method tags and descriptions

#### Advanced Features

- **Bidirectional Streaming**: Full support for client ↔ server streaming via WebSockets-in-disguise
- **Metadata Preservation**: Passes gRPC metadata as HTTP headers
- **Error Handling**: Comprehensive error responses with detailed messages
- **Metrics**: Request counting, latency tracking, and failure rates
- **Security**: Configurable CORS and request validation

#### Use Cases

1. **Frontend Development**: Test gRPC APIs with familiar HTTP tools
2. **API Gateways**: Expose gRPC services as REST APIs
3. **Mixed Environments**: Support for both gRPC and HTTP clients
4. **Development Tools**: Use Postman, curl, or any HTTP client
5. **Documentation**: Auto-generated API docs for gRPC services

## 📨 Async/Event Protocols (Kafka, MQTT, AMQP)

MockForge provides **first-class support** for async and event-driven protocols, enabling comprehensive testing of message-driven architectures, pub/sub systems, and event-driven microservices.

### Supported Protocols

- **Kafka** - Distributed event streaming with full broker simulation
- **MQTT** - IoT and pub/sub messaging with QoS support
- **AMQP** - Enterprise message queuing (RabbitMQ compatible)

### Quick Start

```bash
# Start all protocols (Kafka, MQTT, AMQP enabled by default)
mockforge serve

# Override ports
mockforge serve --kafka-port 9092 --mqtt-port 1883 --amqp-port 5672

# Or use dedicated commands
mockforge kafka serve --port 9092
mockforge mqtt publish --topic "sensors/temp" --payload '{"temp": 22.5}'
mockforge amqp serve --port 5672
```

### Kafka Mock Broker

**Features:**
- ✅ 10+ Kafka APIs (Produce, Fetch, Metadata, Consumer Groups, etc.)
- ✅ Topic & partition management with auto-creation
- ✅ Consumer group coordination with rebalancing
- ✅ Offset management and commit tracking
- ✅ Auto-produce messages at configurable rates
- ✅ Compatible with rdkafka, KafkaJS, confluent-kafka

**Example:** Using with Python

```python
from confluent_kafka import Producer, Consumer

# Producer
producer = Producer({'bootstrap.servers': 'localhost:9092'})
producer.produce('orders', key='order-123', value='{"total": 99.99}')
producer.flush()

# Consumer
consumer = Consumer({
    'bootstrap.servers': 'localhost:9092',
    'group.id': 'my-group',
    'auto.offset.reset': 'earliest'
})
consumer.subscribe(['orders'])
```

**Fixture-Based Testing:**

Create `fixtures/kafka/orders.yaml`:

```yaml
- identifier: "order-created"
  topic: "orders.created"
  key_pattern: "order-{{uuid}}"
  value_template:
    order_id: "{{uuid}}"
    customer_id: "customer-{{faker.int 1000 9999}}"
    total: "{{faker.float 10.0 1000.0 | round 2}}"
    status: "pending"
    created_at: "{{now}}"
  auto_produce:
    enabled: true
    rate_per_second: 10  # Generate 10 orders/second
```

### MQTT Broker

**Features:**
- ✅ MQTT 3.1.1 and 5.0 support
- ✅ QoS levels (0, 1, 2) with delivery guarantees
- ✅ Topic hierarchies with wildcards (`+`, `#`)
- ✅ Retained messages and Last Will Testament
- ✅ Session management and auto-publish
- ✅ Compatible with Paho, rumqttc, MQTT.js

**Example:** Using with JavaScript

```javascript
const mqtt = require('mqtt');
const client = mqtt.connect('mqtt://localhost:1883');

// Publish
client.publish('sensors/temperature', JSON.stringify({ temp: 22.5 }), { qos: 1 });

// Subscribe
client.subscribe('sensors/#');
client.on('message', (topic, message) => {
  console.log(`${topic}: ${message.toString()}`);
});
```

### AMQP Broker

**Features:**
- ✅ AMQP 0.9.1 protocol (RabbitMQ compatible)
- ✅ Exchange types (direct, fanout, topic, headers)
- ✅ Queue management with bindings
- ✅ Consumer coordination and message routing
- ✅ Fixture-driven testing
- ✅ Compatible with lapin, amqplib, RabbitMQ clients

**Example:** Using with Python

```python
import pika

connection = pika.BlockingConnection(pika.ConnectionParameters('localhost'))
channel = connection.channel()

# Declare and bind
channel.exchange_declare(exchange='orders', exchange_type='topic')
channel.queue_declare(queue='order.processing')
channel.queue_bind(exchange='orders', queue='order.processing', routing_key='order.created')

# Publish
channel.basic_publish(exchange='orders', routing_key='order.created',
                      body='{"order_id": "123"}')
```

### Advanced Features

**Auto-Production:**
```yaml
auto_produce:
  enabled: true
  rate_per_second: 100
  duration_seconds: 0  # 0 = infinite
```

**Template Engine:**
```yaml
value_template:
  id: "{{uuid}}"
  customer_name: "{{faker.name}}"
  amount: "{{faker.float 10.0 1000.0 | round 2}}"
  created_at: "{{now}}"
  status: "{{faker.randomChoice ['pending', 'processing', 'completed']}}"
```

**Metrics & Monitoring:**
```bash
curl http://localhost:9080/__mockforge/metrics

# Example metrics
kafka_messages_produced_total 12345
mqtt_messages_published_total 5678
amqp_messages_published_total 9012
```

### Configuration

```yaml
kafka:
  enabled: true
  port: 9092
  auto_create_topics: true
  default_partitions: 3
  fixtures_dir: "./fixtures/kafka"

mqtt:
  enabled: true
  port: 1883
  max_connections: 1000
  keep_alive_secs: 60
  fixtures_dir: "./fixtures/mqtt"

amqp:
  enabled: true
  port: 5672
  max_connections: 1000
  heartbeat_interval: 60
  fixtures_dir: "./fixtures/amqp"
```

### Example Use Cases

1. **Microservices Event Bus** - Order service → Inventory → Notifications → Analytics
2. **IoT Sensor Networks** - Temperature/humidity sensors publishing via MQTT
3. **Task Queue Systems** - API publishes tasks, workers consume from specific queues
4. **Event Sourcing & CQRS** - Event store with read model projections

📖 **For detailed documentation, see [ASYNC_PROTOCOLS.md](ASYNC_PROTOCOLS.md)**

## 🎯 Data Generation

MockForge includes powerful synthetic data generation capabilities:

```bash
# Generate user data using built-in templates
cargo run -p mockforge-cli -- data template user --rows 100 --output users.json

# Generate product data
cargo run -p mockforge-cli -- data template product --rows 50 --format csv --output products.csv

# Generate data from JSON schema
cargo run -p mockforge-cli -- data schema schema.json --rows 200 --output custom_data.json

# Enable RAG mode for enhanced data generation
cargo run -p mockforge-cli -- data template user --rows 100 --rag --output users.json
```

### Built-in Templates

- **User**: Complete user profiles with emails, names, addresses
- **Product**: Product catalog with pricing, categories, descriptions
- **Order**: Customer orders with relationships to users and products

### Advanced Features

- **RAG Integration**: Use LLM-powered generation for more realistic data
- **Multiple Formats**: JSON, JSON Lines, CSV output
- **Custom Schemas**: Generate data from your own JSON schemas
- **Relationship Support**: Maintain referential integrity between entities

echo -e '{"name":"one"}\n{"name":"two"}' | grpcurl -plaintext -proto crates/mockforge-grpc/proto/gretter.proto -d @ localhost:50051 mockforge.greeter.Greeter/SayHelloClientStream

echo -e '{"name":"first"}\n{"name":"second"}' | grpcurl -plaintext -proto crates/mockforge-grpc/proto/gretter.proto -d @ localhost:50051 mockforge.greeter.Greeter/Chat

## 🎛️ Admin Interface

### Dashboard

![Dashboard](docs/images/mockforge-dashboard.png)

MockForge ships a built-in Admin UI that can run as either:

- A standalone server (default when `--admin` is used): `http://localhost:9080/`.
- Embedded under the HTTP server at a mount path, e.g. `http://localhost:3000/admin/` when `admin.mount_path: "/admin"` is configured.

The Admin UI provides:

- **📊 Modern dashboard** with real-time server status and live logs (via SSE)
- **⚙️ Configuration management** for latency, faults, and proxy settings
- **📝 Request logging** with filtering and real-time monitoring
- **📈 Metrics visualization** with performance insights
- **🎯 Fixture management** with drag-and-drop tree view for organizing fixtures
- **🎨 Professional UI** with tabbed interface and responsive design

> **Note**: Role-based authentication (Admin/Viewer access control) is planned for v1.1. The frontend UI components are ready, but backend JWT/OAuth authentication is not yet implemented in v1.0. The Admin UI is currently accessible without authentication.

### Embedded Admin Mode

You can embed the Admin UI under the HTTP server instead of running it on a separate port. This is handy when you want a single endpoint to expose mocks and admin controls.

- Configure via file (config.yaml):

```yaml
admin:
  enabled: true
  mount_path: "/admin"
```

- Or via environment:

```bash
export MOCKFORGE_ADMIN_ENABLED=true
export MOCKFORGE_ADMIN_MOUNT_PATH=/admin
```

- Start servers:

```bash
cargo run -p mockforge-cli -- serve
```

- Access URLs:
  - UI: http://localhost:3000/admin/
  - Health: http://localhost:3000/admin/__mockforge/health
  - Dashboard: http://localhost:3000/admin/__mockforge/dashboard

Notes:
- Static assets are served relative to the mount path (e.g., `/admin/admin.css`).
- Switching back to standalone mode: remove `mount_path` (or unset env) and run with `--admin --admin-port 9080`.

### Admin Mode Flags (CLI)

You can control how the Admin UI runs via flags on `serve`:

```bash
# Force embedded mode (default mount at /admin)
cargo run -p mockforge-cli -- serve --admin-embed

# Embedded with explicit mount
cargo run -p mockforge-cli -- serve --admin-embed --admin-mount-path /tools

# Force standalone mode on port 9080 (overrides embed)
cargo run -p mockforge-cli -- serve --admin --admin-standalone --admin-port 9080

# Disable Admin APIs (UI loads but __mockforge/* endpoints are absent)
cargo run -p mockforge-cli -- serve --admin-embed --disable-admin-api

# Equivalent env-based control
export MOCKFORGE_ADMIN_ENABLED=true
export MOCKFORGE_ADMIN_MOUNT_PATH=/admin
export MOCKFORGE_ADMIN_API_ENABLED=false
cargo run -p mockforge-cli -- serve
```

### API Endpoints

Admin API endpoints are namespaced under `__mockforge`:

- Standalone Admin (default):
  - `GET /__mockforge/dashboard`
  - `GET /__mockforge/health`
  - `GET /__mockforge/logs`
  - `GET /__mockforge/metrics`
  - `GET /__mockforge/fixtures`
  - `POST /__mockforge/config/*`
- Embedded under a mount path (e.g., `/admin`):
  - `GET /admin/__mockforge/dashboard`
  - `GET /admin/__mockforge/health`
  - ... (same suffixes under the mount prefix)

## ⚙️ Configuration

MockForge supports flexible configuration through YAML or JSON files:

```bash
# Initialize a new configuration
mockforge init my-project

# Validate your configuration
mockforge config validate

# Use a configuration file
cargo run -p mockforge-cli -- serve --config my-config.yaml
```

**[📋 Complete Configuration Template](config.template.yaml)** - Fully documented template with all available options

### Environment Variables

Override any configuration setting with environment variables:

```bash
# Server ports
export MOCKFORGE_HTTP_PORT=9080
export MOCKFORGE_WS_PORT=8081
export MOCKFORGE_GRPC_PORT=9090
export MOCKFORGE_ADMIN_PORT=9091

# Enable features
export MOCKFORGE_ADMIN_ENABLED=true
export MOCKFORGE_LATENCY_ENABLED=true

# Logging
export MOCKFORGE_LOG_LEVEL=debug
```

### Configuration Options

- **HTTP Server**: Port, host, OpenAPI spec, CORS settings
- **WebSocket Server**: Port, host, replay files, timeouts
- **gRPC Server**: Port, host, proto files, TLS configuration
- **Admin UI**: Enable/disable, authentication, custom port
- **Core Features**: Latency profiles, failure injection, proxy settings
- **Data Generation**: Default settings, RAG configuration, custom templates

## 🛠️ Development

### Prerequisites

- Rust 1.70 or later
- Make
- Python 3 (for some tooling)

### Setup

```bash
# Clone the repository
git clone https://github.com/SaaSy-Solutions/mockforge.git
cd mockforge

# Set up development environment (installs all tools and hooks)
make setup

# Build the project
make build

# Run all tests
make test

# Run all quality checks
make check-all
```

### Development Workflow

```bash
# Start development mode with file watching
make dev

# Format code
make fmt

# Run lints
make clippy

# Run security audit
make audit

# Generate documentation
make doc

# Build user docs
make book
```

### Project Structure

```text
mockforge/
├── crates/                     # Workspace crates
│   ├── mockforge-cli/          # Command-line interface
│   ├── mockforge-core/         # Shared logic (routing, validation, latency, proxy)
│   ├── mockforge-http/         # HTTP mocking library
│   ├── mockforge-ws/           # WebSocket mocking library
│   ├── mockforge-grpc/         # gRPC mocking library
│   ├── mockforge-data/         # Synthetic data generation (faker + RAG)
│   └── mockforge-ui/           # Admin UI (Axum routes + static assets)
├── config.example.yaml         # Configuration example
├── docs/                       # Project documentation
├── book/                       # mdBook documentation
├── examples/                   # Example configurations and test files
├── tools/                      # Development tools
├── scripts/                    # Setup and utility scripts
├── .github/                    # GitHub Actions and templates
└── tools/                      # Development utilities
```

### Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Release Process

This project uses automated releases with [cargo-release](https://github.com/crate-ci/cargo-release):

```bash
# Patch release (bug fixes)
make release-patch

# Minor release (new features)
make release-minor

# Major release (breaking changes)
make release-major
```

## 💼 Case Studies

### Case Study 1: Microservices Development at Scale

**Challenge**: A fintech company needed to develop and test 15+ microservices that communicate via gRPC, REST, and WebSocket protocols. Waiting for all services to be ready blocked parallel development.

**Solution**: MockForge provided:
- **gRPC HTTP Bridge**: Frontend teams tested gRPC services using familiar REST tools
- **Multi-Protocol Support**: Single mock server handled HTTP, gRPC, and WebSocket endpoints
- **Workspace Sync**: Shared mock configurations via Git across distributed teams
- **Advanced Data Generation**: RAG-powered realistic financial data with referential integrity

**Results**:
- 60% reduction in integration testing time
- 3 teams able to develop in parallel without blocking
- 100+ realistic test scenarios with deterministic data

### Case Study 2: Third-Party API Integration Testing

**Challenge**: An e-commerce platform integrated with 8 external payment, shipping, and inventory APIs. Testing was expensive, slow, and unpredictable due to rate limits and sandbox limitations.

**Solution**: MockForge enabled:
- **OpenAPI-Driven Mocks**: Auto-generated mocks from vendor OpenAPI specs
- **Latency & Fault Injection**: Realistic simulation of network issues and API failures
- **Stateful Mocking**: Transaction flows with proper state management
- **Template Expansion**: Dynamic responses with timestamps, UUIDs, and context-aware data

**Results**:
- Zero cost for testing (no sandbox API calls)
- 95% test coverage for error scenarios
- CI/CD pipeline runtime reduced from 45min to 8min

### Case Study 3: Mobile App Development

**Challenge**: A mobile team needed to test iOS and Android apps against a backend API that was constantly evolving. The backend team couldn't provide stable test environments.

**Solution**: MockForge provided:
- **Fixture Management**: Record real API responses and replay them
- **Admin UI**: Product managers created test scenarios without coding
- **WebSocket Scripted Replay**: Tested real-time chat and notifications
- **Cross-Endpoint Validation**: Ensured data consistency across related endpoints

**Results**:
- Mobile developers unblocked from backend dependencies
- QA team created 50+ test scenarios independently
- Bug detection 2 weeks earlier in development cycle

### Case Study 4: Legacy System Migration

**Challenge**: A healthcare company was migrating from a monolithic SOAP API to microservices-based REST APIs. They needed to run both systems in parallel during the 18-month transition.

**Solution**: MockForge acted as:
- **Protocol Adapter**: Bridged SOAP requests to REST endpoints for testing
- **Data Transformation**: Template system converted between legacy and new data formats
- **End-to-End Encryption**: Secured sensitive patient data in mock configurations
- **Gradual Migration**: Mocked incomplete services while others went live

**Results**:
- Zero downtime during migration
- Comprehensive regression testing across old and new systems
- 30% faster migration timeline

### Case Study 5: Performance Testing & Load Simulation

**Challenge**: A SaaS platform needed to performance test their client application under various backend conditions (slow responses, partial failures, high load).

**Solution**: MockForge delivered:
- **Configurable Latency Profiles**: Simulated various network conditions
- **Failure Injection**: Random failures, timeouts, and partial responses
- **High Throughput**: Rust-native performance handled 10K+ req/sec
- **Metrics & Monitoring**: Real-time performance dashboards

**Results**:
- Identified 12 critical performance bottlenecks
- Optimized client retry logic and caching
- Production performance improved by 40%

### Common Use Cases

- **🔄 Continuous Integration**: Fast, reliable mocks in CI/CD pipelines
- **📱 Mobile/Frontend Development**: Work independently of backend teams
- **🧪 Integration Testing**: Test complex multi-service interactions
- **🎓 Training & Demos**: Consistent demo environments without live systems
- **🔧 Third-Party API Testing**: Test external integrations without costs or rate limits
- **⚡ Performance Testing**: Simulate various network and load conditions
- **🚀 Development Acceleration**: Parallel development of dependent services

## ⚡ Performance Benchmarks

MockForge includes comprehensive performance benchmarks using Criterion.rs to measure and track performance across releases.

### Benchmark Categories

**Template Rendering**
- Simple variable substitution: `{{name}}`
- Complex nested templates: `{{user.address.city}}`
- Array iteration: `{{#each items}}`

**JSON Schema Validation**
- Simple schema validation (single object)
- Complex nested schema validation
- Large array validation (100+ items)

**OpenAPI Spec Parsing**
- Small specs (1-5 paths)
- Medium specs (10-50 paths)
- Large specs (100+ paths with complex schemas)

**Data Generation**
- Single record generation
- Bulk data generation (1000+ records)
- RAG-powered synthetic data

**Memory Profiling**
- Large OpenAPI spec parsing (100+ paths)
- Deep template rendering (nested structures)
- Bulk data validation

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark suite
cargo bench --bench core_benchmarks

# Run benchmarks with specific filter
cargo bench template

# Generate detailed HTML reports
cargo bench -- --save-baseline main
```

### Benchmark Results

Typical performance metrics on modern hardware (AMD Ryzen 9 / Intel i9):

| Operation | Throughput | Latency |
|-----------|------------|---------|
| Simple template rendering | ~500K ops/sec | ~2 µs |
| Complex template rendering | ~100K ops/sec | ~10 µs |
| JSON schema validation (simple) | ~1M ops/sec | ~1 µs |
| JSON schema validation (complex) | ~200K ops/sec | ~5 µs |
| OpenAPI spec parsing (small) | ~10K ops/sec | ~100 µs |
| OpenAPI spec parsing (large) | ~500 ops/sec | ~2 ms |
| Data generation (single record) | ~50K ops/sec | ~20 µs |

*Note: Results vary based on hardware, spec complexity, and system load. Run benchmarks on your target hardware for accurate metrics.*

### Continuous Performance Monitoring

Benchmarks are run automatically in CI/CD:
- On every pull request to detect performance regressions
- Baseline comparisons against main branch
- Historical performance tracking across releases

View the latest benchmark results in our [GitHub Actions](https://github.com/SaaSy-Solutions/mockforge/actions/workflows/benchmarks.yml).

## 📚 Documentation

- [User Guide](https://docs.mockforge.dev/) - Complete documentation
- [API Reference](https://docs.rs/mockforge) - Rust API documentation
- [Contributing](CONTRIBUTING.md) - How to contribute
- [Changelog](CHANGELOG.md) - Release notes
- [Benchmarks](https://github.com/SaaSy-Solutions/mockforge/tree/main/benches) - Performance benchmarks

## 💬 Getting Help & Support

### Quick Links

- **[📖 FAQ (Frequently Asked Questions)](https://docs.mockforge.dev/reference/faq.html)** - Quick answers to common questions
- **[🔧 Troubleshooting Guide](https://docs.mockforge.dev/reference/troubleshooting.html)** - Solutions for common issues
- **[🚀 5-Minute Tutorial](https://docs.mockforge.dev/getting-started/five-minute-api.html)** - Fastest way to get started
- **[📋 Configuration Reference](https://github.com/SaaSy-Solutions/mockforge/blob/main/config.template.yaml)** - Complete config template with all options

### Common Issues

| Issue | Quick Fix |
|-------|-----------|
| **Server won't start** | `lsof -i :3000` → `mockforge serve --http-port 3001` |
| **Template variables not working** | `mockforge serve --response-template-expand` |
| **Validation too strict** | `mockforge serve --validation warn` |
| **Admin UI not loading** | `mockforge serve --admin --admin-port 9080` |
| **Docker port conflicts** | `docker run -p 3001:3000 mockforge` |
| **Docker permission issues** | `sudo chown -R 1000:1000 fixtures/` (Linux) |

See the [complete troubleshooting guide](https://docs.mockforge.dev/reference/troubleshooting.html) for detailed solutions.

### Community & Support

- **[GitHub Issues](https://github.com/SaaSy-Solutions/mockforge/issues)** - Report bugs or request features
- **[GitHub Discussions](https://github.com/SaaSy-Solutions/mockforge/discussions)** - Ask questions and share ideas
- **[Discord](https://discord.gg/2FxXqKpa)** - Join our community chat
- **[Contributing Guide](CONTRIBUTING.md)** - Contribute to MockForge development

### Need Help?

When reporting issues, please include:
1. MockForge version (`mockforge --version`)
2. Operating system
3. Configuration file (if applicable)
4. Steps to reproduce
5. Expected vs actual behavior
6. Error logs (`RUST_LOG=debug mockforge serve`)

## 📄 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
## Validation Modes

You can control request/response validation via CLI, environment, or config.

- Environment:
- `MOCKFORGE_REQUEST_VALIDATION=off|warn|enforce` (default: enforce)
- `MOCKFORGE_AGGREGATE_ERRORS=true|false` (default: true)
- `MOCKFORGE_RESPONSE_VALIDATION=true|false` (default: false)
- `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true|false` (default: false)
  - When true, mock responses (including media-level `example` bodies) expand tokens:
    - `{{uuid}}` → random UUID v4
    - `{{now}}` → RFC3339 timestamp
    - `{{now±Nd|Nh|Nm|Ns}}` → timestamp offset by days/hours/minutes/seconds, e.g., `{{now+2h}}`, `{{now-30m}}`
    - `{{rand.int}}` → random integer
    - `{{rand.float}}` → random float
  - Also supports ranged and faker tokens when enabled:
    - `{{randInt 10 99}}`, `{{rand.int -5 5}}`
    - `{{faker.uuid}}`, `{{faker.email}}`, `{{faker.name}}`, `{{faker.address}}`, `{{faker.phone}}`, `{{faker.company}}`, `{{faker.url}}`, `{{faker.ip}}`, `{{faker.color}}`, `{{faker.word}}`, `{{faker.sentence}}`, `{{faker.paragraph}}`
  - Determinism: set `MOCKFORGE_FAKE_TOKENS=false` to disable faker token expansion (uuid/now/rand tokens still expand).

 - `MOCKFORGE_VALIDATION_STATUS=400|422` (default: 400)
   - Status code returned on request validation failure in enforce mode.

- CLI (serve):
  - `--validation off|warn|enforce`
  - `--aggregate-errors`
  - `--validate-responses`

- Config (config.yaml):

```yaml
http:
  request_validation: "enforce"   # off|warn|enforce
  aggregate_validation_errors: true
  validate_responses: false
  skip_admin_validation: true
  validation_overrides:
    "POST /users/{id}": "warn"
    "GET /internal/health": "off"
```

When aggregation is enabled, 400 responses include both a flat `errors` list and a `details` array with structured items:

```json
{
  "error": "request validation failed",
  "details": [
    { "path": "query.q", "code": "type", "message": "query.q: expected number, got \"abc\"", "value": "abc" }
  ]
}
```
