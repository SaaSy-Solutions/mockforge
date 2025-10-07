# MockForge Plugin SDK

Official SDK for developing MockForge plugins with ease.

## 🚀 Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
mockforge-plugin-sdk = "0.1"

[lib]
crate-type = ["cdylib"]
```

Create your plugin:

```rust
use mockforge_plugin_sdk::prelude::*;

#[derive(Debug)]
pub struct MyAuthPlugin;

#[async_trait]
impl AuthPlugin for MyAuthPlugin {
    async fn authenticate(
        &self,
        context: &PluginContext,
        credentials: &AuthCredentials,
    ) -> PluginResult<AuthResult> {
        // Your authentication logic
        Ok(AuthResult::authenticated("user123"))
    }
}

// Export plugin
export_plugin!(MyAuthPlugin);
```

## 📚 Features

- **Helper Macros**: Simplified plugin creation with `export_plugin!`, `plugin_config!`
- **Builder Patterns**: Easy manifest creation
- **Testing Utilities**: Mock contexts and test harnesses
- **Code Generation**: Generate boilerplate automatically
- **Type Safety**: Full Rust type system support

## 📖 Documentation

- [User Guide](https://docs.mockforge.dev/plugins/sdk)
- [API Reference](https://docs.rs/mockforge-plugin-sdk)
- [Examples](../../examples/plugins/)

## 🔧 Building

```bash
cargo build --target wasm32-wasi --release
```

## 📄 License

MIT OR Apache-2.0
