# Dynamic Data

MockForge provides powerful dynamic data generation capabilities through its templating system and faker integration. This guide covers generating realistic, varied responses for comprehensive API testing and development.

## Template Expansion Basics

MockForge uses a lightweight templating system with `{{token}}` syntax to inject dynamic values into responses.

### Enabling Templates

Templates are disabled by default for security. Enable them using:

```bash
# Environment variable
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true mockforge serve --spec api-spec.json

# Configuration file
http:
  response_template_expand: true
```

### Basic Template Syntax

```yaml
paths:
  /users:
    get:
      responses:
        '200':
          content:
            application/json:
              example:
                users:
                  - id: "{{uuid}}"
                    name: "{{faker.name}}"
                    email: "{{faker.email}}"
                    created_at: "{{now}}"
```

## Time-Based Templates

Generate timestamps and time offsets for realistic temporal data.

### Current Time

```yaml
responses:
  '200':
    content:
      application/json:
        example:
          current_time: "{{now}}"
          server_timestamp: "{{now}}"
```

### Time Offsets

```yaml
responses:
  '200':
    content:
      application/json:
        example:
          created_at: "{{now-7d}}"
          expires_at: "{{now+1h}}"
          last_login: "{{now-30m}}"
          scheduled_for: "{{now+2h}}"
```

**Supported units:**
- `s` - seconds
- `m` - minutes
- `h` - hours
- `d` - days

## Random Data Generation

Generate random values for varied test data.

### Random Integers

```yaml
responses:
  '200':
    content:
      application/json:
        example:
          user_count: "{{randInt 1 100}}"
          age: "{{randInt 18 80}}"
          score: "{{randInt -10 10}}"
```

### Random Floats

```yaml
responses:
  '200':
    content:
      application/json:
        example:
          price: "{{randFloat 9.99 999.99}}"
          rating: "{{randFloat 1.0 5.0}}"
          percentage: "{{randFloat 0.0 100.0}}"
```

## UUID Generation

Generate unique identifiers for entities.

```yaml
responses:
  '200':
    content:
      application/json:
        example:
          id: "{{uuid}}"
          order_id: "{{uuid}}"
          transaction_id: "{{uuid}}"
```

## Faker Data Generation

Generate realistic fake data using the Faker library.

### Basic Faker Functions

```yaml
responses:
  '200':
    content:
      application/json:
        example:
          user:
            id: "{{uuid}}"
            name: "{{faker.name}}"
            email: "{{faker.email}}"
            created_at: "{{now}}"
```

### Extended Faker Functions

When the `data-faker` feature is enabled, additional functions are available:

```yaml
responses:
  '200':
    content:
      application/json:
        example:
          user:
            name: "{{faker.name}}"
            email: "{{faker.email}}"
            phone: "{{faker.phone}}"
            address: "{{faker.address}}"
            company: "{{faker.company}}"
          product:
            name: "{{faker.word}}"
            description: "{{faker.sentence}}"
            color: "{{faker.color}}"
            url: "{{faker.url}}"
            ip_address: "{{faker.ip}}"
```

### Disabling Faker

For deterministic testing, disable faker tokens:

```bash
MOCKFORGE_FAKE_TOKENS=false mockforge serve --spec api-spec.json
```

## Request Data Access

Access data from incoming requests to create dynamic responses.

### Path Parameters

```yaml
paths:
  /users/{userId}:
    get:
      parameters:
        - name: userId
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          content:
            application/json:
              example:
                id: "{{request.path.userId}}"
                name: "User {{request.path.userId}}"
                retrieved_at: "{{now}}"
```

### Query Parameters

```yaml
paths:
  /users:
    get:
      parameters:
        - name: limit
          in: query
          schema:
            type: integer
            default: 10
        - name: format
          in: query
          schema:
            type: string
            enum: [brief, detailed]
      responses:
        '200':
          content:
            application/json:
              example: |
                {{#if (eq request.query.format 'detailed')}}
                {
                  "users": [
                    {
                      "id": "{{uuid}}",
                      "name": "{{faker.name}}",
                      "email": "{{faker.email}}",
                      "profile": {
                        "bio": "{{faker.sentence}}",
                        "location": "{{faker.address}}"
                      }
                    }
                  ],
                  "limit": {{request.query.limit}},
                  "format": "{{request.query.format}}"
                }
                {{else}}
                {
                  "users": [
                    {
                      "id": "{{uuid}}",
                      "name": "{{faker.name}}",
                      "email": "{{faker.email}}"
                    }
                  ],
                  "limit": {{request.query.limit}}
                }
                {{/if}}
```

### Request Body Access

```yaml
paths:
  /users:
    post:
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              properties:
                name:
                  type: string
                email:
                  type: string
      responses:
        '201':
          content:
            application/json:
              example:
                id: "{{uuid}}"
                name: "{{request.body.name}}"
                email: "{{request.body.email}}"
                created_at: "{{now}}"
                welcome_message: "Welcome {{request.body.name}}!"
```

