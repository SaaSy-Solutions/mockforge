# Plugin Development Guide

This guide provides comprehensive instructions for developing plugins for the MockForge plugin system.

## 🚀 Quick Start with cargo-generate

The fastest way to create a new plugin is using our official template:

```bash
# Install cargo-generate if you haven't already
cargo install cargo-generate

# Create a new plugin from the template
cargo generate --git https://github.com/mockforge/mockforge \
  --name my-plugin-name \
  templates/plugin-template

# Or if you have the repository cloned locally:
cargo generate --path ./templates/plugin-template \
  --name my-plugin-name
```

The template will prompt you for:
- Plugin name and description
- Plugin type (auth, template, response, datasource)
- Author information
- Resource limits
- Network and filesystem permissions

This creates a complete project structure with:
- Pre-configured `Cargo.toml` with all necessary dependencies
- `plugin.yaml` manifest with your settings
- Template implementation based on your chosen plugin type
- Example tests and documentation
- Build configuration optimized for WebAssembly

### Building Your New Plugin

```bash
cd my-plugin-name

# Build for WebAssembly
cargo build --target wasm32-wasi --release

# Run tests
cargo test

# Install locally for testing
mockforge plugin install .
```

## 🏗️ Manual Project Structure

If you prefer to set up manually, use this recommended structure:

```
my-plugin/
├── Cargo.toml                 # Rust dependencies
├── plugin.yaml               # Plugin manifest
├── src/
│   ├── lib.rs               # Main plugin implementation
│   └── plugin.rs            # Plugin logic (optional)
├── tests/                    # Integration tests
├── examples/                 # Usage examples
└── README.md                 # Plugin documentation
```

## 📦 Dependencies

### Core Dependencies
```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]  # Required for WebAssembly

[dependencies]
# MockForge plugin core
mockforge-plugin-core = { path = "../../../crates/mockforge-plugin-core" }

# WebAssembly Interface Types
wit-bindgen = "0.34"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Async runtime (if needed)
tokio = { version = "1.0", features = ["macros"] }

# HTTP client (if network access needed)
reqwest = { version = "0.11", features = ["json"], optional = true }

[features]
default = []
network = ["reqwest"]
```

## 🔧 Plugin Implementation

### 1. Define Your Plugin Struct

```rust
use mockforge_plugin_core::*;
use std::sync::Arc;

#[derive(Debug)]
pub struct MyPlugin {
    config: PluginConfig,
}

impl MyPlugin {
    pub fn new() -> Self {
        Self {
            config: PluginConfig::default(),
        }
    }
}
```

### 2. Implement Plugin Traits

Choose the appropriate trait based on your plugin type:

#### Authentication Plugin
```rust
#[async_trait::async_trait]
impl AuthPlugin for MyPlugin {
    async fn authenticate(
        &self,
        context: &PluginContext,
        credentials: &AuthCredentials,
    ) -> PluginResult<AuthResult> {
        // Extract credentials
        let token = match &credentials {
            AuthCredentials::Bearer(token) => token,
            _ => return PluginResult::failure("Unsupported auth type".to_string(), 0),
        };

        // Validate token (your logic here)
        if self.validate_token(token).await {
            PluginResult::success(AuthResult::Authenticated {
                user_id: "user123".to_string(),
                claims: HashMap::from([
                    ("role".to_string(), serde_json::json!("admin")),
                    ("permissions".to_string(), serde_json::json!(["read", "write"])),
                ]),
            })
        } else {
            PluginResult::failure("Invalid token".to_string(), 0)
        }
    }

    fn get_capabilities(&self) -> PluginCapabilities {
        // Define what your plugin can access
        PluginCapabilities {
            network: NetworkCapabilities {
                allow_http_outbound: true,
                allowed_hosts: vec!["auth.example.com".to_string()],
            },
            filesystem: FilesystemCapabilities::default(),
            resources: PluginResources {
                max_memory_bytes: 16 * 1024 * 1024, // 16MB
                max_cpu_time_ms: 2000, // 2 seconds
            },
        }
    }
}
```

#### Template Plugin
```rust
#[async_trait::async_trait]
impl TemplatePlugin for MyPlugin {
    async fn execute_function(
        &self,
        function_name: &str,
        args: &[Value],
        context: &PluginContext,
    ) -> PluginResult<Value> {
        match function_name {
            "my_function" => {
                if args.is_empty() {
                    return PluginResult::failure("Missing argument".to_string(), 0);
                }

                let input = args[0].as_str()
                    .ok_or_else(|| "Argument must be a string".to_string())?;

                let result = self.process_input(input).await?;
                PluginResult::success(serde_json::json!(result))
            }
            _ => PluginResult::failure(
                format!("Unknown function: {}", function_name),
                0
            ),
        }
    }

    fn get_functions(&self) -> Vec<TemplateFunction> {
        vec![
            TemplateFunction {
                name: "my_function".to_string(),
                description: "Process input data".to_string(),
                parameters: vec![
                    FunctionParameter {
                        name: "input".to_string(),
                        param_type: "string".to_string(),
                        required: true,
                        description: "Input to process".to_string(),
                    }
                ],
                return_type: "string".to_string(),
            }
        ]
    }
}
```

