# Request Chaining

MockForge supports request chaining, which allows you to create complex workflows where requests can depend on responses from previous requests in the chain. This is particularly useful for testing API workflows that require authentication, data flow between endpoints, or multi-step operations.

## Overview

Request chaining enables you to:

- Execute requests in a predefined sequence with dependencies
- Reference data from previous responses using template variables
- Extract and store specific values from responses for reuse
- Validate response status codes and content
- Implement parallel execution for independent requests
- Handle complex authentication and authorization flows

## Chain Definition

Chains are defined using YAML or JSON configuration files with the following structure:

```yaml
id: my-chain
name: My Chain
description: A description of what this chain does
config:
  enabled: true
  maxChainLength: 20
  globalTimeoutSecs: 300
  enableParallelExecution: false
links:
  # Define your requests here
  - request:
      id: step1
      method: POST
      url: https://api.example.com/auth/login
      headers:
        Content-Type: application/json
      body:
        username: "testuser"
        password: "password123"
    extract:
      token: body.access_token
    storeAs: login_response
    dependsOn: []
variables:
  base_url: https://api.example.com
tags:
  - authentication
  - workflow
```

## Chain Configuration

The `config` section controls how the chain behaves:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | boolean | `false` | Whether this chain is enabled |
| `maxChainLength` | integer | `20` | Maximum number of requests in the chain |
| `globalTimeoutSecs` | integer | `300` | Total timeout for chain execution |
| `enableParallelExecution` | boolean | `false` | Enable parallel execution of independent requests |

## Request Links

Each link in the chain defines a single HTTP request and its behavior:

### Request Definition

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | Unique identifier for this request |
| `method` | string | Yes | HTTP method (GET, POST, PUT, DELETE, etc.) |
| `url` | string | Yes | Request URL (supports template variables) |
| `headers` | object | No | Request headers |
| `body` | any | No | Request body (supports template variables) |
| `dependsOn` | array | No | List of request IDs this request depends on |
| `timeoutSecs` | number | No | Individual request timeout |
| `expectedStatus` | array | No | Expected status codes for validation |

### Response Processing

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `extract` | object | No | Extract values from response into variables |
| `storeAs` | string | No | Store entire response with this name |

## Template Variables

Chain requests support powerful templating that can reference:

### Previous Response Data

Use `{{chain.<response_name>.<path>}}` to reference data from previous responses:

```yaml
url: https://api.example.com/users/{{chain.login_response.body.user_id}}/posts
headers:
  Authorization: "Bearer {{chain.auth_response.body.access_token}}"
```

### Variable Extraction

Extract values from responses into reusable variables:

```yaml
extract:
  user_id: body.user.id
  token: body.access_token
storeAs: user_response
```

### Built-in Template Functions

All standard MockForge templating functions are available:

- `{{uuid}}` - Random UUID
- `{{faker.email}}` - Fake email address
- `{{faker.name}}` - Fake name
- `{{rand.int}}` - Random integer
- `{{now}}` - Current timestamp

## Advanced Features

### Dependency Resolution

Requests can depend on other requests using the `dependsOn` field. MockForge automatically resolves dependencies using topological sorting:

```yaml
links:
  - request:
      id: login
      method: POST
      url: https://api.example.com/auth/login
      body:
        username: "user"
        password: "pass"
    storeAs: auth

  - request:
      id: get_profile
      method: GET
      url: https://api.example.com/user/profile
      headers:
        Authorization: "Bearer {{chain.auth.body.token}}"
    dependsOn:
      - login
```

### Parallel Execution

Enable `enableParallelExecution: true` to allow independent requests to run simultaneously:

```yaml
config:
  enableParallelExecution: true
links:
  - request:
      id: get_profile
      method: GET
      url: https://api.example.com/profile
    dependsOn:
      - login

  - request:
      id: get_preferences
      method: GET
      url: https://api.example.com/preferences
    dependsOn:
      - login
  # These two requests will run in parallel
```

### Response Validation

Validate response status codes and content:

```yaml
links:
  - request:
      id: create_user
      method: POST
      url: https://api.example.com/users
      body:
        name: "John Doe"
    expectedStatus: [201, 202]  # Expect 201 or 202 status codes
```

## JSON Path Support

Chain templating supports JSON path syntax for accessing nested data:

### Simple Properties

```yaml
extract:
  user_id: body.id
  name: body.profile.name
```

### Array Access

```yaml
extract:
  first_user: body.users.[0].name
  user_count: body.users.[*]  # Get array length
```

### Complex Nesting

```yaml
url: https://api.example.com/users/{{chain.login_response.body.user.id}}/projects/{{chain.project_response.body.data.[0].id}}
```

## Response Function (New UI Feature)

MockForge also supports a `response()` function for use in the Admin UI and other editing contexts:

### Syntax

```javascript
response('request_name', 'json_path')
```

### Examples

