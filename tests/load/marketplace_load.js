// Marketplace Load Test
// Tests marketplace endpoints (plugins, templates, scenarios) under load
//
// Usage:
//   k6 run marketplace_load.js
//   k6 run --vus 100 --duration 5m marketplace_load.js
//
// Environment variables:
//   REGISTRY_URL: Base URL for the registry server (default: http://localhost:8080)
//   AUTH_TOKEN: JWT token for authenticated requests (optional)
//   ORG_ID: Organization ID for org-scoped operations (optional)

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const searchSuccessRate = new Rate('marketplace_search_success');
const publishSuccessRate = new Rate('marketplace_publish_success');
const downloadSuccessRate = new Rate('marketplace_download_success');
const searchDuration = new Trend('marketplace_search_duration');
const publishDuration = new Trend('marketplace_publish_duration');
const downloadDuration = new Trend('marketplace_download_duration');
const searchCounter = new Counter('marketplace_search_count');
const publishCounter = new Counter('marketplace_publish_count');
const downloadCounter = new Counter('marketplace_download_count');

// Configuration
const REGISTRY_URL = __ENV.REGISTRY_URL || 'http://localhost:8080';
const AUTH_TOKEN = __ENV.AUTH_TOKEN || '';
const ORG_ID = __ENV.ORG_ID || '';

// Test data
const pluginNames = [
    'http-auth', 'jwt-validator', 'rate-limiter', 'cors-handler',
    'request-logger', 'response-transformer', 'data-generator', 'mock-ai'
];

const templateNames = [
    'chaos-testing', 'load-testing', 'resilience-testing', 'security-testing',
    'network-chaos', 'service-failure', 'data-corruption', 'multi-protocol'
];

const scenarioNames = [
    'payment-flow', 'user-management', 'ecommerce-store', 'fintech-banking',
    'healthcare-api', 'iot-devices', 'social-media', 'content-management'
];

// Helper function to get auth headers
function getAuthHeaders() {
    const headers = {
        'Content-Type': 'application/json',
    };
    if (AUTH_TOKEN) {
        headers['Authorization'] = `Bearer ${AUTH_TOKEN}`;
    }
    if (ORG_ID) {
        headers['X-Org-Id'] = ORG_ID;
    }
    return headers;
}

// Helper function to generate dummy WASM data (minimal valid WASM)
function generateDummyWasm() {
    // Minimal valid WASM module (just magic bytes and version)
    const wasmBytes = new Uint8Array([
        0x00, 0x61, 0x73, 0x6D, // WASM magic number
        0x01, 0x00, 0x00, 0x00, // Version 1
    ]);
    return btoa(String.fromCharCode(...wasmBytes));
}

// Helper function to generate dummy package data (minimal gzip)
function generateDummyPackage() {
    // Minimal valid gzip file
    const gzipBytes = new Uint8Array([
        0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03,
        0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    ]);
    return btoa(String.fromCharCode(...gzipBytes));
}

// Test scenario: Search plugins
export function searchPlugins() {
    const query = pluginNames[Math.floor(Math.random() * pluginNames.length)];
    const params = {
        headers: getAuthHeaders(),
        tags: { name: 'marketplace_search_plugins' },
    };

    const payload = JSON.stringify({
        query: query,
        page: 0,
        per_page: 20,
    });

    const startTime = Date.now();
    const res = http.post(`${REGISTRY_URL}/api/v1/plugins/search`, payload, params);
    const duration = Date.now() - startTime;

    const success = check(res, {
        'search plugins status is 200': (r) => r.status === 200,
        'search plugins has results': (r) => {
            try {
                const body = JSON.parse(r.body);
                return body.plugins && Array.isArray(body.plugins);
            } catch (e) {
                return false;
            }
        },
    });

    searchSuccessRate.add(success);
    searchDuration.add(duration);
    searchCounter.add(1);

    return success;
}

// Test scenario: Search templates
export function searchTemplates() {
    const query = templateNames[Math.floor(Math.random() * templateNames.length)];
    const params = {
        headers: getAuthHeaders(),
        tags: { name: 'marketplace_search_templates' },
    };

    const payload = JSON.stringify({
        query: query,
        page: 0,
        per_page: 20,
    });

    const startTime = Date.now();
    const res = http.post(`${REGISTRY_URL}/api/v1/templates/search`, payload, params);
    const duration = Date.now() - startTime;

    const success = check(res, {
        'search templates status is 200': (r) => r.status === 200,
        'search templates has results': (r) => {
            try {
                const body = JSON.parse(r.body);
                return body.templates && Array.isArray(body.templates);
            } catch (e) {
                return false;
            }
        },
    });

    searchSuccessRate.add(success);
    searchDuration.add(duration);
    searchCounter.add(1);

    return success;
}

// Test scenario: Search scenarios
export function searchScenarios() {
    const query = scenarioNames[Math.floor(Math.random() * scenarioNames.length)];
    const params = {
        headers: getAuthHeaders(),
        tags: { name: 'marketplace_search_scenarios' },
    };

    const payload = JSON.stringify({
        query: query,
        page: 0,
        per_page: 20,
    });

    const startTime = Date.now();
    const res = http.post(`${REGISTRY_URL}/api/v1/scenarios/search`, payload, params);
    const duration = Date.now() - startTime;

    const success = check(res, {
        'search scenarios status is 200': (r) => r.status === 200,
        'search scenarios has results': (r) => {
            try {
                const body = JSON.parse(r.body);
                return body.scenarios && Array.isArray(body.scenarios);
            } catch (e) {
                return false;
            }
        },
    });

    searchSuccessRate.add(success);
    searchDuration.add(duration);
    searchCounter.add(1);

    return success;
}

