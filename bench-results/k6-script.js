import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics per operation
const plans_list_latency = new Trend('plans_list_latency');
const plans_list_errors = new Rate('plans_list_errors');
const plans_create_latency = new Trend('plans_create_latency');
const plans_create_errors = new Rate('plans_create_errors');
const plans_get_latency = new Trend('plans_get_latency');
const plans_get_errors = new Rate('plans_get_errors');
const plans_patch_latency = new Trend('plans_patch_latency');
const plans_patch_errors = new Rate('plans_patch_errors');
const plans_activate_latency = new Trend('plans_activate_latency');
const plans_activate_errors = new Rate('plans_activate_errors');
const plans_deactivate_latency = new Trend('plans_deactivate_latency');
const plans_deactivate_errors = new Rate('plans_deactivate_errors');
const plans_update_pricing_schemes_latency = new Trend('plans_update_pricing_schemes_latency');
const plans_update_pricing_schemes_errors = new Rate('plans_update_pricing_schemes_errors');
const subscriptions_create_latency = new Trend('subscriptions_create_latency');
const subscriptions_create_errors = new Rate('subscriptions_create_errors');
const subscriptions_get_latency = new Trend('subscriptions_get_latency');
const subscriptions_get_errors = new Rate('subscriptions_get_errors');
const subscriptions_patch_latency = new Trend('subscriptions_patch_latency');
const subscriptions_patch_errors = new Rate('subscriptions_patch_errors');
const subscriptions_activate_latency = new Trend('subscriptions_activate_latency');
const subscriptions_activate_errors = new Rate('subscriptions_activate_errors');
const subscriptions_cancel_latency = new Trend('subscriptions_cancel_latency');
const subscriptions_cancel_errors = new Rate('subscriptions_cancel_errors');
const subscriptions_capture_latency = new Trend('subscriptions_capture_latency');
const subscriptions_capture_errors = new Rate('subscriptions_capture_errors');
const subscriptions_revise_latency = new Trend('subscriptions_revise_latency');
const subscriptions_revise_errors = new Rate('subscriptions_revise_errors');
const subscriptions_suspend_latency = new Trend('subscriptions_suspend_latency');
const subscriptions_suspend_errors = new Rate('subscriptions_suspend_errors');
const subscriptions_transactions_latency = new Trend('subscriptions_transactions_latency');
const subscriptions_transactions_errors = new Rate('subscriptions_transactions_errors');

// Test configuration
export const options = {
  scenarios: {
    constant: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '60s', target: 10 },
      ],
      gracefulRampDown: '10s',
    },
  },
  thresholds: {
    'http_req_duration': ['p(95)<500'],
    'http_req_failed': ['rate<0.05'],
  },
};

const BASE_URL = 'https://example.com';

