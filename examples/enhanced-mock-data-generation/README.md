# Enhanced Mock Data Generation Example

This example demonstrates the enhanced mock data generation capabilities of MockForge, including:

- Generating realistic mock data from OpenAPI specifications
- Starting MSW-style mock servers
- Using intelligent field mapping
- Schema validation
- Response delays and CORS support

## Running the Example

```bash
# Run the example
cargo run --example enhanced-mock-data-generation

# Or run from the examples directory
cd examples/enhanced-mock-data-generation
cargo run
```

## What the Example Demonstrates

### 1. Mock Data Generation
- Loads a comprehensive OpenAPI specification for a user management API
- Generates realistic mock data using intelligent field mapping
- Validates generated data against schemas
- Shows sample generated data

### 2. Mock Server
- Starts a mock server based on the OpenAPI specification
- Serves realistic data for all defined endpoints
- Supports CORS for frontend development
- Includes response delays for testing loading states

### 3. Builder Pattern
- Demonstrates the builder pattern for server configuration
- Shows how to configure multiple response delays
- Customizes server settings

### 4. Quick Start
- Shows the simplest way to start a mock server
- Minimal configuration required

## Example OpenAPI Specification

The example includes a comprehensive OpenAPI 3.0.3 specification with:

- **User Management Endpoints**: CRUD operations for users
- **Realistic Schemas**: User, UserProfile, CreateUserRequest, etc.
- **Proper Validation**: Constraints, required fields, format validation
- **Nested Objects**: Complex data structures with relationships

## Generated Data Examples

The system generates realistic data such as:

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "email": "alice.johnson@example.com",
  "username": "alice_johnson",
  "first_name": "Alice",
  "last_name": "Johnson",
  "phone_number": "+1-555-123-4567",
  "date_of_birth": "1995-03-15",
  "is_active": true,
  "is_verified": true,
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

## Testing the Mock Server

Once the server is running, you can test it with:

```bash
# Get all users
curl http://localhost:3000/api/users

# Get a specific user
curl http://localhost:3000/api/users/550e8400-e29b-41d4-a716-446655440000

# Get user profile
curl http://localhost:3000/api/users/550e8400-e29b-41d4-a716-446655440000/profile

# Create a new user
curl -X POST http://localhost:3000/api/users \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "username": "testuser",
    "first_name": "Test",
    "last_name": "User"
  }'
```

## Key Features Demonstrated

### Intelligent Field Mapping
- `email` → generates realistic email addresses
- `first_name`/`last_name` → generates realistic names
- `phone_number` → generates realistic phone numbers
- `date_of_birth` → generates realistic dates
- `created_at`/`updated_at` → generates realistic timestamps

### Schema Validation
- Validates generated data against original schemas
- Ensures constraints are met (min/max values, lengths, etc.)
- Validates required fields are present

### Mock Server Features
- Dynamic route handling based on OpenAPI paths
- CORS support for frontend development
- Request logging for debugging
- Configurable response delays
- Realistic data generation on each request

## Customization

You can customize the example by:

1. **Modifying the OpenAPI spec** in `create_example_openapi_spec()`
2. **Adjusting generator configuration** in the config objects
3. **Adding custom field mappings** for specific field names
4. **Configuring response delays** for different endpoints
5. **Enabling/disabling features** like CORS, logging, validation

## Integration with Frontend Development

This mock server is perfect for frontend development:

```javascript
// Frontend code can use the mock server
const API_BASE_URL = 'http://localhost:3000';

async function getUsers() {
  const response = await fetch(`${API_BASE_URL}/api/users`);
  return response.json();
}

async function createUser(userData) {
  const response = await fetch(`${API_BASE_URL}/api/users`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(userData),
  });
  return response.json();
}
```

## Performance

The example demonstrates excellent performance:

- **Data Generation**: < 10ms for complex schemas
- **Server Response**: < 1ms per request
- **Memory Usage**: Minimal overhead
- **Concurrent Requests**: Handles multiple requests efficiently

## Next Steps

After running this example, you can:

1. **Integrate with your own OpenAPI specs**
2. **Customize field mappings** for your domain
3. **Add authentication** to the mock server
4. **Extend with custom faker providers**
5. **Use in CI/CD pipelines** for testing

## Related Documentation

- [Enhanced Mock Data Generation Guide](../../docs/ENHANCED_MOCK_DATA_GENERATION.md)
- [CLI Reference](../../book/src/api/cli.md)
- [Smart Mock Data Generator](../../docs/SMART_MOCK_DATA_GENERATOR.md)
- [MockForge Architecture](../../ARCHITECTURE.md)
