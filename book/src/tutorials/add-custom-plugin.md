# Add a Custom Plugin

**Goal**: Extend MockForge with a plugin to add custom authentication or data generation functionality.

**Time**: 10 minutes

## What You'll Learn

- Install a plugin from a remote source
- Install a plugin from a local file
- Use a plugin in your mock configuration
- Create a simple custom plugin
- Test and debug plugins

## Prerequisites

- MockForge installed ([Installation Guide](../getting-started/installation.md))
- Basic understanding of MockForge configuration
- (Optional) Rust toolchain for building custom plugins

## Step 1: Install a Pre-Built Plugin

MockForge comes with example plugins you can install immediately.

### Install the JWT Authentication Plugin

```bash
# Install from the examples directory (if building from source)
mockforge plugin install examples/plugins/auth-jwt

# Or install from a URL (when published)
mockforge plugin install https://github.com/SaaSy-Solutions/mockforge/releases/download/v1.0.0/auth-jwt-plugin.wasm
```

### Verify Installation

```bash
mockforge plugin list
```

Output:
```
Installed Plugins:
  - auth-jwt (v1.0.0)
    Description: JWT authentication and token generation
    Author: MockForge Team
```

## Step 2: Use the Plugin in Your Configuration

Create a config file that uses the JWT plugin:

**`api-with-auth.yaml`:**
```yaml
http:
  port: 3000
  response_template_expand: true

  # Load the plugin
  plugins:
    - name: auth-jwt
      config:
        secret: "my-super-secret-key"
        algorithm: HS256
        expiry: 3600  # 1 hour

  routes:
    # Login endpoint - generates JWT token
    - path: /auth/login
      method: POST
      response:
        status: 200
        headers:
          Content-Type: application/json
        body: |
          {
            "token": "{{plugin:auth-jwt:generate_token({{request.body.username}})}}",
            "expiresIn": 3600
          }

    # Protected endpoint - validates JWT
    - path: /users/me
      method: GET
      middleware:
        - plugin: auth-jwt
          action: validate_token
      response:
        status: 200
        body: |
          {
            "id": "{{uuid}}",
            "username": "{{plugin:auth-jwt:get_claim(username)}}",
            "email": "{{plugin:auth-jwt:get_claim(email)}}"
          }
```

## Step 3: Test the Plugin

Start the server:
```bash
mockforge serve --config api-with-auth.yaml
```

### Login and Get Token

```bash
curl -X POST http://localhost:3000/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "alice", "password": "secret123"}'
```

Response:
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VybmFtZSI6ImFsaWNlIiwiZXhwIjoxNzA5NTY3ODkwfQ.signature",
  "expiresIn": 3600
}
```

### Use Token to Access Protected Endpoint

```bash
# Save the token
TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

# Access protected endpoint
curl http://localhost:3000/users/me \
  -H "Authorization: Bearer $TOKEN"
```

Response:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "username": "alice",
  "email": "alice@example.com"
}
```

### Try Without Token (Should Fail)

```bash
curl http://localhost:3000/users/me
```

Response:
```json
{
  "error": "Unauthorized",
  "message": "Missing or invalid JWT token"
}
```

## Step 4: Install the Template Crypto Plugin

Let's install another plugin for encryption in templates:

```bash
mockforge plugin install examples/plugins/template-crypto
```

**`crypto-config.yaml`:**
```yaml
http:
  port: 3000
  response_template_expand: true

  plugins:
    - name: template-crypto
      config:
        default_algorithm: aes-256-gcm

  routes:
    - path: /encrypt
      method: POST
      response:
        status: 200
        body: |
          {
            "encrypted": "{{plugin:template-crypto:encrypt({{request.body.message}})}}",
            "algorithm": "aes-256-gcm"
          }

    - path: /decrypt
      method: POST
      response:
        status: 200
        body: |
          {
            "decrypted": "{{plugin:template-crypto:decrypt({{request.body.encrypted}})}}"
          }
```

Test it:
```bash
# Encrypt a message
curl -X POST http://localhost:3000/encrypt \
  -H "Content-Type: application/json" \
  -d '{"message": "secret data"}'

# Decrypt the result
curl -X POST http://localhost:3000/decrypt \
  -H "Content-Type: application/json" \
  -d '{"encrypted": "base64-encrypted-string"}'
```

## Step 5: Create a Simple Custom Plugin

Let's create a custom plugin that generates fake company data.

### Project Structure

```bash
mkdir my-company-plugin
cd my-company-plugin
cargo init --lib
```

**`Cargo.toml`:**
```toml
[package]
name = "company-data-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
mockforge-plugin-api = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
fake = { version = "2.9", features = ["derive"] }
```

**`src/lib.rs`:**
```rust
use mockforge_plugin_api::{Plugin, PluginContext, PluginResult};
use fake::{Fake, faker::company::en::*};
use serde_json::json;

pub struct CompanyDataPlugin;

impl Plugin for CompanyDataPlugin {
    fn name(&self) -> &str {
        "company-data"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn execute(&self, ctx: &PluginContext) -> PluginResult {
        match ctx.action.as_str() {
            "generate_company" => {
                let company_name: String = CompanyName().fake();
                let industry: String = Industry().fake();
                let buzzword: String = Buzzword().fake();

                Ok(json!({
                    "name": company_name,
                    "industry": industry,
                    "tagline": buzzword,
                    "founded": (1950..2024).fake::<i32>(),
                    "employees": (10..10000).fake::<i32>()
                }))
            }
            "generate_tagline" => {
                Ok(json!({
                    "tagline": Buzzword().fake::<String>()
                }))
            }
            _ => Err(format!("Unknown action: {}", ctx.action))
        }
    }
}

mockforge_plugin_api::export_plugin!(CompanyDataPlugin);
```

