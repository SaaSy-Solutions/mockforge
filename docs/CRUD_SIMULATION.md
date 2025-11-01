# CRUD Simulation with MockForge

MockForge provides powerful stateful CRUD (Create, Read, Update, Delete) simulation capabilities through its Intelligent Behavior system. This guide demonstrates how to simulate realistic database-like operations with persistent state.

## Overview

MockForge's Intelligent Behavior system acts as a **stateful data store** that maintains resource state across requests, enabling realistic CRUD API simulation without a real database.

### Key Features

- ✅ **Create (POST)**: Create new resources that persist across requests
- ✅ **Read (GET)**: Retrieve previously created resources
- ✅ **Update (PUT/PATCH)**: Modify existing resources
- ✅ **Delete (DELETE)**: Remove resources from the store
- ✅ **State Persistence**: Resources persist across multiple API calls
- ✅ **Relationship Awareness**: Maintains relationships between resources
- ✅ **Query Support**: List and filter resources

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    HTTP Request (POST /api/users)            │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│            Intelligent Behavior Handler                      │
│  - Parses request method and path                           │
│  - Extracts resource data from body                         │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│            Stateful AI Context                               │
│  - Session management                                        │
│  - Resource storage (in-memory or persistent)                │
│  - Conversation history                                      │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│            Vector Memory Store                               │
│  - Long-term persistence                                     │
│  - Semantic search for related resources                    │
│  - State snapshots                                           │
└─────────────────────────────────────────────────────────────┘
```

## Configuration

Enable Intelligent Behavior in your `config.yaml`:

```yaml
intelligent_behavior:
  enabled: true
  session_tracking:
    method: cookie  # or header, query_param
    cookie_name: mockforge_session

  behavior_model:
    llm_provider: openai  # or anthropic, ollama
    model: gpt-3.5-turbo
    system_prompt: |
      You are simulating a REST API with a database. Maintain these rules:
      - POST /api/{resource} creates a new resource
      - GET /api/{resource}/{id} retrieves a resource by ID
      - PUT /api/{resource}/{id} updates an existing resource
      - DELETE /api/{resource}/{id} deletes a resource
      - GET /api/{resource} lists all resources
      - Resources persist across requests
      - Return 404 if resource doesn't exist
      - Return 409 if trying to create duplicate resource

    schemas:
      User:
        type: object
        properties:
          id:
            type: string
            example: "usr_abc123"
          name:
            type: string
            example: "Alice Johnson"
          email:
            type: string
            format: email
            example: "alice@example.com"
          created_at:
            type: string
            format: date-time

      Product:
        type: object
        properties:
          id:
            type: string
          name:
            type: string
          price:
            type: number
          stock:
            type: integer

    consistency_rules:
      - name: create_resource
        condition: "method == 'POST' AND path matches '/api/(users|products|orders)'"
        action:
          type: transform
          description: "Create resource with generated ID, store in session state"

      - name: read_resource
        condition: "method == 'GET' AND path matches '/api/(users|products|orders)/{id}'"
        action:
          type: transform
          description: "Retrieve resource from session state by ID, return 404 if not found"

      - name: update_resource
        condition: "method IN ['PUT', 'PATCH'] AND path matches '/api/(users|products|orders)/{id}'"
        action:
          type: transform
          description: "Update existing resource in session state, return 404 if not found"

      - name: delete_resource
        condition: "method == 'DELETE' AND path matches '/api/(users|products|orders)/{id}'"
        action:
          type: transform
          description: "Remove resource from session state, return 404 if not found"

  vector_store:
    enabled: true
    storage_path: ./data/vector_store  # Optional: persistent storage
```

## CRUD Examples

### 1. Create (POST)

**Request:**
```bash
POST /api/users
Content-Type: application/json

