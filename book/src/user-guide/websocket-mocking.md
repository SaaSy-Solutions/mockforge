# WebSocket Mocking

MockForge provides comprehensive WebSocket connection mocking with support for both scripted replay scenarios and interactive real-time communication. This enables testing of WebSocket-based applications, real-time APIs, and event-driven systems.

## WebSocket Mocking Modes

MockForge supports two primary WebSocket mocking approaches:

### 1. Replay Mode (Scripted)
Pre-recorded message sequences that play back on schedule, simulating server behavior with precise timing control.

### 2. Interactive Mode (Real-time)
Dynamic responses based on client messages, enabling complex interactive scenarios and stateful communication.

## Configuration

### Basic WebSocket Setup

```bash
# Start MockForge with WebSocket support
mockforge serve --ws-port 3001 --ws-replay-file ws-scenario.jsonl
```

### Environment Variables

```bash
# WebSocket configuration
MOCKFORGE_WS_ENABLED=true                    # Enable WebSocket support (default: false)
MOCKFORGE_WS_PORT=3001                       # WebSocket server port
MOCKFORGE_WS_BIND=0.0.0.0                    # Bind address
MOCKFORGE_WS_REPLAY_FILE=path/to/file.jsonl  # Path to replay file
MOCKFORGE_WS_PATH=/ws                         # WebSocket endpoint path (default: /ws)
MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true      # Enable template processing
```

### Command Line Options

```bash
mockforge serve \
  --ws-port 3001 \
  --ws-replay-file examples/ws-demo.jsonl \
  --ws-path /websocket
```

## Replay Mode

Replay mode uses JSONL-formatted files to define scripted message sequences with precise timing control.

### Replay File Format

Each line in the replay file is a JSON object with the following structure:

```json
{
  "ts": 0,
  "dir": "out",
  "text": "Hello, client!",
  "waitFor": "^CLIENT_READY$"
}
```

### Field Definitions

- **`ts`** (number, required): Timestamp offset in milliseconds from connection start
- **`dir`** (string, required): Message direction
  - `"out"` - Message sent from server to client
  - `"in"` - Expected message from client (for validation)
- **`text`** (string, required): Message content (supports templates)
- **`waitFor`** (string, optional): Regular expression to wait for before proceeding

### Basic Replay Example

```jsonl
{"ts":0,"dir":"out","text":"Welcome to MockForge WebSocket server","waitFor":"^HELLO$"}
{"ts":1000,"dir":"out","text":"Connection established"}
{"ts":2000,"dir":"out","text":"Sending data: 42"}
{"ts":3000,"dir":"out","text":"Goodbye"}
```

### Advanced Replay Features

#### Template Support

```jsonl
{"ts":0,"dir":"out","text":"Session {{uuid}} started at {{now}}"}
{"ts":1000,"dir":"out","text":"Random value: {{randInt 1 100}}"}
{"ts":2000,"dir":"out","text":"Future event at {{now+5m}}"}
```

#### Interactive Elements

```jsonl
{"ts":0,"dir":"out","text":"Please authenticate","waitFor":"^AUTH .+$"}
{"ts":100,"dir":"out","text":"Authentication successful"}
{"ts":200,"dir":"out","text":"Choose option (A/B/C)","waitFor":"^(A|B|C)$"}
```

#### Complex Message Structures

```jsonl
{"ts":0,"dir":"out","text":"{\"type\":\"welcome\",\"user\":{\"id\":\"{{uuid}}\",\"name\":\"John\"}}"}
{"ts":1000,"dir":"out","text":"{\"type\":\"data\",\"payload\":{\"items\":[{\"id\":1,\"value\":\"{{randInt 10 99}}\"},{\"id\":2,\"value\":\"{{randInt 100 999}}\"}]}}"}
```

### Replay File Management

#### Creating Replay Files

```bash
# Record from live WebSocket connection
# (Feature in development - manual creation for now)

# Create from application logs
# Extract WebSocket messages and convert to JSONL format

# Generate programmatically
node -e "
const fs = require('fs');
const messages = [
  {ts: 0, dir: 'out', text: 'HELLO', waitFor: '^HI$'},
  {ts: 1000, dir: 'out', text: 'DATA: 42'}
];
fs.writeFileSync('replay.jsonl', messages.map(JSON.stringify).join('\n'));
"
```

