# Streaming

gRPC supports four fundamental communication patterns, with three involving streaming. MockForge provides comprehensive support for all streaming patterns, enabling realistic testing of real-time and batch data scenarios.

## Streaming Patterns

### Unary (Request → Response)
Standard request-response pattern - one message in, one message out.

### Server Streaming (Request → Stream of Responses)
Single request initiates a stream of responses from server to client.

### Client Streaming (Stream of Requests → Response)
Client sends multiple messages, server responds once with aggregated result.

### Bidirectional Streaming (Stream ↔ Stream)
Both client and server can send messages independently and simultaneously.

## Server Streaming

### Basic Server Streaming

```protobuf
service NotificationService {
  rpc Subscribe(SubscribeRequest) returns (stream Notification);
}

message SubscribeRequest {
  repeated string topics = 1;
  SubscriptionType type = 2;
}

message Notification {
  string topic = 1;
  string message = 2;
  google.protobuf.Timestamp timestamp = 3;
  Severity severity = 4;
}

enum SubscriptionType {
  REALTIME = 0;
  BATCH = 1;
}

enum Severity {
  INFO = 0;
  WARNING = 1;
  ERROR = 2;
  CRITICAL = 3;
}
```

### MockForge Configuration

Server streaming generates multiple responses based on configuration:

```jsonl
// Basic server streaming - fixed number of responses
{"ts":0,"dir":"out","text":"{\"topic\":\"system\",\"message\":\"Connected\",\"severity\":\"INFO\"}"}
{"ts":1000,"dir":"out","text":"{\"topic\":\"user\",\"message\":\"New user registered\",\"severity\":\"INFO\"}"}
{"ts":2000,"dir":"out","text":"{\"topic\":\"payment\",\"message\":\"Payment processed\",\"severity\":\"INFO\"}"}
{"ts":3000,"dir":"out","text":"{\"topic\":\"system\",\"message\":\"Maintenance scheduled\",\"severity\":\"WARNING\"}"}
```

### Dynamic Server Streaming

```jsonl
// Template-based dynamic responses
{"ts":0,"dir":"out","text":"{\"topic\":\"{{request.topics[0]}}\",\"message\":\"Subscribed to {{request.topics.length}} topics\",\"timestamp\":\"{{now}}\"}"}
{"ts":1000,"dir":"out","text":"{\"topic\":\"{{randFromArray request.topics}}\",\"message\":\"{{randParagraph}}\",\"timestamp\":\"{{now}}\"}"}
{"ts":2000,"dir":"out","text":"{\"topic\":\"{{randFromArray request.topics}}\",\"message\":\"{{randSentence}}\",\"timestamp\":\"{{now}}\"}"}
{"ts":5000,"dir":"out","text":"{\"topic\":\"system\",\"message\":\"Stream ending\",\"timestamp\":\"{{now}}\"}"}
```

### Testing Server Streaming

#### Using grpcurl

```bash
# Test server streaming
grpcurl -plaintext -d '{"topics": ["user", "payment"], "type": "REALTIME"}' \
  localhost:50051 myapp.NotificationService/Subscribe
```

#### Using Node.js

```javascript
const grpc = require('@grpc/grpc-js');
const protoLoader = require('@grpc/proto-loader');

const packageDefinition = protoLoader.loadSync('proto/notification.proto');
const proto = grpc.loadPackageDefinition(packageDefinition);

const client = new proto.myapp.NotificationService(
  'localhost:50051',
  grpc.credentials.createInsecure()
);

const call = client.Subscribe({
  topics: ['user', 'payment'],
  type: 'REALTIME'
});

call.on('data', (notification) => {
  console.log('Notification:', notification);
});

call.on('end', () => {
  console.log('Stream ended');
});

call.on('error', (error) => {
  console.error('Error:', error);
});
```

## Client Streaming

### Basic Client Streaming

```protobuf
service UploadService {
  rpc UploadFile(stream FileChunk) returns (UploadResponse);
}

message FileChunk {
  bytes data = 1;
  int32 sequence = 2;
  bool is_last = 3;
}

message UploadResponse {
  string file_id = 1;
  int64 total_size = 2;
  string checksum = 3;
  UploadStatus status = 4;
}

enum UploadStatus {
  SUCCESS = 0;
  FAILED = 1;
  PARTIAL = 2;
}
```