export default function () {
  // Operation 0: plans.list
  {
    const headers = {};

    const res = http.get(`${BASE_URL}/v1/billing/plans`, null, { headers });

    const success = check(res, {
      'plans.list: status is OK': (r) => r.status >= 200 && r.status < 300,
      'plans.list: has response': (r) => r.body !== null && r.body.length > 0,
    });

    plans_list_latency.add(res.timings.duration);
    plans_list_errors.add(!success);

    sleep(1);
  }
  // Operation 1: plans.create
  {
    const headers = {"Content-Type":"application/json"};

    const payload = {};
    const res = http.post(`${BASE_URL}/v1/billing/plans`, JSON.stringify(payload), { headers });

    const success = check(res, {
      'plans.create: status is OK': (r) => r.status >= 200 && r.status < 300,
      'plans.create: has response': (r) => r.body !== null && r.body.length > 0,
    });

    plans_create_latency.add(res.timings.duration);
    plans_create_errors.add(!success);

    sleep(1);
  }
  // Operation 2: plans.get
  {
    const headers = {};

    const res = http.get(`${BASE_URL}/v1/billing/plans/{id}`, null, { headers });

    const success = check(res, {
      'plans.get: status is OK': (r) => r.status >= 200 && r.status < 300,
      'plans.get: has response': (r) => r.body !== null && r.body.length > 0,
    });

    plans_get_latency.add(res.timings.duration);
    plans_get_errors.add(!success);

    sleep(1);
  }
  // Operation 3: plans.patch
  {
    const headers = {};

    const res = http.patch(`${BASE_URL}/v1/billing/plans/{id}`, null, { headers });

    const success = check(res, {
      'plans.patch: status is OK': (r) => r.status >= 200 && r.status < 300,
      'plans.patch: has response': (r) => r.body !== null && r.body.length > 0,
    });

    plans_patch_latency.add(res.timings.duration);
    plans_patch_errors.add(!success);

    sleep(1);
  }
  // Operation 4: plans.activate
  {
    const headers = {};

    const res = http.post(`${BASE_URL}/v1/billing/plans/{id}/activate`, null, { headers });

    const success = check(res, {
      'plans.activate: status is OK': (r) => r.status >= 200 && r.status < 300,
      'plans.activate: has response': (r) => r.body !== null && r.body.length > 0,
    });

    plans_activate_latency.add(res.timings.duration);
    plans_activate_errors.add(!success);

    sleep(1);
  }
  // Operation 5: plans.deactivate
  {
    const headers = {};

    const res = http.post(`${BASE_URL}/v1/billing/plans/{id}/deactivate`, null, { headers });

    const success = check(res, {
      'plans.deactivate: status is OK': (r) => r.status >= 200 && r.status < 300,
      'plans.deactivate: has response': (r) => r.body !== null && r.body.length > 0,
    });

    plans_deactivate_latency.add(res.timings.duration);
    plans_deactivate_errors.add(!success);

    sleep(1);
  }
  // Operation 6: plans.update-pricing-schemes
  {
    const headers = {"Content-Type":"application/json"};

    const payload = {};
    const res = http.post(`${BASE_URL}/v1/billing/plans/{id}/update-pricing-schemes`, JSON.stringify(payload), { headers });

    const success = check(res, {
      'plans.update-pricing-schemes: status is OK': (r) => r.status >= 200 && r.status < 300,
      'plans.update-pricing-schemes: has response': (r) => r.body !== null && r.body.length > 0,
    });

    plans_update_pricing_schemes_latency.add(res.timings.duration);
    plans_update_pricing_schemes_errors.add(!success);

    sleep(1);
  }
  // Operation 7: subscriptions.create
  {
    const headers = {"Content-Type":"application/json"};

    const payload = {};
    const res = http.post(`${BASE_URL}/v1/billing/subscriptions`, JSON.stringify(payload), { headers });

    const success = check(res, {
      'subscriptions.create: status is OK': (r) => r.status >= 200 && r.status < 300,
      'subscriptions.create: has response': (r) => r.body !== null && r.body.length > 0,
    });

    subscriptions_create_latency.add(res.timings.duration);
    subscriptions_create_errors.add(!success);

    sleep(1);
  }
  // Operation 8: subscriptions.get
  {
    const headers = {};

    const res = http.get(`${BASE_URL}/v1/billing/subscriptions/{id}`, null, { headers });

    const success = check(res, {
      'subscriptions.get: status is OK': (r) => r.status >= 200 && r.status < 300,
      'subscriptions.get: has response': (r) => r.body !== null && r.body.length > 0,
    });

    subscriptions_get_latency.add(res.timings.duration);
    subscriptions_get_errors.add(!success);

    sleep(1);
  }
  // Operation 9: subscriptions.patch
  {
    const headers = {};

    const res = http.patch(`${BASE_URL}/v1/billing/subscriptions/{id}`, null, { headers });

    const success = check(res, {
      'subscriptions.patch: status is OK': (r) => r.status >= 200 && r.status < 300,
      'subscriptions.patch: has response': (r) => r.body !== null && r.body.length > 0,
    });

    subscriptions_patch_latency.add(res.timings.duration);
    subscriptions_patch_errors.add(!success);

    sleep(1);
  }
  // Operation 10: subscriptions.activate
  {
    const headers = {"Content-Type":"application/json"};

    const payload = {};
    const res = http.post(`${BASE_URL}/v1/billing/subscriptions/{id}/activate`, JSON.stringify(payload), { headers });

    const success = check(res, {
      'subscriptions.activate: status is OK': (r) => r.status >= 200 && r.status < 300,
      'subscriptions.activate: has response': (r) => r.body !== null && r.body.length > 0,
    });

    subscriptions_activate_latency.add(res.timings.duration);
    subscriptions_activate_errors.add(!success);

    sleep(1);
  }
  // Operation 11: subscriptions.cancel
  {
    const headers = {"Content-Type":"application/json"};

    const payload = {};
    const res = http.post(`${BASE_URL}/v1/billing/subscriptions/{id}/cancel`, JSON.stringify(payload), { headers });

    const success = check(res, {
      'subscriptions.cancel: status is OK': (r) => r.status >= 200 && r.status < 300,
      'subscriptions.cancel: has response': (r) => r.body !== null && r.body.length > 0,
    });

    subscriptions_cancel_latency.add(res.timings.duration);
    subscriptions_cancel_errors.add(!success);

    sleep(1);
  }
  // Operation 12: subscriptions.capture
  {
    const headers = {"Content-Type":"application/json"};

    const payload = {};
    const res = http.post(`${BASE_URL}/v1/billing/subscriptions/{id}/capture`, JSON.stringify(payload), { headers });

    const success = check(res, {
      'subscriptions.capture: status is OK': (r) => r.status >= 200 && r.status < 300,
      'subscriptions.capture: has response': (r) => r.body !== null && r.body.length > 0,
    });

    subscriptions_capture_latency.add(res.timings.duration);
    subscriptions_capture_errors.add(!success);

    sleep(1);
  }
  // Operation 13: subscriptions.revise
  {
    const headers = {"Content-Type":"application/json"};

    const payload = {};
    const res = http.post(`${BASE_URL}/v1/billing/subscriptions/{id}/revise`, JSON.stringify(payload), { headers });

    const success = check(res, {
      'subscriptions.revise: status is OK': (r) => r.status >= 200 && r.status < 300,
      'subscriptions.revise: has response': (r) => r.body !== null && r.body.length > 0,
    });

    subscriptions_revise_latency.add(res.timings.duration);
    subscriptions_revise_errors.add(!success);

    sleep(1);
  }
  // Operation 14: subscriptions.suspend
  {
    const headers = {"Content-Type":"application/json"};

    const payload = {};
    const res = http.post(`${BASE_URL}/v1/billing/subscriptions/{id}/suspend`, JSON.stringify(payload), { headers });

    const success = check(res, {
      'subscriptions.suspend: status is OK': (r) => r.status >= 200 && r.status < 300,
      'subscriptions.suspend: has response': (r) => r.body !== null && r.body.length > 0,
    });

    subscriptions_suspend_latency.add(res.timings.duration);
    subscriptions_suspend_errors.add(!success);

    sleep(1);
  }
  // Operation 15: subscriptions.transactions
  {
    const headers = {};

    const res = http.get(`${BASE_URL}/v1/billing/subscriptions/{id}/transactions`, null, { headers });

    const success = check(res, {
      'subscriptions.transactions: status is OK': (r) => r.status >= 200 && r.status < 300,
      'subscriptions.transactions: has response': (r) => r.body !== null && r.body.length > 0,
    });

    subscriptions_transactions_latency.add(res.timings.duration);
    subscriptions_transactions_errors.add(!success);

    sleep(1);
  }
}

