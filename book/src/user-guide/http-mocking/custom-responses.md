# Custom Responses

MockForge provides multiple powerful ways to create custom HTTP responses beyond basic OpenAPI schema generation. This guide covers advanced response customization techniques including plugins, overrides, and dynamic generation.

## Response Override Rules

Override rules allow you to modify OpenAPI-generated responses using JSON patches without changing the original specification.

### Basic Override Configuration

```yaml
# mockforge.yaml
http:
  openapi_spec: api-spec.json
  response_template_expand: true

# Override specific endpoints
overrides:
  - targets: ["path:/users"]
    patch:
      - op: replace
        path: "/responses/200/content/application~1json/example"
        value:
          users:
            - id: "{{uuid}}"
              name: "John Doe"
              email: "john@example.com"
            - id: "{{uuid}}"
              name: "Jane Smith"
              email: "jane@example.com"

  - targets: ["operation:getUser"]
    patch:
      - op: add
        path: "/responses/200/content/application~1json/example/profile"
        value:
          avatar: "https://example.com/avatar.jpg"
          bio: "User biography"
```

### Override Targeting

Target specific operations using different selectors:

```yaml
overrides:
  # By operation ID
  - targets: ["operation:listUsers", "operation:createUser"]
    patch: [...]

  # By path pattern
  - targets: ["path:/users/*"]
    patch: [...]

  # By tag
  - targets: ["tag:Users"]
    patch: [...]

  # By regex
  - targets: ["regex:^/api/v[0-9]+/users$"]
    patch: [...]
```

### Patch Operations

Supported JSON patch operations:

```yaml
overrides:
  - targets: ["path:/users"]
    patch:
      # Add new fields
      - op: add
        path: "/responses/200/content/application~1json/example/metadata"
        value:
          total: 100
          page: 1

      # Replace existing values
      - op: replace
        path: "/responses/200/content/application~1json/example/users/0/name"
        value: "Updated Name"

      # Remove fields
      - op: remove
        path: "/responses/200/content/application~1json/example/users/1/email"

      # Copy values
      - op: copy
        from: "/responses/200/content/application~1json/example/users/0/id"
        path: "/responses/200/content/application~1json/example/primaryUserId"

      # Move values
      - op: move
        from: "/responses/200/content/application~1json/example/temp"
        path: "/responses/200/content/application~1json/example/permanent"
```

### Conditional Overrides

Apply overrides based on request conditions:

```yaml
overrides:
  - targets: ["path:/users"]
    when: "request.query.format == 'detailed'"
    patch:
      - op: add
        path: "/responses/200/content/application~1json/example/users/0/profile"
        value:
          bio: "Detailed user profile"
          preferences: {}

  - targets: ["path:/users"]
    when: "request.header.X-API-Version == 'v2'"
    patch:
      - op: add
        path: "/responses/200/content/application~1json/example/apiVersion"
        value: "v2"
```

### Override Modes

Control how patches are applied:

```yaml
overrides:
  # Replace mode (default) - complete replacement
  - targets: ["path:/users"]
    mode: replace
    patch: [...]

  # Merge mode - deep merge objects and arrays
  - targets: ["path:/users"]
    mode: merge
    patch:
      - op: add
        path: "/responses/200/content/application~1json/example"
        value:
          additionalField: "value"
```

## Response Plugins

Create custom response generation logic using MockForge's plugin system.

### Response Generator Plugin

Implement the `ResponsePlugin` trait for complete response control:

