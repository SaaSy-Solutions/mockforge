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

// === Advanced Testing Features ===
// Security testing payloads
// Total payloads: 13
const securityPayloads = [
  { payload: '<script>alert(\'XSS\')</script>', category: 'xss', description: 'Basic script tag XSS', location: 'uri', headerName: null },
  { payload: '<img src=x onerror=alert(\'XSS\')>', category: 'xss', description: 'Image tag XSS with onerror', location: 'uri', headerName: null },
  { payload: '<svg/onload=alert(\'XSS\')>', category: 'xss', description: 'SVG tag XSS with onload', location: 'uri', headerName: null },
  { payload: 'javascript:alert(\'XSS\')', category: 'xss', description: 'JavaScript protocol XSS', location: 'uri', headerName: null },
  { payload: '<body onload=alert(\'XSS\')>', category: 'xss', description: 'Body tag XSS with onload', location: 'uri', headerName: null },
  { payload: '\'><script>alert(String.fromCharCode(88,83,83))</script>', category: 'xss', description: 'XSS with character encoding', location: 'uri', headerName: null },
  { payload: '<div style=\"background:url(javascript:alert(\'XSS\'))\">', category: 'xss', description: 'CSS-based XSS', location: 'uri', headerName: null },
  { payload: '\' OR \'1\'=\'1', category: 'sqli', description: 'Basic SQL injection tautology', location: 'uri', headerName: null },
  { payload: '\' OR \'1\'=\'1\' --', category: 'sqli', description: 'SQL injection with comment', location: 'uri', headerName: null },
  { payload: '\' UNION SELECT * FROM users --', category: 'sqli', description: 'SQL injection union-based data extraction', location: 'uri', headerName: null },
  { payload: '1\' AND \'1\'=\'1', category: 'sqli', description: 'SQL injection boolean-based blind', location: 'uri', headerName: null },
  { payload: '1; WAITFOR DELAY \'0:0:5\' --', category: 'sqli', description: 'SQL injection time-based blind (MSSQL)', location: 'uri', headerName: null },
  { payload: '1\' AND SLEEP(5) --', category: 'sqli', description: 'SQL injection time-based blind (MySQL)', location: 'uri', headerName: null },
];

// Select random security payload
function getNextSecurityPayload() {
  return securityPayloads[Math.floor(Math.random() * securityPayloads.length)];
}


// Apply security payload to request body
// For POST/PUT/PATCH requests, inject ALL payloads into body for effective WAF testing
// Injects into ALL string fields to maximize WAF detection surface area
function applySecurityPayload(payload, targetFields, secPayload) {
  const result = { ...payload };

  // No specific target fields - inject into ALL string fields for maximum coverage
  // This ensures WAF can detect payloads regardless of which field it scans
  for (const key of Object.keys(result)) {
    if (typeof result[key] === 'string') {
      result[key] = secPayload.payload;
    }
  }

  return result;
}

// Security test response checks
function checkSecurityResponse(res, expectedVulnerable) {
  // Check for common vulnerability indicators
  const body = res.body || '';

  const vulnerabilityIndicators = [
    // SQL injection
    'SQL syntax',
    'mysql_fetch',
    'ORA-',
    'PostgreSQL',

    // Command injection
    'root:',
    '/bin/',
    'uid=',

    // Path traversal
    '[extensions]',
    'passwd',

    // XSS (reflected)
    '<script>alert',
    'onerror=',

    // Error disclosure
    'stack trace',
    'Exception',
    'Error in',
  ];

  const foundIndicator = vulnerabilityIndicators.some(ind =>
    body.toLowerCase().includes(ind.toLowerCase())
  );

  if (foundIndicator) {
    console.warn(`POTENTIAL VULNERABILITY: ${securityPayload.description}`);
    console.warn(`Category: ${securityPayload.category}`);
    console.warn(`Status: ${res.status}`);
  }

  return check(res, {
    'security test: no obvious vulnerability': () => !foundIndicator,
    'security test: proper error handling': (r) => r.status < 500,
  });
}


export const options = {
  insecureSkipTLSVerify: true,
  scenarios: {
    rampup: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '10s', target: 2 },
        { duration: '10s', target: 5 },
        { duration: '20s', target: 10 },
        { duration: '20s', target: 0 },
      ],
      gracefulRampDown: '10s',
    },
  },
  thresholds: {
    'http_req_duration': ['p(95)<500'],
    'http_req_failed': ['rate<0.05'],
  },
};

const BASE_URL = 'http://localhost:8080';

