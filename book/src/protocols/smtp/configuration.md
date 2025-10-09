# SMTP Configuration Reference

This page provides comprehensive documentation for all SMTP configuration options in MockForge.

## Configuration File

Configuration can be provided via YAML or JSON files:

```yaml
# config.yaml
smtp:
  # Server settings
  enabled: true
  port: 1025
  host: "0.0.0.0"
  hostname: "mockforge-smtp"

  # Connection settings
  timeout_secs: 30
  max_connections: 100

  # Mailbox settings
  enable_mailbox: true
  max_mailbox_messages: 1000

  # Fixtures
  fixtures_dir: "./fixtures/smtp"
```

## Configuration Options

### Server Settings

#### `enabled`

- **Type**: `boolean`
- **Default**: `false`
- **Description**: Enable or disable the SMTP server

```yaml
smtp:
  enabled: true
```

#### `port`

- **Type**: `integer`
- **Default**: `1025`
- **Description**: Port number for the SMTP server to listen on
- **Notes**:
  - Standard SMTP port is 25, but requires root/admin privileges
  - Common development ports: 1025, 2525, 5025
  - Must be between 1 and 65535

```yaml
smtp:
  port: 1025
```

#### `host`

- **Type**: `string`
- **Default**: `"0.0.0.0"`
- **Description**: IP address to bind the server to
- **Options**:
  - `"0.0.0.0"` - Listen on all interfaces
  - `"127.0.0.1"` - Listen only on localhost
  - Specific IP for network interface

```yaml
smtp:
  host: "127.0.0.1"  # Localhost only
```

#### `hostname`

- **Type**: `string`
- **Default**: `"mockforge-smtp"`
- **Description**: Server hostname used in SMTP greeting and responses
- **Notes**: Appears in `220` greeting and `250` HELO/EHLO responses

```yaml
smtp:
  hostname: "mail.example.com"
```

### Connection Settings

#### `timeout_secs`

- **Type**: `integer`
- **Default**: `30`
- **Description**: Connection timeout in seconds
- **Range**: `1` to `3600` (1 second to 1 hour)

```yaml
smtp:
  timeout_secs: 60  # 1 minute timeout
```

#### `max_connections`

- **Type**: `integer`
- **Default**: `100`
- **Description**: Maximum number of concurrent SMTP connections
- **Notes**: Prevents resource exhaustion from too many connections

```yaml
smtp:
  max_connections: 500
```

### Mailbox Settings

#### `enable_mailbox`

- **Type**: `boolean`
- **Default**: `true`
- **Description**: Enable in-memory mailbox for storing received emails

```yaml
smtp:
  enable_mailbox: true
```

#### `max_mailbox_messages`

- **Type**: `integer`
- **Default**: `1000`
- **Description**: Maximum number of emails to store in mailbox
- **Notes**:
  - Uses FIFO (First In, First Out) when limit is reached
  - Oldest emails are removed when limit is exceeded
  - Set to `0` for unlimited (not recommended)

```yaml
smtp:
  max_mailbox_messages: 5000
```

### Fixture Settings

#### `fixtures_dir`

- **Type**: `string` (path)
- **Default**: `null` (no fixtures)
- **Description**: Directory containing SMTP fixture files
- **Notes**:
  - Can be absolute or relative path
  - All `.yaml` and `.yml` files in directory will be loaded
  - See [Fixtures documentation](./fixtures.md) for format

```yaml
smtp:
  fixtures_dir: "./fixtures/smtp"
```

Or with absolute path:

```yaml
smtp:
  fixtures_dir: "/opt/mockforge/fixtures/smtp"
```

## Environment Variables

All configuration options can be overridden with environment variables using the prefix `MOCKFORGE_SMTP_`:

| Environment Variable | Config Option | Example |
|---------------------|---------------|---------|
| `MOCKFORGE_SMTP_ENABLED` | `enabled` | `true` |
| `MOCKFORGE_SMTP_PORT` | `port` | `2525` |
| `MOCKFORGE_SMTP_HOST` | `host` | `127.0.0.1` |
| `MOCKFORGE_SMTP_HOSTNAME` | `hostname` | `testmail.local` |