### Headers Access

```yaml
responses:
  '200':
    content:
      application/json:
        example:
          user_agent: "{{request.header.User-Agent}}"
          api_version: "{{request.header.X-API-Version}}"
          authorization: "{{request.header.Authorization}}"
```

## Conditional Templates

Use Handlebars-style conditionals for complex logic.

### Basic Conditionals

```yaml
responses:
  '200':
    content:
      application/json:
        example: |
          {{#if (eq request.query.format 'detailed')}}
          {
            "data": {
              "id": "{{uuid}}",
              "name": "{{faker.name}}",
              "details": {
                "bio": "{{faker.paragraph}}",
                "stats": {
                  "login_count": {{randInt 1 1000}},
                  "last_active": "{{now-1d}}"
                }
              }
            }
          }
          {{else}}
          {
            "data": {
              "id": "{{uuid}}",
              "name": "{{faker.name}}"
            }
          }
          {{/if}}
```

### Multiple Conditions

```yaml
responses:
  '200':
    content:
      application/json:
        example: |
          {{#if (eq request.query.type 'admin')}}
          {
            "user": {
              "id": "{{uuid}}",
              "name": "{{faker.name}}",
              "role": "admin",
              "permissions": ["read", "write", "delete", "admin"]
            }
          }
          {{else if (eq request.query.type 'premium')}}
          {
            "user": {
              "id": "{{uuid}}",
              "name": "{{faker.name}}",
              "role": "premium",
              "permissions": ["read", "write"]
            }
          }
          {{else}}
          {
            "user": {
              "id": "{{uuid}}",
              "name": "{{faker.name}}",
              "role": "basic",
              "permissions": ["read"]
            }
          }
          {{/if}}
```

## Data Generation Templates

MockForge includes built-in data generation templates for common entities.

### User Template

```bash
# Generate user data
mockforge data template user --rows 10 --format json

# Output:
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "john.doe@example.com",
    "name": "John Doe",
    "created_at": "2024-01-15T10:30:00Z",
    "active": true
  }
]
```

### Product Template

```bash
# Generate product data
mockforge data template product --rows 5 --format csv

# Output:
id,name,description,price,category,in_stock
550e8400-e29b-41d4-a716-446655440001,Wireless Headphones,High-quality wireless headphones with noise cancellation,199.99,Electronics,true
```

### Order Template

```bash
# Generate order data with relationships
mockforge data template order --rows 3 --format json --rag

# Output:
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440002",
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "total_amount": 299.97,
    "status": "completed",
    "created_at": "2024-01-16T14:20:00Z"
  }
]
```

## Advanced Templating Features

### Encryption Functions

Secure sensitive data in responses:

```yaml
responses:
  '200':
    content:
      application/json:
        example:
          user:
            id: "{{uuid}}"
            name: "{{encrypt 'user_name' faker.name}}"
            email: "{{encrypt 'user_email' faker.email}}"
            ssn: "{{encrypt 'sensitive' '123-45-6789'}}"
```

### Decryption

Access encrypted data:

```yaml
# In templates that need to decrypt
decrypted_name: "{{decrypt 'user_name' request.body.encrypted_name}}"
```

### File System Access

Read external files for dynamic content:

```yaml
responses:
  '200':
    content:
      application/json:
        example:
          config: "{{fs.readFile 'config.json'}}"
          template: "{{fs.readFile 'templates/welcome.html'}}"
```

## Request Chaining Context

Access data from previous requests in chained scenarios.

### Chain Variables

```yaml
# In chained request templates
responses:
  '200':
    content:
      application/json:
        example:
          previous_request_id: "{{chain.request_id}}"
          previous_user_id: "{{chain.user.id}}"
          session_token: "{{chain.auth.token}}"
```

## Custom Template Plugins

Extend templating with custom functions via plugins.

### Template Plugin Example

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
                let id = format!("BIZ-{:010}", rand::random::<u32>());
                PluginResult::success(id, 0)
            }
            "department" => {
                let depts = ["Engineering", "Sales", "Marketing", "HR"];
                let dept = depts[rand::random::<usize>() % depts.len()];
                PluginResult::success(dept.to_string(), 0)
            }
            "salary" => {
                let salary = rand::random::<u32>() % 150000 + 50000;
                PluginResult::success(salary.to_string(), 0)
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
                description: "Generate business ID".to_string(),
                args: vec![],
                return_type: "string".to_string(),
            },
            TemplateFunction {
                name: "department".to_string(),
                description: "Generate department name".to_string(),
                args: vec![],
                return_type: "string".to_string(),
            },
            TemplateFunction {
                name: "salary".to_string(),
                description: "Generate salary amount".to_string(),
                args: vec![],
                return_type: "string".to_string(),
            },
        ]
    }

    fn get_capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::default()
    }

    fn health_check(&self) -> PluginHealth {
        PluginHealth::healthy("Business template plugin healthy".to_string(), PluginMetrics::default())
    }
}
```

### Using Custom Templates

```yaml
responses:
  '200':
    content:
      application/json:
        example:
          employee:
            id: "{{business_id}}"
            name: "{{faker.name}}"
            department: "{{department}}"
            salary: "{{salary}}"
            hire_date: "{{now-1y}}"
