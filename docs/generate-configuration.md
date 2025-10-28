# MockForge Generate Configuration

MockForge supports configuration files for customizing the mock generation process. This document describes the configuration schema, file formats, and usage examples.

## Overview

The `mockforge generate` command uses configuration files to specify:
- Input OpenAPI specifications
- Output paths and filenames
- Plugins to use
- Generation options (client libraries, runtime, etc.)

## Configuration File Discovery

MockForge automatically discovers configuration files in the current directory with the following priority:

1. `mockforge.toml` (recommended)
2. `mockforge.json`
3. `mockforge.yaml` or `mockforge.yml`
4. `.mockforge.toml`
5. `.mockforge.json`
6. `.mockforge.yaml` or `.mockforge.yml`

## Configuration Precedence

Configuration values are merged in the following order (highest to lowest precedence):

1. **CLI arguments** (highest precedence)
2. **Configuration file**
3. **Environment variables**
4. **Default values** (lowest precedence)

## File Formats

### TOML (Recommended)

```toml
[input]
spec = "openapi.json"

[output]
path = "./generated"
filename = "mock_server.rs"
clean = false

[plugins]
# Simple plugin
"my-plugin" = "package-name"

# Advanced plugin with options
"advanced" = { package = "advanced-plugin", options = { key = "value" } }

[options]
client = "reqwest"
mode = "tags"
include-validation = true
include-examples = true
runtime = "tokio"
```

### JSON

```json
{
  "input": {
    "spec": "openapi.json"
  },
  "output": {
    "path": "./generated",
    "filename": "mock_server.rs",
    "clean": false
  },
  "plugins": {
    "my-plugin": "package-name"
  },
  "options": {
    "client": "reqwest",
    "mode": "tags",
    "include-validation": true,
    "include-examples": true,
    "runtime": "tokio"
  }
}
```

### YAML

```yaml
input:
  spec: openapi.json

output:
  path: ./generated
  filename: mock_server.rs
  clean: false

plugins:
  my-plugin: package-name

options:
  client: reqwest
  mode: tags
  include-validation: true
  include-examples: true
  runtime: tokio
```

## Configuration Schema

### Input Configuration (`input`)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `spec` | `PathBuf` | Yes | Path to OpenAPI specification file |
| `additional` | `Vec<PathBuf>` | No | Additional input files |

### Output Configuration (`output`)

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `path` | `PathBuf` | No | `./generated` | Output directory path |
| `filename` | `String` | No | `generated_mock.rs` | Output file name |
| `clean` | `bool` | No | `false` | Clean output directory before generation |

### Plugin Configuration (`plugins`)

Plugins can be configured in two ways:

**Simple plugin:**
```toml
"plugin-name" = "package-name"
```

**Advanced plugin:**
```toml
"plugin-name" = { package = "package-name", options = { key = "value" } }
```

### Generation Options (`options`)

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `client` | `String` | No | `"reqwest"` | Client library (reqwest, ureq, surf) |
| `mode` | `String` | No | `"tags"` | Generation mode (operations, tags, paths) |
| `include-validation` | `bool` | No | `true` | Include validation in generated code |
| `include-examples` | `bool` | No | `true` | Include examples in responses |
| `runtime` | `String` | No | `"tokio"` | Target runtime (tokio, async-std, sync) |

## Usage Examples

### Basic Usage

Create a `mockforge.toml` file:

```toml
[input]
spec = "api/openapi.json"

[output]
path = "./generated"
```

Run generation:

```bash
mockforge generate
```

### With CLI Overrides

```bash
mockforge generate --spec custom-openapi.yaml --output ./custom-output
```

### Verbose Mode

```bash
mockforge generate --verbose
```

### Dry Run (Validate Only)

```bash
mockforge generate --dry-run
```

### Using Plugins

```toml
[input]
spec = "openapi.json"

[output]
path = "./generated"

[plugins]
"json-schema" = { package = "jsonschema", options = { strict = true } }
```

## CLI Options

The `mockforge generate` command supports the following options:

- `-c, --config <PATH>`: Path to configuration file
- `-s, --spec <PATH>`: OpenAPI specification file (overrides config)
- `-o, --output <PATH>`: Output directory (overrides config)
- `-v, --verbose`: Generate verbose output
- `--dry-run`: Validate configuration without generating

## Examples

See `examples/mockforge.example.toml`, `examples/mockforge.example.json`, and `examples/mockforge.example.yaml` for complete example configurations.

## Default Values

If no configuration file is found, MockForge uses the following defaults:

- **Input spec**: None (must be provided via `--spec` flag or config)
- **Output path**: `./generated`
- **Output filename**: `generated_mock.rs`
- **Clean**: `false`
- **Plugins**: Empty
- **Options**:
  - **Client**: `reqwest`
  - **Mode**: `tags`
  - **Include validation**: `true`
  - **Include examples**: `true`
  - **Runtime**: `tokio`

## Troubleshooting

### Error: "No configuration file found"

Create a `mockforge.toml`, `mockforge.json`, or `mockforge.yaml` file in your project root, or provide the `--spec` flag.

### Error: "Specification file not found"

Ensure the path to your OpenAPI specification is correct and the file exists.

### Error: "Failed to parse config file"

Check that your configuration file is valid JSON, TOML, or YAML format.

## Further Reading

- [MockForge Configuration Reference](../book/src/api/cli.md)
- [OpenAPI Support](../book/src/user-guide/http-mocking.md)
- [Plugin Development](../plugin-marketplace/README.md)
