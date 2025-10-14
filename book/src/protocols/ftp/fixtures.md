# FTP Fixtures

FTP fixtures allow you to pre-configure file structures and upload rules for your mock FTP server.

## Fixture Structure

Fixtures are defined in YAML format and contain:

- **Virtual files**: Pre-defined files in the virtual file system
- **Upload rules**: Rules for accepting and handling file uploads

## Basic Fixture Example

```yaml
fixtures:
  - name: "sample_files"
    description: "Sample files for testing FTP clients"
    virtual_files:
      - path: "/welcome.txt"
        content:
          type: "static"
          content: "Welcome to MockForge FTP Server!"
        permissions: "644"
        owner: "ftp"
        group: "ftp"
      - path: "/data.json"
        content:
          type: "template"
          template: '{"timestamp": "{{now}}", "server": "mockforge"}'
        permissions: "644"
        owner: "ftp"
        group: "ftp"
    upload_rules:
      - path_pattern: "/uploads/.*"
        auto_accept: true
        max_size_bytes: 1048576
        allowed_extensions: ["txt", "json", "xml"]
        storage:
          type: "memory"
```

## Virtual Files

### Static Content Files

```yaml
virtual_files:
  - path: "/readme.txt"
    content:
      type: "static"
      content: |
        This is a mock FTP server.
        You can upload files to the /uploads directory.
    permissions: "644"
    owner: "ftp"
    group: "ftp"
```

### Template Files

```yaml
virtual_files:
  - path: "/status.json"
    content:
      type: "template"
      template: |
        {
          "server": "MockForge FTP",
          "version": "1.0.0",
          "uptime": "{{timestamp}}",
          "status": "running"
        }
    permissions: "644"
    owner: "ftp"
    group: "ftp"
```

### Generated Content Files

```yaml
virtual_files:
  - path: "/random.bin"
    content:
      type: "generated"
      size: 1024
      pattern: "random"
    permissions: "644"
    owner: "ftp"
    group: "ftp"
```

## Upload Rules

Upload rules control how the server handles file uploads.

### Basic Upload Rule

```yaml
upload_rules:
  - path_pattern: "/uploads/.*"
    auto_accept: true
    storage:
      type: "memory"
```

### Advanced Upload Rule

```yaml
upload_rules:
  - path_pattern: "/documents/.*"
    auto_accept: true
    validation:
      max_size_bytes: 5242880  # 5MB
      allowed_extensions: ["pdf", "doc", "docx", "txt"]
      mime_types: ["application/pdf", "application/msword"]
    storage:
      type: "file"
      path: "/tmp/uploads"
```

### Validation Options

#### File Size Limits
```yaml
validation:
  max_size_bytes: 1048576  # 1MB limit
```

#### File Extensions
```yaml
validation:
  allowed_extensions: ["jpg", "png", "gif"]
```

#### MIME Types
```yaml
validation:
  mime_types: ["image/jpeg", "image/png"]
```

### Storage Backends

#### Memory Storage
Files are stored in memory (default):
```yaml
storage:
  type: "memory"
```

#### File Storage
Files are written to disk:
```yaml
storage:
  type: "file"
  path: "/var/ftp/uploads"
```

#### Discard Storage
Files are accepted but not stored:
```yaml
storage:
  type: "discard"
```

## Loading Fixtures

### From Configuration File

```bash
mockforge ftp serve --config ftp-config.yaml
```

### From Directory

```bash
mockforge ftp fixtures load ./fixtures/ftp/
```

### Validate Fixtures

```bash
mockforge ftp fixtures validate fixture.yaml
```

## Example Complete Fixture

```yaml
fixtures:
  - name: "test_environment"
    description: "Complete test environment with various file types"
    virtual_files:
      # Static files
      - path: "/readme.txt"
        content:
          type: "static"
          content: "FTP Test Server - Upload files to /uploads/"
        permissions: "644"
        owner: "ftp"
        group: "ftp"

      # Template files
      - path: "/server-info.json"
        content:
          type: "template"
          template: |
            {
              "server": "MockForge FTP",
              "started_at": "{{now}}",
              "session_id": "{{uuid}}"
            }
        permissions: "644"
        owner: "ftp"
        group: "ftp"

      # Generated files
      - path: "/test-data.bin"
        content:
          type: "generated"
          size: 4096
          pattern: "random"
        permissions: "644"
        owner: "ftp"
        group: "ftp"

    upload_rules:
      # General uploads
      - path_pattern: "/uploads/.*"
        auto_accept: true
        validation:
          max_size_bytes: 10485760  # 10MB
        storage:
          type: "memory"

      # Image uploads
      - path_pattern: "/images/.*"
        auto_accept: true
        validation:
          max_size_bytes: 5242880  # 5MB
          allowed_extensions: ["jpg", "jpeg", "png", "gif"]
          mime_types: ["image/jpeg", "image/png", "image/gif"]
        storage:
          type: "file"
          path: "/tmp/images"

      # Log files (discard)
      - path_pattern: "/logs/.*"
        auto_accept: true
        storage:
          type: "discard"
```

## CLI Management

### List Fixtures

```bash
mockforge ftp fixtures list
```

### Load Fixtures

```bash
# Load from directory
mockforge ftp fixtures load ./fixtures/

# Load specific file
mockforge ftp fixtures load fixture.yaml
```

### Validate Fixtures

```bash
mockforge ftp fixtures validate fixture.yaml
```

## Virtual File System Management

### Add Files

```bash
# Static content
mockforge ftp vfs add /hello.txt --content "Hello World"

# Template content
mockforge ftp vfs add /user.json --template '{"name": "{{faker.name}}"}'

# Generated content
mockforge ftp vfs add /data.bin --generate random --size 1024
```

### List Files

```bash
mockforge ftp vfs list /
```

### Remove Files

```bash
mockforge ftp vfs remove /old-file.txt
```

### Get File Info

```bash
mockforge ftp vfs info /hello.txt
```