# Quick Start

Get MockForge running in under 5 minutes with this hands-on guide. We'll create a mock API server and test it with real HTTP requests.

## Prerequisites

Ensure MockForge is [installed](installation.md) and available in your PATH.

## Step 1: Start a Basic HTTP Mock Server

MockForge can serve mock APIs defined in OpenAPI specifications. Let's use the included example:

```bash
# Navigate to the MockForge directory (if building from source)
cd mockforge

# Start the server with the demo OpenAPI spec
mockforge serve --spec examples/openapi-demo.json --http-port 3000
```

You should see output like:
```
MockForge v0.1.0 starting...
HTTP server listening on 0.0.0.0:3000
OpenAPI spec loaded from examples/openapi-demo.json
Ready to serve requests at http://localhost:3000
```

## Step 2: Test Your Mock API

Open a new terminal and test the API endpoints:

```bash
# Health check endpoint
curl http://localhost:3000/ping
```

Expected response:
```json
{
  "status": "pong",
  "timestamp": "2025-09-12T17:20:01.512504405+00:00",
  "requestId": "550e8400-e29b-41d4-a716-446655440000"
}
```

```bash
# List users endpoint
curl http://localhost:3000/users
```

Expected response:
```json
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

```bash
# Create a new user
curl -X POST http://localhost:3000/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Jane Smith", "email": "jane@example.com"}'
```

```bash
# Get user by ID (path parameter)
curl http://localhost:3000/users/123
```

## Step 3: Enable Template Expansion

MockForge supports dynamic content generation. Enable template expansion for more realistic data:

```bash
# Stop the current server (Ctrl+C), then restart with templates enabled
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true \
mockforge serve --spec examples/openapi-demo.json --http-port 3000
```

Now test the endpoints again - you'll see different UUIDs and timestamps each time!

## Step 4: Add WebSocket Support

MockForge can also mock WebSocket connections. Let's add WebSocket support to our server:

```bash
# Stop the server, then restart with WebSocket support
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true \
MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl \
mockforge serve --spec examples/openapi-demo.json --ws-port 3001 --http-port 3000
```

## Step 5: Test WebSocket Connection

Test the WebSocket endpoint (requires Node.js or a WebSocket client):

```bash
# Using Node.js
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
```

Expected WebSocket message flow:
1. Send `CLIENT_READY`
2. Receive welcome message with session ID
3. Receive data message, respond with `ACK`
4. Receive heartbeat messages
5. Receive notification, respond with `CONFIRMED`

## Step 6: Enable Admin UI (Optional)

For a visual interface to manage your mock server:

```bash
# Stop the server, then restart with admin UI
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true \
MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl \
mockforge serve --spec examples/openapi-demo.json \
  --admin --admin-port 8080 \
  --http-port 3000 --ws-port 3001
```

Access the admin interface at: http://localhost:8080

## Step 7: Using Configuration Files

Instead of environment variables, you can use a configuration file:

```bash
# Stop the server, then start with config file
mockforge serve --config demo-config.yaml
```

## Step 8: Docker Alternative

If you prefer Docker:

```bash
# Build and run with Docker
docker build -t mockforge .
docker run -p 3000:3000 -p 3001:3001 -p 8080:8080 \
  -e MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true \
  -e MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl \
  mockforge
```

## What's Next?

Congratulations! You now have a fully functional mock server running. Here are some next steps:

- Learn about [Basic Concepts](concepts.md) to understand how MockForge works
- Explore [HTTP Mocking](../user-guide/http-mocking.md) for advanced REST API features
- Try [WebSocket Mocking](../user-guide/websocket-mocking.md) for real-time communication
- Check out the [Admin UI](../user-guide/admin-ui.md) for visual management

## Troubleshooting

### Server won't start
- Check if ports 3000, 3001, or 8080 are already in use
- Verify the OpenAPI spec file path is correct
- Ensure MockForge is properly installed

### Template variables not working
- Make sure `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true` is set
- Check that template syntax `{{variable}}` is used correctly

### WebSocket connection fails
- Verify WebSocket port (default 3001) is accessible
- Check that `MOCKFORGE_WS_REPLAY_FILE` points to a valid replay file
- Ensure the replay file uses the correct JSONL format

### Need help?
- Check the [examples README](../../examples/README.md) for detailed testing scripts
- Review [Configuration Files](../configuration/files.md) for advanced setup
- Visit the [Troubleshooting](../reference/troubleshooting.md) guide