#### Response Plugin
```rust
#[async_trait::async_trait]
impl ResponsePlugin for MyPlugin {
    async fn generate_response(
        &self,
        context: &PluginContext,
        request: &ResponseRequest,
        config: &ResponsePluginConfig,
    ) -> PluginResult<ResponseData> {
        // Access request data
        let method = &request.method;
        let path = &request.path;
        let headers = &request.headers;

        // Generate response based on request
        let response = match (method.as_str(), path.as_str()) {
            ("GET", "/api/users") => self.get_users_response(config).await?,
            ("POST", "/api/users") => self.create_user_response(context, config).await?,
            _ => return PluginResult::failure("Unsupported endpoint".to_string(), 0),
        };

        PluginResult::success(response)
    }
}
```

#### Response Modifier Plugin
```rust
#[async_trait::async_trait]
impl ResponseModifierPlugin for MyPlugin {
    async fn should_modify(
        &self,
        context: &PluginContext,
        request: &ResponseRequest,
        response: &ResponseData,
        config: &ResponseModifierConfig,
    ) -> Result<PluginResult<bool>> {
        // Check if we should modify this response
        Ok(PluginResult::success(
            response.content_type.contains("application/json")
        ))
    }

    async fn modify_response(
        &self,
        context: &PluginContext,
        request: &ResponseRequest,
        mut response: ResponseData,
        config: &ResponseModifierConfig,
    ) -> Result<PluginResult<ResponseData>> {
        // Add custom headers
        response.headers.insert(
            "X-Custom-Header".to_string(),
            "Modified by plugin".to_string(),
        );

        // Modify body if needed
        if let Some(mut json) = response.body_as_json() {
            json["modified_by"] = serde_json::json!("my-plugin");
            json["timestamp"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
            response.body = serde_json::to_vec(&json)?;
        }

        Ok(PluginResult::success(response))
    }

    fn priority(&self) -> i32 {
        50 // Lower numbers execute first
    }

    fn validate_config(&self, config: &ResponseModifierConfig) -> Result<()> {
        // Validate configuration
        Ok(())
    }

    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }
}
```

#### Data Source Plugin
```rust
#[async_trait::async_trait]
impl DataSourcePlugin for MyPlugin {
    async fn query(
        &self,
        query: &str,
        parameters: &HashMap<String, Value>,
        context: &PluginContext,
    ) -> PluginResult<DataSet> {
        // Parse query and execute against your data source
        let parsed_query = self.parse_query(query)?;

        match parsed_query.operation {
            Operation::Select => {
                let data = self.execute_select(&parsed_query, parameters).await?;
                PluginResult::success(data)
            }
            Operation::Insert => {
                let result = self.execute_insert(&parsed_query, parameters).await?;
                PluginResult::success(DataSet {
                    columns: vec![ColumnInfo {
                        name: "affected_rows".to_string(),
                        data_type: "integer".to_string(),
                    }],
                    rows: vec![DataRow::from(vec![serde_json::json!(result)])],
                })
            }
            _ => PluginResult::failure("Unsupported operation".to_string(), 0),
        }
    }

    fn get_schema(&self) -> PluginResult<DataSourceSchema> {
        // Return schema information
        PluginResult::success(DataSourceSchema {
            tables: vec![
                TableInfo {
                    name: "users".to_string(),
                    columns: vec![
                        ColumnInfo {
                            name: "id".to_string(),
                            data_type: "integer".to_string(),
                        },
                        ColumnInfo {
                            name: "name".to_string(),
                            data_type: "string".to_string(),
                        },
                        ColumnInfo {
                            name: "email".to_string(),
                            data_type: "string".to_string(),
                        },
                    ],
                }
            ],
        })
    }
}
```

### 3. Export Your Plugin

```rust
// At the end of your lib.rs
mockforge_plugin_core::export_plugin!(MyPlugin);
```

## 📋 Plugin Manifest

Create `plugin.yaml` in your project root:

