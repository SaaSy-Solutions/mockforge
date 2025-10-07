import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const responseTime = new Trend('response_time');
const requestCounter = new Counter('requests_total');

// Test configuration
export const options = {
  stages: [
    { duration: '30s', target: 20 },  // Ramp up to 20 users over 30s
    { duration: '1m', target: 50 },   // Ramp up to 50 users over 1 minute
    { duration: '2m', target: 100 },  // Ramp up to 100 users over 2 minutes
    { duration: '2m', target: 100 },  // Stay at 100 users for 2 minutes
    { duration: '1m', target: 50 },   // Ramp down to 50 users
    { duration: '30s', target: 0 },   // Ramp down to 0 users
  ],
  thresholds: {
    'http_req_duration': ['p(95)<500', 'p(99)<1000'], // 95% of requests under 500ms, 99% under 1s
    'http_req_failed': ['rate<0.05'],                  // Error rate under 5%
    'errors': ['rate<0.05'],                           // Custom error rate under 5%
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

// Test scenarios
export default function () {
  // Scenario 1: Simple GET request
  simpleGetRequest();
  sleep(1);

  // Scenario 2: POST request with JSON payload
  postRequestWithJson();
  sleep(1);

  // Scenario 3: Request with parameters
  requestWithParams();
  sleep(1);

  // Scenario 4: Request with headers
  requestWithHeaders();
  sleep(1);

  // Scenario 5: Multiple endpoints
  multipleEndpoints();
  sleep(1);
}

function simpleGetRequest() {
  const res = http.get(`${BASE_URL}/api/users`);

  const success = check(res, {
    'GET /api/users status is 200': (r) => r.status === 200,
    'GET /api/users response time < 500ms': (r) => r.timings.duration < 500,
    'GET /api/users has content': (r) => r.body.length > 0,
  });

  errorRate.add(!success);
  responseTime.add(res.timings.duration);
  requestCounter.add(1);
}

function postRequestWithJson() {
  const payload = JSON.stringify({
    name: 'Test User',
    email: 'test@example.com',
    age: 25,
  });

  const params = {
    headers: {
      'Content-Type': 'application/json',
    },
  };

  const res = http.post(`${BASE_URL}/api/users`, payload, params);

  const success = check(res, {
    'POST /api/users status is 200 or 201': (r) => r.status === 200 || r.status === 201,
    'POST /api/users response time < 1000ms': (r) => r.timings.duration < 1000,
    'POST /api/users returns user': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.name !== undefined;
      } catch (e) {
        return false;
      }
    },
  });

  errorRate.add(!success);
  responseTime.add(res.timings.duration);
  requestCounter.add(1);
}

function requestWithParams() {
  const res = http.get(`${BASE_URL}/api/users?limit=10&offset=0&sort=name`);

  const success = check(res, {
    'GET /api/users with params status is 200': (r) => r.status === 200,
    'GET /api/users with params response time < 500ms': (r) => r.timings.duration < 500,
  });

  errorRate.add(!success);
  responseTime.add(res.timings.duration);
  requestCounter.add(1);
}

function requestWithHeaders() {
  const params = {
    headers: {
      'Authorization': 'Bearer test-token',
      'X-Request-ID': `req-${Date.now()}`,
    },
  };

  const res = http.get(`${BASE_URL}/api/protected`, params);

  const success = check(res, {
    'GET /api/protected status is 200': (r) => r.status === 200,
    'GET /api/protected response time < 500ms': (r) => r.timings.duration < 500,
  });

  errorRate.add(!success);
  responseTime.add(res.timings.duration);
  requestCounter.add(1);
}

function multipleEndpoints() {
  const responses = http.batch([
    ['GET', `${BASE_URL}/api/users/1`, null, {}],
    ['GET', `${BASE_URL}/api/posts/1`, null, {}],
    ['GET', `${BASE_URL}/api/comments/1`, null, {}],
  ]);

  responses.forEach((res, index) => {
    const success = check(res, {
      [`Batch request ${index} status is 200`]: (r) => r.status === 200,
      [`Batch request ${index} response time < 500ms`]: (r) => r.timings.duration < 500,
    });

    errorRate.add(!success);
    responseTime.add(res.timings.duration);
    requestCounter.add(1);
  });
}

// Teardown function
export function teardown(data) {
  console.log('Load test completed');
}

// Setup function
export function setup() {
  console.log(`Starting load test against ${BASE_URL}`);
  return { startTime: new Date() };
}
