# Rust API Reference

MockForge provides comprehensive Rust libraries for programmatic usage and extension. This reference covers the main crates and their APIs.

## Crate Overview

MockForge consists of several interconnected crates:

- **`mockforge-cli`**: Command-line interface and main executable
- **`mockforge-core`**: Core functionality shared across protocols
- **`mockforge-http`**: HTTP REST API mocking
- **`mockforge-grpc`**: gRPC service mocking
- **`mockforge-ui`**: Web-based admin interface

## Getting Started

Add MockForge to your `Cargo.toml`:

```toml
[dependencies]
mockforge-core = "0.1"
mockforge-http = "0.1"
mockforge-grpc = "0.1"
```

For development or testing, you might want to use path dependencies:

```toml
[dependencies]
mockforge-core = { path = "../mockforge/crates/mockforge-core" }
mockforge-http = { path = "../mockforge/crates/mockforge-http" }
mockforge-grpc = { path = "../mockforge/crates/mockforge-grpc" }
```

## Core Concepts

### Configuration System

MockForge uses a hierarchical configuration system that can be built programmatically:

```rust
use mockforge_core::config::MockForgeConfig;

let config = MockForgeConfig {
    server: ServerConfig {
        http_port: Some(3000),
        ws_port: Some(3001),
        grpc_port: Some(50051),
    },
    validation: ValidationConfig {
        mode: ValidationMode::Enforce,
        aggregate_errors: false,
    },
    response: ResponseConfig {
        template_expand: true,
    },
    ..Default::default()
};
```

### Template System

MockForge includes a powerful template engine for dynamic content generation:

```rust
use mockforge_core::template::{TemplateEngine, Context};

let engine = TemplateEngine::new();
let context = Context::new()
    .with_value("user_id", "12345")
    .with_value("timestamp", "2025-09-12T10:00:00Z");

let result = engine.render("User {{user_id}} logged in at {{timestamp}}", &context)?;
assert_eq!(result, "User 12345 logged in at 2025-09-12T10:00:00Z");
```

### Error Handling

MockForge uses the `anyhow` crate for error handling:

```rust
use anyhow::{Result, Context};

fn start_server(config: &Config) -> Result<()> {
    let server = HttpServer::new(config)
        .context("Failed to create HTTP server")?;

    server.start()
        .context("Failed to start server")?;

    Ok(())
}
```

## HTTP API

### Basic HTTP Server

```rust
use mockforge_http::{HttpServer, HttpConfig};
use mockforge_core::config::ServerConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create HTTP configuration
    let http_config = HttpConfig {
        spec_path: Some("api-spec.yaml".to_string()),
        validation_mode: ValidationMode::Warn,
        template_expand: true,
    };

    // Start HTTP server
    let mut server = HttpServer::new(http_config);
    server.start(([127, 0, 0, 1], 3000)).await?;

    println!("HTTP server running on http://localhost:3000");
    Ok(())
}
```

### Custom Route Handlers

```rust
use mockforge_http::{HttpServer, RouteHandler};
use warp::{Filter, Reply};

struct CustomHandler;

impl RouteHandler for CustomHandler {
    fn handle(&self, path: &str, method: &str) -> Option<Box<dyn Reply>> {
        if path == "/custom" && method == "GET" {
            Some(Box::new(warp::reply::json(&serde_json::json!({
                "message": "Custom response",
                "timestamp": chrono::Utc::now()
            }))))
        } else {
            None
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let handler = CustomHandler;
    let server = HttpServer::with_handler(handler);
    server.start(([127, 0, 0, 1], 3000)).await?;
    Ok(())
}
```

## gRPC API

### Basic gRPC Server

```rust
use mockforge_grpc::{GrpcServer, GrpcConfig};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure proto discovery
    let config = GrpcConfig {
        proto_dir: Path::new("proto/"),
        enable_reflection: true,
        ..Default::default()
    };

    // Start gRPC server
    let server = GrpcServer::new(config);
    server.start("127.0.0.1:50051").await?;

    println!("gRPC server running on 127.0.0.1:50051");
    Ok(())
}
```

### Custom Service Implementation

```rust
use mockforge_grpc::{ServiceRegistry, ServiceImplementation};
use prost::Message;
use tonic::{Request, Response, Status};

// Generated from proto file
mod greeter {
    include!("generated/greeter.rs");
}

pub struct GreeterService;

#[tonic::async_trait]
impl greeter::greeter_server::Greeter for GreeterService {
    async fn say_hello(
        &self,
        request: Request<greeter::HelloRequest>,
    ) -> Result<Response<greeter::HelloReply>, Status> {
        let name = request.into_inner().name;

        let reply = greeter::HelloReply {
            message: format!("Hello, {}!", name),
            timestamp: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = GreeterService {};
    let server = GrpcServer::with_service(service);
    server.start("127.0.0.1:50051").await?;
    Ok(())
}
```

## WebSocket API

### Basic WebSocket Server

```rust
use mockforge_ws::{WebSocketServer, WebSocketConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = WebSocketConfig {
        port: 3001,
        replay_file: Some("ws-replay.jsonl".to_string()),
        ..Default::default()
    };

    let server = WebSocketServer::new(config);
    server.start().await?;

    println!("WebSocket server running on ws://localhost:3001");
    Ok(())
}
```

### Custom Message Handler

```rust
use mockforge_ws::{WebSocketServer, MessageHandler};
use futures_util::{SinkExt, StreamExt};

struct EchoHandler;

impl MessageHandler for EchoHandler {
    async fn handle_message(&self, message: String) -> String {
        format!("Echo: {}", message)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let handler = EchoHandler {};
    let server = WebSocketServer::with_handler(handler);
    server.start().await?;
    Ok(())
}
```

This Rust API reference provides the foundation for programmatic usage of MockForge. For protocol-specific details, see the [HTTP](rust/http.md), [gRPC](rust/grpc.md), and [WebSocket](rust/ws.md) API documentation.
