// High-scale HTTP load test for MockForge
// Tests with 10,000+ concurrent connections
import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const p95Latency = new Trend('p95_latency');
const p99Latency = new Trend('p99_latency');

// Test configuration
export const options = {
    stages: [
        // Ramp up to 5,000 users over 5 minutes
        { duration: '5m', target: 5000 },
        // Sustain 5,000 users for 3 minutes
        { duration: '3m', target: 5000 },
        // Ramp up to 10,000 users over 3 minutes
        { duration: '3m', target: 10000 },
        // Sustain 10,000 users for 5 minutes
        { duration: '5m', target: 10000 },
        // Ramp down gradually
        { duration: '3m', target: 5000 },
        { duration: '2m', target: 2500 },
        { duration: '1m', target: 0 },
    ],
    thresholds: {
        // 95% of requests must complete within 1 second
        http_req_duration: ['p(95)<1000', 'p(99)<2000'],
        // Error rate must be less than 1%
        http_req_failed: ['rate<0.01'],
        // Response time consistency
        http_req_duration: ['avg<500', 'max<5000'],
        // Throughput
        http_reqs: ['rate>100'], // At least 100 req/s minimum
    },
    summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(90)', 'p(95)', 'p(99)', 'p(99.9)', 'count'],
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:3000';

// Test scenarios
const scenarios = [
    {
        name: 'GET /health',
        method: 'GET',
        path: '/health',
        weight: 10, // 10% of requests
    },
    {
        name: 'GET /api/users',
        method: 'GET',
        path: '/api/users',
        weight: 30, // 30% of requests
    },
    {
        name: 'GET /api/users/:id',
        method: 'GET',
        path: '/api/users/123',
        weight: 25, // 25% of requests
    },
    {
        name: 'POST /api/users',
        method: 'POST',
        path: '/api/users',
        body: JSON.stringify({
            name: 'Test User',
            email: `test-${__VU}-${__ITER}@example.com`,
        }),
        weight: 20, // 20% of requests
    },
    {
        name: 'PUT /api/users/:id',
        method: 'PUT',
        path: '/api/users/123',
        body: JSON.stringify({
            name: 'Updated User',
            email: 'updated@example.com',
        }),
        weight: 10, // 10% of requests
    },
    {
        name: 'DELETE /api/users/:id',
        method: 'DELETE',
        path: '/api/users/123',
        weight: 5, // 5% of requests
    },
];

// Weighted random scenario selection
function selectScenario() {
    const totalWeight = scenarios.reduce((sum, s) => sum + s.weight, 0);
    let random = Math.random() * totalWeight;

    for (const scenario of scenarios) {
        random -= scenario.weight;
        if (random <= 0) {
            return scenario;
        }
    }
    return scenarios[0];
}

export default function () {
    const scenario = selectScenario();

    const params = {
        headers: {
            'Content-Type': 'application/json',
            'User-Agent': `k6-load-test-${__VU}`,
        },
        timeout: '10s',
    };

    let response;

    if (scenario.method === 'GET') {
        response = http.get(`${BASE_URL}${scenario.path}`, params);
    } else if (scenario.method === 'POST') {
        response = http.post(`${BASE_URL}${scenario.path}`, scenario.body, params);
    } else if (scenario.method === 'PUT') {
        response = http.put(`${BASE_URL}${scenario.path}`, scenario.body, params);
    } else if (scenario.method === 'DELETE') {
        response = http.del(`${BASE_URL}${scenario.path}`, null, params);
    }

    const success = check(response, {
        'status is 200-299': (r) => r.status >= 200 && r.status < 300,
        'response time < 1s': (r) => r.timings.duration < 1000,
        'response time < 2s': (r) => r.timings.duration < 2000,
        'has response body': (r) => r.body.length > 0,
    });

    if (!success) {
        errorRate.add(1);
    } else {
        errorRate.add(0);
    }

    p95Latency.add(response.timings.duration);
    p99Latency.add(response.timings.duration);

    // Small random sleep to simulate real user behavior
    sleep(Math.random() * 0.1);
}

export function handleSummary(data) {
    return {
        'stdout': textSummary(data, { indent: ' ', enableColors: true }),
        'tests/load/results/http_high_scale_summary.json': JSON.stringify(data),
    };
}

function textSummary(data, options) {
    // Simple text summary
    return `
╔══════════════════════════════════════════════════════════════╗
║           High-Scale HTTP Load Test Summary                  ║
╚══════════════════════════════════════════════════════════════╝

Duration: ${data.state.testRunDurationMs / 1000}s
VUs: ${data.metrics.vus.values.max}
HTTP Requests: ${data.metrics.http_reqs.values.count}
HTTP Requests/sec: ${data.metrics.http_reqs.values.rate.toFixed(2)}
Failed Requests: ${data.metrics.http_req_failed.values.rate * 100}%

Response Times:
  Average: ${data.metrics.http_req_duration.values.avg.toFixed(2)}ms
  Median: ${data.metrics.http_req_duration.values.med.toFixed(2)}ms
  P90: ${data.metrics.http_req_duration.values['p(90)'].toFixed(2)}ms
  P95: ${data.metrics.http_req_duration.values['p(95)'].toFixed(2)}ms
  P99: ${data.metrics.http_req_duration.values['p(99)'].toFixed(2)}ms
  Max: ${data.metrics.http_req_duration.values.max.toFixed(2)}ms

Data Transfer:
  Received: ${(data.metrics.data_received.values.count / 1024 / 1024).toFixed(2)} MB
  Sent: ${(data.metrics.data_sent.values.count / 1024 / 1024).toFixed(2)} MB
`;
}