{
  "name": "Alice Johnson",
  "email": "alice@example.com"
}
```

**Response:**
```json
{
  "id": "usr_abc123",
  "name": "Alice Johnson",
  "email": "alice@example.com",
  "created_at": "2025-01-15T10:00:00Z"
}
```

The resource is now stored in the session state and will be available for subsequent requests.

### 2. Read (GET)

**Request:**
```bash
GET /api/users/usr_abc123
```

**Response:**
```json
{
  "id": "usr_abc123",
  "name": "Alice Johnson",
  "email": "alice@example.com",
  "created_at": "2025-01-15T10:00:00Z"
}
```

**If resource doesn't exist:**
```json
{
  "error": "Resource not found",
  "status": 404
}
```

### 3. List (GET Collection)

**Request:**
```bash
GET /api/users
```

**Response:**
```json
{
  "users": [
    {
      "id": "usr_abc123",
      "name": "Alice Johnson",
      "email": "alice@example.com",
      "created_at": "2025-01-15T10:00:00Z"
    },
    {
      "id": "usr_def456",
      "name": "Bob Smith",
      "email": "bob@example.com",
      "created_at": "2025-01-15T11:00:00Z"
    }
  ],
  "total": 2
}
```

### 4. Update (PUT)

**Request:**
```bash
PUT /api/users/usr_abc123
Content-Type: application/json

{
  "name": "Alice Williams",
  "email": "alice.williams@example.com"
}
```

**Response:**
```json
{
  "id": "usr_abc123",
  "name": "Alice Williams",
  "email": "alice.williams@example.com",
  "created_at": "2025-01-15T10:00:00Z",
  "updated_at": "2025-01-15T12:00:00Z"
}
```

### 5. Partial Update (PATCH)

**Request:**
```bash
PATCH /api/users/usr_abc123
Content-Type: application/json

{
  "email": "newemail@example.com"
}
```

**Response:**
```json
{
  "id": "usr_abc123",
  "name": "Alice Williams",
  "email": "newemail@example.com",
  "created_at": "2025-01-15T10:00:00Z",
  "updated_at": "2025-01-15T12:30:00Z"
}
```

### 6. Delete (DELETE)

**Request:**
```bash
DELETE /api/users/usr_abc123
```

**Response:**
```json
{
  "message": "User deleted successfully",
  "id": "usr_abc123"
}
```

**Subsequent GET request:**
```bash
GET /api/users/usr_abc123
```

**Response:**
```json
{
  "error": "Resource not found",
  "status": 404
}
```

## Advanced Features

### Relationships Between Resources

The Intelligent Behavior system maintains relationships between resources:

```yaml
behavior_model:
  system_prompt: |
    Maintain relationships:
    - Users can have multiple Orders
    - Orders contain OrderItems that reference Products
    - When deleting a User, cascade delete their Orders
    - When getting a User, include their Orders if requested

    schemas:
      Order:
        type: object
        properties:
          id: {type: string}
          user_id: {type: string}
          items: {type: array}
          total: {type: number}
```

**Example:**
```bash
# Create user
POST /api/users
{"name": "Alice", "email": "alice@example.com"}
→ {"id": "usr_1", ...}

# Create order for user
POST /api/orders
{"user_id": "usr_1", "items": [...]}
→ {"id": "ord_1", "user_id": "usr_1", ...}

# Get user with orders
GET /api/users/usr_1?include=orders
→ {
     "id": "usr_1",
     "name": "Alice",
     "orders": [
       {"id": "ord_1", "user_id": "usr_1", ...}
     ]
   }
```

### Query Parameters

Simulate query filtering:

```yaml
behavior_model:
  system_prompt: |
    Support query parameters:
    - ?limit=N - Limit results
    - ?offset=N - Pagination offset
    - ?filter=field:value - Filter by field
    - ?sort=field - Sort results
