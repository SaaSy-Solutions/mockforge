# Replay Mode

Replay mode provides precise, scripted WebSocket message sequences that execute on a predetermined schedule. This mode is ideal for testing deterministic scenarios, reproducing specific interaction patterns, and validating client behavior against known server responses.

## Core Concepts

### Message Timeline
Replay files define a sequence of messages that execute based on timestamps relative to connection establishment. Each message has a precise timing offset ensuring consistent playback.

### Deterministic Execution
Replay scenarios execute identically each time, making them perfect for:
- Automated testing
- Regression testing
- Client behavior validation
- Demo environments

## Replay File Structure

### JSONL Format
Replay files use JSON Lines format where each line contains a complete JSON object representing a single message or directive.

```jsonl
{"ts":0,"dir":"out","text":"Welcome message"}
{"ts":1000,"dir":"out","text":"Data update","waitFor":"^ACK$"}
{"ts":2000,"dir":"out","text":"Connection closing"}
```

### Message Object Schema

```typescript
interface ReplayMessage {
  ts: number;           // Timestamp offset in milliseconds
  dir: "out" | "in";    // Message direction
  text: string;         // Message content
  waitFor?: string;     // Optional regex pattern to wait for
  binary?: boolean;     // Binary message flag
  close?: boolean;      // Close connection after this message
  error?: boolean;      // Send as error frame
}
```

## Basic Replay Examples

### Simple Chat Simulation

```jsonl
{"ts":0,"dir":"out","text":"Chat server connected. Welcome!"}
{"ts":500,"dir":"out","text":"Type 'hello' to start chatting","waitFor":"^hello$"}
{"ts":100,"dir":"out","text":"Hello! How can I help you today?"}
{"ts":2000,"dir":"out","text":"Are you still there?","waitFor":".*"}
{"ts":500,"dir":"out","text":"Thanks for chatting! Goodbye."}
```

### API Status Monitoring

```jsonl
{"ts":0,"dir":"out","text":"{\"type\":\"status\",\"message\":\"Monitor connected\"}"}
{"ts":1000,"dir":"out","text":"{\"type\":\"metrics\",\"cpu\":45,\"memory\":67}"}
{"ts":2000,"dir":"out","text":"{\"type\":\"metrics\",\"cpu\":42,\"memory\":68}"}
{"ts":3000,"dir":"out","text":"{\"type\":\"metrics\",\"cpu\":47,\"memory\":66}"}
{"ts":4000,"dir":"out","text":"{\"type\":\"alert\",\"level\":\"warning\",\"message\":\"High CPU usage\"}"}
```

### Game State Synchronization

```jsonl
{"ts":0,"dir":"out","text":"{\"action\":\"game_start\",\"player_id\":\"{{uuid}}\",\"game_id\":\"{{uuid}}\"}"}
{"ts":1000,"dir":"out","text":"{\"action\":\"state_update\",\"position\":{\"x\":10,\"y\":20},\"score\":0}"}
{"ts":2000,"dir":"out","text":"{\"action\":\"enemy_spawn\",\"enemy_id\":\"{{uuid}}\",\"position\":{\"x\":50,\"y\":30}}"}
{"ts":1500,"dir":"out","text":"{\"action\":\"powerup\",\"type\":\"speed\",\"position\":{\"x\":25,\"y\":15}}"}
{"ts":3000,"dir":"out","text":"{\"action\":\"game_over\",\"final_score\":1250,\"reason\":\"timeout\"}"}
```

## Advanced Replay Techniques

### Conditional Branching

While replay mode is inherently linear, you can simulate branching using multiple replay files and external logic:

```jsonl
// File: login-success.jsonl
{"ts":0,"dir":"out","text":"Login successful","waitFor":"^ready$"}
{"ts":100,"dir":"out","text":"Welcome to your dashboard"}

// File: login-failed.jsonl
{"ts":0,"dir":"out","text":"Invalid credentials"}
{"ts":500,"dir":"out","text":"Connection will close","close":true}
```

### Template Integration

```jsonl
{"ts":0,"dir":"out","text":"Session {{uuid}} established at {{now}}"}
{"ts":1000,"dir":"out","text":"Your lucky number is: {{randInt 1 100}}"}
{"ts":2000,"dir":"out","text":"Next maintenance window: {{now+24h}}"}
{"ts":3000,"dir":"out","text":"Server load: {{randInt 20 80}}%"}
```

### Binary Message Support

```jsonl
{"ts":0,"dir":"out","text":"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==","binary":true}
{"ts":1000,"dir":"out","text":"Image sent successfully"}
```

### Error Simulation