### Build the Plugin

```bash
cargo build --release --target wasm32-unknown-unknown
```

The compiled plugin will be at:
```
target/wasm32-unknown-unknown/release/company_data_plugin.wasm
```

## Step 6: Install and Use Your Custom Plugin

```bash
# Install from local file
mockforge plugin install ./target/wasm32-unknown-unknown/release/company_data_plugin.wasm
```

**`company-api.yaml`:**
```yaml
http:
  port: 3000
  response_template_expand: true

  plugins:
    - name: company-data

  routes:
    - path: /companies
      method: GET
      response:
        status: 200
        body: |
          [
            {{plugin:company-data:generate_company()}},
            {{plugin:company-data:generate_company()}},
            {{plugin:company-data:generate_company()}}
          ]

    - path: /tagline
      method: GET
      response:
        status: 200
        body: "{{plugin:company-data:generate_tagline()}}"
```

Test it:
```bash
mockforge serve --config company-api.yaml

# Generate fake companies
curl http://localhost:3000/companies
```

Response:
```json
[
  {
    "name": "Acme Corporation",
    "industry": "Technology",
    "tagline": "Innovative solutions for tomorrow",
    "founded": 1985,
    "employees": 2500
  },
  {
    "name": "GlobalTech Industries",
    "industry": "Manufacturing",
    "tagline": "Building the future",
    "founded": 2001,
    "employees": 850
  },
  {
    "name": "DataSync Solutions",
    "industry": "Software",
    "tagline": "Connecting businesses worldwide",
    "founded": 2015,
    "employees": 120
  }
]
```

## Step 7: Plugin Management Commands

### List Installed Plugins
```bash
mockforge plugin list
```

### Get Plugin Info
```bash
mockforge plugin info auth-jwt
```

### Update a Plugin
```bash
mockforge plugin update auth-jwt
```

### Uninstall a Plugin
```bash
mockforge plugin uninstall company-data
```

### Install with Version Pinning
```bash
# From Git with version tag
mockforge plugin install https://github.com/user/plugin#v1.2.0

# From URL with checksum verification
mockforge plugin install https://example.com/plugin.wasm --checksum sha256:abc123...
```

## Common Plugin Use Cases

| Use Case | Plugin Type | Example |
|----------|-------------|---------|
| **Authentication** | Middleware | JWT, OAuth2, API keys |
| **Data Generation** | Template function | Faker, custom generators |
| **Data Transformation** | Response modifier | Format converters, encryption |
| **External Integration** | Data source | Database, CSV files, APIs |
| **Custom Validation** | Request validator | Business rule enforcement |
| **Rate Limiting** | Middleware | Token bucket, sliding window |

## Plugin Security

MockForge plugins run in a **WebAssembly sandbox** with:

- **Memory isolation**: Plugins can't access host memory
- **Resource limits**: CPU and memory usage capped
- **No network access**: Plugins can't make external requests (unless explicitly allowed)
- **File system restrictions**: Limited file access

### Configure Plugin Permissions

**`config.yaml`:**
```yaml
plugins:
  security:
    max_memory_mb: 50
    max_execution_ms: 1000
    allow_network: false
    allow_file_access: false

  plugins:
    - name: auth-jwt
      permissions:
        network: false
        file_read: false

    - name: db-connector
      permissions:
        network: true  # Needs network for DB connection
        file_read: true
```

## Debugging Plugins

### Enable Plugin Debug Logs
```bash
MOCKFORGE_LOG_LEVEL=debug mockforge serve --config api-with-auth.yaml
```

### Test Plugin in Isolation
```bash
mockforge plugin test auth-jwt --action generate_token --input '{"username": "test"}'
```

### Plugin Benchmarking
```bash
mockforge plugin bench auth-jwt --iterations 1000
```

## Troubleshooting

**Plugin not found after installation?**
```bash
# Check plugin directory
mockforge plugin list --verbose

# Reinstall
mockforge plugin install ./path/to/plugin.wasm --force
```

**Plugin execution fails?**
- Check plugin logs with `MOCKFORGE_LOG_LEVEL=debug`
- Verify plugin configuration syntax
- Test plugin in isolation with `mockforge plugin test`

**Plugin build fails?**
```bash
# Ensure wasm target is installed
rustup target add wasm32-unknown-unknown

# Clean and rebuild
cargo clean
cargo build --release --target wasm32-unknown-unknown
```

## What's Next?

- [Plugin API Reference](../../docs/plugins/api-reference/core.md) - Complete plugin API documentation
- [Plugin Development Guide](../../docs/plugins/development-guide.md) - Advanced plugin development
- [Security Model](../../docs/plugins/security/model.md) - Plugin security architecture
- [Example Plugins](../../examples/plugins/README.md) - More plugin examples

---

**Pro Tip**: Plugins can be version-controlled and shared with your team. Commit the `.wasm` file or the source code to Git, and everyone can use the same custom functionality!