```rust
use mockforge_plugin_core::*;

pub struct CustomResponsePlugin;

#[async_trait::async_trait]
impl ResponsePlugin for CustomResponsePlugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            network: NetworkCapabilities {
                allow_http_outbound: true,
                allowed_hosts: vec!["api.example.com".to_string()],
            },
            filesystem: FilesystemCapabilities::default(),
            resources: PluginResources {
                max_memory_bytes: 50 * 1024 * 1024,
                max_cpu_time_ms: 5000,
            },
            custom: HashMap::new(),
        }
    }

    async fn initialize(&self, config: &ResponsePluginConfig) -> Result<()> {
        // Plugin initialization
        Ok(())
    }

    async fn can_handle(
        &self,
        _context: &PluginContext,
        request: &ResponseRequest,
        _config: &ResponsePluginConfig,
    ) -> Result<PluginResult<bool>> {
        // Check if this plugin should handle the request
        let should_handle = request.path.starts_with("/api/custom/");
        Ok(PluginResult::success(should_handle, 0))
    }

    async fn generate_response(
        &self,
        _context: &PluginContext,
        request: &ResponseRequest,
        _config: &ResponsePluginConfig,
    ) -> Result<PluginResult<ResponseData>> {
        // Generate custom response
        match request.path.as_str() {
            "/api/custom/weather" => {
                let weather_data = serde_json::json!({
                    "temperature": 22,
                    "condition": "sunny",
                    "location": request.query_param("location").unwrap_or("Unknown")
                });
                Ok(PluginResult::success(
                    ResponseData::json(200, &weather_data)?,
                    0
                ))
            }
            "/api/custom/time" => {
                let time_data = serde_json::json!({
                    "current_time": chrono::Utc::now().to_rfc3339(),
                    "timezone": request.query_param("tz").unwrap_or("UTC")
                });
                Ok(PluginResult::success(
                    ResponseData::json(200, &time_data)?,
                    0
                ))
            }
            _ => Ok(PluginResult::success(
                ResponseData::not_found("Custom endpoint not found"),
                0
            ))
        }
    }

    fn priority(&self) -> i32 { 100 }

    fn validate_config(&self, _config: &ResponsePluginConfig) -> Result<()> {
        Ok(())
    }

    fn supported_content_types(&self) -> Vec<String> {
        vec!["application/json".to_string()]
    }
}
```

### Plugin Configuration

Configure response plugins in your MockForge setup:

```yaml
# plugin.yaml
name: custom-response-plugin
version: "1.0.0"
type: response

config:
  enabled: true
  priority: 100
  content_types:
    - "application/json"
  url_patterns:
    - "/api/custom/*"
  methods:
    - "GET"
    - "POST"
  settings:
    external_api_timeout: 5000
    cache_enabled: true
```

### Response Modifier Plugin

Modify responses after generation using the `ResponseModifierPlugin` trait:

```rust
use mockforge_plugin_core::*;

pub struct ResponseModifierPlugin;

#[async_trait::async_trait]
impl ResponseModifierPlugin for ResponseModifierPlugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::default()
    }

    async fn initialize(&self, _config: &ResponseModifierConfig) -> Result<()> {
        Ok(())
    }

    async fn should_modify(
        &self,
        _context: &PluginContext,
        _request: &ResponseRequest,
        response: &ResponseData,
        _config: &ResponseModifierConfig,
    ) -> Result<PluginResult<bool>> {
        // Modify successful JSON responses
        let should_modify = response.status_code == 200 &&
                           response.content_type == "application/json";
        Ok(PluginResult::success(should_modify, 0))
    }

    async fn modify_response(
        &self,
        _context: &PluginContext,
        _request: &ResponseRequest,
        mut response: ResponseData,
        _config: &ResponseModifierConfig,
    ) -> Result<PluginResult<ResponseData>> {
        // Add custom headers
        response.headers.insert(
            "X-Custom-Header".to_string(),
            "Modified by plugin".to_string()
        );

        // Add metadata to JSON responses
        if let Some(json_str) = response.body_as_string() {
            if let Ok(mut json_value) = serde_json::from_str::<serde_json::Value>(&json_str) {
                if let Some(obj) = json_value.as_object_mut() {
                    obj.insert("_metadata".to_string(), serde_json::json!({
                        "modified_by": "ResponseModifierPlugin",
                        "timestamp": chrono::Utc::now().timestamp()
                    }));
                }

                let modified_body = serde_json::to_vec(&json_value)
                    .map_err(|e| PluginError::execution(format!("JSON serialization error: {}", e)))?;
                response.body = modified_body;
            }
        }

        Ok(PluginResult::success(response, 0))
    }

    fn priority(&self) -> i32 { 50 }

    fn validate_config(&self, _config: &ResponseModifierConfig) -> Result<()> {
        Ok(())
    }
}
```

