// High-scale HTTP load test for MockForge
// Tests with up to 8,000 concurrent connections
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
        // Ramp up to 8,000 users over 3 minutes. The peak was 10,000, which
        // offered ~5k req/s against the ~7k req/s the shared CI runner
        // sustains (~71% utilization): queueing put p95 at ~750ms against the
        // 1s threshold, so a busy neighbour on the host could flip the result.
        // 8,000 offers ~4k req/s (~57%), leaving real headroom while still
        // tripping on a genuine throughput regression.
        { duration: '3m', target: 8000 },
        // Sustain 8,000 users for 5 minutes
        { duration: '5m', target: 8000 },
        // Ramp down gradually
        { duration: '3m', target: 5000 },
        { duration: '2m', target: 2500 },
        { duration: '1m', target: 0 },
    ],
    thresholds: {
        // 95% within 1s, 99% within 2s, mean under 500ms. (These used to be
        // two separate `http_req_duration` keys; the second silently
        // replaced the first, so the percentiles were never enforced.)
        http_req_duration: ['p(95)<1000', 'p(99)<2000', 'avg<500'],
        // Error rate must be less than 1%
        http_req_failed: ['rate<0.01'],
        // Throughput
        // The stages average ~5.2k VUs, which is ~2.6k req/s at a 2s think
        // time when the server keeps up. Under 2k req/s means requests are
        // queueing.
        http_reqs: ['rate>2000'],
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
        name: 'GET /users',
        method: 'GET',
        path: '/users',
        weight: 30, // 30% of requests
    },
    {
        name: 'GET /users/:id',
        method: 'GET',
        path: '/users/123',
        weight: 25, // 25% of requests
    },
    {
        name: 'POST /users',
        method: 'POST',
        path: '/users',
        // Body is a factory so __VU / __ITER evaluate inside a VU context
        // (at request time) rather than at module load. k6 0.50+ removed the
        // module-scope fallback, which is what the 128 consecutive failures
        // were hitting as ReferenceError: __ITER is not defined.
        body: () => JSON.stringify({
            name: 'Test User',
            email: `test-${__VU}-${__ITER}@example.com`,
        }),
        weight: 20, // 20% of requests
    },
    {
        name: 'PUT /users/:id',
        method: 'PUT',
        path: '/users/123',
        body: JSON.stringify({
            name: 'Updated User',
            email: 'updated@example.com',
        }),
        weight: 10, // 10% of requests
    },
    {
        name: 'DELETE /users/:id',
        method: 'DELETE',
        path: '/users/123',
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

    // Resolve body now — some scenarios define it as a factory so __VU /
    // __ITER get evaluated in the VU context instead of at module load.
    const body = typeof scenario.body === 'function' ? scenario.body() : scenario.body;

    if (scenario.method === 'GET') {
        response = http.get(`${BASE_URL}${scenario.path}`, params);
    } else if (scenario.method === 'POST') {
        response = http.post(`${BASE_URL}${scenario.path}`, body, params);
    } else if (scenario.method === 'PUT') {
        response = http.put(`${BASE_URL}${scenario.path}`, body, params);
    } else if (scenario.method === 'DELETE') {
        response = http.del(`${BASE_URL}${scenario.path}`, null, params);
    }

    const success = check(response, {
        'status is 200-299': (r) => r.status >= 200 && r.status < 300,
        'response time < 1s': (r) => r.timings.duration < 1000,
        'response time < 2s': (r) => r.timings.duration < 2000,
        // DELETE answers 204 No Content, which by definition has no body.
        'has response body': (r) => r.status === 204 || (r.body || '').length > 0,
    });

    if (!success) {
        errorRate.add(1);
    } else {
        errorRate.add(0);
    }

    p95Latency.add(response.timings.duration);
    p99Latency.add(response.timings.duration);

    // Think time of 1-3s (mean 2s). With the old 0-100ms sleep, 10k VUs was
    // a closed loop pinned at the server's capacity: latency was just
    // VUs / throughput (Little's law), about 1.4s at ~7k req/s on the CI
    // runner, so the latency thresholds could never pass. At a 2s mean,
    // the 8k-VU peak offers ~4k req/s, well below the ~7k req/s the runner
    // sustains. A throughput regression past that margin saturates the
    // server again and trips the thresholds.
    sleep(1 + Math.random() * 2);
}

export function handleSummary(data) {
    return {
        'stdout': textSummary(data, { indent: ' ', enableColors: true }),
        // Relative to k6's cwd. The old 'tests/load/results/...' path did not
        // exist when run from tests/load (as CI does), so nothing was written.
        [__ENV.SUMMARY_FILE || 'http_high_scale_summary.json']: JSON.stringify(data),
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
