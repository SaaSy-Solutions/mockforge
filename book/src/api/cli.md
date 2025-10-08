# CLI Reference

MockForge provides a comprehensive command-line interface for managing mock servers and generating test data. This reference covers all available commands, options, and usage patterns.

## Global Options

All MockForge commands support the following global options:

```bash
mockforge-cli [OPTIONS] <COMMAND>
```

### Global Options

- `-h, --help`: Display help information

## Commands

### `serve` - Start Mock Servers

The primary command for starting MockForge's mock servers with support for HTTP, WebSocket, and gRPC protocols.

```bash
mockforge-cli serve [OPTIONS]
```

#### Server Options

**Port Configuration:**
- `--http-port <PORT>`: HTTP server port (default: 3000)
- `--ws-port <PORT>`: WebSocket server port (default: 3001)
- `--grpc-port <PORT>`: gRPC server port (default: 50051)

**API Specification:**
- `--spec <PATH>`: OpenAPI spec file for HTTP server (JSON or YAML format)

**Configuration:**
- `-c, --config <PATH>`: Path to configuration file

#### Admin UI Options

**Admin UI Control:**
- `--admin`: Enable admin UI
- `--admin-port <PORT>`: Admin UI port (default: 9080)
- `--admin-embed`: Force embedding Admin UI under HTTP server
- `--admin-mount-path <PATH>`: Explicit mount path for embedded Admin UI (implies `--admin-embed`)
- `--admin-standalone`: Force standalone Admin UI on separate port (overrides embed)
- `--disable-admin-api`: Disable Admin API endpoints (UI loads but API routes are absent)

#### Validation Options

**Request Validation:**
- `--validation <MODE>`: Request validation mode (default: enforce)
  - `off`: Disable validation
  - `warn`: Log warnings but allow requests
  - `enforce`: Reject invalid requests
- `--aggregate-errors`: Aggregate request validation errors into JSON array
- `--validate-responses`: Validate responses (warn-only)
- `--validation-status <CODE>`: Validation error HTTP status code (default: 400)

#### Response Processing

**Template Expansion:**
- `--response-template-expand`: Expand templating tokens in responses/examples

#### Chaos Engineering

**Latency Simulation:**
- `--latency-enabled`: Enable latency simulation

**Failure Injection:**
- `--failures-enabled`: Enable failure injection

#### Examples

**Basic HTTP Server:**
```bash
mockforge-cli serve --spec examples/openapi-demo.json --http-port 3000
```

**Full Multi-Protocol Setup:**
```bash
mockforge-cli serve \
  --spec examples/openapi-demo.json \
  --http-port 3000 \
  --ws-port 3001 \
  --grpc-port 50051 \
  --admin \
  --admin-port 9080 \
  --response-template-expand
```

**Development Configuration:**
```bash
mockforge-cli serve \
  --config demo-config.yaml \
  --validation warn \
  --response-template-expand \
  --latency-enabled
```

**Production Configuration:**
```bash
mockforge-cli serve \
  --config production-config.yaml \
  --validation enforce \
  --admin-standalone
```

### `init` - Initialize New Project

Create a new MockForge project with a template configuration file.

```bash
mockforge-cli init [OPTIONS] <NAME>
```

#### Arguments

- `<NAME>`: Project name or directory path
  - Use `.` to initialize in the current directory
  - Use a project name to create a new directory

#### Options

- `--no-examples`: Skip creating example files (only create `mockforge.yaml`)

#### Examples

```bash
# Create a new project in a new directory
mockforge-cli init my-mock-api

# Initialize in the current directory
mockforge-cli init .

# Initialize without examples
mockforge-cli init my-project --no-examples
```

#### What Gets Created

1. **mockforge.yaml**: Main configuration file with:
   - HTTP, WebSocket, gRPC server configurations
   - Admin UI settings
   - Core features (latency, failures, overrides)
   - Observability configuration
   - Data generation settings
   - Logging configuration

