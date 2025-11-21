# Plugin Starter Guide

This guide walks you through creating your first MockForge plugin from scratch. You'll learn how to scaffold a plugin project, implement plugin functionality, build it, and use it in MockForge.

## Overview

MockForge plugins extend the platform's capabilities through a WebAssembly-based plugin system. Plugins can:
- Add custom template functions
- Implement custom authentication logic
- Generate dynamic responses
- Connect to external data sources
- Trigger webhooks
- Add chaos engineering patterns

## Plugin Types

MockForge supports several plugin types:

| Type | Description | Use Case |
|------|-------------|----------|
| **template** | Custom template functions | Add domain-specific data generation functions |
| **auth** | Authentication handlers | Custom authentication logic (JWT, OAuth, etc.) |
| **response** | Response generators | Generate dynamic responses based on request data |
| **datasource** | Data source connectors | Connect to external data (CSV, databases, APIs) |
| **webhook** | Webhook triggers | Send outbound HTTP requests to external services |
| **chaos** | Chaos patterns | Add custom failure modes and latency patterns |

## Step 1: Scaffold Your Plugin

Use the `mockforge plugin init` command to create a new plugin project:

```bash
mockforge plugin init my-custom-plugin --plugin-type template
```

**Options:**
- `--plugin-type` (default: `template`): The type of plugin to create
- `--output` (optional): Output directory (defaults to plugin name)
- `--force`: Overwrite existing directory

**Example for different types:**

```bash
# Template plugin
mockforge plugin init my-template-plugin --plugin-type template

# Authentication plugin
mockforge plugin init my-auth-plugin --plugin-type auth

# Response plugin
mockforge plugin init my-response-plugin --plugin-type response

# Data source plugin
mockforge plugin init my-datasource-plugin --plugin-type datasource

# Webhook plugin
mockforge plugin init my-webhook-plugin --plugin-type webhook

# Chaos plugin
mockforge plugin init my-chaos-plugin --plugin-type chaos
```

### What Gets Created

The command creates a complete plugin project structure:

```
my-custom-plugin/
├── Cargo.toml          # Rust dependencies and build config
├── plugin.yaml         # Plugin manifest with metadata
├── src/
│   └── lib.rs         # Plugin implementation
├── README.md          # Plugin documentation template
└── .gitignore        # Git ignore file
```

## Step 2: Understand the Plugin Structure

### Cargo.toml

The `Cargo.toml` file defines your plugin's dependencies:

```toml
[package]
name = "my-custom-plugin"
version = "0.1.0"
edition = "2021"

[dependencies]
mockforge-plugin-core = { path = "../../../crates/mockforge-plugin-core" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
async-trait = "0.1"
```

### plugin.yaml

The `plugin.yaml` file is the plugin manifest:

```yaml
id: my-custom-plugin
name: My Custom Plugin
version: 0.1.0
description: Description of what this plugin does
author: Your Name
license: MIT OR Apache-2.0

type: template  # template, auth, response, datasource, webhook, chaos

capabilities:
  network:
    allow_http: false
    allowed_hosts: []
  filesystem:
    read_paths: []
    write_paths: []
  resources:
    max_memory_bytes: 10485760  # 10MB
    max_cpu_percent: 50
    max_execution_time_ms: 1000

config_schema:
  type: object
  properties:
    # Plugin-specific configuration
```

### src/lib.rs

The `src/lib.rs` file contains your plugin implementation. The scaffolded code provides a template based on your plugin type.

## Step 3: Implement Your Plugin

### Template Plugin Example

Template plugins add custom functions to MockForge's templating system:

```rust
use mockforge_plugin_core::{
    TemplatePlugin, TemplatePluginConfig, TemplateFunction, 
    FunctionParameter, PluginContext, PluginResult, Result, Value
};
use std::collections::HashMap;

pub struct MyTemplatePlugin {
    config: TemplatePluginConfig,
}

#[async_trait::async_trait]
impl TemplatePlugin for MyTemplatePlugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            network: NetworkPermissions {
                allow_http: false,
                allowed_hosts: vec![],
                max_connections: 10,
            },
            filesystem: FilesystemPermissions {
                read_paths: vec![],
                write_paths: vec![],
                allow_temp_files: false,
            },
            resources: ResourceLimits {
                max_memory_bytes: 10 * 1024 * 1024, // 10MB
                max_cpu_percent: 50,
                max_execution_time_ms: 1000,
                max_concurrent_executions: 5,
            },
            custom: HashMap::new(),
        }
    }

    async fn initialize(&self, _config: &TemplatePluginConfig) -> Result<()> {
        Ok(())
    }

    async fn register_functions(
        &self,
        _context: &PluginContext,
        _config: &TemplatePluginConfig,
    ) -> Result<PluginResult<HashMap<String, TemplateFunction>>> {
        let mut functions = HashMap::new();

        // Register a custom function
        functions.insert(
            "my_custom_function".to_string(),
            TemplateFunction::new(
                "my_custom_function",
                "Does something custom",
                "string"
            )
            .with_parameter(
                FunctionParameter::required("input", "string", "Input value")
            )
            .with_example("{{my_custom_function \"hello\"}}")
            .with_category("custom"),
        );

        Ok(PluginResult::success(functions, 0))
    }

    async fn execute_function(
        &self,
        _context: &PluginContext,
        function_name: &str,
        args: &[Value],
        _config: &TemplatePluginConfig,
    ) -> Result<PluginResult<Value>> {
        match function_name {
            "my_custom_function" => {
                let input = args.get(0)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| PluginError::execution("Missing input argument"))?;
                
                let result = format!("Processed: {}", input);
                Ok(PluginResult::success(Value::String(result), 0))
            }
            _ => Ok(PluginResult::failure(
                format!("Unknown function: {}", function_name),
                0
            )),
        }
    }

    // ... other required methods
}
```