export function handleSummary(data) {
  return {
    'stdout': textSummary(data, { indent: ' ', enableColors: true }),
  };
}

function textSummary(data, options) {
  const indent = options.indent || '';
  const enableColors = options.enableColors || false;

  const metrics = data.metrics;
  let output = '\n';

  output += indent + '='.repeat(60) + '\n';
  output += indent + 'Load Test Summary\n';
  output += indent + '='.repeat(60) + '\n\n';

  // Request metrics
  if (metrics.http_reqs) {
    output += indent + `Total Requests: ${metrics.http_reqs.values.count}\n`;
    output += indent + `Request Rate: ${metrics.http_reqs.values.rate.toFixed(2)} req/s\n\n`;
  }

  // Duration metrics
  if (metrics.http_req_duration) {
    output += indent + 'Response Times:\n';
    output += indent + `  Min: ${metrics.http_req_duration.values.min.toFixed(2)}ms\n`;
    output += indent + `  Avg: ${metrics.http_req_duration.values.avg.toFixed(2)}ms\n`;
    output += indent + `  Med: ${metrics.http_req_duration.values.med.toFixed(2)}ms\n`;
    output += indent + `  p90: ${metrics.http_req_duration.values['p(90)'].toFixed(2)}ms\n`;
    output += indent + `  p95: ${metrics.http_req_duration.values['p(95)'].toFixed(2)}ms\n`;
    output += indent + `  p99: ${metrics.http_req_duration.values['p(99)'].toFixed(2)}ms\n`;
    output += indent + `  Max: ${metrics.http_req_duration.values.max.toFixed(2)}ms\n\n`;
  }

  // Error rate
  if (metrics.http_req_failed) {
    const errorRate = (metrics.http_req_failed.values.rate * 100).toFixed(2);
    output += indent + `Error Rate: ${errorRate}%\n\n`;
  }

  output += indent + '='.repeat(60) + '\n';

  return output;
}