## Template Plugins

Extend MockForge's templating system with custom functions.

### Custom Template Functions

```rust
use mockforge_plugin_core::*;

pub struct BusinessTemplatePlugin;

impl TemplatePlugin for BusinessTemplatePlugin {
    fn execute_function(
        &mut self,
        function_name: &str,
        args: &[TemplateArg],
        _context: &PluginContext,
    ) -> PluginResult<String> {
        match function_name {
            "business_id" => {
                // Generate business-specific ID
                let id = format!("BIZ-{:010}", rand::random::<u32>());
                PluginResult::success(id, 0)
            }
            "department_name" => {
                // Generate department name
                let departments = ["Engineering", "Sales", "Marketing", "HR", "Finance"];
                let dept = departments[rand::random::<usize>() % departments.len()];
                PluginResult::success(dept.to_string(), 0)
            }
            "employee_data" => {
                // Generate complete employee object
                let employee = serde_json::json!({
                    "id": format!("EMP-{:06}", rand::random::<u32>() % 1000000),
                    "name": "{{faker.name}}",
                    "department": "{{department_name}}",
                    "salary": rand::random::<u32>() % 50000 + 50000,
                    "hire_date": "{{faker.date.past 365}}"
                });
                PluginResult::success(employee.to_string(), 0)
            }
            _ => PluginResult::failure(
                format!("Unknown function: {}", function_name),
                0
            )
        }
    }

    fn get_available_functions(&self) -> Vec<TemplateFunction> {
        vec![
            TemplateFunction {
                name: "business_id".to_string(),
                description: "Generate a business ID".to_string(),
                args: vec![],
                return_type: "string".to_string(),
            },
            TemplateFunction {
                name: "department_name".to_string(),
                description: "Generate a department name".to_string(),
                args: vec![],
                return_type: "string".to_string(),
            },
            TemplateFunction {
                name: "employee_data".to_string(),
                description: "Generate complete employee data".to_string(),
                args: vec![],
                return_type: "json".to_string(),
            },
        ]
    }

    fn get_capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::default()
    }

    fn health_check(&self) -> PluginHealth {
        PluginHealth::healthy("Template plugin healthy".to_string(), PluginMetrics::default())
    }
}
```

### Using Custom Templates

```yaml
# OpenAPI spec with custom templates
paths:
  /employees:
    get:
      responses:
        '200':
          content:
            application/json:
              example:
                employees:
                  - "{{employee_data}}"
                  - "{{employee_data}}"
                business_id: "{{business_id}}"
```

## Configuration-Based Custom Responses

Define custom responses directly in configuration files.

### Route-Specific Responses

```yaml
# mockforge.yaml
http:
  port: 3000
  routes:
    - path: /api/custom/dashboard
      method: GET
      response:
        status: 200
        headers:
          Content-Type: application/json
          X-Custom-Header: Dashboard-Data
        body: |
          {
            "widgets": [
              {
                "id": "sales-chart",
                "type": "chart",
                "data": [120, 150, 180, 200, 250]
              },
              {
                "id": "user-stats",
                "type": "stats",
                "data": {
                  "total_users": 15420,
                  "active_users": 8920,
                  "new_signups": 245
                }
              }
            ],
            "last_updated": "{{now}}"
          }

    - path: /api/custom/report
      method: POST
      response:
        status: 201
        headers:
          Location: /api/reports/123
        body: |
          {
            "report_id": "RPT-{{randInt 1000 9999}}",
            "status": "processing",
            "estimated_completion": "{{now+5m}}"
          }
```

