# Basic Concepts

Understanding MockForge's core concepts will help you make the most of its capabilities. This guide explains the fundamental ideas behind MockForge's design and functionality.

## Multi-Protocol Architecture

MockForge is designed to mock multiple communication protocols within a single, unified framework:

### HTTP/REST APIs
- **OpenAPI/Swagger Support**: Define API contracts using industry-standard OpenAPI specifications
- **Dynamic Response Generation**: Generate realistic responses based on request parameters
- **Request/Response Matching**: Route requests to appropriate mock responses based on HTTP methods, paths, and parameters

### WebSocket Connections
- **Replay Mode**: Simulate scripted message sequences from recorded interactions
- **Interactive Mode**: Respond dynamically to client messages
- **State Management**: Maintain connection state across message exchanges

### gRPC Services
- **Protocol Buffer Integration**: Mock services defined with .proto files
- **Dynamic Service Discovery**: Automatically discover and compile .proto files
- **Streaming Support**: Handle unary, server streaming, client streaming, and bidirectional streaming
- **Reflection Support**: Built-in gRPC reflection for service discovery

## Response Generation Strategies

MockForge offers multiple approaches to generating mock responses:

### 1. Static Responses
Define fixed response payloads that are returned for matching requests:

```json
{
  "status": "success",
  "data": {
    "id": 123,
    "name": "Example Item"
  }
}
```

### 2. Template-Based Dynamic Responses
Use template variables for dynamic content generation:

```json
{
  "id": "{{uuid}}",
  "timestamp": "{{now}}",
  "randomValue": "{{randInt 1 100}}",
  "userData": "{{request.body}}"
}
```

### 3. Scenario-Based Responses
Define complex interaction scenarios with conditional logic and state management.

## Template System

MockForge's template system enables dynamic content generation using Handlebars-style syntax:

### Built-in Template Functions

#### Data Generation
- `{{uuid}}` - Generate unique UUID v4 identifiers
- `{{now}}` - Current timestamp in ISO 8601 format
- `{{now+1h}}` - Future timestamps with offset support
- `{{randInt min max}}` - Random integers within a range
- `{{randFloat min max}}` - Random floating-point numbers

#### Request Data Access
- `{{request.body}}` - Access complete request body
- `{{request.body.field}}` - Access specific JSON fields
- `{{request.path.param}}` - Access URL path parameters
- `{{request.query.param}}` - Access query string parameters
- `{{request.header.name}}` - Access HTTP headers

#### Conditional Logic
- `{{#if condition}}content{{/if}}` - Conditional content rendering
- `{{#each array}}item{{/each}}` - Iterate over arrays

### Template Expansion Control

Templates are only processed when explicitly enabled:

```bash
# Enable template expansion
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true
```

This security feature prevents accidental template processing in production environments.

## Configuration Hierarchy

MockForge supports multiple configuration methods with clear precedence:

### 1. Command Line Arguments (Highest Priority)
```bash
mockforge serve --http-port 3000 --ws-port 3001 --spec api.json
```

### 2. Environment Variables
```bash
MOCKFORGE_HTTP_PORT=3000
MOCKFORGE_WS_PORT=3001
MOCKFORGE_OPENAPI_SPEC=api.json
```

### 3. Configuration Files (Lowest Priority)
```yaml
# config.yaml
server:
  http_port: 3000
  ws_port: 3001
spec: api.json
```

## Server Modes

### Development Mode
- **Template Expansion**: Enabled by default for dynamic content
- **Verbose Logging**: Detailed request/response logging
- **Admin UI**: Enabled for visual server management
- **CORS**: Permissive cross-origin requests

### Production Mode
- **Template Expansion**: Disabled by default for security
- **Minimal Logging**: Essential information only
- **Performance Optimized**: Reduced overhead for high-throughput scenarios

## Request Matching

MockForge uses a sophisticated matching system to route requests to appropriate responses:

### HTTP Request Matching
1. **Method Matching**: GET, POST, PUT, DELETE, PATCH
2. **Path Matching**: Exact path or parameterized routes
3. **Query Parameter Matching**: Optional query string conditions
4. **Header Matching**: Conditional responses based on request headers
5. **Body Matching**: Match against request payload structure

### Priority Order
1. Most specific match first (method + path + query + headers + body)
2. Fall back to less specific matches
3. Default response for unmatched requests

## State Management

For complex scenarios, MockForge supports maintaining state across requests:

### Session State
- **Connection-specific data** persists across WebSocket messages
- **HTTP session cookies** maintain state between requests
- **Scenario progression** tracks interaction flow

### Global State
- **Shared data** accessible across all connections
- **Configuration updates** applied dynamically
- **Metrics and counters** maintained server-wide

## Extensibility

MockForge is designed for extension through multiple mechanisms:

### Custom Response Generators
Implement custom logic for generating complex responses based on business rules.

### Plugin System
Extend functionality through compiled plugins for specialized use cases.

### Configuration Extensions
Add custom configuration options for domain-specific requirements.

## Security Considerations

### Template Injection Prevention
- Templates are disabled by default in production
- Explicit opt-in required for template processing
- Input validation prevents malicious template injection

### Access Control
- Configurable CORS policies
- Request rate limiting options
- Authentication simulation support

### Data Privacy
- Request/response logging controls
- Sensitive data masking capabilities
- Compliance-friendly configuration options

## Performance Characteristics

### Throughput
- **HTTP APIs**: 10,000+ requests/second (depending on response complexity)
- **WebSocket**: 1,000+ concurrent connections
- **Memory Usage**: Minimal overhead per connection

### Scalability
- **Horizontal Scaling**: Multiple instances behind load balancer
- **Resource Efficiency**: Low CPU and memory footprint
- **Concurrent Users**: Support for thousands of simultaneous connections

## Integration Patterns

MockForge works well in various development and testing scenarios:

### API Development
- **Contract-First Development**: Mock APIs before implementation
- **Parallel Development**: Frontend and backend teams work independently
- **Integration Testing**: Validate API contracts between services

### Microservices Testing
- **Service Virtualization**: Mock dependent services during testing
- **Chaos Engineering**: Simulate service failures and latency
- **Load Testing**: Generate realistic traffic patterns

### CI/CD Pipelines
- **Automated Testing**: Mock external dependencies in test environments
- **Deployment Validation**: Verify application behavior with mock services
- **Performance Benchmarking**: Consistent test conditions across environments

This foundation will help you understand how to effectively use MockForge for your specific use case. The following guides provide detailed instructions for configuring and using each protocol and feature.