// Test scenario: Get plugin details
export function getPlugin() {
    const pluginName = pluginNames[Math.floor(Math.random() * pluginNames.length)];
    const params = {
        headers: getAuthHeaders(),
        tags: { name: 'marketplace_get_plugin' },
    };

    const startTime = Date.now();
    const res = http.get(`${REGISTRY_URL}/api/v1/plugins/${pluginName}`, params);
    const duration = Date.now() - startTime;

    const success = check(res, {
        'get plugin status is 200 or 404': (r) => r.status === 200 || r.status === 404,
    });

    downloadSuccessRate.add(success);
    downloadDuration.add(duration);
    downloadCounter.add(1);

    return success;
}

// Test scenario: Get template details
export function getTemplate() {
    const templateName = templateNames[Math.floor(Math.random() * templateNames.length)];
    const version = '1.0.0';
    const params = {
        headers: getAuthHeaders(),
        tags: { name: 'marketplace_get_template' },
    };

    const startTime = Date.now();
    const res = http.get(`${REGISTRY_URL}/api/v1/templates/${templateName}/versions/${version}`, params);
    const duration = Date.now() - startTime;

    const success = check(res, {
        'get template status is 200 or 404': (r) => r.status === 200 || r.status === 404,
    });

    downloadSuccessRate.add(success);
    downloadDuration.add(duration);
    downloadCounter.add(1);

    return success;
}

// Test scenario: Get scenario details
export function getScenario() {
    const scenarioName = scenarioNames[Math.floor(Math.random() * scenarioNames.length)];
    const params = {
        headers: getAuthHeaders(),
        tags: { name: 'marketplace_get_scenario' },
    };

    const startTime = Date.now();
    const res = http.get(`${REGISTRY_URL}/api/v1/scenarios/${scenarioName}`, params);
    const duration = Date.now() - startTime;

    const success = check(res, {
        'get scenario status is 200 or 404': (r) => r.status === 200 || r.status === 404,
    });

    downloadSuccessRate.add(success);
    downloadDuration.add(duration);
    downloadCounter.add(1);

    return success;
}

// Test scenario: Publish plugin (requires auth)
export function publishPlugin() {
    if (!AUTH_TOKEN) {
        return false; // Skip if no auth token
    }

    const pluginName = `test-plugin-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    const wasmData = generateDummyWasm();
    const checksum = 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'; // SHA-256 of empty string

    const params = {
        headers: getAuthHeaders(),
        tags: { name: 'marketplace_publish_plugin' },
    };

    const payload = JSON.stringify({
        name: pluginName,
        version: '1.0.0',
        description: 'Test plugin for load testing',
        category: 'testing',
        license: 'MIT',
        tags: ['test', 'load-test'],
        checksum: checksum,
        file_size: wasmData.length,
        wasm_data: wasmData,
    });

    const startTime = Date.now();
    const res = http.post(`${REGISTRY_URL}/api/v1/plugins/publish`, payload, params);
    const duration = Date.now() - startTime;

    const success = check(res, {
        'publish plugin status is 200 or 409': (r) => r.status === 200 || r.status === 409, // 409 = already exists
    });

    publishSuccessRate.add(success);
    publishDuration.add(duration);
    publishCounter.add(1);

    return success;
}

// Main test function
export default function () {
    // Weighted distribution of operations
    // 60% search operations, 30% get operations, 10% publish operations
    const rand = Math.random();

    if (rand < 0.3) {
        // 30% - Search plugins
        searchPlugins();
    } else if (rand < 0.5) {
        // 20% - Search templates
        searchTemplates();
    } else if (rand < 0.7) {
        // 20% - Search scenarios
        searchScenarios();
    } else if (rand < 0.8) {
        // 10% - Get plugin
        getPlugin();
    } else if (rand < 0.9) {
        // 10% - Get template
        getTemplate();
    } else if (rand < 0.95) {
        // 5% - Get scenario
        getScenario();
    } else {
        // 5% - Publish plugin (if authenticated)
        publishPlugin();
    }

    // Random sleep between 0.5 and 2 seconds to simulate realistic user behavior
    sleep(Math.random() * 1.5 + 0.5);
}

// Load test configuration
export const options = {
    stages: [
        // Ramp up to 50 users over 1 minute
        { duration: '1m', target: 50 },
        // Stay at 50 users for 2 minutes
        { duration: '2m', target: 50 },
        // Ramp up to 100 users over 1 minute
        { duration: '1m', target: 100 },
        // Stay at 100 users for 3 minutes
        { duration: '3m', target: 100 },
        // Ramp down to 0 users over 1 minute
        { duration: '1m', target: 0 },
    ],
    thresholds: {
        // Overall HTTP request thresholds
        http_req_duration: ['p(95)<1000', 'p(99)<2000'], // 95% of requests < 1s, 99% < 2s
        http_req_failed: ['rate<0.01'], // Error rate < 1%
        http_reqs: ['rate>10'], // Minimum 10 requests per second

        // Marketplace-specific thresholds
        'marketplace_search_success': ['rate>0.95'], // 95% search success rate
        'marketplace_search_duration': ['p(95)<500', 'p(99)<1000'], // Search latency
        'marketplace_download_success': ['rate>0.95'], // 95% download success rate
        'marketplace_download_duration': ['p(95)<500', 'p(99)<1000'], // Download latency
        'marketplace_publish_success': ['rate>0.90'], // 90% publish success rate (may fail due to conflicts)
        'marketplace_publish_duration': ['p(95)<2000', 'p(99)<5000'], // Publish latency (higher due to file upload)
    },
    summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(90)', 'p(95)', 'p(99)', 'count'],
};