```jsonl
{"ts":0,"dir":"out","text":"Connection established"}
{"ts":5000,"dir":"out","text":"Internal server error","error":true}
{"ts":1000,"dir":"out","text":"Attempting reconnection..."}
{"ts":2000,"dir":"out","text":"Reconnection failed","close":true}
```

## Creating Replay Files

### Manual Creation

```bash
# Create a new replay file
cat > chat-replay.jsonl << 'EOF'
{"ts":0,"dir":"out","text":"Welcome to support chat!"}
{"ts":1000,"dir":"out","text":"How can I help you today?","waitFor":".*"}
{"ts":500,"dir":"out","text":"Thanks for your question. Let me check..."}
{"ts":2000,"dir":"out","text":"I found the solution! Here's what you need to do:"}
{"ts":1000,"dir":"out","text":"1. Go to settings\n2. Click preferences\n3. Enable feature X"}
{"ts":3000,"dir":"out","text":"Does this solve your issue?","waitFor":"^(yes|no)$"}
{"ts":500,"dir":"out","text":"Great! Glad I could help. Have a nice day!"}
EOF
```

### From Application Logs

```bash
#!/bin/bash
# extract-websocket-logs.sh

# Extract WebSocket messages from application logs
grep "WEBSOCKET_MSG" app.log | \
  # Parse log entries and convert to JSONL
  awk '{
    # Extract timestamp, direction, and message
    match($0, /([0-9]+).*dir=([^ ]*).*msg=(.*)/, arr)
    printf "{\"ts\":%d,\"dir\":\"%s\",\"text\":\"%s\"}\n", arr[1], arr[2], arr[3]
  }' > replay-from-logs.jsonl
```

### Programmatic Generation

```javascript
// generate-replay.js
const fs = require('fs');

function generateHeartbeatReplay(interval = 30000, duration = 300000) {
  const messages = [];
  const messageCount = duration / interval;

  for (let i = 0; i < messageCount; i++) {
    messages.push({
      ts: i * interval,
      dir: "out",
      text: JSON.stringify({
        type: "heartbeat",
        timestamp: `{{now+${i * interval}ms}}`,
        sequence: i + 1
      })
    });
  }

  fs.writeFileSync('heartbeat-replay.jsonl',
    messages.map(JSON.stringify).join('\n'));
}

generateHeartbeatReplay();
```

```python
# generate-replay.py
import json
import random

def generate_data_stream(count=100, interval=1000):
    messages = []
    for i in range(count):
        messages.append({
            "ts": i * interval,
            "dir": "out",
            "text": json.dumps({
                "type": "data_point",
                "id": f"{{{{uuid}}}}",
                "value": random.randint(1, 100),
                "timestamp": f"{{{{now+{i * interval}ms}}}}}"
            })
        })
    return messages

# Write to file
with open('data-stream-replay.jsonl', 'w') as f:
    for msg in generate_data_stream():
        f.write(json.dumps(msg) + '\n')
```

## Validation and Testing

### Replay File Validation

```bash
# Validate JSONL syntax
node -e "
const fs = require('fs');
const lines = fs.readFileSync('replay.jsonl', 'utf8').split('\n');
let valid = true;

lines.forEach((line, i) => {
  if (line.trim()) {
    try {
      const msg = JSON.parse(line);
      if (!msg.ts || !msg.dir || !msg.text) {
        console.log(\`Line \${i+1}: Missing required fields\`);
        valid = false;
      }
      if (typeof msg.ts !== 'number' || msg.ts < 0) {
        console.log(\`Line \${i+1}: Invalid timestamp\`);
        valid = false;
      }
      if (!['in', 'out'].includes(msg.dir)) {
        console.log(\`Line \${i+1}: Invalid direction\`);
        valid = false;
      }
    } catch (e) {
      console.log(\`Line \${i+1}: Invalid JSON - \${e.message}\`);
      valid = false;
    }
  }
});

console.log(valid ? '✓ Replay file is valid' : '✗ Replay file has errors');
"
```

### Timing Analysis

```bash
# Analyze replay timing
node -e "
const fs = require('fs');
const messages = fs.readFileSync('replay.jsonl', 'utf8')
  .split('\n')
  .filter(line => line.trim())
  .map(line => JSON.parse(line));

const timings = messages.map((msg, i) => ({
  index: i + 1,
  ts: msg.ts,
  interval: i > 0 ? msg.ts - messages[i-1].ts : 0
}));

console.log('Timing Analysis:');
timings.forEach(t => {
  console.log(\`Message \${t.index}: \${t.ts}ms (interval: \${t.interval}ms)\`);
});

const totalDuration = Math.max(...messages.map(m => m.ts));
console.log(\`Total duration: \${totalDuration}ms (\${(totalDuration/1000).toFixed(1)}s)\`);
"
```

### Functional Testing

