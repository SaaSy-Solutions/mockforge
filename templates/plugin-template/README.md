# {{plugin_title}}

{{plugin_description}}

## Building

Build the plugin as a WebAssembly module:

```bash
cargo build --target wasm32-wasi --release
```

The compiled plugin will be available at:
```
target/wasm32-wasi/release/mockforge_plugin_{{plugin_name | snake_case}}.wasm
```

## Installing

Install the plugin into MockForge:

```bash
mockforge plugin install target/wasm32-wasi/release/mockforge_plugin_{{plugin_name | snake_case}}.wasm
```

Or from the project directory:

```bash
mockforge plugin install .
```

## Testing

Run the plugin tests:

```bash
cargo test
```

## Configuration

The plugin accepts the following configuration in `plugin.yaml`:

```yaml
configuration:
  example_setting: "value"
```

## Usage

{% if plugin_type == "auth" %}
This authentication plugin can be used in MockForge routes:

```yaml
routes:
  - path: /api/secure
    method: GET
    auth:
      type: plugin
      plugin_id: "{{plugin_name}}"
      config:
        example_setting: "value"
```
{% elsif plugin_type == "template" %}
This template plugin provides the following functions:

- `example_function(input)` - An example template function

Use it in response templates:

```yaml
routes:
  - path: /api/data
    response:
      body: |
        {
          "result": "{% raw %}{{ example_function("test") }}{% endraw %}"
        }
```
{% elsif plugin_type == "response" %}
This response plugin can generate dynamic responses:

```yaml
routes:
  - path: /api/dynamic
    response:
      plugin_id: "{{plugin_name}}"
      config:
        example_setting: "value"
```
{% elsif plugin_type == "datasource" %}
This data source plugin can be used to query data:

```yaml
data_sources:
  - id: my_data
    plugin_id: "{{plugin_name}}"
    config:
      example_setting: "value"

routes:
  - path: /api/data
    response:
      body: |
        {
          "data": "{% raw %}{{ query('SELECT * FROM example_table') }}{% endraw %}"
        }
```
{% endif %}

## Development

To modify this plugin:

1. Edit the implementation in `src/lib.rs`
2. Update the configuration schema in `plugin.yaml`
3. Build and test your changes
4. Publish to the MockForge plugin registry (optional)

## Publishing

To publish this plugin to the MockForge registry:

```bash
# Login to the registry
mockforge plugin registry login

# Publish
mockforge plugin registry publish
```

## License

This plugin is licensed under {{license | default: "MIT OR Apache-2.0"}}.
