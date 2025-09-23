# MockForge Example Plugins

This directory contains example plugins that demonstrate the MockForge plugin system capabilities. Each plugin showcases different aspects of the plugin architecture.

## Plugin Types

### 🔐 Authentication Plugins (`auth-basic/`)
**Basic HTTP Authentication Plugin**

Provides HTTP Basic Authentication support for protecting API endpoints with username/password authentication.

**Features:**
- HTTP Basic Authentication validation
- Configurable user credentials
- Custom authentication realm
- Secure password handling

**Configuration:**
```yaml
realm: "MyAPI"
users:
  - username: "admin"
    password: "secret123"
  - username: "user"
    password: "password"
```

**Usage:**
```bash
mockforge plugin install examples/plugins/auth-basic/
```

### 📝 Template Plugins (`template-custom/`)
**Custom Template Functions Plugin**

Extends MockForge's templating system with custom functions for generating domain-specific mock data.

**Features:**
- Business domain data generation (orders, customers, products)
- Custom formatting functions
- Dynamic content generation
- Template helper functions

**Available Functions:**
- `{{order_id()}}` - Generate random order IDs
- `{{customer_name()}}` - Generate customer names
- `{{product_name()}}` - Generate product names based on domain
- `{{currency(amount, currency)}}` - Format currency
- `{{business_status()}}` - Generate business statuses
- `{{domain_data(type)}}` - Generate complete domain objects

**Configuration:**
```yaml
business_domain: "ecommerce"
enable_advanced_functions: true
```

**Usage:**
```bash
mockforge plugin install examples/plugins/template-custom/
```

### 🔄 Response Plugins (`response-graphql/`)
**GraphQL Response Generator Plugin**

Automatically generates mock GraphQL responses by analyzing GraphQL queries and generating appropriate mock data.

**Features:**
- GraphQL query parsing and field analysis
- Type-aware mock data generation
- Support for nested queries and fragments
- Configurable data complexity levels

**Configuration:**
```yaml
schema_file: "schema.graphql"  # Optional
enable_introspection: true
mock_data_complexity: "medium"  # simple, medium, complex
```

**Usage:**
```bash
mockforge plugin install examples/plugins/response-graphql/
```

### 📊 Data Source Plugins (`datasource-csv/`)
**CSV Data Source Plugin**

Provides access to CSV files as mock data sources, allowing you to load and query CSV data for responses.

**Features:**
- CSV file parsing and loading
- Data querying with filtering and pagination
- Multiple CSV datasets support
- Type inference for CSV columns

**Configuration:**
```yaml
csv_files:
  - name: "users"
    path: "data/users.csv"
    has_headers: true
  - name: "products"
    path: "data/products.csv"
    has_headers: true
cache_enabled: true
max_rows_per_query: 1000
```

**Usage:**
```bash
mockforge plugin install examples/plugins/datasource-csv/
```

## Building Plugins

Each plugin can be built as a WebAssembly module for production use:

```bash
# Build a plugin for WebAssembly
cd examples/plugins/auth-basic/
cargo build --target wasm32-wasi --release

# The .wasm file will be in target/wasm32-wasi/release/
```

## Testing Plugins

Run the plugin tests:

```bash
cd examples/plugins/auth-basic/
cargo test
```

## Plugin Development

### Plugin Structure

Each plugin consists of:

1. **`Cargo.toml`** - Rust dependencies and build configuration
2. **`plugin.yaml`** - Plugin manifest with metadata and capabilities
3. **`src/lib.rs`** - Main plugin implementation

### Required Functions

Each plugin must export these C functions:

- `create_*_plugin()` - Factory function to create plugin instances
- `destroy_*_plugin()` - Cleanup function to destroy plugin instances

### Plugin Traits

Implement the appropriate trait for your plugin type:

- `AuthPlugin` - For authentication plugins
- `TemplatePlugin` - For template function plugins
- `ResponsePlugin` - For response generation plugins
- `DataSourcePlugin` - For data source plugins

### Security Considerations

- Plugins run in WebAssembly sandbox with restricted capabilities
- File system and network access must be explicitly granted
- Resource limits (memory, CPU) are enforced
- All plugin capabilities are validated before loading

## CLI Usage

```bash
# List installed plugins
mockforge plugin list

# Show plugin details
mockforge plugin show auth-basic

# Install a plugin
mockforge plugin install /path/to/plugin/

# Validate plugin without installing
mockforge plugin validate /path/to/plugin/

# Reload plugins
mockforge plugin reload

# Remove a plugin
mockforge plugin remove auth-basic

# Show plugin status
mockforge plugin status

# Clean plugin cache
mockforge plugin clean
```

## Development Workflow

1. Create plugin directory structure
2. Implement plugin trait in `src/lib.rs`
3. Configure plugin in `plugin.yaml`
4. Add dependencies to `Cargo.toml`
5. Test plugin functionality with `cargo test`
6. Build WebAssembly module for production
7. Install and test with MockForge CLI

## Contributing

When creating new example plugins:

1. Follow the established directory structure
2. Include comprehensive documentation
3. Add configuration examples
4. Provide usage examples
5. Include unit tests
6. Update this README

## License

All example plugins are licensed under MIT OR Apache-2.0.
