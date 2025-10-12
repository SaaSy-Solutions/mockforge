# Protocol Implementation Quick Reference

This guide provides concrete code templates and patterns for implementing new protocols in MockForge.

## Table of Contents

- [Getting Started](#getting-started)
- [Creating a New Protocol Crate](#creating-a-new-protocol-crate)
- [Implementing Core Traits](#implementing-core-traits)
- [Configuration Integration](#configuration-integration)
- [CLI Integration](#cli-integration)
- [Testing](#testing)
- [Complete Example: SMTP](#complete-example-smtp)

## Getting Started

### Prerequisites

```bash
# Ensure you have the required tools
rustup target add wasm32-wasi  # For plugin support
cargo install cargo-expand     # For debugging macros
cargo install cargo-criterion  # For benchmarks
```

### Architecture Overview

```
New Protocol Implementation
│
├── Protocol Abstraction Integration
│   ├── Add Protocol enum variant
│   ├── Implement SpecRegistry trait
│   └── Implement ProtocolMiddleware support
│
├── Protocol-Specific Crate
│   ├── Server implementation
│   ├── Protocol handler
│   ├── Fixture loader
│   └── Tests
│
└── Integration
    ├── Configuration
    ├── CLI commands
    └── Admin UI (optional)
```

## Creating a New Protocol Crate

### Step 1: Create Crate Structure

```bash
# From MockForge root
cd crates/
cargo new mockforge-{protocol} --lib

cd mockforge-{protocol}
mkdir -p src/{server,fixtures,middleware}
mkdir -p tests
mkdir -p benches
mkdir -p examples
```

### Step 2: Setup Cargo.toml

```toml
[package]
name = "mockforge-{protocol}"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
description = "{Protocol} mocking support for MockForge"
repository.workspace = true

[dependencies]
# MockForge core
mockforge-core = { path = "../mockforge-core" }

# Standard workspace dependencies
tokio = { workspace = true }
async-trait = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }

# Protocol-specific dependencies
# {protocol-library} = "x.y.z"

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
criterion = { workspace = true }
tempfile = "3.0"

[[bench]]
name = "{protocol}_benchmarks"
harness = false
```

### Step 3: Add to Workspace

Edit root `Cargo.toml`:

```toml
[workspace]
members = [
    "crates/mockforge-cli",
    "crates/mockforge-core",
    # ... existing crates ...
    "crates/mockforge-{protocol}",  # Add this line
]
```

## Implementing Core Traits

### Step 1: Extend Protocol Enum

Edit `crates/mockforge-core/src/protocol_abstraction/mod.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    Http,
    GraphQL,
    Grpc,
    WebSocket,
    YourProtocol,  // Add your protocol
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::Http => write!(f, "HTTP"),
            Protocol::GraphQL => write!(f, "GraphQL"),
            Protocol::Grpc => write!(f, "gRPC"),
            Protocol::WebSocket => write!(f, "WebSocket"),
            Protocol::YourProtocol => write!(f, "YourProtocol"),  // Add display
        }
    }
}
```

### Step 2: Implement SpecRegistry Trait

Create `src/spec_registry.rs`:

```rust
use mockforge_core::protocol_abstraction::{
    Protocol, ProtocolRequest, ProtocolResponse, ResponseStatus,
    SpecOperation, SpecRegistry, ValidationError, ValidationResult,
};
use mockforge_core::Result;
use std::collections::HashMap;
use std::sync::Arc;

pub struct YourProtocolSpecRegistry {
    operations: Vec<SpecOperation>,
    fixtures: HashMap<String, YourProtocolFixture>,
}

impl YourProtocolSpecRegistry {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            fixtures: HashMap::new(),
        }
    }

    pub fn load_fixtures(&mut self, path: &Path) -> Result<()> {
        // Load fixtures from YAML/JSON files
        let files = std::fs::read_dir(path)?;

        for entry in files {
            let entry = entry?;
            let content = std::fs::read_to_string(entry.path())?;
            let fixture: YourProtocolFixture = serde_yaml::from_str(&content)?;

            self.fixtures.insert(
                fixture.identifier.clone(),
                fixture,
            );

            // Create SpecOperation for each fixture
            self.operations.push(SpecOperation {
                name: fixture.identifier.clone(),
                path: fixture.path.clone(),
                operation_type: fixture.operation_type.clone(),
                input_schema: fixture.input_schema.clone(),
                output_schema: fixture.output_schema.clone(),
                metadata: fixture.metadata.clone(),
            });
        }

        Ok(())
    }

    fn find_matching_fixture(&self, request: &ProtocolRequest) -> Option<&YourProtocolFixture> {
        // Implement fixture matching logic
        self.fixtures.values().find(|fixture| {
            fixture.matches(request)
        })
    }
}

impl SpecRegistry for YourProtocolSpecRegistry {
    fn protocol(&self) -> Protocol {
        Protocol::YourProtocol
    }

    fn operations(&self) -> Vec<SpecOperation> {
        self.operations.clone()
    }

    fn find_operation(&self, operation: &str, path: &str) -> Option<SpecOperation> {
        self.operations.iter().find(|op| {
            op.operation_type == operation && op.path == path
        }).cloned()
    }

    fn validate_request(&self, request: &ProtocolRequest) -> Result<ValidationResult> {
        // Implement request validation
        if request.protocol != Protocol::YourProtocol {
            return Ok(ValidationResult::failure(vec![
                ValidationError {
                    message: "Invalid protocol".to_string(),
                    path: None,
                    code: Some("INVALID_PROTOCOL".to_string()),
                }
            ]));
        }

        // Protocol-specific validation
        // Example: validate required fields, format, etc.

        Ok(ValidationResult::success())
    }

    fn generate_mock_response(&self, request: &ProtocolRequest) -> Result<ProtocolResponse> {
        // Find matching fixture
        let fixture = self.find_matching_fixture(request)
            .ok_or_else(|| anyhow::anyhow!("No matching fixture found"))?;

        // Generate response from fixture
        let body = fixture.generate_response_body(request)?;

        Ok(ProtocolResponse {
            status: ResponseStatus::YourProtocolStatus(fixture.status_code),
            metadata: fixture.metadata.clone(),
            body: body.into_bytes(),
            content_type: fixture.content_type.clone(),
        })
    }
}

// Response status variant (add to ResponseStatus enum in core)
impl ResponseStatus {
    // Add to mockforge-core/src/protocol_abstraction/mod.rs
    // YourProtocolStatus(YourStatusType),
}
```

### Step 3: Implement Server

Create `src/server.rs`:

```rust
use mockforge_core::protocol_abstraction::{
    MiddlewareChain, Protocol, ProtocolRequest, ProtocolResponse,
};
use mockforge_core::Result;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

pub struct YourProtocolServer {
    config: YourProtocolConfig,
    spec_registry: Arc<YourProtocolSpecRegistry>,
    middleware_chain: Arc<MiddlewareChain>,
}

impl YourProtocolServer {
    pub fn new(
        config: YourProtocolConfig,
        spec_registry: Arc<YourProtocolSpecRegistry>,
    ) -> Self {
        // Build default middleware chain
        let middleware_chain = Arc::new(
            MiddlewareChain::new()
                .with_middleware(Arc::new(LoggingMiddleware::new(false)))
                .with_middleware(Arc::new(MetricsMiddleware::new()))
        );

        Self {
            config,
            spec_registry,
            middleware_chain,
        }
    }

    pub async fn start(&self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await?;

        info!("YourProtocol server listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    info!("New connection from {}", peer_addr);

                    let registry = self.spec_registry.clone();
                    let middleware = self.middleware_chain.clone();

                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, peer_addr, registry, middleware).await {
                            error!("Error handling connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}

async fn handle_connection(
    stream: TcpStream,
    peer_addr: SocketAddr,
    registry: Arc<YourProtocolSpecRegistry>,
    middleware: Arc<MiddlewareChain>,
) -> Result<()> {
    // Protocol-specific connection handling

    // 1. Parse protocol messages from stream
    let mut protocol_parser = YourProtocolParser::new(stream);

    while let Some(message) = protocol_parser.next_message().await? {
        // 2. Convert to ProtocolRequest
        let mut request = convert_to_protocol_request(message, peer_addr)?;

        // 3. Run middleware (request processing)
        middleware.process_request(&mut request).await?;

        // 4. Generate response
        let mut response = registry.generate_mock_response(&request)?;

        // 5. Run middleware (response processing)
        middleware.process_response(&request, &mut response).await?;

        // 6. Send response back through protocol
        send_protocol_response(&mut protocol_parser, response).await?;
    }

    Ok(())
}

fn convert_to_protocol_request(
    message: YourProtocolMessage,
    peer_addr: SocketAddr,
) -> Result<ProtocolRequest> {
    Ok(ProtocolRequest {
        protocol: Protocol::YourProtocol,
        operation: message.operation.clone(),
        path: message.path.clone(),
        metadata: message.headers.clone(),
        body: message.body.clone(),
        client_ip: Some(peer_addr.ip().to_string()),
    })
}

async fn send_protocol_response(
    parser: &mut YourProtocolParser,
    response: ProtocolResponse,
) -> Result<()> {
    // Convert ProtocolResponse back to your protocol's message format
    let protocol_message = YourProtocolMessage {
        status: extract_status(&response.status),
        headers: response.metadata,
        body: Some(response.body),
    };

    parser.send_message(protocol_message).await
}
```

### Step 4: Define Fixture Format

Create `src/fixtures.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use mockforge_core::templating::TemplateEngine;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YourProtocolFixture {
    pub identifier: String,
    pub path: String,
    pub operation_type: String,

    #[serde(default)]
    pub metadata: HashMap<String, String>,

    pub input_schema: Option<String>,
    pub output_schema: Option<String>,

    pub response: FixtureResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureResponse {
    pub status_code: u16,  // Or protocol-specific status type
    pub content_type: String,

    #[serde(default)]
    pub headers: HashMap<String, String>,

    // Support both static and template-based responses
    pub body: Option<String>,
    pub template: Option<String>,

    // Latency simulation
    #[serde(default)]
    pub delay_ms: u64,

    // Failure simulation
    #[serde(default)]
    pub failure_rate: f64,
}

impl YourProtocolFixture {
    pub fn matches(&self, request: &ProtocolRequest) -> bool {
        // Implement matching logic
        request.path == self.path && request.operation == self.operation_type
    }

    pub fn generate_response_body(&self, request: &ProtocolRequest) -> Result<String> {
        if let Some(template) = &self.response.template {
            // Use template engine for dynamic responses
            let engine = TemplateEngine::new();
            engine.render(template, &request)
        } else if let Some(body) = &self.response.body {
            Ok(body.clone())
        } else {
            Ok(String::new())
        }
    }
}
```

## Configuration Integration

### Step 1: Add Protocol Config

Edit `crates/mockforge-core/src/config.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YourProtocolConfig {
    pub enabled: bool,
    pub port: u16,
    pub host: String,
    pub fixtures_dir: Option<PathBuf>,

    // Protocol-specific options
    pub timeout_secs: u64,
    pub max_connections: usize,

    // Optional TLS
    pub tls: Option<TlsConfig>,
}

impl Default for YourProtocolConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 9999,  // Choose appropriate default port
            host: "0.0.0.0".to_string(),
            fixtures_dir: Some(PathBuf::from("./fixtures/your-protocol")),
            timeout_secs: 30,
            max_connections: 100,
            tls: None,
        }
    }
}

// Add to ServerConfig
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerConfig {
    pub http: HttpConfig,
    pub websocket: WebSocketConfig,
    pub grpc: GrpcConfig,
    pub your_protocol: YourProtocolConfig,  // Add this
    // ... rest
}
```

### Step 2: Environment Variable Support

Add to `apply_env_overrides` function in `config.rs`:

```rust
pub fn apply_env_overrides(mut config: ServerConfig) -> ServerConfig {
    // ... existing overrides ...

    // YourProtocol server overrides
    if let Ok(port) = std::env::var("MOCKFORGE_YOURPROTOCOL_PORT") {
        if let Ok(port_num) = port.parse() {
            config.your_protocol.port = port_num;
        }
    }

    if let Ok(enabled) = std::env::var("MOCKFORGE_YOURPROTOCOL_ENABLED") {
        config.your_protocol.enabled = enabled == "1" || enabled.eq_ignore_ascii_case("true");
    }

    if let Ok(host) = std::env::var("MOCKFORGE_YOURPROTOCOL_HOST") {
        config.your_protocol.host = host;
    }

    config
}
```

## CLI Integration

### Step 1: Add CLI Commands

Edit `crates/mockforge-cli/src/commands/serve.rs`:

```rust
use mockforge_your_protocol::YourProtocolServer;

pub async fn start_servers(config: ServerConfig) -> Result<()> {
    let mut tasks = Vec::new();

    // ... existing server starts ...

    // Start YourProtocol server if enabled
    if config.your_protocol.enabled {
        let protocol_config = config.your_protocol.clone();

        let task = tokio::spawn(async move {
            let mut registry = YourProtocolSpecRegistry::new();

            if let Some(fixtures_dir) = &protocol_config.fixtures_dir {
                registry.load_fixtures(fixtures_dir)
                    .expect("Failed to load fixtures");
            }

            let server = YourProtocolServer::new(
                protocol_config,
                Arc::new(registry),
            );

            server.start().await
                .expect("YourProtocol server failed");
        });

        tasks.push(task);
    }

    // Wait for all servers
    for task in tasks {
        task.await??;
    }

    Ok(())
}
```

### Step 2: Add Protocol-Specific Commands

Create `crates/mockforge-cli/src/commands/your_protocol.rs`:

```rust
use clap::{Args, Subcommand};
use mockforge_your_protocol::*;

#[derive(Debug, Args)]
pub struct YourProtocolCommand {
    #[command(subcommand)]
    command: YourProtocolSubcommand,
}

#[derive(Debug, Subcommand)]
enum YourProtocolSubcommand {
    /// Send a test message
    Send {
        #[arg(long)]
        path: String,

        #[arg(long)]
        data: String,
    },

    /// List configured fixtures
    ListFixtures {
        #[arg(long)]
        dir: Option<PathBuf>,
    },

    /// Validate fixture files
    Validate {
        #[arg(long)]
        path: PathBuf,
    },
}

impl YourProtocolCommand {
    pub async fn execute(self) -> Result<()> {
        match self.command {
            YourProtocolSubcommand::Send { path, data } => {
                // Implement send logic
                println!("Sending to {}: {}", path, data);
                Ok(())
            }
            YourProtocolSubcommand::ListFixtures { dir } => {
                // List fixtures
                let fixtures = load_fixtures(dir.unwrap_or_default())?;
                for fixture in fixtures {
                    println!("- {} ({})", fixture.identifier, fixture.path);
                }
                Ok(())
            }
            YourProtocolSubcommand::Validate { path } => {
                // Validate fixture file
                validate_fixture_file(path)?;
                println!("✓ Fixture file is valid");
                Ok(())
            }
        }
    }
}
```

Add to `main.rs`:

```rust
#[derive(Debug, Subcommand)]
enum Commands {
    // ... existing commands ...

    #[command(name = "your-protocol")]
    YourProtocol(YourProtocolCommand),
}
```

## Testing

### Unit Tests

Create `src/lib.rs` with tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spec_registry_creation() {
        let registry = YourProtocolSpecRegistry::new();
        assert_eq!(registry.protocol(), Protocol::YourProtocol);
    }

    #[tokio::test]
    async fn test_fixture_loading() {
        let mut registry = YourProtocolSpecRegistry::new();
        let temp_dir = tempfile::tempdir().unwrap();

        // Create test fixture
        let fixture = YourProtocolFixture {
            identifier: "test".to_string(),
            path: "/test".to_string(),
            operation_type: "SEND".to_string(),
            metadata: HashMap::new(),
            input_schema: None,
            output_schema: None,
            response: FixtureResponse {
                status_code: 200,
                content_type: "text/plain".to_string(),
                headers: HashMap::new(),
                body: Some("OK".to_string()),
                template: None,
                delay_ms: 0,
                failure_rate: 0.0,
            },
        };

        // Write fixture file
        let fixture_path = temp_dir.path().join("test.yaml");
        std::fs::write(&fixture_path, serde_yaml::to_string(&fixture).unwrap()).unwrap();

        // Load fixtures
        registry.load_fixtures(temp_dir.path()).unwrap();

        assert_eq!(registry.operations().len(), 1);
    }

    #[tokio::test]
    async fn test_request_validation() {
        let registry = YourProtocolSpecRegistry::new();

        let request = ProtocolRequest {
            protocol: Protocol::YourProtocol,
            operation: "TEST".to_string(),
            path: "/test".to_string(),
            metadata: HashMap::new(),
            body: None,
            client_ip: None,
        };

        let result = registry.validate_request(&request).unwrap();
        assert!(result.valid);
    }
}
```

### Integration Tests

Create `tests/integration.rs`:

```rust
use mockforge_your_protocol::*;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_server_startup() {
    let config = YourProtocolConfig {
        enabled: true,
        port: 0,  // Random port
        ..Default::default()
    };

    let registry = Arc::new(YourProtocolSpecRegistry::new());
    let server = YourProtocolServer::new(config, registry);

    // Start server in background
    tokio::spawn(async move {
        server.start().await.unwrap();
    });

    sleep(Duration::from_millis(100)).await;

    // Server should be running
    // Add actual protocol client test here
}

#[tokio::test]
async fn test_end_to_end_request() {
    // 1. Start server
    // 2. Send protocol-specific request
    // 3. Verify response
}
```

### Benchmarks

Create `benches/your_protocol_benchmarks.rs`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mockforge_your_protocol::*;

fn bench_request_processing(c: &mut Criterion) {
    let registry = YourProtocolSpecRegistry::new();

    c.bench_function("process_request", |b| {
        b.iter(|| {
            let request = create_test_request();
            registry.generate_mock_response(black_box(&request))
        });
    });
}

fn bench_fixture_matching(c: &mut Criterion) {
    let mut registry = YourProtocolSpecRegistry::new();
    // Load test fixtures

    c.bench_function("match_fixture", |b| {
        b.iter(|| {
            let request = create_test_request();
            registry.find_matching_fixture(black_box(&request))
        });
    });
}

criterion_group!(benches, bench_request_processing, bench_fixture_matching);
criterion_main!(benches);
```

## Complete Example: SMTP

Here's a minimal but complete SMTP protocol implementation:

```rust
// crates/mockforge-smtp/src/lib.rs
use mockforge_core::protocol_abstraction::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

mod fixtures;
mod spec_registry;

pub use fixtures::*;
pub use spec_registry::*;

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub port: u16,
    pub host: String,
    pub hostname: String,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            port: 1025,
            host: "0.0.0.0".to_string(),
            hostname: "mockforge-smtp".to_string(),
        }
    }
}

pub struct SmtpServer {
    config: SmtpConfig,
    registry: Arc<SmtpSpecRegistry>,
}

impl SmtpServer {
    pub fn new(config: SmtpConfig, registry: Arc<SmtpSpecRegistry>) -> Self {
        Self { config, registry }
    }

    pub async fn start(&self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await?;

        tracing::info!("SMTP server listening on {}", addr);

        loop {
            let (stream, addr) = listener.accept().await?;
            let registry = self.registry.clone();
            let hostname = self.config.hostname.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_smtp_session(stream, addr, registry, hostname).await {
                    tracing::error!("SMTP session error: {}", e);
                }
            });
        }
    }
}

async fn handle_smtp_session(
    stream: TcpStream,
    _addr: std::net::SocketAddr,
    registry: Arc<SmtpSpecRegistry>,
    hostname: String,
) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Send greeting
    writer.write_all(format!("220 {} ESMTP MockForge\r\n", hostname).as_bytes()).await?;

    let mut mail_from = None;
    let mut rcpt_to = Vec::new();
    let mut data = String::new();
    let mut in_data = false;

    let mut line = String::new();
    while reader.read_line(&mut line).await? > 0 {
        let cmd = line.trim();

        if in_data {
            if cmd == "." {
                // End of DATA
                in_data = false;

                // Process email
                let request = ProtocolRequest {
                    protocol: Protocol::Smtp,
                    operation: "SEND".to_string(),
                    path: mail_from.clone().unwrap_or_default(),
                    metadata: [
                        ("from".to_string(), mail_from.clone().unwrap_or_default()),
                        ("to".to_string(), rcpt_to.join(",")),
                    ].into_iter().collect(),
                    body: Some(data.as_bytes().to_vec()),
                    client_ip: None,
                };

                let response = registry.generate_mock_response(&request)?;

                writer.write_all(String::from_utf8_lossy(&response.body).as_bytes()).await?;
                writer.write_all(b"\r\n").await?;

                // Reset state
                mail_from = None;
                rcpt_to.clear();
                data.clear();
            } else {
                data.push_str(cmd);
                data.push_str("\n");
            }
        } else if cmd.starts_with("HELLO") || cmd.starts_with("EHLO") {
            writer.write_all(format!("250 {} Hello\r\n", hostname).as_bytes()).await?;
        } else if cmd.starts_with("MAIL FROM:") {
            mail_from = Some(cmd.strip_prefix("MAIL FROM:").unwrap().trim().to_string());
            writer.write_all(b"250 OK\r\n").await?;
        } else if cmd.starts_with("RCPT TO:") {
            rcpt_to.push(cmd.strip_prefix("RCPT TO:").unwrap().trim().to_string());
            writer.write_all(b"250 OK\r\n").await?;
        } else if cmd == "DATA" {
            writer.write_all(b"354 Start mail input; end with <CRLF>.<CRLF>\r\n").await?;
            in_data = true;
        } else if cmd == "QUIT" {
            writer.write_all(b"221 Bye\r\n").await?;
            break;
        } else {
            writer.write_all(b"502 Command not implemented\r\n").await?;
        }

        line.clear();
    }

    Ok(())
}
```

This implementation provides:
- ✅ Basic SMTP protocol handling
- ✅ Integration with protocol abstraction layer
- ✅ Fixture-based response generation
- ✅ Async/await based architecture
- ✅ Proper error handling

## Next Steps

1. **Implement remaining protocol features**
2. **Add comprehensive tests**
3. **Create example fixtures**
4. **Write documentation**
5. **Submit PR for review**

## Resources

- [Protocol Abstraction Layer](./PROTOCOL_ABSTRACTION.md)
- [Protocol Expansion Roadmap](./PROTOCOL_EXPANSION_ROADMAP.md)
- [Plugin Development Guide](./plugins/development-guide.md)
- [MockForge Architecture](./ARCHITECTURE.md)