### Response Plugin Example

Response plugins generate custom responses based on request data:

```rust
use mockforge_plugin_core::{
    ResponsePlugin, ResponsePluginConfig, ResponseRequest,
    ResponseData, PluginContext, PluginResult, Result
};
use std::collections::HashMap;

pub struct MyResponsePlugin {
    config: ResponsePluginConfig,
}

#[async_trait::async_trait]
impl ResponsePlugin for MyResponsePlugin {
    fn capabilities(&self) -> PluginCapabilities {
        // Define capabilities
    }

    async fn initialize(&self, _config: &ResponsePluginConfig) -> Result<()> {
        Ok(())
    }

    async fn can_handle(
        &self,
        _context: &PluginContext,
        request: &ResponseRequest,
        _config: &ResponsePluginConfig,
    ) -> Result<PluginResult<bool>> {
        // Determine if this plugin should handle the request
        let should_handle = request.path.starts_with("/api/custom");
        Ok(PluginResult::success(should_handle, 0))
    }

    async fn generate_response(
        &self,
        _context: &PluginContext,
        request: &ResponseRequest,
        _config: &ResponsePluginConfig,
    ) -> Result<PluginResult<ResponseData>> {
        // Generate custom response
        let body = json!({
            "message": "Custom response",
            "path": request.path,
            "method": request.method.as_str(),
        });

        let response_data = ResponseData {
            status_code: 200,
            headers: HashMap::new(),
            body: serde_json::to_vec(&body)?,
            content_type: "application/json".to_string(),
            metadata: HashMap::new(),
            cache_control: None,
            custom: HashMap::new(),
        };

        Ok(PluginResult::success(response_data, 0))
    }

    // ... other required methods
}
```

### Webhook Plugin Example

Webhook plugins make outbound HTTP requests:

```rust
use mockforge_plugin_core::{
    ResponsePlugin, ResponsePluginConfig, ResponseRequest,
    ResponseData, PluginContext, PluginResult, Result
};

pub struct MyWebhookPlugin {
    config: ResponsePluginConfig,
    webhook_url: String,
}

#[async_trait::async_trait]
impl ResponsePlugin for MyWebhookPlugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            network: NetworkPermissions {
                allow_http: true,  // Enable network access
                allowed_hosts: vec![
                    self.webhook_url
                        .strip_prefix("https://")
                        .or_else(|| self.webhook_url.strip_prefix("http://"))
                        .and_then(|url| url.split('/').next())
                        .unwrap_or("*")
                        .to_string()
                ],
                max_connections: 10,
            },
            // ... other capabilities
        }
    }

    async fn generate_response(
        &self,
        _context: &PluginContext,
        request: &ResponseRequest,
        _config: &ResponsePluginConfig,
    ) -> Result<PluginResult<ResponseData>> {
        // Generate webhook payload
        let payload = json!({
            "event": "mockforge.request",
            "path": request.path,
            "method": request.method.as_str(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        // In a real implementation, make HTTP request here
        // For security, this requires network capabilities

        let response_data = ResponseData {
            status_code: 200,
            headers: HashMap::new(),
            body: serde_json::to_vec(&payload)?,
            content_type: "application/json".to_string(),
            metadata: HashMap::new(),
            cache_control: None,
            custom: HashMap::new(),
        };

        Ok(PluginResult::success(response_data, 0))
    }

    // ... other required methods
}
```

## Step 4: Build Your Plugin

Build your plugin for WebAssembly:

```bash
cd my-custom-plugin
cargo build --target wasm32-wasi --release
```

The compiled `.wasm` file will be in:
```
target/wasm32-wasi/release/my_custom_plugin.wasm
```

## Step 5: Test Your Plugin

### Unit Tests