### MockForge Configuration

Client streaming processes multiple incoming messages and returns a single response:

```jsonl
// Client streaming - processes multiple chunks
{"ts":0,"dir":"in","text":".*","response":"{\"file_id\":\"{{uuid}}\",\"total_size\":1024,\"status\":\"SUCCESS\"}"}
```

### Advanced Client Streaming

```jsonl
// Process chunks and maintain state
{"ts":0,"dir":"in","text":"{\"sequence\":0}","response":"Chunk 0 received","state":"uploading","chunks":1}
{"ts":0,"dir":"in","text":"{\"sequence\":1}","response":"Chunk 1 received","chunks":"{{request.ws.state.chunks + 1}}"}
{"ts":0,"dir":"in","text":"{\"is_last\":true}","response":"{\"file_id\":\"{{uuid}}\",\"total_size\":\"{{request.ws.state.chunks * 1024}}\",\"status\":\"SUCCESS\"}"}
```

### Testing Client Streaming

#### Using grpcurl

```bash
# Send multiple messages for client streaming
echo '{"data": "chunk1", "sequence": 0}' | \
grpcurl -plaintext -d @ localhost:50051 myapp.UploadService/UploadFile

echo '{"data": "chunk2", "sequence": 1}' | \
grpcurl -plaintext -d @ localhost:50051 myapp.UploadService/UploadFile

echo '{"data": "chunk3", "sequence": 2, "is_last": true}' | \
grpcurl -plaintext -d @ localhost:50051 myapp.UploadService/UploadFile
```

#### Using Python

```python
import grpc
from upload_pb2 import FileChunk
from upload_pb2_grpc import UploadServiceStub

def generate_chunks():
    # Simulate file chunks
    chunks = [
        b"chunk1",
        b"chunk2",
        b"chunk3"
    ]

    for i, chunk in enumerate(chunks):
        yield FileChunk(
            data=chunk,
            sequence=i,
            is_last=(i == len(chunks) - 1)
        )

channel = grpc.insecure_channel('localhost:50051')
stub = UploadServiceStub(channel)

response = stub.UploadFile(generate_chunks())
print(f"Upload result: {response}")
```

## Bidirectional Streaming

### Basic Bidirectional Streaming

```protobuf
service ChatService {
  rpc Chat(stream ChatMessage) returns (stream ChatMessage);
}

message ChatMessage {
  string user_id = 1;
  string content = 2;
  MessageType type = 3;
  google.protobuf.Timestamp timestamp = 4;
}

enum MessageType {
  TEXT = 0;
  JOIN = 1;
  LEAVE = 2;
  SYSTEM = 3;
}
```

### MockForge Configuration

Bidirectional streaming handles both incoming and outgoing messages:

```jsonl
// Welcome message on connection
{"ts":0,"dir":"out","text":"{\"user_id\":\"system\",\"content\":\"Welcome to chat!\",\"type\":\"SYSTEM\"}"}

// Handle join messages
{"ts":0,"dir":"in","text":"{\"type\":\"JOIN\"}","response":"{\"user_id\":\"system\",\"content\":\"{{request.ws.message.user_id}} joined the chat\",\"type\":\"SYSTEM\"}"}

// Handle text messages
{"ts":0,"dir":"in","text":"{\"type\":\"TEXT\"}","response":"{\"user_id\":\"{{request.ws.message.user_id}}\",\"content\":\"{{request.ws.message.content}}\",\"type\":\"TEXT\"}"}

// Handle leave messages
{"ts":0,"dir":"in","text":"{\"type\":\"LEAVE\"}","response":"{\"user_id\":\"system\",\"content\":\"{{request.ws.message.user_id}} left the chat\",\"type\":\"SYSTEM\"}"}

// Periodic system messages
{"ts":30000,"dir":"out","text":"{\"user_id\":\"system\",\"content\":\"Server uptime: {{randInt 1 24}} hours\",\"type\":\"SYSTEM\"}"}
```

### Advanced Bidirectional Patterns

