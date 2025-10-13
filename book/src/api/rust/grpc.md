# gRPC Module

The `mockforge_grpc` crate provides dynamic gRPC service discovery and mocking with HTTP bridge capabilities.

## Modules

### Core Functions

#### `start`

```rust
pub async fn start(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
```

Starts a gRPC server with default configuration on the specified port.

**Parameters:**
- `port`: Port number to bind the gRPC server to

**Returns:** `Result<(), Error>` indicating server startup success

**Example:**
```rust
use mockforge_grpc::start;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    start(50051).await?;
    Ok(())
}
```

#### `start_with_config`

```rust
pub async fn start_with_config(
    port: u16,
    latency_profile: Option<LatencyProfile>,
    config: DynamicGrpcConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
```

Starts a gRPC server with custom configuration and optional latency simulation.

**Parameters:**
- `port`: Port number to bind the gRPC server to
- `latency_profile`: Optional latency injection profile
- `config`: Dynamic gRPC configuration

**Returns:** `Result<(), Error>` indicating server startup success

**Example:**
```rust
use mockforge_grpc::{start_with_config, DynamicGrpcConfig};
use mockforge_core::LatencyProfile;

let config = DynamicGrpcConfig {
    proto_dir: "./proto".to_string(),
    enable_reflection: true,
    ..Default::default()
};

start_with_config(50051, Some(LatencyProfile::normal()), config).await?;
```

### Configuration Types

#### `DynamicGrpcConfig`

```rust
pub struct DynamicGrpcConfig {
    pub proto_dir: String,
    pub enable_reflection: bool,
    pub excluded_services: Vec<String>,
    pub http_bridge: Option<HttpBridgeConfig>,
}
```

Configuration for dynamic gRPC service discovery.

**Fields:**
- `proto_dir`: Directory containing `.proto` files (default: "proto")
- `enable_reflection`: Whether to enable gRPC reflection (default: false)
- `excluded_services`: List of services to exclude from discovery
- `http_bridge`: Optional HTTP bridge configuration

**Methods:**
```rust
impl DynamicGrpcConfig {
    pub fn default() -> Self
}
```

**Example:**
```rust
let config = DynamicGrpcConfig {
    proto_dir: "./my-protos".to_string(),
    enable_reflection: true,
    excluded_services: vec!["HealthService".to_string()],
    http_bridge: Some(HttpBridgeConfig {
        enabled: true,
        port: 8080,
        generate_openapi: true,
    }),
};
```

#### `HttpBridgeConfig`

```rust
pub struct HttpBridgeConfig {
    pub enabled: bool,
    pub port: u16,
    pub generate_openapi: bool,
    pub cors_enabled: bool,
}
```

Configuration for HTTP bridge functionality.

**Fields:**
- `enabled`: Whether HTTP bridge is enabled (default: true)
- `port`: HTTP server port (default: 8080)
- `generate_openapi`: Whether to generate OpenAPI specs (default: true)
- `cors_enabled`: Whether CORS is enabled (default: false)

**Methods:**
```rust
impl HttpBridgeConfig {
    pub fn default() -> Self
}
```

### Service Registry

#### `ServiceRegistry`

```rust
pub struct ServiceRegistry { /* fields omitted */ }
```

Registry containing discovered gRPC services.

**Methods:**
```rust
impl ServiceRegistry {
    pub fn new() -> Self
    pub fn with_descriptor_pool(descriptor_pool: DescriptorPool) -> Self
    pub fn descriptor_pool(&self) -> &DescriptorPool
    pub fn register(&mut self, name: String, service: DynamicGrpcService)
    pub fn get(&self, name: &str) -> Option<&Arc<DynamicGrpcService>>
    pub fn service_names(&self) -> Vec<String>
}
```

**Example:**
```rust
use mockforge_grpc::ServiceRegistry;

let mut registry = ServiceRegistry::new();
registry.register("MyService".to_string(), dynamic_service);
println!("Registered services: {:?}", registry.service_names());
```

### Dynamic Service Types

#### `DynamicGrpcService`

```rust
pub struct DynamicGrpcService { /* fields omitted */ }
```

Dynamically generated gRPC service implementation.

**Methods:**
```rust
impl DynamicGrpcService {
    pub fn new(
        proto_service: ProtoService,
        config: Option<ServiceConfig>,
    ) -> Self
}
```

#### `ProtoService`

```rust
pub struct ProtoService {
    pub name: String,
    pub methods: HashMap<String, ProtoMethod>,
    pub package: String,
}
```

Parsed protobuf service definition.