#### Validation

```bash
# Validate replay file syntax
node -e "
const fs = require('fs');
const lines = fs.readFileSync('replay.jsonl', 'utf8').split('\n');
lines.forEach((line, i) => {
  if (line.trim()) {
    try {
      const msg = JSON.parse(line);
      if (!msg.ts || !msg.dir || !msg.text) {
        console.log(\`Line \${i+1}: Missing required fields\`);
      }
    } catch (e) {
      console.log(\`Line \${i+1}: Invalid JSON\`);
    }
  }
});
console.log('Validation complete');
"
```

## Interactive Mode

Interactive mode enables dynamic responses based on client messages, supporting complex conversational patterns and state management.

### Basic Interactive Setup

```jsonl
{"ts":0,"dir":"out","text":"What is your name?","waitFor":"^NAME .+$"}
{"ts":100,"dir":"out","text":"Hello {{request.ws.lastMessage.match(/^NAME (.+)$/)[1]}}!"}
```

### State Management

```jsonl
{"ts":0,"dir":"out","text":"Welcome! Type 'START' to begin","waitFor":"^START$"}
{"ts":100,"dir":"out","text":"Game started. Score: 0","state":"playing"}
{"ts":200,"dir":"out","text":"Choose: ROCK/PAPER/SCISSORS","waitFor":"^(ROCK|PAPER|SCISSORS)$"}
{"ts":300,"dir":"out","text":"You chose {{request.ws.lastMessage}}. I chose ROCK. You win!","waitFor":"^PLAY_AGAIN$"}
```

### Conditional Logic

```jsonl
{"ts":0,"dir":"out","text":"Enter command","waitFor":".+","condition":"{{request.ws.message.length > 0}}"}
{"ts":100,"dir":"out","text":"Processing: {{request.ws.message}}"}
{"ts":200,"dir":"out","text":"Command completed"}
```

## Testing WebSocket Connections

### Using WebSocket Clients

#### Node.js Client

```javascript
const WebSocket = require('ws');

const ws = new WebSocket('ws://localhost:3001/ws');

ws.on('open', () => {
  console.log('Connected to MockForge WebSocket');
  ws.send('CLIENT_READY');
});

ws.on('message', (data) => {
  const message = data.toString();
  console.log('Received:', message);

  // Auto-respond to common prompts
  if (message.includes('ACK')) {
    ws.send('ACK');
  }
  if (message.includes('CONFIRMED')) {
    ws.send('CONFIRMED');
  }
  if (message.includes('AUTH')) {
    ws.send('AUTH token123');
  }
});

ws.on('close', () => {
  console.log('Connection closed');
});

ws.on('error', (err) => {
  console.error('WebSocket error:', err);
});
```

#### Browser JavaScript

```javascript
const ws = new WebSocket('ws://localhost:3001/ws');

ws.onopen = () => {
  console.log('Connected');
  ws.send('CLIENT_READY');
};

ws.onmessage = (event) => {
  console.log('Received:', event.data);
  // Handle server messages
};

ws.onclose = () => {
  console.log('Connection closed');
};
```

#### Command Line Tools

```bash
# Using websocat
websocat ws://localhost:3001/ws

# Using curl (WebSocket support experimental)
curl --include \
     --no-buffer \
     --header "Connection: Upgrade" \
     --header "Upgrade: websocket" \
     --header "Sec-WebSocket-Key: x3JJHMbDL1EzLkh9GBhXDw==" \
     --header "Sec-WebSocket-Version: 13" \
     ws://localhost:3001/ws
```

### Automated Testing

