# MockForge FTP

FTP protocol support for MockForge.

This crate provides FTP-specific functionality for creating mock FTP servers, including virtual file systems, fixture-driven responses, and file transfer simulation. Perfect for testing FTP clients, file upload workflows, and FTP-based integrations.

## Features

- **FTP Server Mocking**: Full FTP protocol support with standard commands
- **Virtual File System**: In-memory file storage with configurable content
- **Fixture Management**: YAML-based configuration for file structures
- **Upload Handling**: Configurable validation and storage for file uploads
- **Template Support**: Dynamic content generation using Handlebars templates
- **Protocol Compliance**: Supports passive/active modes and authentication

## Quick Start

### Basic FTP Server

```rust,no_run
use mockforge_ftp::FtpServer;
use mockforge_core::config::FtpConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create default FTP configuration
    let config = FtpConfig::default();

    // Initialize FTP server
    let server = FtpServer::new(config).await?;

    // Start the server
    server.start().await?;

    Ok(())
}
```

### Server with Custom Configuration

```rust,no_run
use mockforge_core::config::FtpConfig;
use mockforge_ftp::FtpServer;

let config = FtpConfig {
    host: "127.0.0.1".to_string(),
    port: 2121,
    virtual_root: "/ftp".to_string(),
    ..Default::default()
};

let server = FtpServer::new(config).await?;
server.start().await?;
```

## Virtual File System

MockForge FTP includes a powerful virtual file system for creating realistic file structures:

### Static Files

```rust,no_run
use mockforge_ftp::vfs::{VirtualFileSystem, FileContent, FileMetadata};
use std::path::PathBuf;

let mut vfs = VirtualFileSystem::new("/ftp".to_string());

// Add a static file
let content = FileContent::Static(b"Hello, FTP!".to_vec());
let metadata = FileMetadata {
    size: 11,
    permissions: "644".to_string(),
    owner: "user".to_string(),
    group: "users".to_string(),
};

let file = VirtualFile::new(
    PathBuf::from("/ftp/hello.txt"),
    content,
    metadata
);

vfs.add_file(PathBuf::from("/ftp/hello.txt"), file)?;
```

### Generated Files

```rust,no_run
use mockforge_ftp::vfs::{FileContent, GenerationPattern};

// Generate a 1MB file with random content
let content = FileContent::Generated {
    size: 1024 * 1024,
    pattern: GenerationPattern::Random,
};

// Generate a file with incremental bytes (0, 1, 2, ...)
let incremental = FileContent::Generated {
    size: 1000,
    pattern: GenerationPattern::Incremental,
};
```

### Template Files

```rust,no_run
use mockforge_ftp::vfs::FileContent;

// Use Handlebars templates for dynamic content
let content = FileContent::Template(
    "Hello {{name}}! Today is {{date}}.".to_string()
);
```

## Fixture System

Define complete FTP server configurations using YAML fixtures:

```yaml
identifier: "test-server"
name: "Test FTP Server"
description: "A test FTP server with sample files"

virtual_files:
  - path: "/welcome.txt"
    content:
      type: "static"
      content: "Welcome to MockForge FTP!"
    permissions: "644"
    owner: "ftp"
    group: "ftp"

  - path: "/data.bin"
    content:
      type: "generated"
      size: 1048576  # 1MB
      pattern: "random"
    permissions: "644"
    owner: "ftp"
    group: "ftp"

upload_rules:
  - path_pattern: "/uploads/.*\\.txt"
    auto_accept: true
    validation:
      max_size_bytes: 1048576  # 1MB limit
      allowed_extensions: ["txt"]
    storage:
      type: "memory"

  - path_pattern: "/logs/.*\\.log"
    auto_accept: true
    storage:
      type: "file"
      path: "/tmp/ftp_logs"
```

### Loading Fixtures

