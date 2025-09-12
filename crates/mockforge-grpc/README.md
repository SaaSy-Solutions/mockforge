# MockForge gRPC

Flexible gRPC mocking and service discovery for MockForge.

## Features

- **Dynamic Proto Discovery**: Automatically discovers and compiles `.proto` files from configurable directories
- **Flexible Service Registration**: Register and mock any gRPC service without hardcoding
- **Reflection Support**: Built-in gRPC reflection for service discovery
- **Environment Configuration**: Configure proto directories via environment variables

## Quick Start

### Basic Usage

```rust
use mockforge_grpc::{start, DynamicGrpcConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Start with default configuration
    start(50051).await?;
    Ok(())
}
```

### Flexible Configuration

```rust
use mockforge_grpc::{start_with_latency_and_config, DynamicGrpcConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = DynamicGrpcConfig {
        proto_dir: "my-protos".to_string(),
        enable_reflection: true,
        excluded_services: vec!["grpc.reflection.v1alpha.ServerReflection".to_string()],
    };

    start_with_latency_and_config(50051, None, Some(config)).await?;
    Ok(())
}
```

## Environment Variables

- `MOCKFORGE_PROTO_DIR`: Directory containing `.proto` files (default: `proto/`)
- `MOCKFORGE_GRPC_PORT`: gRPC server port (default: `50051`)

## Proto File Discovery

The build system automatically discovers all `.proto` files in the configured directory and subdirectories. This means:

1. **No hardcoded proto files**: The system finds proto files dynamically
2. **Recursive discovery**: Searches subdirectories for proto files
3. **Automatic compilation**: All discovered proto files are compiled to Rust code
4. **Change detection**: Rebuilds when proto files change

## Directory Structure

```
your-project/
├── proto/                    # Default proto directory
│   ├── service1.proto       # Will be discovered
│   ├── service2.proto       # Will be discovered
│   └── subdir/
│       └── service3.proto   # Will be discovered
├── src/
└── Cargo.toml
```

## Custom Proto Directory

Set the `MOCKFORGE_PROTO_DIR` environment variable to use a different directory:

```bash
export MOCKFORGE_PROTO_DIR="my-custom-protos"
cargo build
```

## Examples

See the `examples/` directory for complete examples:

- `flexible_grpc.rs`: Demonstrates dynamic service discovery
- `reflection_example.rs`: Shows how to use gRPC reflection

## Migration from Hardcoded Approach

If you're migrating from the old hardcoded approach:

1. **Remove hardcoded proto references** from your `build.rs`
2. **Set `MOCKFORGE_PROTO_DIR`** to point to your proto files
3. **Use `DynamicGrpcConfig`** for advanced configuration
4. **Update your service registration** to use the dynamic system

## Advanced Configuration

```rust
use mockforge_grpc::{DynamicGrpcConfig, ServiceRegistry, ServiceImplementation};

// Create custom service implementations
struct MyCustomService {
    name: String,
}

impl ServiceImplementation for MyCustomService {
    // Implement your custom service logic
}

// Register services dynamically
let mut registry = ServiceRegistry::new();
registry.register("my.service.Service".to_string(), MyCustomService::new());
```

## Troubleshooting

### No Proto Files Found

If you see "No .proto files found", check:

1. The `MOCKFORGE_PROTO_DIR` environment variable
2. That the directory exists and contains `.proto` files
3. File permissions on the proto directory

### Compilation Errors

If proto compilation fails:

1. Check that your `.proto` files are valid
2. Ensure all dependencies are properly imported
3. Verify that the proto directory structure is correct

### Service Not Found

If services aren't being discovered:

1. Check the service names in your proto files
2. Verify that the proto files are being compiled
3. Check the logs for discovery information