### Example

```bash
export MOCKFORGE_SMTP_ENABLED=true
export MOCKFORGE_SMTP_PORT=2525
export MOCKFORGE_SMTP_HOST=0.0.0.0
export MOCKFORGE_SMTP_HOSTNAME=test-server

mockforge serve
```

## Command-Line Arguments

Override configuration via CLI arguments:

```bash
mockforge serve \
  --smtp-port 2525 \
  --config ./config.yaml
```

### Priority Order

Configuration is applied in the following order (highest to lowest priority):

1. Command-line arguments
2. Environment variables
3. Configuration file
4. Default values

## Complete Example

### Development Configuration

```yaml
# config.dev.yaml
smtp:
  enabled: true
  port: 1025
  host: "127.0.0.1"
  hostname: "dev-smtp"
  timeout_secs: 30
  max_connections: 50
  enable_mailbox: true
  max_mailbox_messages: 500
  fixtures_dir: "./fixtures/smtp"
```

### Production-Like Configuration

```yaml
# config.prod.yaml
smtp:
  enabled: true
  port: 2525
  host: "0.0.0.0"
  hostname: "mockforge.example.com"
  timeout_secs: 60
  max_connections: 1000
  enable_mailbox: true
  max_mailbox_messages: 10000
  fixtures_dir: "/opt/mockforge/smtp-fixtures"
```

### CI/CD Configuration

```yaml
# config.ci.yaml
smtp:
  enabled: true
  port: 1025
  host: "127.0.0.1"
  hostname: "ci-smtp"
  timeout_secs: 10
  max_connections: 10
  enable_mailbox: true
  max_mailbox_messages: 100
  fixtures_dir: "./test/fixtures/smtp"
```

## Performance Tuning

### High-Volume Scenarios

For testing high-volume email sending:

```yaml
smtp:
  max_connections: 2000
  max_mailbox_messages: 50000
  timeout_secs: 120
```

**Memory considerations**: Each stored email uses approximately 1-5 KB of memory depending on size. 50,000 emails ≈ 50-250 MB.

### Low-Resource Environments

For constrained environments (CI, containers):

```yaml
smtp:
  max_connections: 25
  max_mailbox_messages: 100
  timeout_secs: 15
```

## Best Practices

### Security

1. **Bind to localhost in development**:
   ```yaml
   host: "127.0.0.1"
   ```

2. **Use non-privileged ports**:
   ```yaml
   port: 1025  # Not 25
   ```

3. **Limit connections**:
   ```yaml
   max_connections: 100
   ```

### Testing

1. **Use fixtures for deterministic tests**:
   ```yaml
   fixtures_dir: "./fixtures/smtp"
   ```

2. **Configure appropriate mailbox size**:
   ```yaml
   max_mailbox_messages: 1000  # Adjust based on test suite
   ```

3. **Set realistic timeouts**:
   ```yaml
   timeout_secs: 30  # Not too short, not too long
   ```

### CI/CD

1. **Use environment variables** for flexibility:
   ```bash
   MOCKFORGE_SMTP_PORT=1025
   ```

2. **Start server in background**:
   ```bash
   mockforge serve --smtp &
   ```

3. **Use localhost binding** for security:
   ```yaml
   host: "127.0.0.1"
   ```

## Troubleshooting

### Port Already in Use

**Error**: `Address already in use`

**Solution**:
```bash
# Check what's using the port
lsof -i :1025

# Use a different port
mockforge serve --smtp-port 2525
```

### Too Many Open Files

**Error**: `Too many open files`

**Solution**: Reduce `max_connections`:
```yaml
smtp:
  max_connections: 50
```

### Out of Memory

**Error**: OOM or slowdown with large mailbox

**Solution**: Reduce `max_mailbox_messages`:
```yaml
smtp:
  max_mailbox_messages: 1000
```

## Related Documentation

- [Getting Started](./getting-started.md)
- [Fixtures](./fixtures.md)
- [Examples](./examples.md)