```bash
#!/bin/bash
# test-websocket.sh

echo "Testing WebSocket connection..."

# Test with Node.js
node -e "
const WebSocket = require('ws');
const ws = new WebSocket('ws://localhost:3001/ws');

ws.on('open', () => {
  console.log('✓ Connection established');
  ws.send('CLIENT_READY');
});

ws.on('message', (data) => {
  console.log('✓ Message received:', data.toString());
  ws.close();
});

ws.on('close', () => {
  console.log('✓ Connection closed successfully');
  process.exit(0);
});

ws.on('error', (err) => {
  console.error('✗ WebSocket error:', err);
  process.exit(1);
});

// Timeout after 10 seconds
setTimeout(() => {
  console.error('✗ Test timeout');
  process.exit(1);
}, 10000);
"
```

## Advanced Features

### Connection Pooling

```bash
# Support multiple concurrent connections
MOCKFORGE_WS_MAX_CONNECTIONS=100
MOCKFORGE_WS_CONNECTION_TIMEOUT=30000
```

### Message Filtering

```jsonl
{"ts":0,"dir":"in","text":".*","filter":"{{request.ws.message.startsWith('VALID_')}}"}
{"ts":100,"dir":"out","text":"Valid message received"}
```

### Error Simulation

```jsonl
{"ts":0,"dir":"out","text":"Error occurred","error":"true","code":1006}
{"ts":100,"dir":"out","text":"Connection will close","close":"true"}
```

### Binary Message Support

```jsonl
{"ts":0,"dir":"out","text":"AQIDBAU=","binary":"true"}
{"ts":1000,"dir":"out","text":"Binary data sent"}
```

## Integration Patterns

### Real-time Applications

- **Chat Applications**: Mock user conversations and bot responses
- **Live Updates**: Simulate real-time data feeds and notifications
- **Gaming**: Mock multiplayer game state and player interactions

### API Testing

- **WebSocket APIs**: Test GraphQL subscriptions and real-time queries
- **Event Streams**: Mock server-sent events and push notifications
- **Live Dashboards**: Simulate real-time metrics and monitoring data

### Development Workflows

- **Frontend Development**: Mock WebSocket backends during UI development
- **Integration Testing**: Test WebSocket handling in microservices
- **Load Testing**: Simulate thousands of concurrent WebSocket connections

## Best Practices

### Replay File Organization

1. **Modular Files**: Break complex scenarios into smaller, focused replay files
2. **Version Control**: Keep replay files in Git for collaboration
3. **Documentation**: Comment complex scenarios with clear descriptions
4. **Validation**: Always validate replay files before deployment

### Performance Considerations

1. **Message Volume**: Limit concurrent connections based on system resources
2. **Memory Usage**: Monitor memory usage with large replay files
3. **Timing Accuracy**: Consider system clock precision for time-sensitive scenarios
4. **Connection Limits**: Set appropriate connection pool sizes

### Security Considerations

1. **Input Validation**: Validate all client messages in interactive mode
2. **Rate Limiting**: Implement connection rate limits for production
3. **Authentication**: Mock authentication handshakes appropriately
4. **Data Sanitization**: Avoid exposing sensitive data in replay files

### Debugging Tips

1. **Verbose Logging**: Enable detailed WebSocket logging for troubleshooting
2. **Connection Monitoring**: Track connection lifecycle and message flow
3. **Replay Debugging**: Step through replay files manually
4. **Client Compatibility**: Test with multiple WebSocket client libraries

## Troubleshooting

### Common Issues

**Connection fails**: Check that WebSocket port is not blocked by firewall

**Messages not received**: Verify replay file path and JSONL format

**Templates not expanding**: Ensure `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true`

**Timing issues**: Check system clock and timestamp calculations

### Debug Commands

```bash
# Check WebSocket port
netstat -tlnp | grep :3001

# Monitor connections
ss -tlnp | grep :3001

# Test basic connectivity
curl -I http://localhost:3001/health  # If HTTP health endpoint exists
```

### Log Analysis

```bash
# View WebSocket logs
tail -f mockforge.log | grep -i websocket

# Count connections
grep "WebSocket connection" mockforge.log | wc -l

# Find errors
grep -i "websocket.*error" mockforge.log
```

For detailed implementation guides, see:
- [Replay Mode](websocket-mocking/replay.md) - Advanced scripted scenarios
- [Interactive Mode](websocket-mocking/interactive.md) - Dynamic real-time communication