```

**Example:**
```bash
GET /api/users?limit=10&offset=0&filter=email:alice@example.com&sort=created_at
```

### Data Persistence

Resources can persist across server restarts when using persistent vector storage:

```yaml
vector_store:
  enabled: true
  storage_path: ./data/vector_store  # Persistent storage
  embedding_provider: openai
  embedding_model: text-embedding-ada-002
```

## Complete CRUD Workflow Example

```bash
# 1. Create multiple users
POST /api/users
{"name": "Alice", "email": "alice@example.com"}
→ {"id": "usr_1", ...}

POST /api/users
{"name": "Bob", "email": "bob@example.com"}
→ {"id": "usr_2", ...}

# 2. List all users
GET /api/users
→ {"users": [{"id": "usr_1", ...}, {"id": "usr_2", ...}], "total": 2}

# 3. Get specific user
GET /api/users/usr_1
→ {"id": "usr_1", "name": "Alice", ...}

# 4. Update user
PUT /api/users/usr_1
{"name": "Alice Williams", "email": "alice.williams@example.com"}
→ {"id": "usr_1", "name": "Alice Williams", ...}

# 5. Verify update
GET /api/users/usr_1
→ {"id": "usr_1", "name": "Alice Williams", ...}

# 6. Delete user
DELETE /api/users/usr_1
→ {"message": "User deleted successfully"}

# 7. Verify deletion
GET /api/users/usr_1
→ {"error": "Resource not found", "status": 404}

# 8. List users again
GET /api/users
→ {"users": [{"id": "usr_2", ...}], "total": 1}
```

## Programmatic Access (Rust API)

```rust
use mockforge_core::intelligent_behavior::{
    IntelligentBehaviorConfig,
    StatefulAiContext,
    BehaviorModel,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = IntelligentBehaviorConfig::default();
    let mut context = StatefulAiContext::new("session_123", config.clone());
    let behavior = BehaviorModel::new(config.behavior_model.clone());

    // CREATE
    let create_response = behavior.generate_response(
        "POST",
        "/api/users",
        Some(serde_json::json!({
            "name": "Alice",
            "email": "alice@example.com"
        })),
        &context,
    ).await?;

    context.record_interaction(
        "POST",
        "/api/users",
        Some(serde_json::json!({"name": "Alice", "email": "alice@example.com"})),
        Some(create_response.clone()),
    ).await?;

    // Extract ID from response
    let user_id = create_response["id"].as_str().unwrap();

    // READ
    let read_response = behavior.generate_response(
        "GET",
        &format!("/api/users/{}", user_id),
        None,
        &context,
    ).await?;

    println!("Read user: {}", read_response);

    // UPDATE
    let update_response = behavior.generate_response(
        "PUT",
        &format!("/api/users/{}", user_id),
        Some(serde_json::json!({
            "name": "Alice Williams",
            "email": "alice.williams@example.com"
        })),
        &context,
    ).await?;

    println!("Updated user: {}", update_response);

    // DELETE
    let delete_response = behavior.generate_response(
        "DELETE",
        &format!("/api/users/{}", user_id),
        None,
        &context,
    ).await?;

    println!("Delete result: {}", delete_response);

    Ok(())
}
```

## Best Practices

1. **Use Descriptive System Prompts**: Clearly define resource schemas and CRUD behavior
2. **Enable Vector Store**: For persistent storage across sessions
3. **Define Consistency Rules**: Enforce business logic (e.g., no duplicate emails)
4. **Use Session Management**: Track users across requests via cookies/headers
5. **Test State Transitions**: Verify that UPDATE and DELETE operations work correctly

## Limitations

- **Session-Based**: State is per-session (unless using persistent vector store)
- **No ACID Transactions**: Not a real database, but sufficient for mocking
- **LLM Dependency**: Requires LLM provider (OpenAI, Anthropic, or Ollama) for intelligent responses

## Conclusion

MockForge's Intelligent Behavior system provides a powerful, stateful CRUD simulation that behaves like a real database-backed API. With proper configuration, you can simulate complex resource management workflows without setting up a database.
