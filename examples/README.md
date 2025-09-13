# MockForge Examples

This directory contains example files demonstrating MockForge's capabilities with different protocols and configurations.

## 📋 Examples Overview

### 1. OpenAPI Example (`openapi-demo.json`)

A comprehensive OpenAPI 3.0.3 specification demonstrating various HTTP endpoints with MockForge template features.

#### Endpoints Available:

- `GET /ping` - Health check with dynamic timestamps
- `GET /users` - List users with query parameters
- `POST /users` - Create new user with request body handling
- `GET /users/{id}` - Get user by ID with path parameters
- `GET /health` - System health check with random values

#### Template Features Demonstrated:

- `{{uuid}}` - Generates unique UUIDs
- `{{now}}` - Current timestamp in ISO format
- `{{now+1h}}` - Future timestamps
- `{{randInt 10 99}}` - Random integers in range
- `{{request.body.name}}` - Access request body data
- `{{request.path.id}}` - Access path parameters

#### Testing the OpenAPI Example:

```bash
# Test health endpoint
curl http://localhost:3000/ping

# Test users endpoint
curl http://localhost:3000/users

# Test user creation
curl -X POST http://localhost:3000/users \
  -H "Content-Type: application/json" \
  -d '{"name": "John Doe", "email": "john@example.com"}'

# Test path parameters
curl http://localhost:3000/users/123
```

### 2. WebSocket Example (`ws-demo.jsonl`)

A WebSocket replay file demonstrating scripted message sequences with interactive elements.

#### Message Sequence:

1. **Welcome Message** (ts: 0) - Waits for `CLIENT_READY` from client
2. **Connection Established** (ts: 10) - Sends welcome with session ID
3. **Data Message** (ts: 15) - Sends data and waits for `ACK`
4. **Heartbeat** (ts: 25) - Regular heartbeat message
5. **Notification** (ts: 30) - System notification waiting for `CONFIRMED`
6. **Final Data** (ts: 40) - Additional data message

#### Template Features:

- `{{uuid}}` - Unique session IDs
- `{{now}}` - Current timestamps
- `{{now+1m}}` - Future timestamps (1 minute)
- `{{now+1h}}` - Future timestamps (1 hour)
- `{{randInt 10 99}}` - Random data IDs
- `{{randInt 100 999}}` - Random values

#### Testing the WebSocket Example:

```bash
# Using Node.js (if installed):
node -e "
const WebSocket = require('ws');
const ws = new WebSocket('ws://localhost:3001/ws');
ws.on('open', () => {
  console.log('Connected! Sending CLIENT_READY...');
  ws.send('CLIENT_READY');
});
ws.on('message', (data) => {
  console.log('Received:', data.toString());
  if (data.toString().includes('ACK')) {
    ws.send('ACK');
  }
  if (data.toString().includes('CONFIRMED')) {
    ws.send('CONFIRMED');
  }
});
ws.on('close', () => console.log('Connection closed'));
"

# Using websocat (command-line WebSocket client):
websocat ws://localhost:3001/ws
# Then type: CLIENT_READY
# Follow prompts for ACK and CONFIRMED
```

## 🚀 Running the Examples

### Method 1: Using Configuration File

```bash
# Start with the demo configuration
cargo run -p mockforge-cli -- serve --config demo-config.yaml
```

### Method 2: Using Environment Variables

```bash
# Set environment variables
MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl \
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true \
cargo run -p mockforge-cli -- serve --spec examples/openapi-demo.json --admin
```

### Method 3: Using Makefile

```bash
# Run the updated example target
make run-example
```

## 🔧 Configuration Notes

### Required Environment Variables:
- `MOCKFORGE_WS_REPLAY_FILE` - Path to WebSocket replay file
- `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true` - Enable template expansion

### Port Configuration:
- HTTP: 3000 (configurable)
- WebSocket: 3001 (configurable)
- Admin UI: 8080 (configurable)
- gRPC: 50051 (configurable)

### Template Expansion:
When `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true`, these tokens are replaced:
- `{{uuid}}` → Random UUID v4
- `{{now}}` → Current ISO timestamp
- `{{now+1h}}` → Timestamp 1 hour in future
- `{{randInt 1 100}}` → Random integer between 1-100
- `{{request.body.field}}` → Access request body data
- `{{request.path.param}}` → Access path parameters

## 🧪 Testing Scripts

### Automated HTTP Testing:

```bash
#!/bin/bash
echo "Testing OpenAPI endpoints..."

# Test ping
echo "=== Ping Test ==="
curl -s http://localhost:3000/ping

# Test users
echo -e "\n=== Users Test ==="
curl -s http://localhost:3000/users

# Test health
echo -e "\n=== Health Test ==="
curl -s http://localhost:3000/health

# Test user creation
echo -e "\n=== Create User Test ==="
curl -s -X POST http://localhost:3000/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Test User", "email": "test@example.com"}'
```

### WebSocket Testing Script:

```javascript
// ws-test.js
const WebSocket = require('ws');

const ws = new WebSocket('ws://localhost:3001/ws');

ws.on('open', () => {
  console.log('Connected to WebSocket');
  ws.send('CLIENT_READY');
});

ws.on('message', (data) => {
  const message = data.toString();
  console.log('Received:', message);

  // Auto-respond to expected prompts
  if (message.includes('ACK')) {
    ws.send('ACK');
  }
  if (message.includes('CONFIRMED')) {
    ws.send('CONFIRMED');
  }
});

ws.on('close', () => {
  console.log('Connection closed');
});

ws.on('error', (err) => {
  console.error('WebSocket error:', err);
});
```

## 📊 Expected Outputs

### HTTP Responses:
```json
// GET /ping
{
  "status": "pong",
  "timestamp": "2025-09-12T17:20:01.512504405+00:00",
  "requestId": "550e8400-e29b-41d4-a716-446655440000"
}

// GET /users
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440001",
    "name": "John Doe",
    "email": "john@example.com",
    "createdAt": "2025-09-12T17:20:01.512504405+00:00",
    "active": true
  }
]
```

### WebSocket Messages:
```json
{"type":"welcome","message":"WebSocket connection established","sessionId":"550e8400-e29b-41d4-a716-446655440002","timestamp":"2025-09-12T17:20:01.512504405+00:00"}
{"type":"data","id":"42","value":"256","timestamp":"2025-09-12T17:20:01.512504405+00:00"}
{"type":"heartbeat","timestamp":"2025-09-12T17:20:01.512504405+00:00"}
{"type":"notification","title":"System Update","message":"Server maintenance scheduled","timestamp":"2025-09-12T18:20:01.512504405+00:00"}
```

## 🔍 Troubleshooting

### Common Issues:

1. **Port conflicts**: Kill existing processes with `./scripts/clear-ports.sh`
2. **Template not expanding**: Ensure `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true`
3. **WebSocket not responding**: Check that replay file path is correct
4. **OpenAPI not loading**: Verify JSON syntax and file path

### Debug Commands:

```bash
# Check running processes
ps aux | grep mockforge

# Check listening ports
ss -tlnp | grep -E ":(3000|3001|8080)"

# Test basic connectivity
curl -v http://localhost:3000/

# Test WebSocket port
nc -z localhost 3001 && echo "WebSocket port open" || echo "WebSocket port closed"
```
