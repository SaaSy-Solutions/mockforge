# Using the MockForge Plugin Template

This template helps you quickly create a new MockForge plugin using `cargo-generate`.

## Prerequisites

Install `cargo-generate`:

```bash
cargo install cargo-generate
```

Ensure you have the `wasm32-wasi` target installed:

```bash
rustup target add wasm32-wasi
```

## Creating a New Plugin

Use `cargo-generate` to create a new plugin from this template:

```bash
cargo generate --git https://github.com/mockforge/mockforge \
  --name my-plugin-name \
  templates/plugin-template
```

Or if you have the MockForge repository cloned locally:

```bash
cargo generate --path /path/to/mockforge/templates/plugin-template \
  --name my-plugin-name
```

You will be prompted for:

- **Plugin name**: Lowercase with hyphens (e.g., `my-auth-plugin`)
- **Plugin title**: Human-readable name (e.g., `My Auth Plugin`)
- **Plugin description**: What your plugin does
- **Plugin type**: Choose from `auth`, `template`, `response`, or `datasource`
- **Author name**: Your name
- **Author email**: Your email address
- **Maximum memory**: Memory limit in MB (default: 10)
- **Maximum CPU time**: CPU time limit in milliseconds (default: 1000)
- **Allow network**: Whether the plugin needs network access (default: false)
- **Allow filesystem**: Whether the plugin needs filesystem access (default: false)

## After Generation

1. Navigate to your new plugin directory:
   ```bash
   cd my-plugin-name
   ```

2. Implement your plugin logic in `src/lib.rs`

3. Update `plugin.yaml` with your specific configuration schema

4. Build the plugin:
   ```bash
   cargo build --target wasm32-wasi --release
   ```

5. Test the plugin:
   ```bash
   cargo test
   ```

6. Install locally for testing:
   ```bash
   mockforge plugin install .
   ```

## Plugin Types

### Authentication Plugin (`auth`)
Implements custom authentication logic. Perfect for:
- OAuth/OAuth2 flows
- SAML authentication
- Custom token validation
- API key verification

### Template Plugin (`template`)
Adds custom template functions for dynamic content. Perfect for:
- Custom data transformations
- Encryption/decryption functions
- Custom formatters
- Complex calculations

### Response Plugin (`response`)
Generates dynamic responses based on request context. Perfect for:
- GraphQL response generation
- Complex data generation
- External API integration
- ML-based response generation

### Data Source Plugin (`datasource`)
Connects to external data sources. Perfect for:
- Database connections
- File-based data (CSV, JSON, XML)
- External API data fetching
- Cache systems

## Next Steps

- Read the [Plugin Development Guide](https://docs.mockforge.dev/plugins/development)
- Check out [example plugins](https://github.com/mockforge/mockforge/tree/main/examples/plugins)
- Join the [MockForge community](https://github.com/mockforge/mockforge/discussions)
- Publish to the [Plugin Registry](https://registry.mockforge.dev)
