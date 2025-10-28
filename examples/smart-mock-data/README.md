# Smart Mock Data Generator Example

This example demonstrates the Smart Mock Data Generator feature with token-based templating and domain-specific generators.

## Running the Example

```bash
# From the workspace root
cargo run --example smart_mock_data
```

## Features Demonstrated

1. **Token-based templating** - Using `$random` and `$faker` tokens
2. **Domain-specific generators** - Finance, IoT, and Healthcare domains
3. **Nested objects and arrays** - Complex data structures
4. **Real-world scenarios** - E-commerce orders and IoT sensor readings

## Example Output

The example generates various mock data scenarios:

### Basic Tokens

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "Alice Johnson",
  "email": "alice.johnson@example.com",
  "phone": "+1-555-0123",
  "created_at": "2025-01-15T10:30:00Z",
  "is_active": true
}
```

### E-commerce Order

```json
{
  "order_id": "c7d9b823-f142-4b6a-9e21-8a3f5c6d7e8f",
  "customer": {
    "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "name": "Bob Smith",
    "email": "bob.smith@example.com"
  },
  "items": [
    {
      "id": "item-001",
      "name": "Laptop",
      "price": 1299.99,
      "quantity": 1
    }
  ],
  "total": 1299.99,
  "status": "pending",
  "created_at": "2025-01-15T10:30:00Z"
}
```

### IoT Sensor Reading

```json
{
  "device_id": "device-a1b2c3d4",
  "sensor_id": "sensor-123456",
  "readings": [
    {
      "temperature": 22.5,
      "humidity": 45.3,
      "pressure": 1013.25,
      "timestamp": "2025-01-15T10:30:00Z"
    }
  ],
  "location": {
    "latitude": 37.7749,
    "longitude": -122.4194
  },
  "status": "online"
}
```

## Using with MockForge Server

Create a configuration file with smart mock data:

```yaml
# config.yaml
routes:
  - path: /api/users/:id
    method: GET
    response:
      status: 200
      body:
        type: Static
        content:
          id: "$random.uuid"
          name: "$faker.name"
          email: "$faker.email"
          created_at: "$faker.datetime"
```

Then start the server:

```bash
mockforge serve --config config.yaml
```

Test the endpoint:

```bash
curl http://localhost:3000/api/users/123
```

Each request will return different realistic data!
