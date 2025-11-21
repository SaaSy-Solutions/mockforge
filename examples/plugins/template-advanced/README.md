# Advanced Template Functions Plugin

A MockForge template plugin that provides advanced template functions for data transformations, aggregations, and complex data generation.

## Overview

This plugin extends MockForge's templating system with advanced functions for:
- Mathematical operations and aggregations
- Collection operations (grouping, sorting, filtering)
- Date/time manipulation and formatting
- UUID and random number generation
- JSON data transformations

## Features

### Mathematical Functions
- **`sum`**: Calculate sum of numbers
- **`average`**: Calculate average of numbers

### Collection Functions
- **`group_by`**: Group array elements by a key
- **`sort`**: Sort arrays (with optional key)

### Date/Time Functions
- **`format_date`**: Format timestamps as date strings

### Generator Functions
- **`uuid`**: Generate UUID v4
- **`random_int`**: Generate random integers in a range

## Installation

```bash
# Build the plugin
cd examples/plugins/template-advanced
cargo build --target wasm32-wasi --release

# Install the plugin
mockforge plugin install .
```

## Configuration

```yaml
# In your mockforge.yaml
plugins:
  - id: template-advanced
    config:
      enable_math: true
      enable_collections: true
      enable_datetime: true
      locale: "en-US"
```

## Usage Examples

### Mathematical Functions

```yaml
# Sum of numbers
response:
  body: |
    {
      "total": {{sum 10 20 30 40}},
      "average": {{average 10 20 30}}
    }
```

### Collection Operations

```yaml
# Group users by role
response:
  body: |
    {
      "grouped": {{group_by users "role"}}
    }

# Sort items by price
response:
  body: |
    {
      "sorted": {{sort items "price"}}
    }
```

### Date Formatting

```yaml
# Format timestamp
response:
  body: |
    {
      "date": "{{format_date 1640995200 \"%Y-%m-%d\"}}",
      "datetime": "{{format_date 1640995200 \"%Y-%m-%d %H:%M:%S\"}}"
    }
```

### Generators

```yaml
# Generate UUID
response:
  body: |
    {
      "id": "{{uuid}}",
      "random": {{random_int 1 100}}
    }
```

## Available Functions

| Function | Description | Arguments | Example |
|----------|-------------|-----------|---------|
| `sum` | Sum of numbers | `...numbers` | `{{sum 1 2 3}}` |
| `average` | Average of numbers | `...numbers` | `{{average 10 20 30}}` |
| `format_date` | Format timestamp | `timestamp, format?` | `{{format_date 1640995200}}` |
| `group_by` | Group array by key | `array, key` | `{{group_by users "role"}}` |
| `sort` | Sort array | `array, key?` | `{{sort items "price"}}` |
| `uuid` | Generate UUID | none | `{{uuid}}` |
| `random_int` | Random integer | `min?, max?` | `{{random_int 1 100}}` |

## Function Categories

Functions are organized into categories:

- **math**: Mathematical operations (`sum`, `average`)
- **collection**: Array/object operations (`group_by`, `sort`)
- **datetime**: Date/time functions (`format_date`)
- **generator**: Data generation (`uuid`, `random_int`)

## Development

### Building

```bash
cargo build --target wasm32-wasi --release
```

### Testing

```bash
cargo test
```

### Code Structure

- `src/lib.rs`: Main plugin implementation
  - `AdvancedTemplateConfig`: Plugin configuration
  - `AdvancedTemplatePlugin`: Plugin implementation
  - Mathematical functions: `sum()`, `average()`
  - Collection functions: `group_by()`, `sort()`
  - Date functions: `format_date()`
  - Generator functions: `uuid()`, `random_int()`

## Performance Considerations

- Mathematical functions are optimized for small to medium datasets
- Collection operations (`group_by`, `sort`) work best with arrays < 1000 items
- UUID generation is fast and suitable for high-frequency use
- Date formatting uses chrono and supports standard strftime formats

## Limitations

- Collection operations are in-memory only (no streaming)
- Date formatting supports standard formats only
- Mathematical functions work with numbers only (no complex types)
- No support for custom comparators in sorting (uses string comparison)

## See Also

- [Template Plugin API](../../../docs/plugins/README.md)
- [Template Custom Plugin](../template-custom/README.md) - Basic template functions
- [Plugin Development Guide](../../../docs/plugins/development-guide.md)

## License

MIT OR Apache-2.0
