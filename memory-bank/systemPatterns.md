# System Patterns: MockForge Architecture

## Core Architecture Patterns

### Modular Crate Structure
```
mockforge/
├── mockforge-cli/          # Command-line interface and import logic
├── mockforge-core/         # Shared utilities (routing, validation, latency)
├── mockforge-http/         # HTTP mocking with OpenAPI support
├── mockforge-ws/           # WebSocket mocking with script replay
├── mockforge-grpc/         # gRPC service mocking
├── mockforge-data/         # Synthetic data generation (faker + RAG)
└── mockforge-ui/           # Admin web interface (Axum routes + assets)
```

### Import Pipeline Pattern
```
Import Source → Format Detection → Parser → Route Generation → Config Output
     ↓              ↓              ↓           ↓              ↓
   File/Text    Confidence     Postman/    MockForge     YAML/JSON
   Content      Scoring       Insomnia/     Routes       Config
                              Curl
```

### Route Generation Pattern
```
OpenAPI Spec → Route Registry → Validation → Response Generation → Axum Router
     ↓              ↓              ↓              ↓              ↓
   Schema       Path/Method    Request       Mock Data      HTTP
   Parsing      Extraction     Validation    Synthesis      Handler
```

## Key Design Patterns

### Builder Pattern (Route Generation)
- `OpenApiRouteRegistry::new()` builds routes from specifications
- Configurable validation options and response generation
- Extensible through plugin architecture

### Strategy Pattern (Import Formats)
- Separate parser implementations for each format (Postman, Insomnia, Curl)
- Common interface with format-specific implementations
- Factory pattern for parser selection based on detection

### Observer Pattern (Admin UI)
- Centralized request logging for real-time monitoring
- Event-driven updates to dashboard metrics
- Decoupled UI updates from server operations

### Template Method Pattern (Data Generation)
- Base data generation framework with extensible templates
- RAG integration for enhanced data quality
- Pluggable generation strategies

## Communication Patterns

### Async/Await with Tokio
- All server components use async runtimes
- Non-blocking I/O for concurrent request handling
- Task spawning for independent service management

### Channel-Based Communication
- Request logging uses channels for thread-safe communication
- Admin UI receives real-time updates via WebSocket connections
- Inter-service communication through structured message passing

## Configuration Patterns

### Environment Override Hierarchy
```
CLI Args → Environment Variables → Config File → Defaults
```

### Feature Flags Pattern
- Runtime feature toggling (latency, failures, validation)
- Environment-based configuration
- Graceful degradation when features are disabled

## Error Handling Patterns

### Result-Based Error Propagation
- Comprehensive error types with context
- Validation error aggregation for better UX
- Warning collection during import operations

### Graceful Degradation
- Services continue operating when non-critical components fail
- Fallback responses for mock generation failures
- Logging without breaking request flow

## Testing Patterns

### Integration Test Suite
- End-to-end testing across all protocols
- Import functionality validation
- UI interaction testing

### Mock Data Validation
- Schema validation for generated responses
- Relationship integrity checking
- Deterministic seeding for reproducible tests