```bash
#!/bin/bash
# test-replay.sh

REPLAY_FILE=$1
WS_URL="ws://localhost:3001/ws"

echo "Testing replay file: $REPLAY_FILE"

# Validate file exists and is readable
if [ ! -f "$REPLAY_FILE" ]; then
  echo "✗ Replay file not found"
  exit 1
fi

# Basic syntax check
if ! node -e "
  const fs = require('fs');
  const content = fs.readFileSync('$REPLAY_FILE', 'utf8');
  const lines = content.split('\n').filter(l => l.trim());
  lines.forEach((line, i) => {
    try {
      JSON.parse(line);
    } catch (e) {
      console.error(\`Line \${i+1}: \${e.message}\`);
      process.exit(1);
    }
  });
  console.log(\`✓ Valid JSONL: \${lines.length} messages\`);
"; then
  echo "✗ Syntax validation failed"
  exit 1
fi

echo "✓ Replay file validation passed"
echo "Ready to test with: mockforge serve --ws-replay-file $REPLAY_FILE"
```

## Best Practices

### File Organization

1. **Descriptive Names**: Use clear, descriptive filenames
   ```
   user-authentication-flow.jsonl
   real-time-data-stream.jsonl
   error-handling-scenarios.jsonl
   ```

2. **Modular Scenarios**: Break complex interactions into focused files
   ```
   login-flow.jsonl
   main-interaction.jsonl
   logout-flow.jsonl
   ```

3. **Version Control**: Keep replay files in Git with meaningful commit messages

### Performance Optimization

1. **Message Batching**: Group related messages with minimal intervals
2. **Memory Management**: Monitor memory usage with large replay files
3. **Connection Limits**: Consider concurrent connection impact

### Maintenance

1. **Regular Updates**: Keep replay files synchronized with application changes
2. **Documentation**: Comment complex scenarios inline
3. **Versioning**: Tag replay files with application versions

### Debugging

1. **Verbose Logging**: Enable detailed WebSocket logging during development
2. **Step-through Testing**: Test replay files incrementally
3. **Timing Verification**: Validate message timing against expectations

## Common Patterns

### Authentication Flow

```jsonl
{"ts":0,"dir":"out","text":"Please authenticate","waitFor":"^AUTH .+$"}
{"ts":100,"dir":"out","text":"Authenticating..."}
{"ts":500,"dir":"out","text":"Authentication successful"}
{"ts":200,"dir":"out","text":"Welcome back, user!"}
```

### Streaming Data

```jsonl
{"ts":0,"dir":"out","text":"{\"type\":\"stream_start\",\"stream_id\":\"{{uuid}}\"}"}
{"ts":100,"dir":"out","text":"{\"type\":\"data\",\"value\":{{randInt 1 100}}}"}
{"ts":100,"dir":"out","text":"{\"type\":\"data\",\"value\":{{randInt 1 100}}}"}
{"ts":100,"dir":"out","text":"{\"type\":\"data\",\"value\":{{randInt 1 100}}}"}
{"ts":5000,"dir":"out","text":"{\"type\":\"stream_end\",\"total_messages\":3}"}
```

### Error Recovery

```jsonl
{"ts":0,"dir":"out","text":"System operational"}
{"ts":30000,"dir":"out","text":"Warning: High load detected"}
{"ts":10000,"dir":"out","text":"Error: Service unavailable","error":true}
{"ts":5000,"dir":"out","text":"Attempting recovery..."}
{"ts":10000,"dir":"out","text":"Recovery successful"}
{"ts":1000,"dir":"out","text":"System back to normal"}
```

## Integration with CI/CD

### Automated Testing

```yaml
# .github/workflows/test.yml
name: WebSocket Tests
on: [push, pull_request]

jobs:
  websocket-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '18'
      - name: Install dependencies
        run: npm install ws
      - name: Start MockForge
        run: |
          cargo install mockforge-cli
          mockforge serve --ws-replay-file examples/ws-demo.jsonl &
          sleep 2
      - name: Run WebSocket tests
        run: node test-websocket.js
```

### Performance Benchmarking

```bash
#!/bin/bash
# benchmark-replay.sh

CONCURRENT_CONNECTIONS=100
DURATION=60

echo "Benchmarking WebSocket replay with $CONCURRENT_CONNECTIONS connections for ${DURATION}s"

# Start MockForge
mockforge serve --ws-replay-file benchmark-replay.jsonl &
SERVER_PID=$!
sleep 2

# Run benchmark
node benchmark-websocket.js $CONCURRENT_CONNECTIONS $DURATION

# Cleanup
kill $SERVER_PID
```

This comprehensive approach to replay mode ensures reliable, deterministic WebSocket testing scenarios that can be easily created, validated, and maintained as part of your testing infrastructure.