```jsonl
// State-aware responses
{"ts":0,"dir":"in","text":".*","condition":"{{!request.ws.state.authenticated}}","response":"Please authenticate first"}
{"ts":0,"dir":"in","text":"AUTH","response":"Authenticated","state":"authenticated"}

{"ts":0,"dir":"in","text":".*","condition":"{{request.ws.state.authenticated}}","response":"{{request.ws.message}}"}

{"ts":0,"dir":"in","text":"HELP","response":"Available commands: MSG, QUIT, STATUS"}
{"ts":0,"dir":"in","text":"STATUS","response":"Connected users: {{randInt 1 50}}"}
{"ts":0,"dir":"in","text":"QUIT","response":"Goodbye!","close":true}
```

### Testing Bidirectional Streaming

#### Using Node.js

```javascript
const grpc = require('@grpc/grpc-js');
const protoLoader = require('@grpc/proto-loader');

const packageDefinition = protoLoader.loadSync('proto/chat.proto');
const proto = grpc.loadPackageDefinition(packageDefinition);

const client = new proto.myapp.ChatService(
  'localhost:50051',
  grpc.credentials.createInsecure()
);

const call = client.Chat();

// Handle incoming messages
call.on('data', (message) => {
  console.log('Received:', message);
});

// Send messages
setInterval(() => {
  call.write({
    user_id: 'user123',
    content: 'Hello from client',
    type: 'TEXT'
  });
}, 2000);

// Send join message
call.write({
  user_id: 'user123',
  content: 'Joined chat',
  type: 'JOIN'
});

// Handle stream end
call.on('end', () => {
  console.log('Stream ended');
});

// Close after 30 seconds
setTimeout(() => {
  call.write({
    user_id: 'user123',
    content: 'Leaving chat',
    type: 'LEAVE'
  });
  call.end();
}, 30000);
```

## Streaming Configuration

### Environment Variables

```bash
# Streaming behavior
MOCKFORGE_GRPC_STREAM_TIMEOUT=30000        # Stream timeout in ms
MOCKFORGE_GRPC_MAX_STREAM_MESSAGES=1000    # Max messages per stream
MOCKFORGE_GRPC_STREAM_BUFFER_SIZE=1024     # Buffer size for streaming

# Response timing
MOCKFORGE_GRPC_LATENCY_MIN_MS=10          # Minimum response latency
MOCKFORGE_GRPC_LATENCY_MAX_MS=100         # Maximum response latency
```

### Stream Control Templates

```jsonl
// Conditional streaming
{"ts":0,"dir":"out","text":"Starting stream","condition":"{{request.stream_enabled}}"}
{"ts":1000,"dir":"out","text":"Stream data","condition":"{{request.ws.state.active}}"}
{"ts":0,"dir":"out","text":"Stream ended","condition":"{{request.ws.message.type === 'END'}}","close":true}

// Dynamic intervals
{"ts":"{{randInt 1000 5000}}","dir":"out","text":"Random interval message"}
{"ts":"{{request.interval || 2000}}","dir":"out","text":"Custom interval message"}
```

## Performance Considerations

### Memory Management

```jsonl
// Limit message history
{"ts":0,"dir":"in","text":".*","condition":"{{(request.ws.state.messageCount || 0) < 100}}","response":"Message received","messageCount":"{{(request.ws.state.messageCount || 0) + 1}}"}
{"ts":0,"dir":"in","text":".*","condition":"{{(request.ws.state.messageCount || 0) >= 100}}","response":"Message limit reached"}
```

### Connection Limits

```jsonl
// Global connection tracking (requires custom implementation)
{"ts":0,"dir":"out","text":"Connection {{request.ws.connectionId}} established"}
{"ts":300000,"dir":"out","text":"Connection timeout","close":true}
```

### Load Balancing

```jsonl
// Simulate load balancer behavior
{"ts":"{{randInt 100 1000}}","dir":"out","text":"Response from server {{randInt 1 3}}"}
{"ts":"{{randInt 2000 5000}}","dir":"out","text":"Health check from server {{randInt 1 3}}"}
```

## Error Handling in Streams

### Stream Errors

```jsonl
// Handle invalid messages
{"ts":0,"dir":"in","text":"","response":"Empty message not allowed"}
{"ts":0,"dir":"in","text":".{500,}","response":"Message too long (max 500 chars)"}

// Simulate network errors
{"ts":5000,"dir":"out","text":"Network error occurred","error":true,"close":true}
```

