import grpc from 'k6/net/grpc';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('grpc_errors');
const requestDuration = new Trend('grpc_request_duration');
const requestCounter = new Counter('grpc_requests_total');
const streamMessages = new Counter('grpc_stream_messages');

// Test configuration
export const options = {
  stages: [
    { duration: '30s', target: 20 },   // Ramp up to 20 users
    { duration: '1m', target: 50 },    // Ramp up to 50 users
    { duration: '2m', target: 100 },   // Ramp up to 100 users
    { duration: '2m', target: 100 },   // Stay at 100 users for 2 minutes
    { duration: '1m', target: 50 },    // Ramp down
    { duration: '30s', target: 0 },    // Ramp down to 0
  ],
  thresholds: {
    'grpc_request_duration': ['p(95)<500', 'p(99)<1000'],
    'grpc_errors': ['rate<0.05'],
  },
};

const GRPC_ADDR = __ENV.GRPC_ADDR || 'localhost:50051';
const USE_TLS = __ENV.USE_TLS === 'true';

const client = new grpc.Client();

export function setup() {
  console.log(`Connecting to gRPC server at ${GRPC_ADDR}`);

  // Connect to the gRPC server
  if (USE_TLS) {
    client.connect(GRPC_ADDR, {
      plaintext: false,
    });
  } else {
    client.connect(GRPC_ADDR, {
      plaintext: true,
    });
  }
}

export default function () {
  // Test unary RPC calls
  testUnaryCall();
  sleep(1);

  // Test server streaming
  testServerStreaming();
  sleep(1);

  // Test client streaming
  testClientStreaming();
  sleep(1);

  // Test bidirectional streaming
  testBidirectionalStreaming();
  sleep(1);
}

function testUnaryCall() {
  const startTime = Date.now();

  const data = {
    name: `user-${__VU}`,
    message: `Hello from VU ${__VU}, iteration ${__ITER}`,
    timestamp: Date.now(),
  };

  const response = client.invoke('mockforge.MockService/UnaryCall', data, {
    metadata: {
      'x-request-id': `req-${__VU}-${__ITER}`,
    },
  });

  const duration = Date.now() - startTime;
  requestDuration.add(duration);
  requestCounter.add(1);

  const success = check(response, {
    'Unary call status is OK': (r) => r && r.status === grpc.StatusOK,
    'Unary call response has data': (r) => r && r.message !== undefined,
    'Unary call response time < 500ms': () => duration < 500,
  });

  if (!success) {
    errorRate.add(1);
    console.error(`Unary call failed: ${JSON.stringify(response)}`);
  }
}

function testServerStreaming() {
  const startTime = Date.now();
  let messageCount = 0;

  const stream = client.invoke('mockforge.MockService/ServerStream', {
    count: 10,
    interval: 100,
  });

  stream.on('data', (message) => {
    messageCount++;
    streamMessages.add(1);

    check(message, {
      'Server stream message has data': (m) => m !== undefined,
      'Server stream message has sequence': (m) => m.sequence !== undefined,
    });
  });

  stream.on('end', () => {
    const duration = Date.now() - startTime;
    requestDuration.add(duration);
    requestCounter.add(1);

    const success = check(null, {
      'Server stream received all messages': () => messageCount === 10,
      'Server stream completed in reasonable time': () => duration < 5000,
    });

    if (!success) {
      errorRate.add(1);
    }
  });

  stream.on('error', (error) => {
    console.error(`Server streaming error: ${error}`);
    errorRate.add(1);
  });
}

function testClientStreaming() {
  const startTime = Date.now();

  const stream = client.invoke('mockforge.MockService/ClientStream', {});

  // Send multiple messages to the server
  for (let i = 0; i < 10; i++) {
    stream.write({
      sequence: i,
      data: `Message ${i} from VU ${__VU}`,
      timestamp: Date.now(),
    });
    streamMessages.add(1);
  }

  stream.end();

  const response = stream.waitForResponse();
  const duration = Date.now() - startTime;
  requestDuration.add(duration);
  requestCounter.add(1);

  const success = check(response, {
    'Client stream status is OK': (r) => r && r.status === grpc.StatusOK,
    'Client stream response has summary': (r) => r && r.message && r.message.totalReceived === 10,
    'Client stream completed in reasonable time': () => duration < 5000,
  });

  if (!success) {
    errorRate.add(1);
  }
}

function testBidirectionalStreaming() {
  const startTime = Date.now();
  let sentCount = 0;
  let receivedCount = 0;

  const stream = client.invoke('mockforge.MockService/BidirectionalStream', {});

  stream.on('data', (message) => {
    receivedCount++;
    streamMessages.add(1);

    check(message, {
      'Bidirectional stream message has echo': (m) => m !== undefined,
    });

    // Send response for each message received
    if (sentCount < 10) {
      stream.write({
        sequence: sentCount,
        data: `Bidirectional message ${sentCount}`,
        timestamp: Date.now(),
      });
      sentCount++;
      streamMessages.add(1);
    }
  });

  stream.on('end', () => {
    const duration = Date.now() - startTime;
    requestDuration.add(duration);
    requestCounter.add(1);

    const success = check(null, {
      'Bidirectional stream sent messages': () => sentCount > 0,
      'Bidirectional stream received messages': () => receivedCount > 0,
      'Bidirectional stream completed in reasonable time': () => duration < 10000,
    });

    if (!success) {
      errorRate.add(1);
    }
  });

  stream.on('error', (error) => {
    console.error(`Bidirectional streaming error: ${error}`);
    errorRate.add(1);
  });

  // Send initial message to start the stream
  stream.write({
    sequence: sentCount,
    data: `Initial message from VU ${__VU}`,
    timestamp: Date.now(),
  });
  sentCount++;
  streamMessages.add(1);
}

export function teardown() {
  client.close();
  console.log('gRPC load test completed');
}

// Alternative scenario-based configuration
export const scenarios = {
  unary_only: {
    executor: 'constant-vus',
    vus: 50,
    duration: '2m',
    exec: 'unaryOnly',
    tags: { scenario: 'unary' },
  },
  streaming_only: {
    executor: 'ramping-vus',
    startVUs: 10,
    stages: [
      { duration: '1m', target: 30 },
      { duration: '2m', target: 30 },
      { duration: '1m', target: 0 },
    ],
    exec: 'streamingOnly',
    tags: { scenario: 'streaming' },
  },
  mixed_load: {
    executor: 'per-vu-iterations',
    vus: 20,
    iterations: 100,
    exec: 'mixedLoad',
    tags: { scenario: 'mixed' },
  },
};

export function unaryOnly() {
  testUnaryCall();
  sleep(0.5);
}

export function streamingOnly() {
  testServerStreaming();
  sleep(2);
  testClientStreaming();
  sleep(2);
}

export function mixedLoad() {
  const scenario = Math.random();

  if (scenario < 0.4) {
    testUnaryCall();
  } else if (scenario < 0.7) {
    testServerStreaming();
  } else if (scenario < 0.9) {
    testClientStreaming();
  } else {
    testBidirectionalStreaming();
  }

  sleep(1);
}
