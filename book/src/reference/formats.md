# Supported Formats

MockForge supports various data formats for configuration, specifications, and data exchange. This reference documents all supported formats, their usage, and conversion utilities.

## OpenAPI Specifications

### JSON Format (Primary)

MockForge primarily supports OpenAPI 3.0+ specifications in JSON format:

```json
{
  "openapi": "3.0.3",
  "info": {
    "title": "User API",
    "version": "1.0.0"
  },
  "paths": {
    "/users": {
      "get": {
        "summary": "List users",
        "responses": {
          "200": {
            "description": "Success",
            "content": {
              "application/json": {
                "schema": {
                  "type": "array",
                  "items": {
                    "$ref": "#/components/schemas/User"
                  }
                }
              }
            }
          }
        }
      }
    }
  },
  "components": {
    "schemas": {
      "User": {
        "type": "object",
        "properties": {
          "id": {"type": "string"},
          "name": {"type": "string"},
          "email": {"type": "string"}
        }
      }
    }
  }
}
```

### YAML Format (Alternative)

OpenAPI specifications can also be provided in YAML format:

```yaml
openapi: 3.0.3
info:
  title: User API
  version: 1.0.0
paths:
  /users:
    get:
      summary: List users
      responses:
        '200':
          description: Success
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/User'
components:
  schemas:
    User:
      type: object
      properties:
        id:
          type: string
        name:
          type: string
        email:
          type: string
```

### Conversion Between Formats

```bash
# Convert JSON to YAML
node -e "
const fs = require('fs');
const yaml = require('js-yaml');
const spec = JSON.parse(fs.readFileSync('api.json', 'utf8'));
fs.writeFileSync('api.yaml', yaml.dump(spec));
"

# Convert YAML to JSON
node -e "
const fs = require('fs');
const yaml = require('js-yaml');
const spec = yaml.load(fs.readFileSync('api.yaml', 'utf8'));
fs.writeFileSync('api.json', JSON.stringify(spec, null, 2));
"
```

## Protocol Buffers

### .proto Files

gRPC services use Protocol Buffer definitions:

```protobuf
syntax = "proto3";

package myapp.user;

service UserService {
  rpc GetUser(GetUserRequest) returns (User);
  rpc ListUsers(ListUsersRequest) returns (stream User);
  rpc CreateUser(CreateUserRequest) returns (User);
}

message GetUserRequest {
  string user_id = 1;
}

message User {
  string user_id = 1;
  string name = 2;
  string email = 3;
  google.protobuf.Timestamp created_at = 4;
}

message ListUsersRequest {
  int32 page_size = 1;
  string page_token = 2;
}

message CreateUserRequest {
  string name = 1;
  string email = 2;
}
```

### Generated Code

MockForge automatically generates Rust code from `.proto` files:

```rust
// Generated code structure
pub mod myapp {
    pub mod user {
        tonic::include_proto!("myapp.user");

        // Generated service trait
        #[tonic::async_trait]
        pub trait UserService: Send + Sync + 'static {
            async fn get_user(
                &self,
                request: tonic::Request<GetUserRequest>,
            ) -> Result<tonic::Response<User>, tonic::Status>;

            async fn list_users(
                &self,
                request: tonic::Request<ListUsersRequest>,
            ) -> Result<tonic::Response<Self::ListUsersStream>, tonic::Status>;
        }
    }
}
```

## WebSocket Replay Files

### JSONL Format

WebSocket interactions use JSON Lines format:

```jsonl
{"ts":0,"dir":"out","text":"Welcome to chat!","waitFor":"^HELLO$"}
{"ts":1000,"dir":"out","text":"How can I help you?"}
{"ts":2000,"dir":"out","text":"Please wait while I process your request..."}
{"ts":5000,"dir":"out","text":"Here's your response: ..."}
```

### Extended JSONL with Templates

```jsonl
{"ts":0,"dir":"out","text":"Session {{uuid}} started at {{now}}"}
{"ts":1000,"dir":"out","text":"Connected to server {{server_id}}"}
{"ts":2000,"dir":"out","text":"{{#if authenticated}}Welcome back!{{else}}Please authenticate{{/if}}"}
```

### Binary Message Support