**Fields:**
- `name`: Service name
- `methods`: Map of method names to method definitions
- `package`: Protobuf package name

#### `ProtoMethod`

```rust
pub struct ProtoMethod {
    pub name: String,
    pub input_type: String,
    pub output_type: String,
    pub is_client_streaming: bool,
    pub is_server_streaming: bool,
}
```

Parsed protobuf method definition.

**Fields:**
- `name`: Method name
- `input_type`: Input message type name
- `output_type`: Output message type name
- `is_client_streaming`: Whether method accepts client streaming
- `is_server_streaming`: Whether method returns server streaming

### Mock Response Types

#### `MockResponse`

```rust
pub enum MockResponse {
    Unary(Value),
    ServerStream(Vec<Value>),
    ClientStream(Value),
    BidiStream(Vec<Value>),
}
```

Mock response types for different gRPC method patterns.

**Variants:**
- `Unary(Value)`: Single request-response
- `ServerStream(Vec<Value>)`: Server streaming response
- `ClientStream(Value)`: Client streaming response
- `BidiStream(Vec<Value>)`: Bidirectional streaming

### Reflection Types

#### `MockReflectionProxy`

```rust
pub struct MockReflectionProxy { /* fields omitted */ }
```

Proxy for gRPC reflection protocol.

**Methods:**
```rust
impl MockReflectionProxy {
    pub async fn new(
        config: ProxyConfig,
        registry: Arc<ServiceRegistry>,
    ) -> Result<Self>
}
```

#### `ReflectionProxy`

```rust
pub trait ReflectionProxy {
    fn list_services(&self) -> Vec<String>;
    fn get_service_descriptor(&self, service_name: &str) -> Option<&prost_reflect::ServiceDescriptor>;
    fn get_method_descriptor(&self, service_name: &str, method_name: &str) -> Option<&prost_reflect::MethodDescriptor>;
}
```

Trait for gRPC reflection functionality.

#### `ProxyConfig`

```rust
pub struct ProxyConfig {
    pub max_message_size: usize,
    pub connection_timeout: Duration,
    pub request_timeout: Duration,
}
```

Configuration for reflection proxy.

**Fields:**
- `max_message_size`: Maximum message size in bytes (default: 4MB)
- `connection_timeout`: Connection timeout duration
- `request_timeout`: Request timeout duration

### Proto Parser

#### `ProtoParser`

```rust
pub struct ProtoParser { /* fields omitted */ }
```

Parser for protobuf files.

**Methods:**
```rust
impl ProtoParser {
    pub fn new() -> Self
    pub async fn parse_directory(&mut self, dir: &str) -> Result<()>
    pub fn services(&self) -> &HashMap<String, ProtoService>
    pub fn into_pool(self) -> DescriptorPool
}
```

**Example:**
```rust
use mockforge_grpc::dynamic::proto_parser::ProtoParser;

let mut parser = ProtoParser::new();
parser.parse_directory("./proto").await?;
let services = parser.services();
println!("Found {} services", services.len());
```

### Discovery Functions

#### `discover_services`

```rust
pub async fn discover_services(
    config: &DynamicGrpcConfig,
) -> Result<ServiceRegistry, Box<dyn std::error::Error + Send + Sync>>
```

Discovers and registers gRPC services from proto files.

**Parameters:**
- `config`: Discovery configuration

**Returns:** `Result<ServiceRegistry, Error>` with discovered services

**Example:**
```rust
use mockforge_grpc::{discover_services, DynamicGrpcConfig};

let config = DynamicGrpcConfig {
    proto_dir: "./proto".to_string(),
    ..Default::default()
};

let registry = discover_services(&config).await?;
println!("Discovered services: {:?}", registry.service_names());
```

### Generated Types

#### `Greeter` Service

```rust
pub mod generated {
    pub mod greeter_server {
        pub trait Greeter: Send + Sync + 'static {
            type SayHelloStreamStream: Stream<Item = Result<HelloReply, Status>> + Send + 'static;

            async fn say_hello(
                &self,
                request: Request<HelloRequest>,
            ) -> Result<Response<HelloReply>, Status>;

            async fn say_hello_stream(
                &self,
                request: Request<HelloRequest>,
            ) -> Result<Response<Self::SayHelloStreamStream>, Status>;

            async fn say_hello_client_stream(
                &self,
                request: Request<Streaming<HelloRequest>>,
            ) -> Result<Response<HelloReply>, Status>;

            async fn chat(
                &self,
                request: Request<Streaming<HelloRequest>>,
            ) -> Result<Response<Self::ChatStream>, Status>;
        }
    }
}
```