Add tests to your plugin:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_my_custom_function() {
        let plugin = MyTemplatePlugin::new();
        let config = TemplatePluginConfig::default();
        
        plugin.initialize(&config).await.unwrap();
        
        let functions = plugin.register_functions(
            &PluginContext::default(),
            &config
        ).await.unwrap();
        
        assert!(functions.result.contains_key("my_custom_function"));
    }
}
```

Run tests:

```bash
cargo test
```

### Integration Testing

Test your plugin with MockForge:

1. **Install the plugin:**
   ```bash
   mockforge plugin install ./my-custom-plugin
   ```

2. **List installed plugins:**
   ```bash
   mockforge plugin list
   ```

3. **Use in mockforge.yaml:**
   ```yaml
   plugins:
     - id: my-custom-plugin
       type: template
       config:
         # Plugin-specific config
   ```

4. **Start MockForge:**
   ```bash
   mockforge serve
   ```

5. **Test the plugin:**
   ```bash
   # For template plugins, use in templates
   curl http://localhost:3000/api/test
   ```

## Step 6: Learn from Examples

The MockForge repository includes comprehensive example plugins you can learn from:

### Template Plugins

**`template-advanced/`** - Advanced template functions:
- Mathematical operations (sum, average)
- Collection operations (group_by, sort)
- Date/time formatting
- UUID generation

**Inspect:**
```bash
cd examples/plugins/template-advanced/
cat src/lib.rs
cat README.md
```

**`template-custom/`** - Domain-specific functions:
- Business data generation
- Custom formatting
- Domain-specific helpers

### Response Plugins

**`webhook-example/`** - Webhook functionality:
- Outbound HTTP requests
- Payload signing
- Event-based triggering

**Inspect:**
```bash
cd examples/plugins/webhook-example/
cat src/lib.rs
cat plugin.yaml
```

### Authentication Plugins

**`auth-basic/`** - HTTP Basic Authentication:
- Credential validation
- Realm configuration
- Secure password handling

**Inspect:**
```bash
cd examples/plugins/auth-basic/
cat src/lib.rs
```

### Data Source Plugins

**`datasource-csv/`** - CSV data source:
- CSV file parsing
- Data querying
- Type inference

**Inspect:**
```bash
cd examples/plugins/datasource-csv/
cat src/lib.rs
```

## Complete Example: Creating a Template Plugin

Let's create a complete template plugin step-by-step:

### 1. Scaffold the Plugin

```bash
mockforge plugin init currency-formatter --plugin-type template
cd currency-formatter
```

### 2. Implement the Plugin

Edit `src/lib.rs`:

```rust
use mockforge_plugin_core::{
    TemplatePlugin, TemplatePluginConfig, TemplateFunction,
    FunctionParameter, PluginContext, PluginResult, Result, Value
};
use std::collections::HashMap;

pub struct CurrencyFormatterPlugin {
    config: TemplatePluginConfig,
}

#[async_trait::async_trait]
impl TemplatePlugin for CurrencyFormatterPlugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            network: NetworkPermissions {
                allow_http: false,
                allowed_hosts: vec![],
                max_connections: 10,
            },
            filesystem: FilesystemPermissions {
                read_paths: vec![],
                write_paths: vec![],
                allow_temp_files: false,
            },
            resources: ResourceLimits {
                max_memory_bytes: 5 * 1024 * 1024, // 5MB
                max_cpu_percent: 50,
                max_execution_time_ms: 500,
                max_concurrent_executions: 5,
            },
            custom: HashMap::new(),
        }
    }

    async fn initialize(&self, _config: &TemplatePluginConfig) -> Result<()> {
        Ok(())
    }

    async fn register_functions(
        &self,
        _context: &PluginContext,
        _config: &TemplatePluginConfig,
    ) -> Result<PluginResult<HashMap<String, TemplateFunction>>> {
        let mut functions = HashMap::new();

        functions.insert(
            "format_currency".to_string(),
            TemplateFunction::new(
                "format_currency",
                "Format a number as currency",
                "string"
            )
            .with_parameter(
                FunctionParameter::required("amount", "number", "Amount to format")
            )
            .with_parameter(
                FunctionParameter::optional("currency", "string", "Currency code (default: USD)")
            )
            .with_example("{{format_currency 1234.56 \"USD\"}}")
            .with_category("formatting"),
        );

        Ok(PluginResult::success(functions, 0))
    }

    async fn execute_function(
        &self,
        _context: &PluginContext,
        function_name: &str,
        args: &[Value],
        _config: &TemplatePluginConfig,
    ) -> Result<PluginResult<Value>> {
        match function_name {
            "format_currency" => {
                let amount = args.get(0)
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| PluginError::execution("Amount must be a number"))?;
                
                let currency = args.get(1)
                    .and_then(|v| v.as_str())
                    .unwrap_or("USD");
                
                let formatted = match currency {
                    "USD" => format!("${:.2}", amount),
                    "EUR" => format!("€{:.2}", amount),
                    "GBP" => format!("£{:.2}", amount),
                    _ => format!("{:.2} {}", amount, currency),
                };
                
                Ok(PluginResult::success(Value::String(formatted), 0))
            }
            _ => Ok(PluginResult::failure(
                format!("Unknown function: {}", function_name),
                0
            )),
        }
    }

    async fn get_data_source(
        &self,
        _context: &PluginContext,
        _data_source: &str,
        _config: &TemplatePluginConfig,
    ) -> Result<PluginResult<Value>> {
        Ok(PluginResult::failure("No data sources available".to_string(), 0))
    }

    fn validate_config(&self, _config: &TemplatePluginConfig) -> Result<()> {
        Ok(())
    }

    fn available_data_sources(&self) -> Vec<String> {
        vec![]
    }

    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }
}