```yaml
plugin:
  id: "my-plugin"
  version: "0.1.0"
  name: "My Awesome Plugin"
  description: "A plugin that does amazing things"
  types: ["template", "response"]  # Specify plugin types
  author:
    name: "Your Name"
    email: "your.email@example.com"
    homepage: "https://your-website.com"
  homepage: "https://github.com/your/plugin"
  repository: "https://github.com/your/plugin"
  license: "MIT OR Apache-2.0"
  keywords: ["mocking", "api", "testing"]

capabilities:
  network:
    allow_http_outbound: true
    allowed_hosts:
      - "api.example.com"
      - "*.trusted-service.com"
  filesystem:
    allow_read: true
    allow_write: false
    allowed_paths:
      - "/data/input"
      - "/tmp/plugin-cache"
  resources:
    max_memory_bytes: 67108864    # 64MB
    max_cpu_time_ms: 5000         # 5 seconds

dependencies:
  - id: "other-plugin"
    version: "^1.0.0"
    optional: false

configuration:
  schema:
    type: object
    properties:
      api_key:
        type: string
        description: "API key for external service"
      timeout:
        type: integer
        default: 30
        description: "Request timeout in seconds"
    required: ["api_key"]
```

## 🔨 Building and Testing

### Build for WebAssembly
```bash
# Build for WebAssembly
cargo build --target wasm32-wasi --release

# The output will be in target/wasm32-wasi/release/
```

### Testing Your Plugin
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_plugin_core::PluginContext;

    #[tokio::test]
    async fn test_plugin_functionality() {
        let plugin = MyPlugin::new();
        let context = PluginContext::new(
            "GET".to_string(),
            "/test".to_string(),
            HashMap::new(),
            None,
        );

        // Test your plugin logic
        let result = plugin.my_function(&context).await;
        assert!(result.success);
    }
}
```

### Integration Testing
```rust
#[cfg(test)]
mod integration_tests {
    use mockforge_plugin_loader::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_plugin_loading() {
        let loader = PluginLoader::new(PluginLoaderConfig::default());

        // Test loading your plugin
        let result = loader.validate_plugin(&PathBuf::from("path/to/plugin.wasm")).await;
        assert!(result.is_ok());

        let manifest = result.unwrap();
        assert_eq!(manifest.plugin.id, "my-plugin");
    }
}
```

## 📤 Distribution

### Packaging
```bash
# Create a plugin package
tar -czf my-plugin-0.1.0.tar.gz \
    plugin.yaml \
    target/wasm32-wasi/release/my_plugin.wasm \
    README.md \
    examples/
```

### Publishing
1. **Create GitHub Release** with plugin package
2. **Update Plugin Registry** (if applicable)
3. **Document Installation** instructions

### Installation by Users
```bash
# Install from local file
mockforge plugin install ./my-plugin-0.1.0.tar.gz

# Install from URL
mockforge plugin install https://github.com/user/plugin/releases/download/v0.1.0/my-plugin-0.1.0.tar.gz

# Install from directory
mockforge plugin install /path/to/plugin/directory
```

## 🐛 Debugging

### Common Issues

#### 1. Compilation Errors
```bash
# Check target installation
rustup target list --installed
rustup target add wasm32-wasi

# Clean and rebuild
cargo clean
cargo build --target wasm32-wasi
```

#### 2. Plugin Not Loading
- Check `plugin.yaml` syntax
- Verify WASM target compilation
- Check plugin dependencies
- Review logs: `mockforge plugin validate /path/to/plugin`

#### 3. Runtime Errors
- Check capabilities in manifest
- Verify resource limits
- Monitor plugin logs
- Test in isolation

### Logging
```rust
use tracing::{info, error, warn};

#[async_trait::async_trait]
impl TemplatePlugin for MyPlugin {
    async fn execute_function(
        &self,
        function_name: &str,
        args: &[Value],
        context: &PluginContext,
    ) -> PluginResult<Value> {
        info!("Executing function: {} with args: {:?}", function_name, args);

        // Your logic here

        info!("Function {} completed successfully", function_name);
        PluginResult::success(result)
    }
}
```

## 🚀 Best Practices

### Performance
- Keep resource usage within limits
- Use efficient data structures
- Cache expensive operations
- Avoid blocking operations

### Security
- Validate all inputs
- Use safe deserialization
- Limit network requests
- Handle errors gracefully

### Error Handling
- Return meaningful error messages
- Use appropriate error types
- Log errors for debugging
- Fail fast on invalid inputs

### Documentation
- Document all functions and parameters
- Provide usage examples
- Include configuration options
- Explain capabilities and limitations

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/your-org/mockforge/issues)
- **Discussions**: [GitHub Discussions](https://github.com/your-org/mockforge/discussions)
- **Documentation**: [Plugin Docs](https://docs.mockforge.dev/plugins)

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Update documentation
5. Submit a pull request