Generated gRPC service trait with all streaming patterns.

### Message Types

#### `HelloRequest`

```rust
pub struct HelloRequest {
    pub name: String,
}
```

Request message for greeting service.

**Fields:**
- `name`: Name to greet

#### `HelloReply`

```rust
pub struct HelloReply {
    pub message: String,
    pub metadata: Option<HashMap<String, String>>,
    pub items: Vec<String>,
}
```

Response message for greeting service.

**Fields:**
- `message`: Greeting message
- `metadata`: Optional metadata map
- `items`: Optional list of items

### Error Handling

All functions return `Result<T, Box<dyn std::error::Error + Send + Sync>>`. Common errors include:

- File I/O errors (proto file reading)
- Protobuf parsing errors
- Server binding errors
- Reflection setup errors
- HTTP bridge configuration errors

### Constants

- `DEFAULT_MAX_MESSAGE_SIZE`: Default maximum message size (4MB)

### Feature Flags

- `data-faker`: Enables advanced data synthesis features

## Examples

### Basic gRPC Server

```rust
use mockforge_grpc::start;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Starts server on port 50051 with default config
    // Automatically discovers services from ./proto directory
    start(50051).await?;
    Ok(())
}
```

### Server with Reflection

```rust
use mockforge_grpc::{start_with_config, DynamicGrpcConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = DynamicGrpcConfig {
        proto_dir: "./proto".to_string(),
        enable_reflection: true,  // Enable gRPC reflection
        ..Default::default()
    };

    start_with_config(50051, None, config).await?;

    // Now you can use grpcurl:
    // grpcurl -plaintext localhost:50051 list
    // grpcurl -plaintext localhost:50051 describe MyService
    Ok(())
}
```

### Server with HTTP Bridge

```rust
use mockforge_grpc::{start_with_config, DynamicGrpcConfig};
use mockforge_grpc::dynamic::http_bridge::HttpBridgeConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = DynamicGrpcConfig {
        proto_dir: "./proto".to_string(),
        http_bridge: Some(HttpBridgeConfig {
            enabled: true,
            port: 8080,
            generate_openapi: true,
        }),
        ..Default::default()
    };

    start_with_config(50051, None, config).await?;

    // gRPC available on localhost:50051
    // REST API available on localhost:8080
    // OpenAPI docs at http://localhost:8080/api/docs
    Ok(())
}
```

### Manual Service Discovery

```rust
use mockforge_grpc::{discover_services, DynamicGrpcConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = DynamicGrpcConfig {
        proto_dir: "./proto".to_string(),
        excluded_services: vec!["HealthService".to_string()],
        ..Default::default()
    };

    let registry = discover_services(&config).await?;

    println!("Discovered services:");
    for service_name in registry.service_names() {
        println!("  - {}", service_name);
    }

    // Access service descriptors
    if let Some(descriptor) = registry.descriptor_pool().get_service_by_name("MyService") {
        println!("Service methods:");
        for method in descriptor.methods() {
            println!("  - {}", method.name());
        }
    }

    Ok(())
}
```

### Custom Service Implementation

```rust
use mockforge_grpc::dynamic::service_generator::DynamicGrpcService;
use mockforge_grpc::dynamic::proto_parser::{ProtoParser, ProtoService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Parse proto files
    let mut parser = ProtoParser::new();
    parser.parse_directory("./proto").await?;

    // Get a specific service
    if let Some(proto_service) = parser.services().get("MyService") {
        // Create dynamic service
        let dynamic_service = DynamicGrpcService::new(proto_service.clone(), None);

        // The service will automatically handle all RPC methods
        // with mock responses based on the protobuf definitions
    }

    Ok(())
}
```

### Using gRPC Reflection

```rust
use mockforge_grpc::reflection::{MockReflectionProxy, ProxyConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = DynamicGrpcConfig {
        proto_dir: "./proto".to_string(),
        enable_reflection: true,
        ..Default::default()
    };

    let registry = discover_services(&config).await?;
    let registry_arc = Arc::new(registry);

    let proxy_config = ProxyConfig::default();
    let reflection_proxy = MockReflectionProxy::new(proxy_config, registry_arc).await?;

    // The reflection proxy enables:
    // - Service listing: reflection_proxy.list_services()
    // - Service descriptors: reflection_proxy.get_service_descriptor("MyService")
    // - Method descriptors: reflection_proxy.get_method_descriptor("MyService", "MyMethod")

    Ok(())
}
```