### Dynamic Route Matching

```yaml
routes:
  # Path parameters
  - path: /api/users/{userId}/profile
    method: GET
    response:
      status: 200
      body: |
        {
          "user_id": "{{request.path.userId}}",
          "name": "{{faker.name}}",
          "email": "{{faker.email}}",
          "profile": {
            "bio": "{{faker.sentence}}",
            "location": "{{faker.city}}, {{faker.country}}"
          }
        }

  # Query parameter conditions
  - path: /api/search
    method: GET
    response:
      status: 200
      body: |
        {{#if (eq request.query.type 'users')}}
        {
          "results": [
            {"id": 1, "name": "John", "type": "user"},
            {"id": 2, "name": "Jane", "type": "user"}
          ]
        }
        {{else if (eq request.query.type 'posts')}}
        {
          "results": [
            {"id": 1, "title": "Post 1", "type": "post"},
            {"id": 2, "title": "Post 2", "type": "post"}
          ]
        }
        {{else}}
        {
          "results": [],
          "message": "No results found for type: {{request.query.type}}"
        }
        {{/if}}
```

## Error Response Customization

Create sophisticated error responses for different scenarios.

### Structured Error Responses

```yaml
routes:
  - path: /api/users/{userId}
    method: GET
    response:
      status: 404
      headers:
        Content-Type: application/json
      body: |
        {
          "error": {
            "code": "USER_NOT_FOUND",
            "message": "User with ID {{request.path.userId}} not found",
            "details": {
              "user_id": "{{request.path.userId}}",
              "requested_at": "{{now}}",
              "request_id": "{{uuid}}"
            },
            "suggestions": [
              "Check if the user ID is correct",
              "Verify the user exists in the system",
              "Try searching by email instead"
            ]
          }
        }

  - path: /api/orders
    method: POST
    response:
      status: 422
      body: |
        {
          "error": {
            "code": "VALIDATION_ERROR",
            "message": "Request validation failed",
            "validation_errors": [
              {
                "field": "customer_email",
                "code": "invalid_format",
                "message": "Email format is invalid"
              },
              {
                "field": "order_items",
                "code": "min_items",
                "message": "At least one order item is required"
              }
            ]
          }
        }
```

### Conditional Error Responses

```yaml
routes:
  - path: /api/payments
    method: POST
    response:
      status: 402
      condition: "request.header.X-Test-Mode == 'insufficient_funds'"
      body: |
        {
          "error": "INSUFFICIENT_FUNDS",
          "message": "Payment failed due to insufficient funds",
          "details": {
            "available_balance": 50.00,
            "requested_amount": 100.00,
            "currency": "USD"
          }
        }

  - path: /api/payments
    method: POST
    response:
      status: 500
      condition: "request.header.X-Test-Mode == 'server_error'"
      body: |
        {
          "error": "INTERNAL_SERVER_ERROR",
          "message": "An unexpected error occurred while processing payment",
          "reference_id": "ERR-{{randInt 100000 999999}}",
          "timestamp": "{{now}}"
        }
```

## Advanced Response Features

### Response Delays and Latency

```yaml
routes:
  - path: /api/slow-endpoint
    method: GET
    response:
      status: 200
      delay_ms: 2000  # 2 second delay
      body: |
        {
          "message": "This response was delayed",
          "timestamp": "{{now}}"
        }

  - path: /api/variable-delay
    method: GET
    response:
      status: 200
      delay_ms: "{{randInt 100 5000}}"  # Random delay between 100ms-5s
      body: |
        {
          "message": "Random delay applied",
          "delay_applied_ms": "{{_delay_ms}}"
        }
```

### Response Caching