// Export the plugin
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn TemplatePlugin {
    Box::into_raw(Box::new(CurrencyFormatterPlugin {
        config: TemplatePluginConfig::default(),
    }))
}

#[no_mangle]
pub extern "C" fn destroy_plugin(plugin: *mut dyn TemplatePlugin) {
    unsafe {
        drop(Box::from_raw(plugin));
    }
}
```

### 3. Update plugin.yaml

```yaml
id: currency-formatter
name: Currency Formatter
version: 0.1.0
description: Format numbers as currency strings
author: Your Name
license: MIT OR Apache-2.0

type: template

capabilities:
  network:
    allow_http: false
  filesystem:
    read_paths: []
    write_paths: []
  resources:
    max_memory_bytes: 5242880  # 5MB
    max_cpu_percent: 50
    max_execution_time_ms: 500
```

### 4. Build and Test

```bash
# Build
cargo build --target wasm32-wasi --release

# Test
cargo test

# Install
mockforge plugin install .

# Use in mockforge.yaml
```

### 5. Use in Templates

```yaml
# mockforge.yaml
responses:
  - path: /api/products
    method: GET
    body: |
      {
        "products": [
          {
            "name": "Product 1",
            "price": "{{format_currency 29.99 \"USD\"}}"
          }
        ]
      }
```

## Best Practices

### 1. Error Handling

Always handle errors gracefully:

```rust
async fn execute_function(
    &self,
    _context: &PluginContext,
    function_name: &str,
    args: &[Value],
    _config: &TemplatePluginConfig,
) -> Result<PluginResult<Value>> {
    match function_name {
        "my_function" => {
            let input = args.get(0)
                .and_then(|v| v.as_str())
                .ok_or_else(|| PluginError::execution("Missing required argument"))?;
            
            // Your logic here
            Ok(PluginResult::success(Value::String(result), 0))
        }
        _ => Ok(PluginResult::failure(
            format!("Unknown function: {}", function_name),
            0
        )),
    }
}
```

### 2. Resource Limits

Set appropriate resource limits:

```rust
resources: ResourceLimits {
    max_memory_bytes: 10 * 1024 * 1024,  // 10MB
    max_cpu_percent: 50,
    max_execution_time_ms: 1000,  // 1 second
    max_concurrent_executions: 5,
}
```

### 3. Security

- Only request necessary capabilities
- Validate all inputs
- Sanitize outputs
- Use allowed_hosts for network access

### 4. Documentation

- Document all functions in `register_functions`
- Provide examples in function metadata
- Include usage examples in README.md

## Troubleshooting

### Plugin Won't Build

**Issue:** Compilation errors

**Solution:**
- Check that all dependencies are in `Cargo.toml`
- Ensure you're using the correct Rust edition (2021)
- Verify `mockforge-plugin-core` path is correct

### Plugin Not Loading

**Issue:** Plugin fails to load in MockForge

**Solution:**
- Check `plugin.yaml` syntax
- Verify plugin type matches implementation
- Check capability requirements
- Review MockForge logs for errors

### Function Not Found

**Issue:** Template function not available

**Solution:**
- Ensure function is registered in `register_functions`
- Check function name matches exactly
- Verify plugin is installed and enabled
- Check plugin configuration

## Next Steps

1. **Explore Example Plugins**: Study the example plugins in `examples/plugins/`
2. **Read Plugin API Docs**: See `crates/mockforge-plugin-core/src/` for trait definitions
3. **Join the Community**: Share your plugins and get feedback
4. **Contribute**: Submit your plugins to the marketplace

## Related Documentation

- [Plugin Development Guide](../user-guide/plugins.md) - Detailed plugin development
- [Plugin API Reference](../../crates/mockforge-plugin-core/README.md) - Complete API documentation
- [Example Plugins](../../examples/plugins/README.md) - All available examples
- [Plugin Marketplace](../../plugin-marketplace/README.md) - Share and discover plugins