```jsonl
{"ts":0,"dir":"out","text":"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==","binary":true}
{"ts":1000,"dir":"out","text":"Image data sent"}
```

## Configuration Files

### YAML Configuration

MockForge uses YAML for configuration files:

```yaml
# Server configuration
server:
  http_port: 3000
  ws_port: 3001
  grpc_port: 50051

# Validation settings
validation:
  mode: enforce
  aggregate_errors: false

# Response processing
response:
  template_expand: true

# Protocol-specific settings
grpc:
  proto_dir: "proto/"
  enable_reflection: true

websocket:
  replay_file: "examples/demo.jsonl"
```

### JSON Configuration (Alternative)

Configuration can also be provided as JSON:

```json
{
  "server": {
    "http_port": 3000,
    "ws_port": 3001,
    "grpc_port": 50051
  },
  "validation": {
    "mode": "enforce",
    "aggregate_errors": false
  },
  "response": {
    "template_expand": true
  },
  "grpc": {
    "proto_dir": "proto/",
    "enable_reflection": true
  },
  "websocket": {
    "replay_file": "examples/demo.jsonl"
  }
}
```

## Data Generation Formats

### JSON Output

Generated test data in JSON format:

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "John Doe",
    "email": "john.doe@example.com",
    "created_at": "2025-09-12T10:00:00Z"
  },
  {
    "id": "550e8400-e29b-41d4-a716-446655440001",
    "name": "Jane Smith",
    "email": "jane.smith@example.com",
    "created_at": "2025-09-12T11:00:00Z"
  }
]
```

### YAML Output

Same data in YAML format:

```yaml
- id: 550e8400-e29b-41d4-a716-446655440000
  name: John Doe
  email: john.doe@example.com
  created_at: '2025-09-12T10:00:00Z'
- id: 550e8400-e29b-41d4-a716-446655440001
  name: Jane Smith
  email: jane.smith@example.com
  created_at: '2025-09-12T11:00:00Z'
