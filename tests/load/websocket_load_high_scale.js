// High-scale WebSocket load test for MockForge
// Tests with 10,000+ concurrent WebSocket connections
import ws from 'k6/ws';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Custom metrics
const connectionErrorRate = new Rate('ws_connection_errors');
const messageErrorRate = new Rate('ws_message_errors');
const connectionLatency = new Trend('ws_connection_latency');
const messageLatency = new Trend('ws_message_latency');

// Test configuration
export const options = {
    stages: [
        // Ramp up to 5,000 connections over 5 minutes
        { duration: '5m', target: 5000 },
        // Sustain 5,000 connections for 3 minutes
        { duration: '3m', target: 5000 },
        // Ramp up to 10,000 connections over 3 minutes
        { duration: '3m', target: 10000 },
        // Sustain 10,000 connections for 5 minutes
        { duration: '5m', target: 10000 },
        // Ramp down gradually
        { duration: '3m', target: 5000 },
        { duration: '2m', target: 2500 },
        { duration: '1m', target: 0 },
    ],
    thresholds: {
        // Connection establishment must be fast
        ws_connecting: ['p(95)<2000', 'p(99)<5000'],
        // Connection errors must be minimal
        ws_connection_errors: ['rate<0.01'],
        // Message round-trip time
        ws_message_latency: ['p(95)<500', 'p(99)<1000'],
        // Message errors
        ws_message_errors: ['rate<0.01'],
    },
    summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(90)', 'p(95)', 'p(99)', 'p(99.9)', 'count'],
};

const WS_URL = __ENV.WS_URL || 'ws://localhost:3001/ws';

export default function () {
    const connectStart = Date.now();

    const response = ws.connect(WS_URL, {}, function (socket) {
        const connectTime = Date.now() - connectStart;
        connectionLatency.add(connectTime);

        socket.on('open', function () {
            check(response, {
                'WebSocket connection established': (r) => r && r.status === 101,
            }) || connectionErrorRate.add(1);

            // Send periodic messages
            const messageInterval = setInterval(function () {
                const messageStart = Date.now();
                const message = JSON.stringify({
                    type: 'ping',
                    timestamp: Date.now(),
                    vu: __VU,
                    iteration: __ITER,
                });

                socket.send(message);

                socket.on('message', function (data) {
                    const messageTime = Date.now() - messageStart;
                    messageLatency.add(messageTime);

                    try {
                        const parsed = JSON.parse(data);
                        check(parsed, {
                            'message received': () => true,
                            'message type is pong': (m) => m.type === 'pong',
                        }) || messageErrorRate.add(1);
                    } catch (e) {
                        messageErrorRate.add(1);
                    }
                });
            }, 2000); // Send message every 2 seconds

            // Keep connection alive for duration of test
            socket.setTimeout(function () {
                clearInterval(messageInterval);
                socket.close();
            }, 300000); // 5 minutes
        });

        socket.on('error', function (e) {
            connectionErrorRate.add(1);
            console.error('WebSocket error:', e);
        });

        socket.on('close', function () {
            // Connection closed normally
        });
    });

    // Wait for connection to establish
    sleep(0.1);
}

export function handleSummary(data) {
    return {
        'stdout': textSummary(data, { indent: ' ', enableColors: true }),
        'tests/load/results/websocket_high_scale_summary.json': JSON.stringify(data),
    };
}

function textSummary(data, options) {
    return `
╔══════════════════════════════════════════════════════════════╗
║        High-Scale WebSocket Load Test Summary                ║
╚══════════════════════════════════════════════════════════════╝

Duration: ${data.state.testRunDurationMs / 1000}s
Max Concurrent Connections: ${data.metrics.vus.values.max}

Connection Metrics:
  Connection Errors: ${(data.metrics.ws_connection_errors?.values.rate || 0) * 100}%
  Connection Latency (P95): ${data.metrics.ws_connection_latency?.values['p(95)']?.toFixed(2) || 'N/A'}ms
  Connection Latency (P99): ${data.metrics.ws_connection_latency?.values['p(99)']?.toFixed(2) || 'N/A'}ms

Message Metrics:
  Message Errors: ${(data.metrics.ws_message_errors?.values.rate || 0) * 100}%
  Message Latency (P95): ${data.metrics.ws_message_latency?.values['p(95)']?.toFixed(2) || 'N/A'}ms
  Message Latency (P99): ${data.metrics.ws_message_latency?.values['p(99)']?.toFixed(2) || 'N/A'}ms
`;
}