2. **examples/** directory (unless `--no-examples`):
   - `openapi.json`: Sample OpenAPI specification
   - Example data files

#### See Also

- [Configuration Files Guide](../configuration/files.md)
- [Complete Config Template](https://github.com/SaaSy-Solutions/mockforge/blob/main/config.template.yaml)

---

### `config` - Configuration Management

Validate and manage MockForge configuration files.

```bash
mockforge-cli config <SUBCOMMAND>
```

#### Subcommands

##### `validate` - Validate Configuration File

Validate a MockForge configuration file for syntax and structure errors.

```bash
mockforge-cli config validate [OPTIONS]
```

**Options:**
- `--config <PATH>`: Path to config file to validate
  - If omitted, auto-discovers `mockforge.yaml` or `mockforge.yml` in current and parent directories

**What Gets Validated:**
- YAML syntax and structure
- File existence
- HTTP endpoints count
- Request chains count
- Missing sections (warnings)

**Examples:**

```bash
# Validate config in current directory
mockforge-cli config validate

# Validate specific config file
mockforge-cli config validate --config my-config.yaml

# Validate before starting server
mockforge-cli config validate && mockforge-cli serve
```

**Output Example:**
```
🔍 Validating MockForge configuration...
📄 Checking configuration file: mockforge.yaml
✅ Configuration is valid

📊 Summary:
   Found 5 HTTP endpoints
   Found 2 chains

⚠️  Warnings:
   - No WebSocket configuration found
```

**Common Issues:**
- **Invalid YAML syntax**: Fix indentation, quotes, or structure
- **File not found**: Check path or run `mockforge init`
- **Missing sections**: Add HTTP, admin, or other required sections

**Note**: Current validation is basic (syntax, structure, counts). For comprehensive field validation, see the [Configuration Validation Guide](../reference/config-validation.md).

#### See Also

- [Configuration Validation Guide](../reference/config-validation.md)
- [Configuration Schema Reference](../reference/config-schema.md)
- [Troubleshooting Guide](../reference/troubleshooting.md)

---

### `data` - Generate Synthetic Data

Generate synthetic test data using various templates and schemas.

```bash
mockforge-cli data <SUBCOMMAND>
```

#### Subcommands

##### `template` - Generate from Built-in Templates

Generate data using MockForge's built-in data generation templates.

```bash
mockforge-cli data template [OPTIONS]
```

**Options:**
- `--count <N>`: Number of items to generate (default: 1)
- `--format <FORMAT>`: Output format (json, yaml, csv)
- `--template <NAME>`: Template name (user, product, order, etc.)
- `--output <PATH>`: Output file path

**Examples:**

```bash
# Generate 10 user records as JSON
mockforge-cli data template --template user --count 10 --format json

# Generate product data to file
mockforge-cli data template --template product --count 50 --output products.json
```

##### `schema` - Generate from JSON Schema

Generate data conforming to a JSON Schema specification.

```bash
mockforge-cli data schema [OPTIONS] <SCHEMA>
```

**Parameters:**
- `<SCHEMA>`: Path to JSON Schema file

**Options:**
- `--count <N>`: Number of items to generate (default: 1)
- `--format <FORMAT>`: Output format (json, yaml)
- `--output <PATH>`: Output file path

**Examples:**

```bash
# Generate data from user schema
mockforge-cli data schema --count 5 user-schema.json

# Generate and save to file
mockforge-cli data schema --count 100 --output generated-data.json api-schema.json
```

##### `open-api` - Generate from OpenAPI Spec

Generate mock data based on OpenAPI specification schemas.

```bash
mockforge-cli data open-api [OPTIONS] <SPEC>
```

**Parameters:**
- `<SPEC>`: Path to OpenAPI specification file

**Options:**
- `--endpoint <PATH>`: Specific endpoint to generate data for
- `--method <METHOD>`: HTTP method (get, post, put, delete)
- `--count <N>`: Number of items to generate (default: 1)
- `--format <FORMAT>`: Output format (json, yaml)
- `--output <PATH>`: Output file path

**Examples:**

```bash
# Generate data for all endpoints in OpenAPI spec
mockforge-cli data open-api api-spec.yaml

# Generate data for specific endpoint
mockforge-cli data open-api --endpoint /users --method get --count 20 api-spec.yaml

# Generate POST request body data
mockforge-cli data open-api --endpoint /users --method post api-spec.yaml
```

### `admin` - Admin UI Server

Start the Admin UI as a standalone server without the main mock servers.

```bash
mockforge-cli admin [OPTIONS]
```

#### Options

- `--port <PORT>`: Server port (default: 9080)

#### Examples

```bash
# Start admin UI on default port
mockforge-cli admin

# Start admin UI on custom port
mockforge-cli admin --port 9090
```

### `sync` - Workspace Synchronization Daemon

Start a background daemon that monitors a workspace directory for file changes and automatically syncs them to MockForge workspaces.

```bash
mockforge-cli sync [OPTIONS]
```

#### Options

**Required:**
- `--workspace-dir <PATH>` or `-w <PATH>`: Workspace directory to monitor for changes

**Optional:**
- `--config <PATH>` or `-c <PATH>`: Configuration file path for sync settings

#### How It Works

The sync daemon provides bidirectional synchronization between workspace files and MockForge's internal workspace storage:

1. **File Monitoring**: Watches for `.yaml` and `.yml` files in the workspace directory
2. **Automatic Import**: When files are created or modified, they're automatically imported into the workspace
3. **Real-time Updates**: Changes are detected and processed immediately
4. **Visual Feedback**: Clear console output shows what's happening in real-time

**File Requirements:**
- Only `.yaml` and `.yml` files are monitored
- Hidden files (starting with `.`) are ignored
- Files must be valid MockRequest YAML format

**What You'll See:**
- File creation notifications with import status
- File modification notifications with update status
- File deletion notifications (files are not auto-deleted from workspace)
- Error messages if imports fail
- Real-time feedback for all sync operations

#### Examples

**Basic Usage:**

```bash
# Start sync daemon for a workspace directory
mockforge-cli sync --workspace-dir ./my-workspace

# Use short form
mockforge-cli sync -w ./my-workspace

# With custom config
mockforge-cli sync --workspace-dir /path/to/workspace --config sync-config.yaml
```

**Git Integration:**

```bash
# Monitor a Git repository directory
mockforge-cli sync --workspace-dir /path/to/git/repo/workspaces

# Changes you make in Git will automatically sync to MockForge
# Perfect for team collaboration via Git
```

**Development Workflow:**

```bash
# 1. Start the sync daemon in one terminal
mockforge-cli sync --workspace-dir ./workspaces

# 2. In another terminal, edit workspace files
vim ./workspaces/my-request.yaml

# 3. Save the file - it will automatically import to MockForge
# You'll see output like:
#   🔄 Detected 1 file change in workspace 'default'
#     📝 Modified: my-request.yaml
#        ✅ Successfully updated
```

#### Example Output

When you start the sync daemon, you'll see:

```
🔄 Starting MockForge Sync Daemon...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📁 Workspace directory: ./my-workspace

ℹ️  What the sync daemon does:
   • Monitors the workspace directory for .yaml/.yml file changes
   • Automatically imports new or modified request files
   • Syncs changes bidirectionally between files and workspace
   • Skips hidden files (starting with .)

🔍 Monitoring for file changes...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ Sync daemon started successfully!
💡 Press Ctrl+C to stop

📂 Monitoring workspace 'default' in directory: ./my-workspace
```

When files change, you'll see:

```
🔄 Detected 1 file change in workspace 'default'
  ➕ Created: new-endpoint.yaml
     ✅ Successfully imported

🔄 Detected 2 file changes in workspace 'default'
  📝 Modified: user-api.yaml
     ✅ Successfully updated
  🗑️  Deleted: old-endpoint.yaml
     ℹ️  Auto-deletion from workspace is disabled
```

If errors occur:

```
🔄 Detected 1 file change in workspace 'default'
  📝 Modified: invalid-file.yaml
     ⚠️  Failed to import: File is not a recognized format (expected MockRequest YAML)
```

#### Stopping the Daemon

Press `Ctrl+C` to gracefully stop the sync daemon:

```
^C
🛑 Received shutdown signal
⏹️  Stopped monitoring workspace 'default' in directory: ./my-workspace
👋 Sync daemon stopped
```

#### Best Practices

**Version Control:**
```bash
# Use sync with Git for team collaboration
cd /path/to/git/repo
mockforge-cli sync --workspace-dir ./workspaces

# Team members can push/pull changes
# The sync daemon will automatically import updates
```

**Development Workflow:**
```bash
# Keep sync daemon running during development
# Edit files in your favorite editor
# Changes automatically sync to MockForge
# Perfect for file-based workflows
```

**Directory Organization:**
```bash
# Organize workspace files in subdirectories
workspaces/
├── api-v1/
│   ├── users.yaml
│   └── products.yaml
├── api-v2/
│   └── users.yaml
└── internal/
    └── admin.yaml

# All .yaml files will be monitored
mockforge-cli sync --workspace-dir ./workspaces
```

#### Troubleshooting

**Files not importing:**
- Ensure files have `.yaml` or `.yml` extension
- Check that files are valid MockRequest YAML format
- Look for error messages in the console output
- Verify files are not hidden (don't start with `.`)

**Permission errors:**
- Ensure MockForge has read access to the workspace directory
- Check file permissions: `ls -la workspace-dir/`

**Changes not detected:**
- The sync daemon uses filesystem notifications
- Some network filesystems may not support change notifications
- Try editing the file locally rather than over a network mount

**Enable debug logging:**
```bash
RUST_LOG=mockforge_core::sync_watcher=debug mockforge-cli sync --workspace-dir ./workspace
```

## Configuration File Format

MockForge supports YAML configuration files that can be used instead of command-line options.

### Basic Configuration Structure

```yaml
# Server configuration
server:
  http_port: 3000
  ws_port: 3001
  grpc_port: 50051

# API specification
spec: examples/openapi-demo.json

# Admin UI configuration
admin:
  enabled: true
  port: 9080
  embedded: false
  mount_path: "/admin"
  standalone: true
  disable_api: false

# Validation settings
validation:
  mode: enforce
  aggregate_errors: false
  validate_responses: false
  status_code: 400

# Response processing
response:
  template_expand: true

# Chaos engineering
chaos:
  latency_enabled: false
  failures_enabled: false

# Protocol-specific settings
grpc:
  proto_dir: "proto/"
  enable_reflection: true

websocket:
  replay_file: "examples/ws-demo.jsonl"
```

### Configuration Precedence

Configuration values are applied in the following order (later sources override earlier ones):

1. **Default values** (compiled into the binary)
2. **Configuration file** (`-c/--config` option)
3. **Environment variables**
4. **Command-line arguments** (highest priority)

### Environment Variables

All configuration options can be set via environment variables using the `MOCKFORGE_` prefix:

```bash
# Server ports
export MOCKFORGE_HTTP_PORT=3000
export MOCKFORGE_WS_PORT=3001
export MOCKFORGE_GRPC_PORT=50051

# Admin UI
export MOCKFORGE_ADMIN_ENABLED=true
export MOCKFORGE_ADMIN_PORT=9080
export MOCKFORGE_ADMIN_JWT_SECRET=your-secret-key
export MOCKFORGE_ADMIN_SESSION_TIMEOUT=86400
export MOCKFORGE_ADMIN_AUTH_ENABLED=true

# Validation
export MOCKFORGE_VALIDATION_MODE=enforce
export MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true

# gRPC settings
export MOCKFORGE_PROTO_DIR=proto/
export MOCKFORGE_GRPC_REFLECTION_ENABLED=true

# WebSocket settings
export MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl

# Plugin system
export MOCKFORGE_PLUGINS_ENABLED=true
export MOCKFORGE_PLUGINS_DIRECTORY=~/.mockforge/plugins
export MOCKFORGE_PLUGIN_MEMORY_LIMIT=64
export MOCKFORGE_PLUGIN_CPU_LIMIT=10
export MOCKFORGE_PLUGIN_TIMEOUT=5000

# Encryption
export MOCKFORGE_ENCRYPTION_ENABLED=true
export MOCKFORGE_ENCRYPTION_ALGORITHM=aes-256-gcm
export MOCKFORGE_KEY_STORE_PATH=~/.mockforge/keys

# Synchronization
export MOCKFORGE_SYNC_ENABLED=true
export MOCKFORGE_SYNC_DIRECTORY=./workspace-sync
export MOCKFORGE_SYNC_MODE=bidirectional
export MOCKFORGE_SYNC_WATCH=true

# Data generation
export MOCKFORGE_DATA_RAG_ENABLED=true
export MOCKFORGE_DATA_RAG_PROVIDER=openai
export MOCKFORGE_DATA_RAG_API_KEY=your-api-key
```

## Exit Codes

MockForge uses standard exit codes:

- **0**: Success
- **1**: General error
- **2**: Configuration error
- **3**: Validation error
- **4**: File I/O error
- **5**: Network error

## Logging

MockForge provides configurable logging output to help with debugging and monitoring.

### Log Levels

- `error`: Only error messages
- `warn`: Warnings and errors
- `info`: General information (default)
- `debug`: Detailed debugging information
- `trace`: Very verbose tracing information

### Log Configuration

```bash
# Set log level via environment variable
export RUST_LOG=mockforge=debug

# Or via configuration file
logging:
  level: debug
  format: json
```

### Log Output

Logs include structured information about:
- HTTP requests/responses
- WebSocket connections and messages
- gRPC calls and streaming
- Configuration loading
- Template expansion
- Validation errors

## Examples

### Complete Development Setup

```bash
# Start all servers with admin UI
mockforge-cli serve \
  --spec examples/openapi-demo.json \
  --http-port 3000 \
  --ws-port 3001 \
  --grpc-port 50051 \
  --admin \
  --admin-port 9080 \
  --response-template-expand \
  --validation warn
```

### CI/CD Testing Pipeline

```bash
#!/bin/bash
# test-mockforge.sh

# Start MockForge in background
mockforge-cli serve --spec api-spec.yaml --http-port 3000 &
MOCKFORGE_PID=$!

# Wait for server to start
sleep 5

# Run API tests
npm test

# Generate test data
mockforge-cli data open-api --endpoint /users --count 100 api-spec.yaml > test-users.json

# Stop MockForge
kill $MOCKFORGE_PID
```

### Load Testing Setup

```bash
#!/bin/bash
# load-test-setup.sh

# Start MockForge with minimal validation for performance
MOCKFORGE_VALIDATION_MODE=off \
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=false \
mockforge-cli serve \
  --spec load-test-spec.yaml \
  --http-port 3000 \
  --validation off

# Now run your load testing tool against localhost:3000
# Example: hey -n 10000 -c 100 http://localhost:3000/api/test
```

### Docker Integration

```bash
# Run MockForge in Docker with CLI commands
docker run --rm -v $(pwd)/examples:/examples \
  mockforge \
  serve --spec /examples/openapi-demo.json --http-port 3000
```

## Troubleshooting

### Common Issues

**Server won't start:**
```bash
# Check if ports are available
lsof -i :3000
lsof -i :3001

# Try different ports
mockforge-cli serve --http-port 3001 --ws-port 3002
```

**Configuration not loading:**
```bash
# Validate YAML syntax
yamllint config.yaml

# Check file permissions
ls -la config.yaml
```

**OpenAPI spec not found:**
```bash
# Verify file exists and path is correct
ls -la examples/openapi-demo.json

# Use absolute path
mockforge-cli serve --spec /full/path/to/examples/openapi-demo.json
```

**Template expansion not working:**
```bash
# Ensure template expansion is enabled
mockforge-cli serve --response-template-expand --spec api-spec.yaml
```

### Debug Mode

Run with debug logging for detailed information:

```bash
RUST_LOG=mockforge=debug mockforge-cli serve --spec api-spec.yaml
```

### Health Checks

Test basic functionality:

```bash
# HTTP health check
curl http://localhost:3000/health

# WebSocket connection test
websocat ws://localhost:3001/ws

# gRPC service discovery
grpcurl -plaintext localhost:50051 list
```

This CLI reference provides comprehensive coverage of MockForge's command-line interface. For programmatic usage, see the [Rust API Reference](rust.md).