```

### CSV Output

Tabular data in CSV format:

```csv
id,name,email,created_at
550e8400-e29b-41d4-a716-446655440000,John Doe,john.doe@example.com,2025-09-12T10:00:00Z
550e8400-e29b-41d4-a716-446655440001,Jane Smith,jane.smith@example.com,2025-09-12T11:00:00Z
```

## Log Formats

### Text Format (Default)

Human-readable log output:

```
2025-09-12T10:00:00Z INFO mockforge::http: Server started on 0.0.0.0:3000
2025-09-12T10:00:01Z INFO mockforge::http: Request: GET /users
2025-09-12T10:00:01Z DEBUG mockforge::template: Template expanded: {{uuid}} -> 550e8400-e29b-41d4-a716-446655440000
2025-09-12T10:00:01Z INFO mockforge::http: Response: 200 OK
```

### JSON Format

Structured JSON logging:

```json
{"timestamp":"2025-09-12T10:00:00Z","level":"INFO","module":"mockforge::http","message":"Server started on 0.0.0.0:3000"}
{"timestamp":"2025-09-12T10:00:01Z","level":"INFO","module":"mockforge::http","message":"Request: GET /users","method":"GET","path":"/users","user_agent":"curl/7.68.0"}
{"timestamp":"2025-09-12T10:00:01Z","level":"DEBUG","module":"mockforge::template","message":"Template expanded","template":"{{uuid}}","result":"550e8400-e29b-41d4-a716-446655440000"}
{"timestamp":"2025-09-12T10:00:01Z","level":"INFO","module":"mockforge::http","message":"Response: 200 OK","status":200,"duration_ms":15}
```

## Template Syntax

### Handlebars Templates

MockForge uses Handlebars-style templates:

```handlebars
{{variable}}
{{object.property}}
{{array.[0]}}
{{#if condition}}content{{/if}}
{{#each items}}{{this}}{{/each}}
{{helper arg1 arg2}}
```

### Built-in Helpers

```handlebars
<!-- Data generation -->
{{uuid}}                    <!-- Random UUID -->
{{now}}                     <!-- Current timestamp -->
{{now+1h}}                  <!-- Future timestamp -->
{{randInt 1 100}}          <!-- Random integer -->
{{randFloat 0.0 1.0}}      <!-- Random float -->
{{randWord}}               <!-- Random word -->
{{randSentence}}           <!-- Random sentence -->
{{randParagraph}}          <!-- Random paragraph -->

<!-- Request context -->
{{request.path.id}}        <!-- URL path parameter -->
{{request.query.limit}}    <!-- Query parameter -->
{{request.header.auth}}    <!-- HTTP header -->
{{request.body.name}}      <!-- Request body field -->

<!-- Logic helpers -->
{{#if user.authenticated}}
  Welcome back, {{user.name}}!
{{else}}
  Please log in.
{{/if}}

{{#each users}}
  <li>{{name}} - {{email}}</li>
{{/each}}
```

## Conversion Utilities

### Format Conversion Scripts

```bash
#!/bin/bash
# convert-format.sh - Convert between supported formats

input_file=$1
output_format=$2

case $output_format in
    "yaml")
        python3 -c "
import sys, yaml, json
data = json.load(sys.stdin)
yaml.dump(data, sys.stdout, default_flow_style=False)
" < "$input_file"
        ;;
    "json")
        python3 -c "
import sys, yaml, json
data = yaml.safe_load(sys.stdin)
json.dump(data, sys.stdout, indent=2)
" < "$input_file"
        ;;
    "xml")
        python3 -c "
import sys, json, dicttoxml
data = json.load(sys.stdin)
xml = dicttoxml.dicttoxml(data, custom_root='root', attr_type=False)
print(xml.decode())
" < "$input_file"
        ;;
    *)
        echo "Unsupported format: $output_format"
        echo "Supported: yaml, json, xml"
        exit 1
        ;;
esac
```

### Validation Scripts

```bash
#!/bin/bash
# validate-format.sh - Validate file formats

file=$1
format=$(basename "$file" | sed 's/.*\.//')

case $format in
    "json")
        python3 -c "
import sys, json
try:
    json.load(sys.stdin)
    print('✓ Valid JSON')
except Exception as e:
    print('✗ Invalid JSON:', e)
    sys.exit(1)
" < "$file"
        ;;
    "yaml")
        python3 -c "
import sys, yaml
try:
    yaml.safe_load(sys.stdin)
    print('✓ Valid YAML')
except Exception as e:
    print('✗ Invalid YAML:', e)
    sys.exit(1)
" < "$file"
        ;;
    "xml")
        python3 -c "
import sys, xml.etree.ElementTree as ET
try:
    ET.parse(sys.stdin)
    print('✓ Valid XML')
except Exception as e:
    print('✗ Invalid XML:', e)
    sys.exit(1)
" < "$file"
        ;;
    *)
        echo "Unsupported format: $format"
        exit 1
        ;;
esac
```

## Best Practices

### Choosing the Right Format

| Use Case | Recommended Format | Reason |
|----------|-------------------|---------|
| API Specifications | OpenAPI YAML | More readable, better for version control |
| Configuration | YAML | Human-readable, supports comments |
| Data Exchange | JSON | Universally supported, compact |
| Logs | JSON | Structured, searchable |
| Templates | Handlebars | Expressive, logic support |

### Format Conversion Workflow

```bash
# API development workflow
# 1. Design API in YAML (readable)
swagger-editor

# 2. Convert to JSON for tools that require it
./convert-format.sh api.yaml json > api.json

# 3. Validate both formats
./validate-format.sh api.yaml
./validate-format.sh api.json

# 4. Generate documentation
swagger-codegen generate -i api.yaml -l html -o docs/

# 5. Commit YAML version (better diff)
git add api.yaml
```

### Performance Considerations

- **JSON**: Fastest parsing, smallest size
- **YAML**: Slower parsing, larger size, better readability
- **XML**: Slowest parsing, largest size, most verbose
- **Binary formats**: Fastest for large data, not human-readable

### Compatibility Matrix

| Format | MockForge Support | Readability | Tool Support | Size |
|--------|------------------|-------------|--------------|------|
| JSON | ✅ Full | Medium | Excellent | Small |
| YAML | ✅ Full | High | Good | Medium |
| XML | ❌ None | Low | Good | Large |
| Protocol Buffers | ✅ gRPC only | Low | Limited | Small |
| JSONL | ✅ WebSocket | Medium | Basic | Medium |

This format reference ensures you can work effectively with all data formats supported by MockForge across different use cases and workflows.
