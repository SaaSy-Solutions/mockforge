import { WebSocket } from 'k6/experimental/websockets';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('ws_errors');
const connectionTime = new Trend('ws_connection_time');
const messageLatency = new Trend('ws_message_latency');
const messagesReceived = new Counter('ws_messages_received');
const messagesSent = new Counter('ws_messages_sent');

// Test configuration
export const options = {
  stages: [
    { duration: '30s', target: 10 },   // Ramp up to 10 concurrent connections
    { duration: '1m', target: 50 },    // Ramp up to 50 connections
    { duration: '2m', target: 100 },   // Ramp up to 100 connections
    { duration: '2m', target: 100 },   // Stay at 100 connections for 2 minutes
    { duration: '1m', target: 50 },    // Ramp down
    { duration: '30s', target: 0 },    // Ramp down to 0
  ],
  thresholds: {
    'ws_connection_time': ['p(95)<1000', 'p(99)<2000'],  // Connection time thresholds
    'ws_message_latency': ['p(95)<200', 'p(99)<500'],    // Message latency thresholds
    'ws_errors': ['rate<0.05'],                          // Error rate under 5%
  },
};

const BASE_URL = __ENV.BASE_URL || 'ws://localhost:8080';
const WS_URL = `${BASE_URL}/ws`;

export default function () {
  const startTime = Date.now();
  let messagesSentCount = 0;
  let messagesReceivedCount = 0;

  const ws = new WebSocket(WS_URL);

  ws.addEventListener('open', () => {
    const connectionDuration = Date.now() - startTime;
    connectionTime.add(connectionDuration);

    console.log(`WebSocket connection established in ${connectionDuration}ms`);

    // Send initial message
    const msg = JSON.stringify({
      type: 'hello',
      timestamp: Date.now(),
      payload: {
        user: `user-${__VU}`,
        session: `session-${__ITER}`,
      },
    });

    ws.send(msg);
    messagesSent.add(1);
    messagesSentCount++;
  });

  ws.addEventListener('message', (event) => {
    const receivedAt = Date.now();
    messagesReceived.add(1);
    messagesReceivedCount++;

    try {
      const data = JSON.parse(event.data);

      check(data, {
        'Message has type': (d) => d.type !== undefined,
        'Message has timestamp': (d) => d.timestamp !== undefined,
      });

      // Calculate latency if timestamp is present
      if (data.timestamp) {
        const latency = receivedAt - data.timestamp;
        messageLatency.add(latency);
      }

      // Send echo response
      if (data.type === 'ping') {
        const pongMsg = JSON.stringify({
          type: 'pong',
          timestamp: Date.now(),
          originalTimestamp: data.timestamp,
        });
        ws.send(pongMsg);
        messagesSent.add(1);
        messagesSentCount++;
      }

      // Send periodic messages
      if (messagesReceivedCount % 5 === 0) {
        const dataMsg = JSON.stringify({
          type: 'data',
          timestamp: Date.now(),
          payload: {
            counter: messagesReceivedCount,
            vu: __VU,
            iteration: __ITER,
          },
        });
        ws.send(dataMsg);
        messagesSent.add(1);
        messagesSentCount++;
      }

    } catch (e) {
      console.error(`Error parsing message: ${e}`);
      errorRate.add(1);
    }
  });

  ws.addEventListener('error', (event) => {
    console.error(`WebSocket error: ${JSON.stringify(event)}`);
    errorRate.add(1);
  });

  ws.addEventListener('close', () => {
    console.log(`WebSocket connection closed. Sent: ${messagesSentCount}, Received: ${messagesReceivedCount}`);

    check(null, {
      'At least one message received': () => messagesReceivedCount > 0,
      'At least one message sent': () => messagesSentCount > 0,
    });
  });

  // Keep connection open for a period
  sleep(10);

  // Send burst of messages
  for (let i = 0; i < 10; i++) {
    const burstMsg = JSON.stringify({
      type: 'burst',
      timestamp: Date.now(),
      sequence: i,
      payload: { data: `burst-message-${i}` },
    });
    ws.send(burstMsg);
    messagesSent.add(1);
    messagesSentCount++;
    sleep(0.1);
  }

  // Keep connection open to receive responses
  sleep(5);

  // Send close message
  const closeMsg = JSON.stringify({
    type: 'close',
    timestamp: Date.now(),
  });
  ws.send(closeMsg);
  messagesSent.add(1);
  messagesSentCount++;

  sleep(1);
  ws.close();
}

// Scenario-based test (alternative)
export function scenarioTest() {
  const scenarios = {
    continuous_connection: {
      executor: 'constant-vus',
      vus: 50,
      duration: '5m',
      exec: 'continuousConnection',
    },
    burst_connections: {
      executor: 'ramping-arrival-rate',
      startRate: 10,
      timeUnit: '1s',
      preAllocatedVUs: 100,
      maxVUs: 200,
      stages: [
        { duration: '30s', target: 50 },
        { duration: '1m', target: 100 },
        { duration: '30s', target: 50 },
      ],
      exec: 'burstConnection',
    },
  };

  return scenarios;
}

export function continuousConnection() {
  const ws = new WebSocket(WS_URL);

  ws.addEventListener('open', () => {
    // Send messages continuously
    const interval = setInterval(() => {
      const msg = JSON.stringify({
        type: 'heartbeat',
        timestamp: Date.now(),
      });
      ws.send(msg);
      messagesSent.add(1);
    }, 1000);

    // Stop after 4 minutes
    setTimeout(() => {
      clearInterval(interval);
      ws.close();
    }, 4 * 60 * 1000);
  });

  ws.addEventListener('message', (event) => {
    messagesReceived.add(1);
    const receivedAt = Date.now();
    try {
      const data = JSON.parse(event.data);
      if (data.timestamp) {
        messageLatency.add(receivedAt - data.timestamp);
      }
    } catch (e) {
      errorRate.add(1);
    }
  });

  sleep(240); // 4 minutes
}

export function burstConnection() {
  const startTime = Date.now();
  const ws = new WebSocket(WS_URL);

  ws.addEventListener('open', () => {
    connectionTime.add(Date.now() - startTime);

    // Send burst of 100 messages
    for (let i = 0; i < 100; i++) {
      const msg = JSON.stringify({
        type: 'burst',
        timestamp: Date.now(),
        sequence: i,
      });
      ws.send(msg);
      messagesSent.add(1);
    }

    sleep(2);
    ws.close();
  });

  ws.addEventListener('message', () => {
    messagesReceived.add(1);
  });

  ws.addEventListener('error', () => {
    errorRate.add(1);
  });

  sleep(3);
}

export function setup() {
  console.log(`Starting WebSocket load test against ${WS_URL}`);
  return { startTime: new Date() };
}

export function teardown(data) {
  console.log('WebSocket load test completed');
}
