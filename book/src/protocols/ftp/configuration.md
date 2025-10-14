# FTP Server Configuration

MockForge FTP servers can be configured through command-line options or configuration files.

## Command Line Options

### Server Options

```bash
mockforge ftp serve [OPTIONS]
```

| Option | Description | Default |
|--------|-------------|---------|
| `--port <PORT>` | FTP server port | `2121` |
| `--host <HOST>` | FTP server host | `127.0.0.1` |
| `--virtual-root <PATH>` | Virtual file system root path | `/` |
| `--config <FILE>` | Configuration file path | - |

### Examples

```bash
# Basic server
mockforge ftp serve

# Custom port and host
mockforge ftp serve --port 2122 --host 0.0.0.0

# With configuration file
mockforge ftp serve --config ftp-config.yaml
```

## Configuration File

FTP servers can be configured using a YAML configuration file:

```yaml
ftp:
  host: "127.0.0.1"
  port: 2121
  virtual_root: "/"
  fixtures:
    - name: "sample_files"
      description: "Sample files for testing"
      virtual_files:
        - path: "/welcome.txt"
          content:
            type: "static"
            content: "Welcome to MockForge FTP!"
          permissions: "644"
          owner: "ftp"
          group: "ftp"
      upload_rules:
        - path_pattern: "/uploads/.*"
          auto_accept: true
          max_size_bytes: 1048576  # 1MB
          allowed_extensions: ["txt", "json", "xml"]
          storage:
            type: "memory"
```

## Virtual File System Configuration

### File Content Types

#### Static Content
```yaml
content:
  type: "static"
  content: "Hello, World!"
```

#### Template Content
```yaml
content:
  type: "template"
  template: '{"user": "{{faker.name}}", "id": "{{uuid}}", "time": "{{now}}"}'
```

#### Generated Content
```yaml
content:
  type: "generated"
  size: 1024
  pattern: "random"  # random, zeros, ones, incremental
```

### Upload Rules

Upload rules control how files are accepted and stored:

```yaml
upload_rules:
  - path_pattern: "/uploads/.*"  # Regex pattern
    auto_accept: true           # Auto-accept uploads
    max_size_bytes: 1048576     # Maximum file size
    allowed_extensions:         # Allowed file extensions
      - "txt"
      - "json"
    storage:                    # Storage backend
      type: "memory"           # memory, file, discard
```

### Storage Options

#### Memory Storage
Files are stored in memory (default):
```yaml
storage:
  type: "memory"
```

#### File Storage
Files are written to the local filesystem:
```yaml
storage:
  type: "file"
  path: "/tmp/uploads"
```

#### Discard Storage
Files are accepted but not stored:
```yaml
storage:
  type: "discard"
```

## Template Variables

When using template content, the following variables are available:

### Timestamps
- `{{now}}` - Current timestamp in RFC3339 format
- `{{timestamp}}` - Unix timestamp (seconds)
- `{{date}}` - Current date (YYYY-MM-DD)
- `{{time}}` - Current time (HH:MM:SS)

### Random Values
- `{{random_int}}` - Random 64-bit integer
- `{{random_float}}` - Random float (0.0-1.0)
- `{{uuid}}` - Random UUID v4

### Sample Data
- `{{faker.name}}` - Random name
- `{{faker.email}}` - Random email address
- `{{faker.age}}` - Random age (18-80)

### Example Templates

```yaml
# JSON response with dynamic data
content:
  type: "template"
  template: |
    {
      "id": "{{uuid}}",
      "name": "{{faker.name}}",
      "email": "{{faker.email}}",
      "created_at": "{{now}}",
      "age": {{faker.age}}
    }

# Log file with timestamps
content:
  type: "template"
  template: "[{{timestamp}}] INFO: Application started at {{time}}"
```

## Passive Mode Configuration

FTP passive mode uses dynamic port ranges. The server automatically configures passive ports in the range 49152-65535.

## Authentication

Currently, MockForge FTP servers support anonymous access only. Authentication can be added in future versions.

## Performance Tuning

### Memory Usage
- Virtual file system stores all files in memory
- Large files or many files may consume significant memory
- Consider using file-based storage for large uploads

### Connection Limits
- No built-in connection limits
- Consider system ulimits for production use

### Timeouts
- No configurable timeouts
- Uses libunftp defaults