```yaml
routes:
  - path: /api/cached-data
    method: GET
    response:
      status: 200
      headers:
        Cache-Control: max-age=300
        X-Cache-Status: "{{_cache_hit ? 'HIT' : 'MISS'}}"
      cache: true
      cache_ttl_seconds: 300
      body: |
        {
          "data": "This response may be cached",
          "generated_at": "{{now}}",
          "cache_expires_at": "{{now+5m}}"
        }
```

### Binary Response Handling

```yaml
routes:
  - path: /api/download/{filename}
    method: GET
    response:
      status: 200
      headers:
        Content-Type: application/octet-stream
        Content-Disposition: attachment; filename="{{request.path.filename}}"
      body_file: "/path/to/binary/files/{{request.path.filename}}"

  - path: /api/images/{imageId}
    method: GET
    response:
      status: 200
      headers:
        Content-Type: image/png
        Cache-Control: max-age=3600
      body_base64: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg=="
```

## Testing Custom Responses

### Manual Testing

```bash
# Test custom route
curl http://localhost:3000/api/custom/dashboard

# Test with parameters
curl "http://localhost:3000/api/users/123/profile"

# Test error conditions
curl -H "X-Test-Mode: insufficient_funds" \
     http://localhost:3000/api/payments \
     -X POST \
     -d '{}'
```

### Automated Testing

```bash
#!/bin/bash
# test-custom-responses.sh

BASE_URL="http://localhost:3000"

echo "Testing custom responses..."

# Test dashboard endpoint
DASHBOARD_RESPONSE=$(curl -s $BASE_URL/api/custom/dashboard)
echo "Dashboard response:"
echo $DASHBOARD_RESPONSE | jq '.'

# Test user profile with path parameter
USER_RESPONSE=$(curl -s $BASE_URL/api/users/456/profile)
echo "User profile response:"
echo $USER_RESPONSE | jq '.'

# Test error responses
ERROR_RESPONSE=$(curl -s -H "X-Test-Mode: insufficient_funds" \
                      -X POST \
                      -d '{}' \
                      $BASE_URL/api/payments)
echo "Error response:"
echo $ERROR_RESPONSE | jq '.'

echo "Custom response tests completed!"
```

## Best Practices

### Plugin Development

1. **Resource Limits**: Set appropriate memory and CPU limits for plugins
2. **Error Handling**: Implement proper error handling and logging
3. **Testing**: Thoroughly test plugins with various inputs
4. **Documentation**: Document plugin capabilities and configuration options

### Override Usage

1. **Selective Application**: Use specific targets to avoid unintended modifications
2. **Version Control**: Keep override configurations in version control
3. **Testing**: Test overrides with different request scenarios
4. **Performance**: Minimize complex conditions and patch operations

### Response Design

1. **Consistency**: Maintain consistent response formats across endpoints
2. **Error Details**: Provide meaningful error messages and codes
3. **Metadata**: Include relevant metadata like timestamps and request IDs
4. **Content Types**: Set appropriate Content-Type headers

### Security Considerations

1. **Input Validation**: Validate all inputs in custom plugins
2. **Resource Limits**: Prevent resource exhaustion attacks
3. **Authentication**: Implement proper authentication for sensitive endpoints
4. **Logging**: Log security-relevant events without exposing sensitive data

## Troubleshooting

### Plugin Issues

**Plugin not loading**: Check plugin configuration and file paths
**Plugin timeout**: Increase resource limits or optimize plugin code
**Plugin errors**: Check plugin logs and error messages

### Override Problems

**Overrides not applying**: Verify target selectors and patch syntax
**JSON patch errors**: Validate patch operations against JSON structure
**Condition evaluation**: Test conditional expressions with sample requests

### Performance Issues

**Slow responses**: Profile plugin execution and optimize bottlenecks
**Memory usage**: Monitor plugin memory consumption and adjust limits
**Template expansion**: Simplify complex templates or use static responses

For basic HTTP mocking features, see the [HTTP Mocking guide](../http-mocking.md). For advanced templating, see the [Dynamic Data guide](dynamic-data.md).