```

## Configuration and Security

### Template Security Settings

```yaml
# mockforge.yaml
http:
  response_template_expand: true
  template_security:
    allow_file_access: false
    allow_encryption: true
    max_template_depth: 10
    timeout_ms: 5000
```

### Environment Variables

```bash
# Enable template expansion
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true

# Disable faker for deterministic tests
MOCKFORGE_FAKE_TOKENS=false

# Set validation status for template errors
MOCKFORGE_VALIDATION_STATUS=422

# Control template execution timeout
MOCKFORGE_TEMPLATE_TIMEOUT_MS=5000
```

## Testing with Dynamic Data

### Manual Testing

```bash
# Test template expansion
curl http://localhost:3000/users

# Test with query parameters
curl "http://localhost:3000/users?format=detailed&limit=5"

# Test path parameters
curl http://localhost:3000/users/123

# Test POST with body access
curl -X POST http://localhost:3000/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Test User", "email": "test@example.com"}'
```

### Automated Testing

```bash
#!/bin/bash
# test-dynamic-data.sh

BASE_URL="http://localhost:3000"

echo "Testing dynamic data generation..."

# Test basic templates
USER_RESPONSE=$(curl -s $BASE_URL/users)
echo "User response with templates:"
echo $USER_RESPONSE | jq '.'

# Test conditional templates
DETAILED_RESPONSE=$(curl -s "$BASE_URL/users?format=detailed")
echo "Detailed format response:"
echo $DETAILED_RESPONSE | jq '.'

BASIC_RESPONSE=$(curl -s "$BASE_URL/users?format=basic")
echo "Basic format response:"
echo $BASIC_RESPONSE | jq '.'

# Test faker data
PRODUCT_RESPONSE=$(curl -s $BASE_URL/products)
echo "Product response with faker data:"
echo $PRODUCT_RESPONSE | jq '.'

echo "Dynamic data tests completed!"
```

## Best Practices

### Template Usage

1. **Enable Selectively**: Only enable template expansion where needed for security
2. **Validate Input**: Sanitize request data used in templates
3. **Test Thoroughly**: Test template expansion with various inputs
4. **Monitor Performance**: Templates add processing overhead

### Data Generation

1. **Use Appropriate Faker**: Choose faker functions that match your domain
2. **Maintain Consistency**: Use consistent data patterns across endpoints
3. **Consider Relationships**: Generate related data that makes sense together
4. **Balance Realism**: Generate realistic but not sensitive data

### Security Considerations

1. **Input Sanitization**: Never trust request data in templates
2. **File Access**: Disable file system access in production if not needed
3. **Encryption**: Use encryption functions for sensitive data
4. **Rate Limiting**: Consider rate limiting for expensive template operations

### Performance Optimization

1. **Cache Static Parts**: Cache template parsing for frequently used templates
2. **Limit Complexity**: Avoid deeply nested conditionals and complex logic
3. **Profile Execution**: Monitor template execution time and optimize slow functions
4. **Use Appropriate Timeouts**: Set reasonable timeouts for template execution

## Troubleshooting

### Template Not Expanding

**Problem**: Templates appear as literal text in responses

**Solutions**:
```bash
# Enable template expansion
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true mockforge serve --spec api-spec.json

# Check configuration
# Ensure response_template_expand: true in config
```

### Faker Functions Not Working

**Problem**: Faker functions return empty or error values

**Solutions**:
```bash
# Ensure faker is enabled
MOCKFORGE_FAKE_TOKENS=true mockforge serve --spec api-spec.json

# Check if data-faker feature is enabled
# For extended faker functions, ensure the feature is compiled in
```

### Request Data Access Issues

**Problem**: `request.*` variables are empty or undefined

**Solutions**:
- Verify request format (JSON for body access)
- Check parameter names match exactly
- Ensure path/query parameters are properly defined in OpenAPI spec

### Performance Issues

**Problem**: Template expansion is slow

**Solutions**:
- Simplify template logic
- Cache frequently used values
- Use static responses where dynamic data isn't needed
- Profile and optimize custom template functions

For basic HTTP mocking features, see the [HTTP Mocking guide](../http-mocking.md). For custom response generation, see the [Custom Responses guide](custom-responses.md).