```javascript
// Simple usage
response('login', 'body.user_id')

// Complex JSON path
response('user_profile', 'body.data.employee.name')

// Environment variable usage
let userId = response('login', 'body.user_id');
let updateUrl = `/users/${userId}/profile`;
```

### UI Integration

1. **Autocomplete**: Type `response(` in any input field in the UI and use Ctrl+Space for autocomplete
2. **Configuration Dialog**: Click the blue template tag next to the function to open the configuration dialog
3. **Request Selection**: Choose from available requests in the current chain
4. **Path Specification**: Enter the JSONPath to extract the desired value

## Pre/Post Request Scripting

MockForge supports JavaScript scripting for complex request processing and data manipulation in request chains.

### Enable Scripting

Add scripting configuration to any request in your chain:

```yaml
links:
  - request:
      id: process_data
      method: POST
      url: https://api.example.com/process
      scripting:
        pre_script: |
          // Execute before request
          console.log('Processing request with mockforge context');
          console.log('Request URL:', mockforge.request.url);

          if (mockforge.variables.skip_processing) {
            request.body.skip_processing = true;
          }
        post_script: |
          // Execute after request
          console.log('Request completed in', mockforge.response.duration_ms, 'ms');

          if (mockforge.response.status === 429) {
            throw new Error('Rate limited - retry needed');
          }

          // Store custom data for next request
          setVariable('processed_user_id', mockforge.response.body.user_id);
        runtime: javascript
        timeout_ms: 5000
```

### Pre-Scripts

Executed before the HTTP request:

```javascript
// Available context in mockforge object:
mockforge.request     // Current request (id, method, url, headers)
mockforge.chain       // Previous responses: mockforge.chain.login.body.user_id
mockforge.variables   // Chain variables
mockforge.env         // Environment variables

// Direct access to functions:
console.log('Starting request processing');

// Modify request before it goes out
if (mockforge.variables.enable_debug) {
  request.headers['X-Debug'] = 'true';
  request.body.debug_mode = true;
}

// Set variables for this request
setVariable('request_start_time', Date.now());

// Example: Add authentication from previous response
request.headers['Authorization'] = 'Bearer ' + mockforge.chain.login.body.token;
```

### Post-Scripts

Executed after the HTTP response:

```javascript
// Available context in mockforge object:
mockforge.response    // Current response (status, headers, body, duration_ms)
mockforge.request     // Original request
mockforge.chain       // Previous responses
mockforge.variables   // Chain variables
mockforge.env         // Environment variables

// Example: Validate response and extract data
if (mockforge.response.status !== 200) {
  throw new Error('Request failed with status ' + mockforge.response.status);
}

// Extract and store data for next requests
setVariable('user_profile', mockforge.response.body);
setVariable('session_cookie', mockforge.response.headers['Set-Cookie']);

// Example: Transform response data
if (mockforge.response.body && mockforge.response.body.user) {
  mockforge.response.body.processed_user = {
    fullName: mockforge.response.body.user.first_name + ' ' + mockforge.response.body.user.last_name,
    age: mockforge.response.body.user.age,
    isActive: mockforge.response.body.user.status === 'active'
  };
}
```

### Built-in Functions

#### Logging and Diagnostics

```javascript
console.log('Debug message:', mockforge.request.url);
console.warn('Warning:', mockforge.response.status);
console.error('Error occurred');
```

#### Variable Management

```javascript
// Set a variable for use in next requests
setVariable('api_token', mockforge.response.body.token);

// Access environment variables
const configUrl = mockforge.env['API_CONFIG_URL'];
```

#### Data Validation

```javascript
// Simple assertions
assert(mockforge.response.status === 200, 'Expected status 200');

// Complex validation
if (!mockforge.response.body || !mockforge.response.body.items) {
  throw new Error('Response missing required "items" field');
}

if (mockforge.response.body.items.length === 0) {
  console.warn('Response contains empty items array');
}
```

### Error Handling

Scripts can throw errors to fail the chain:

```javascript
if (mockforge.response.status >= 400) {
  throw new Error('HTTP ' + mockforge.response.status + ': ' + mockforge.response.body.error);
}

if (mockforge.response.duration_ms > 30000) {
  throw new Error('Request took too long: ' + mockforge.response.duration_ms + 'ms');
}
```

### Security and Isolation

- **Timeout Protection**: Scripts are limited by `timeout_ms` (default: 5 seconds)
- **Sandboxing**: Scripts run in isolated JavaScript contexts
- **Resource Limits**: CPU and memory usage is monitored and limited
- **Network Restrictions**: Scripts cannot make outbound network calls
- **File System Access**: Read-only file access through `fs.readFile()` function

### Best Practices

1. **Keep Scripts Simple**: Break complex logic into smaller, focused scripts
2. **Validate Inputs**: Always check that expected data exists before processing
3. **Set Appropriate Timeouts**: Use shorter timeouts for simple scripts
4. **Use Environment Variables**: Store configuration in environment variables
5. **Error Handling**: Always check for error conditions and fail fast when needed
6. **Documentation**: Comment complex business logic in your scripts
7. **Testing**: Test scripts with various response scenarios