```rust,no_run
use mockforge_ftp::{FtpServer, FtpSpecRegistry};
use mockforge_core::config::FtpConfig;

let config = FtpConfig::default();
let server = FtpServer::new(config).await?;

// Load fixture from file
server.spec_registry().load_fixture_from_file("ftp-fixture.yaml").await?;

// Or create fixture programmatically
use mockforge_ftp::fixtures::{FtpFixture, VirtualFileConfig, UploadRule};

let fixture = FtpFixture {
    identifier: "programmatic".to_string(),
    name: "Programmatic Fixture".to_string(),
    description: Some("Created in code".to_string()),
    virtual_files: vec![/* ... */],
    upload_rules: vec![/* ... */],
};

server.spec_registry().add_fixture(fixture)?;
```

## Upload Handling

Configure how the server handles file uploads:

### Upload Rules

```rust,no_run
use mockforge_ftp::fixtures::{UploadRule, FileValidation, UploadStorage};
use regex::Regex;

let rule = UploadRule {
    path_pattern: r"/uploads/.*".to_string(),
    auto_accept: true,
    validation: Some(FileValidation {
        max_size_bytes: Some(10 * 1024 * 1024), // 10MB
        allowed_extensions: Some(vec!["txt".to_string(), "csv".to_string()]),
        mime_types: Some(vec!["text/plain".to_string(), "text/csv".to_string()]),
    }),
    storage: UploadStorage::Memory, // Store in VFS
};
```

### Storage Options

- **Memory**: Store uploaded files in the virtual file system
- **File**: Write to the local file system
- **Discard**: Accept uploads but don't store them

## FTP Protocol Support

MockForge FTP supports standard FTP commands:

- **Connection**: USER, PASS, QUIT
- **Navigation**: CWD, CDUP, PWD
- **Directory**: LIST, NLST, MKD, RMD
- **File Operations**: RETR, STOR, DELE, RNFR/RNTO
- **System**: SYST, FEAT, OPTS
- **Transfer Modes**: Passive and active mode support

## Testing FTP Clients

Use MockForge FTP to test FTP client applications:

```rust,no_run
use suppaftp::FtpStream; // FTP client library
use mockforge_ftp::FtpServer;
use mockforge_core::config::FtpConfig;
use tokio::task;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start FTP server in background
    let config = FtpConfig {
        host: "127.0.0.1".to_string(),
        port: 2121,
        ..Default::default()
    };

    let server = FtpServer::new(config.clone()).await?;
    task::spawn(async move {
        server.start().await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test FTP client
    let mut ftp_stream = FtpStream::connect(format!("{}:{}", config.host, config.port))?;
    ftp_stream.login("anonymous", "anonymous")?;

    // List files
    let files = ftp_stream.list(None)?;
    println!("Files: {:?}", files);

    // Download a file
    let mut reader = ftp_stream.get("welcome.txt")?;
    // ... process file content ...

    ftp_stream.quit()?;

    Ok(())
}
```

## Configuration

### FtpConfig

```rust,no_run
use mockforge_core::config::FtpConfig;

let config = FtpConfig {
    host: "0.0.0.0".to_string(),     // Bind address
    port: 21,                        // FTP port
    virtual_root: "/".to_string(),    // Virtual filesystem root
    greeting: "MockForge FTP".to_string(), // Server greeting
    max_connections: 100,            // Connection limit
    ..Default::default()
};
```

## Integration with MockForge

MockForge FTP integrates seamlessly with the MockForge ecosystem:

- **MockForge Core**: Shared configuration and logging
- **MockForge CLI**: Command-line interface for FTP server management
- **MockForge Plugins**: Extend FTP functionality with custom plugins

## Performance

MockForge FTP is optimized for testing scenarios:

- **In-Memory Storage**: Fast file operations without disk I/O
- **Lazy Content Generation**: Files generated on-demand
- **Concurrent Connections**: Handle multiple simultaneous FTP clients
- **Low Memory Footprint**: Efficient storage for large file simulations

## Examples

See the [examples directory](https://github.com/SaaSy-Solutions/mockforge/tree/main/examples) for complete working examples including:

- Basic FTP server setup
- Fixture-driven configurations
- Upload handling scenarios
- Integration testing patterns

## Related Crates

- [`mockforge-core`](https://docs.rs/mockforge-core): Core mocking functionality
- [`libunftp`](https://docs.rs/libunftp): Underlying FTP server implementation
- [`suppaftp`](https://docs.rs/suppaftp): FTP client for testing

## License

Licensed under MIT OR Apache-2.0