### Recovery Patterns

```jsonl
// Automatic reconnection
{"ts":0,"dir":"out","text":"Connection lost, attempting reconnect..."}
{"ts":2000,"dir":"out","text":"Reconnected successfully"}
{"ts":100,"dir":"out","text":"Resuming stream from message {{request.ws.state.lastMessageId}}"}
```

## Testing Strategies

### Unit Testing Streams

```javascript
// test-streaming.js
const { expect } = require('chai');

describe('gRPC Streaming', () => {
  it('should handle server streaming', (done) => {
    const call = client.subscribeNotifications({ topics: ['test'] });

    let messageCount = 0;
    call.on('data', (notification) => {
      messageCount++;
      expect(notification).to.have.property('topic');
      expect(notification).to.have.property('message');
    });

    call.on('end', () => {
      expect(messageCount).to.be.greaterThan(0);
      done();
    });

    // End test after 5 seconds
    setTimeout(() => call.cancel(), 5000);
  });

  it('should handle client streaming', (done) => {
    const call = client.uploadFile((error, response) => {
      expect(error).to.be.null;
      expect(response).to.have.property('file_id');
      expect(response.status).to.equal('SUCCESS');
      done();
    });

    // Send test chunks
    call.write({ data: Buffer.from('test'), sequence: 0 });
    call.write({ data: Buffer.from('data'), sequence: 1, is_last: true });
    call.end();
  });
});
```

### Load Testing

```bash
#!/bin/bash
# load-test-streams.sh

CONCURRENT_STREAMS=10
DURATION=60

echo "Load testing $CONCURRENT_STREAMS concurrent streams for ${DURATION}s"

for i in $(seq 1 $CONCURRENT_STREAMS); do
  node stream-client.js &
done

# Wait for test duration
sleep $DURATION

# Kill all clients
pkill -f stream-client.js

echo "Load test completed"
```

## Best Practices

### Stream Design

1. **Appropriate Patterns**: Choose the right streaming pattern for your use case
2. **Message Size**: Keep individual messages reasonably sized
3. **Heartbeat Messages**: Include periodic keepalive messages for long-running streams
4. **Error Recovery**: Implement proper error handling and recovery mechanisms

### Performance Optimization

1. **Buffering**: Use appropriate buffer sizes for your throughput requirements
2. **Compression**: Enable compression for large message streams
3. **Connection Reuse**: Reuse connections when possible
4. **Resource Limits**: Set appropriate limits on concurrent streams and message rates

### Monitoring and Debugging

1. **Stream Metrics**: Monitor stream duration, message counts, and error rates
2. **Logging**: Enable detailed logging for debugging streaming issues
3. **Tracing**: Implement request tracing across stream messages
4. **Health Checks**: Regular health checks for long-running streams

### Client Compatibility

1. **Protocol Versions**: Ensure compatibility with different gRPC versions
2. **Language Support**: Test with multiple client language implementations
3. **Network Conditions**: Test under various network conditions (latency, packet loss)
4. **Browser Support**: Consider WebSocket fallback for web clients

## Troubleshooting

### Common Streaming Issues

**Stream doesn't start**: Check proto file definitions and service registration

**Messages not received**: Verify message encoding and template syntax

**Stream hangs**: Check for proper stream termination and timeout settings

**Performance degradation**: Monitor resource usage and adjust buffer sizes

**Client disconnects**: Implement proper heartbeat and reconnection logic

### Debug Commands

```bash
# Monitor active streams
grpcurl -plaintext localhost:50051 list

# Check stream status
netstat -tlnp | grep :50051

# View stream logs
tail -f mockforge.log | grep -E "(stream|grpc)"

# Test basic connectivity
grpcurl -plaintext localhost:50051 grpc.reflection.v1alpha.ServerReflection/ServerReflectionInfo
```

### Performance Profiling

```bash
# Profile gRPC performance
cargo flamegraph --bin mockforge-cli -- serve --grpc-port 50051

# Monitor system resources
htop -p $(pgrep mockforge)

# Network monitoring
iftop -i lo
```

Streaming patterns enable powerful real-time communication scenarios. MockForge's comprehensive streaming support allows you to create sophisticated mock environments that accurately simulate production streaming services for thorough testing and development.