### Environment Variables

For multiple uses of the same response value, store it in an environment variable:

```javascript
// In environment variables
RESPONSE_USER_ID = response('login', 'body.user_id')

// Then use in multiple places
let url1 = `/users/${RESPONSE_USER_ID}`;
let url2 = `/profile/${RESPONSE_USER_ID}`;
```

### Benefits Over Traditional Templates

- **Cleaner Syntax**: More readable than `{{chain.request_name.body.path}}`
- **Type Safety**: JSONPath validation in the UI
- **Better UX**: Visual configuration through dialogs
- **Autocomplete**: Intelligent suggestions for request names and paths

## Error Handling

Chains provide comprehensive error handling:

- **Dependency errors**: Missing or invalid dependencies
- **Circular dependencies**: Automatic detection and prevention
- **Timeout errors**: Individual and global timeouts
- **Status validation**: Expected status code validation
- **Network errors**: Connection and HTTP errors

## Chain Management

Chains can be managed programmatically or via configuration files:

### Loading Chains

```rust
use mockforge_core::RequestChainRegistry;

let registry = RequestChainRegistry::new(chain_config);
// Load from YAML
registry.register_from_yaml(yaml_content).await?;
// Load from JSON
registry.register_from_json(json_content).await?;
```

### Executing Chains

```rust
use mockforge_core::ChainExecutionEngine;

let engine = ChainExecutionEngine::new(registry, config);
// Execute a chain
let result = engine.execute_chain("my-chain").await?;
println!("Chain executed in {}ms", result.total_duration_ms);
```

## Complete Example

See the provided examples in the `examples/` directory:

- `examples/chain-example.yaml` - Comprehensive user management workflow
- `examples/simple-chain.json` - Simple authentication chain

## Working With Large Values

MockForge provides several strategies to handle large values efficiently without affecting performance or crashing the user interface. The system automatically hides large text values by default, but extremely large values can still impact performance.

### File System Template Functions

MockForge supports the `fs.readFile()` template function for reading file contents directly into templates. This is particularly useful for including large text content within structured data.

**Syntax:**
```yaml
{{fs.readFile "path/to/file.txt"}}
{{fs.readFile('path/to/file.txt')}}
```

**Example usage in request chaining:**
```yaml
links:
  - request:
      id: upload_large_data
      method: POST
      url: https://api.example.com/upload
      headers:
        Content-Type: application/json
      body:
        metadata:
          filename: "large_document.txt"
          size: 1048576
        content: "{{fs.readFile('/path/to/large/file.txt')}}"
```

**Error handling:**
- If the file doesn't exist: `<fs.readFile error: No such file or directory (os error 2)>`
- If the path is empty: `<fs.readFile: empty path>`

### Binary File Request Bodies

For truly large binary files (images, videos, documents), MockForge supports binary file request bodies that reference files on disk rather than loading them into memory.

**YAML Configuration:**
```yaml
links:
  - request:
      id: upload_image
      method: POST
      url: https://api.example.com/upload
      body:
        type: binary_file
        data:
          path: "/path/to/image.jpg"
          content_type: "image/jpeg"
```

**JSON Configuration:**
```json
{
  "id": "upload_image",
  "method": "POST",
  "url": "https://api.example.com/upload",
  "body": {
    "type": "binary_file",
    "data": {
      "path": "/path/to/image.jpg",
      "content_type": "image/jpeg"
    }
  }
}
```

**Key Features:**
- **Path templating**: File paths support template expansion (e.g., `"{{chain.previous_response.body.file_path}}"`)
- **Content type**: Optional content-type header (defaults to none for binary files)
- **Memory efficient**: Files are read only when the request is executed
- **Error handling**: Clear error messages for missing files

### Performance Best Practices

1. **Use binary_file for large binary content** (images, videos, large documents)
2. **Use fs.readFile for large text content** within structured JSON/XML bodies
3. **Template file paths** to make configurations dynamic
4. **Validate file paths** before running chains to avoid runtime errors
5. **Consider file size limits** based on your system's memory constraints

## Best Practices

1. **Keep chains focused**: Each chain should have a single, clear purpose
2. **Use meaningful IDs**: Choose descriptive names for requests and chains
3. **Handle dependencies carefully**: Ensure dependency chains are logical and avoid cycles
4. **Validate responses**: Use `expectedStatus` and `extract` for critical paths
5. **Use parallel execution**: Enable for independent requests to improve performance
6. **Template effectively**: Leverage chain context variables for dynamic content
7. **Error handling**: Plan for failure scenarios in your chains
8. **Handle large values efficiently**: Use `fs.readFile()` for large text content and `binary_file` request bodies for large binary files to maintain performance

## Limitations

- Maximum chain length is configurable (default: 20 requests)
- Global execution timeout applies to entire chain
- Circular dependencies are automatically prevented
- Parallel execution requires careful dependency management