export default function () {
  // Operation 0: plans.list
  {
    // Get a fresh security payload for each operation so all payloads cycle
    const secPayload = typeof getNextSecurityPayload === 'function' ? getNextSecurityPayload() : null;
    // Copy headers and apply security payload to headers if needed
    const requestHeaders = { ...{"Prefer":"test-value"} };
    if (secPayload) {
      if (secPayload.location === 'header' && secPayload.headerName) {
        requestHeaders[secPayload.headerName] = secPayload.payload;
      }
    }
    // Build request URL with optional URI security payload injection
    let requestUrl = `${BASE_URL}/v1/billing/plans?total_required=true&page_size=42&page=42&product_id=test-value`;
    if (secPayload && secPayload.location === 'uri') {
      requestUrl += (requestUrl.includes('?') ? '&' : '?') + 'test=' + encodeURIComponent(secPayload.payload);
    }

    // GET and HEAD only take 2 args: http.get(url, params)
    const res = http.get(requestUrl, { headers: requestHeaders, jar: null });

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
    // Get a fresh security payload for each operation so all payloads cycle
    const secPayload = typeof getNextSecurityPayload === 'function' ? getNextSecurityPayload() : null;
    // Copy headers and apply security payload to headers if needed
    const requestHeaders = { ...{"Prefer":"test-value","PayPal-Request-Id":"test-value","Content-Type":"application/json"} };
    if (secPayload) {
      if (secPayload.location === 'header' && secPayload.headerName) {
        requestHeaders[secPayload.headerName] = secPayload.payload;
      }
    }
    // Build request URL with optional URI security payload injection
    let requestUrl = `${BASE_URL}/v1/billing/plans`;
    if (secPayload && secPayload.location === 'uri') {
      requestUrl += (requestUrl.includes('?') ? '&' : '?') + 'test=' + encodeURIComponent(secPayload.payload);
    }

    let payload = {};
    // Apply security payload to body if available
    if (secPayload && typeof applySecurityPayload === 'function') {
      payload = applySecurityPayload(payload, [], secPayload);
    }
    const res = http.post(requestUrl, JSON.stringify(payload), { headers: requestHeaders, jar: null });

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
    // Get a fresh security payload for each operation so all payloads cycle
    const secPayload = typeof getNextSecurityPayload === 'function' ? getNextSecurityPayload() : null;
    // Copy headers and apply security payload to headers if needed
    const requestHeaders = { ...{} };
    if (secPayload) {
      if (secPayload.location === 'header' && secPayload.headerName) {
        requestHeaders[secPayload.headerName] = secPayload.payload;
      }
    }
    // Build request URL with optional URI security payload injection
    let requestUrl = `${BASE_URL}/v1/billing/plans/test-value`;
    if (secPayload && secPayload.location === 'uri') {
      requestUrl += (requestUrl.includes('?') ? '&' : '?') + 'test=' + encodeURIComponent(secPayload.payload);
    }

    // GET and HEAD only take 2 args: http.get(url, params)
    const res = http.get(requestUrl, { headers: requestHeaders, jar: null });

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
    // Get a fresh security payload for each operation so all payloads cycle
    const secPayload = typeof getNextSecurityPayload === 'function' ? getNextSecurityPayload() : null;
    // Copy headers and apply security payload to headers if needed
    const requestHeaders = { ...{} };
    if (secPayload) {
      if (secPayload.location === 'header' && secPayload.headerName) {
        requestHeaders[secPayload.headerName] = secPayload.payload;
      }
    }
    // Build request URL with optional URI security payload injection
    let requestUrl = `${BASE_URL}/v1/billing/plans/test-value`;
    if (secPayload && secPayload.location === 'uri') {
      requestUrl += (requestUrl.includes('?') ? '&' : '?') + 'test=' + encodeURIComponent(secPayload.payload);
    }

    // POST, PUT, PATCH, DELETE take 3 args: http.post(url, body, params)
    const res = http.patch(requestUrl, null, { headers: requestHeaders, jar: null });

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
    // Get a fresh security payload for each operation so all payloads cycle
    const secPayload = typeof getNextSecurityPayload === 'function' ? getNextSecurityPayload() : null;
    // Copy headers and apply security payload to headers if needed
    const requestHeaders = { ...{} };
    if (secPayload) {
      if (secPayload.location === 'header' && secPayload.headerName) {
        requestHeaders[secPayload.headerName] = secPayload.payload;
      }
    }
    // Build request URL with optional URI security payload injection
    let requestUrl = `${BASE_URL}/v1/billing/plans/test-value/activate`;
    if (secPayload && secPayload.location === 'uri') {
      requestUrl += (requestUrl.includes('?') ? '&' : '?') + 'test=' + encodeURIComponent(secPayload.payload);
    }

    // POST, PUT, PATCH, DELETE take 3 args: http.post(url, body, params)
    const res = http.post(requestUrl, null, { headers: requestHeaders, jar: null });

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
    // Get a fresh security payload for each operation so all payloads cycle
    const secPayload = typeof getNextSecurityPayload === 'function' ? getNextSecurityPayload() : null;
    // Copy headers and apply security payload to headers if needed
    const requestHeaders = { ...{} };
    if (secPayload) {
      if (secPayload.location === 'header' && secPayload.headerName) {
        requestHeaders[secPayload.headerName] = secPayload.payload;
      }
    }
    // Build request URL with optional URI security payload injection
    let requestUrl = `${BASE_URL}/v1/billing/plans/test-value/deactivate`;
    if (secPayload && secPayload.location === 'uri') {
      requestUrl += (requestUrl.includes('?') ? '&' : '?') + 'test=' + encodeURIComponent(secPayload.payload);
    }

    // POST, PUT, PATCH, DELETE take 3 args: http.post(url, body, params)
    const res = http.post(requestUrl, null, { headers: requestHeaders, jar: null });

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
    // Get a fresh security payload for each operation so all payloads cycle
    const secPayload = typeof getNextSecurityPayload === 'function' ? getNextSecurityPayload() : null;
    // Copy headers and apply security payload to headers if needed
    const requestHeaders = { ...{"Content-Type":"application/json"} };
    if (secPayload) {
      if (secPayload.location === 'header' && secPayload.headerName) {
        requestHeaders[secPayload.headerName] = secPayload.payload;
      }
    }
    // Build request URL with optional URI security payload injection
    let requestUrl = `${BASE_URL}/v1/billing/plans/test-value/update-pricing-schemes`;
    if (secPayload && secPayload.location === 'uri') {
      requestUrl += (requestUrl.includes('?') ? '&' : '?') + 'test=' + encodeURIComponent(secPayload.payload);
    }

    let payload = {};
    // Apply security payload to body if available
    if (secPayload && typeof applySecurityPayload === 'function') {
      payload = applySecurityPayload(payload, [], secPayload);
    }
    const res = http.post(requestUrl, JSON.stringify(payload), { headers: requestHeaders, jar: null });

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
    // Get a fresh security payload for each operation so all payloads cycle
    const secPayload = typeof getNextSecurityPayload === 'function' ? getNextSecurityPayload() : null;
    // Copy headers and apply security payload to headers if needed
    const requestHeaders = { ...{"PayPal-Request-Id":"test-value","Prefer":"test-value","Content-Type":"application/json"} };
    if (secPayload) {
      if (secPayload.location === 'header' && secPayload.headerName) {
        requestHeaders[secPayload.headerName] = secPayload.payload;
      }
    }
    // Build request URL with optional URI security payload injection
    let requestUrl = `${BASE_URL}/v1/billing/subscriptions`;
    if (secPayload && secPayload.location === 'uri') {
      requestUrl += (requestUrl.includes('?') ? '&' : '?') + 'test=' + encodeURIComponent(secPayload.payload);
    }

    let payload = {};
    // Apply security payload to body if available
    if (secPayload && typeof applySecurityPayload === 'function') {
      payload = applySecurityPayload(payload, [], secPayload);
    }
    const res = http.post(requestUrl, JSON.stringify(payload), { headers: requestHeaders, jar: null });

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
    // Get a fresh security payload for each operation so all payloads cycle
    const secPayload = typeof getNextSecurityPayload === 'function' ? getNextSecurityPayload() : null;
    // Copy headers and apply security payload to headers if needed
    const requestHeaders = { ...{} };
    if (secPayload) {
      if (secPayload.location === 'header' && secPayload.headerName) {
        requestHeaders[secPayload.headerName] = secPayload.payload;
      }
    }
    // Build request URL with optional URI security payload injection
    let requestUrl = `${BASE_URL}/v1/billing/subscriptions/test-value?fields=test-value`;
    if (secPayload && secPayload.location === 'uri') {
      requestUrl += (requestUrl.includes('?') ? '&' : '?') + 'test=' + encodeURIComponent(secPayload.payload);
    }

    // GET and HEAD only take 2 args: http.get(url, params)
    const res = http.get(requestUrl, { headers: requestHeaders, jar: null });

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
    // Get a fresh security payload for each operation so all payloads cycle
    const secPayload = typeof getNextSecurityPayload === 'function' ? getNextSecurityPayload() : null;
    // Copy headers and apply security payload to headers if needed
    const requestHeaders = { ...{} };
    if (secPayload) {
      if (secPayload.location === 'header' && secPayload.headerName) {
        requestHeaders[secPayload.headerName] = secPayload.payload;
      }
    }
    // Build request URL with optional URI security payload injection
    let requestUrl = `${BASE_URL}/v1/billing/subscriptions/test-value`;
    if (secPayload && secPayload.location === 'uri') {
      requestUrl += (requestUrl.includes('?') ? '&' : '?') + 'test=' + encodeURIComponent(secPayload.payload);
    }

    // POST, PUT, PATCH, DELETE take 3 args: http.post(url, body, params)
    const res = http.patch(requestUrl, null, { headers: requestHeaders, jar: null });

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
    // Get a fresh security payload for each operation so all payloads cycle
    const secPayload = typeof getNextSecurityPayload === 'function' ? getNextSecurityPayload() : null;
    // Copy headers and apply security payload to headers if needed
    const requestHeaders = { ...{"Content-Type":"application/json"} };
    if (secPayload) {
      if (secPayload.location === 'header' && secPayload.headerName) {
        requestHeaders[secPayload.headerName] = secPayload.payload;
      }
    }
    // Build request URL with optional URI security payload injection
    let requestUrl = `${BASE_URL}/v1/billing/subscriptions/test-value/activate`;
    if (secPayload && secPayload.location === 'uri') {
      requestUrl += (requestUrl.includes('?') ? '&' : '?') + 'test=' + encodeURIComponent(secPayload.payload);
    }

    let payload = {};
    // Apply security payload to body if available
    if (secPayload && typeof applySecurityPayload === 'function') {
      payload = applySecurityPayload(payload, [], secPayload);
    }
    const res = http.post(requestUrl, JSON.stringify(payload), { headers: requestHeaders, jar: null });

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
    // Get a fresh security payload for each operation so all payloads cycle
    const secPayload = typeof getNextSecurityPayload === 'function' ? getNextSecurityPayload() : null;
    // Copy headers and apply security payload to headers if needed
    const requestHeaders = { ...{"Content-Type":"application/json"} };
    if (secPayload) {
      if (secPayload.location === 'header' && secPayload.headerName) {
        requestHeaders[secPayload.headerName] = secPayload.payload;
      }
    }
    // Build request URL with optional URI security payload injection
    let requestUrl = `${BASE_URL}/v1/billing/subscriptions/test-value/cancel`;
    if (secPayload && secPayload.location === 'uri') {
      requestUrl += (requestUrl.includes('?') ? '&' : '?') + 'test=' + encodeURIComponent(secPayload.payload);
    }

    let payload = {};
    // Apply security payload to body if available
    if (secPayload && typeof applySecurityPayload === 'function') {
      payload = applySecurityPayload(payload, [], secPayload);
    }
    const res = http.post(requestUrl, JSON.stringify(payload), { headers: requestHeaders, jar: null });

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
    // Get a fresh security payload for each operation so all payloads cycle
    const secPayload = typeof getNextSecurityPayload === 'function' ? getNextSecurityPayload() : null;
    // Copy headers and apply security payload to headers if needed
    const requestHeaders = { ...{"PayPal-Request-Id":"test-value","Content-Type":"application/json"} };
    if (secPayload) {
      if (secPayload.location === 'header' && secPayload.headerName) {
        requestHeaders[secPayload.headerName] = secPayload.payload;
      }
    }
    // Build request URL with optional URI security payload injection
    let requestUrl = `${BASE_URL}/v1/billing/subscriptions/test-value/capture`;
    if (secPayload && secPayload.location === 'uri') {
      requestUrl += (requestUrl.includes('?') ? '&' : '?') + 'test=' + encodeURIComponent(secPayload.payload);
    }

    let payload = {};
    // Apply security payload to body if available
    if (secPayload && typeof applySecurityPayload === 'function') {
      payload = applySecurityPayload(payload, [], secPayload);
    }
    const res = http.post(requestUrl, JSON.stringify(payload), { headers: requestHeaders, jar: null });

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
    // Get a fresh security payload for each operation so all payloads cycle
    const secPayload = typeof getNextSecurityPayload === 'function' ? getNextSecurityPayload() : null;
    // Copy headers and apply security payload to headers if needed
    const requestHeaders = { ...{"Content-Type":"application/json"} };
    if (secPayload) {
      if (secPayload.location === 'header' && secPayload.headerName) {
        requestHeaders[secPayload.headerName] = secPayload.payload;
      }
    }
    // Build request URL with optional URI security payload injection
    let requestUrl = `${BASE_URL}/v1/billing/subscriptions/test-value/revise`;
    if (secPayload && secPayload.location === 'uri') {
      requestUrl += (requestUrl.includes('?') ? '&' : '?') + 'test=' + encodeURIComponent(secPayload.payload);
    }

    let payload = {};
    // Apply security payload to body if available
    if (secPayload && typeof applySecurityPayload === 'function') {
      payload = applySecurityPayload(payload, [], secPayload);
    }
    const res = http.post(requestUrl, JSON.stringify(payload), { headers: requestHeaders, jar: null });

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
    // Get a fresh security payload for each operation so all payloads cycle
    const secPayload = typeof getNextSecurityPayload === 'function' ? getNextSecurityPayload() : null;
    // Copy headers and apply security payload to headers if needed
    const requestHeaders = { ...{"Content-Type":"application/json"} };
    if (secPayload) {
      if (secPayload.location === 'header' && secPayload.headerName) {
        requestHeaders[secPayload.headerName] = secPayload.payload;
      }
    }
    // Build request URL with optional URI security payload injection
    let requestUrl = `${BASE_URL}/v1/billing/subscriptions/test-value/suspend`;
    if (secPayload && secPayload.location === 'uri') {
      requestUrl += (requestUrl.includes('?') ? '&' : '?') + 'test=' + encodeURIComponent(secPayload.payload);
    }

    let payload = {};
    // Apply security payload to body if available
    if (secPayload && typeof applySecurityPayload === 'function') {
      payload = applySecurityPayload(payload, [], secPayload);
    }
    const res = http.post(requestUrl, JSON.stringify(payload), { headers: requestHeaders, jar: null });

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
    // Get a fresh security payload for each operation so all payloads cycle
    const secPayload = typeof getNextSecurityPayload === 'function' ? getNextSecurityPayload() : null;
    // Copy headers and apply security payload to headers if needed
    const requestHeaders = { ...{} };
    if (secPayload) {
      if (secPayload.location === 'header' && secPayload.headerName) {
        requestHeaders[secPayload.headerName] = secPayload.payload;
      }
    }
    // Build request URL with optional URI security payload injection
    let requestUrl = `${BASE_URL}/v1/billing/subscriptions/test-value/transactions?end_time=test-value&start_time=test-value`;
    if (secPayload && secPayload.location === 'uri') {
      requestUrl += (requestUrl.includes('?') ? '&' : '?') + 'test=' + encodeURIComponent(secPayload.payload);
    }

    // GET and HEAD only take 2 args: http.get(url, params)
    const res = http.get(requestUrl, { headers: requestHeaders, jar: null });

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
  if (metrics.http_reqs && metrics.http_reqs.values) {
    const count = metrics.http_reqs.values.count || 0;
    const rate = metrics.http_reqs.values.rate;
    output += indent + `Total Requests: ${count}\n`;
    output += indent + `Request Rate: ${rate != null ? rate.toFixed(2) : '0.00'} req/s\n\n`;
  } else {
    output += indent + 'Total Requests: 0\n';
    output += indent + 'Request Rate: 0.00 req/s\n\n';
  }

  // Duration metrics
  if (metrics.http_req_duration && metrics.http_req_duration.values) {
    const v = metrics.http_req_duration.values;
    output += indent + 'Response Times:\n';
    output += indent + `  Min: ${v.min != null ? v.min.toFixed(2) : 'N/A'}ms\n`;
    output += indent + `  Avg: ${v.avg != null ? v.avg.toFixed(2) : 'N/A'}ms\n`;
    output += indent + `  Med: ${v.med != null ? v.med.toFixed(2) : 'N/A'}ms\n`;
    output += indent + `  p90: ${v['p(90)'] != null ? v['p(90)'].toFixed(2) : 'N/A'}ms\n`;
    output += indent + `  p95: ${v['p(95)'] != null ? v['p(95)'].toFixed(2) : 'N/A'}ms\n`;
    output += indent + `  p99: ${v['p(99)'] != null ? v['p(99)'].toFixed(2) : 'N/A'}ms\n`;
    output += indent + `  Max: ${v.max != null ? v.max.toFixed(2) : 'N/A'}ms\n\n`;
  } else {
    output += indent + 'Response Times: No successful requests\n\n';
  }

  // Error rate
  if (metrics.http_req_failed && metrics.http_req_failed.values) {
    const rate = metrics.http_req_failed.values.rate;
    const errorRate = rate != null ? (rate * 100).toFixed(2) : '100.00';
    output += indent + `Error Rate: ${errorRate}%\n\n`;
  } else {
    output += indent + 'Error Rate: N/A\n\n';
  }

  output += indent + '='.repeat(60) + '\n';

  return output;
}
