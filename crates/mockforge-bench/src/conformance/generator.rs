//! k6 script generator for OpenAPI 3.0.0 conformance testing

use crate::error::{BenchError, Result};
use std::path::Path;

/// Configuration for conformance test generation
pub struct ConformanceConfig {
    /// Target base URL
    pub target_url: String,
    /// API key for security scheme tests
    pub api_key: Option<String>,
    /// Basic auth credentials (user:pass) for security scheme tests
    pub basic_auth: Option<String>,
    /// Skip TLS verification
    pub skip_tls_verify: bool,
    /// Optional category filter — None means all categories
    pub categories: Option<Vec<String>>,
    /// Optional base path prefix for all generated URLs (e.g., "/api")
    pub base_path: Option<String>,
    /// Custom headers to inject into every conformance request (e.g., auth headers).
    /// Each entry is (header_name, header_value). When a custom header matches
    /// a spec-derived header name, the custom value replaces the placeholder.
    pub custom_headers: Vec<(String, String)>,
}

impl ConformanceConfig {
    /// Check if a category should be included based on the filter
    pub fn should_include_category(&self, category: &str) -> bool {
        match &self.categories {
            None => true,
            Some(cats) => cats.iter().any(|c| c.eq_ignore_ascii_case(category)),
        }
    }

    /// Returns true if custom headers are configured
    pub fn has_custom_headers(&self) -> bool {
        !self.custom_headers.is_empty()
    }

    /// Format custom headers as a JS object literal string
    pub fn custom_headers_js_object(&self) -> String {
        let entries: Vec<String> = self
            .custom_headers
            .iter()
            .map(|(k, v)| format!("'{}': '{}'", k, v.replace('\'', "\\'")))
            .collect();
        format!("{{ {} }}", entries.join(", "))
    }

    /// Returns the effective base URL with base_path appended.
    /// Handles trailing/leading slash normalization to avoid double slashes.
    pub fn effective_base_url(&self) -> String {
        match &self.base_path {
            None => self.target_url.clone(),
            Some(bp) if bp.is_empty() => self.target_url.clone(),
            Some(bp) => {
                let url = self.target_url.trim_end_matches('/');
                let path = if bp.starts_with('/') {
                    bp.as_str()
                } else {
                    return format!("{}/{}", url, bp);
                };
                format!("{}{}", url, path)
            }
        }
    }
}

/// Generates k6 scripts for OpenAPI 3.0.0 conformance testing
pub struct ConformanceGenerator {
    config: ConformanceConfig,
}

impl ConformanceGenerator {
    pub fn new(config: ConformanceConfig) -> Self {
        Self { config }
    }

    /// Generate the conformance test k6 script
    pub fn generate(&self) -> Result<String> {
        let mut script = String::with_capacity(16384);

        // Imports
        script.push_str("import http from 'k6/http';\n");
        script.push_str("import { check, group } from 'k6';\n\n");

        // Options: 1 VU, 1 iteration (functional test, not load test)
        script.push_str("export const options = {\n");
        script.push_str("  vus: 1,\n");
        script.push_str("  iterations: 1,\n");
        if self.config.skip_tls_verify {
            script.push_str("  insecureSkipTLSVerify: true,\n");
        }
        script.push_str("  thresholds: {\n");
        script.push_str("    checks: ['rate>0'],\n");
        script.push_str("  },\n");
        script.push_str("};\n\n");

        // Base URL (includes base_path if configured)
        script.push_str(&format!("const BASE_URL = '{}';\n\n", self.config.effective_base_url()));

        // Helper: JSON headers
        script.push_str("const JSON_HEADERS = { 'Content-Type': 'application/json' };\n\n");

        // Custom auth headers (injected via --conformance-header)
        if self.config.has_custom_headers() {
            script.push_str(&format!(
                "const CUSTOM_HEADERS = {};\n\n",
                self.config.custom_headers_js_object()
            ));
        }

        // Default function
        script.push_str("export default function () {\n");

        if self.config.should_include_category("Parameters") {
            self.generate_parameters_group(&mut script);
        }
        if self.config.should_include_category("Request Bodies") {
            self.generate_request_bodies_group(&mut script);
        }
        if self.config.should_include_category("Schema Types") {
            self.generate_schema_types_group(&mut script);
        }
        if self.config.should_include_category("Composition") {
            self.generate_composition_group(&mut script);
        }
        if self.config.should_include_category("String Formats") {
            self.generate_string_formats_group(&mut script);
        }
        if self.config.should_include_category("Constraints") {
            self.generate_constraints_group(&mut script);
        }
        if self.config.should_include_category("Response Codes") {
            self.generate_response_codes_group(&mut script);
        }
        if self.config.should_include_category("HTTP Methods") {
            self.generate_http_methods_group(&mut script);
        }
        if self.config.should_include_category("Content Types") {
            self.generate_content_negotiation_group(&mut script);
        }
        if self.config.should_include_category("Security") {
            self.generate_security_group(&mut script);
        }

        script.push_str("}\n\n");

        // handleSummary for conformance report output
        self.generate_handle_summary(&mut script);

        Ok(script)
    }

    /// Write the generated script to a file
    pub fn write_script(&self, path: &Path) -> Result<()> {
        let script = self.generate()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, script)
            .map_err(|e| BenchError::Other(format!("Failed to write conformance script: {}", e)))
    }

    /// Returns a JS expression for merging custom headers with provided headers.
    /// If no custom headers, returns the input as-is.
    /// If custom headers exist, wraps with Object.assign.
    fn merge_with_custom_headers(&self, headers_expr: &str) -> String {
        if self.config.has_custom_headers() {
            format!("Object.assign({{}}, {}, CUSTOM_HEADERS)", headers_expr)
        } else {
            headers_expr.to_string()
        }
    }

    /// Emit a GET request with optional custom headers merged in.
    fn emit_get(&self, script: &mut String, url: &str, extra_headers: Option<&str>) {
        let has_custom = self.config.has_custom_headers();
        match (extra_headers, has_custom) {
            (None, false) => {
                script.push_str(&format!("      let res = http.get(`{}`);\n", url));
            }
            (None, true) => {
                script.push_str(&format!(
                    "      let res = http.get(`{}`, {{ headers: CUSTOM_HEADERS }});\n",
                    url
                ));
            }
            (Some(hdrs), false) => {
                script.push_str(&format!(
                    "      let res = http.get(`{}`, {{ headers: {} }});\n",
                    url, hdrs
                ));
            }
            (Some(hdrs), true) => {
                script.push_str(&format!(
                    "      let res = http.get(`{}`, {{ headers: Object.assign({{}}, {}, CUSTOM_HEADERS) }});\n",
                    url, hdrs
                ));
            }
        }
    }

    /// Emit a POST/PUT/PATCH request with optional custom headers merged in.
    fn emit_post_like(
        &self,
        script: &mut String,
        method: &str,
        url: &str,
        body: &str,
        headers_expr: &str,
    ) {
        let merged = self.merge_with_custom_headers(headers_expr);
        script.push_str(&format!(
            "      let res = http.{}(`{}`, {}, {{ headers: {} }});\n",
            method, url, body, merged
        ));
    }

    /// Emit a DELETE/HEAD/OPTIONS request with optional custom headers.
    fn emit_no_body(&self, script: &mut String, method: &str, url: &str) {
        if self.config.has_custom_headers() {
            script.push_str(&format!(
                "      let res = http.{}(`{}`, {{ headers: CUSTOM_HEADERS }});\n",
                method, url
            ));
        } else {
            script.push_str(&format!("      let res = http.{}(`{}`);\n", method, url));
        }
    }

    fn generate_parameters_group(&self, script: &mut String) {
        script.push_str("  group('Parameters', function () {\n");

        // Path param: string
        script.push_str("    {\n");
        self.emit_get(script, "${BASE_URL}/conformance/params/hello", None);
        script.push_str(
            "      check(res, { 'param:path:string': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        // Path param: integer
        script.push_str("    {\n");
        self.emit_get(script, "${BASE_URL}/conformance/params/42", None);
        script.push_str(
            "      check(res, { 'param:path:integer': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        // Query param: string
        script.push_str("    {\n");
        self.emit_get(script, "${BASE_URL}/conformance/params/query?name=test", None);
        script.push_str(
            "      check(res, { 'param:query:string': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        // Query param: integer
        script.push_str("    {\n");
        self.emit_get(script, "${BASE_URL}/conformance/params/query?count=10", None);
        script.push_str(
            "      check(res, { 'param:query:integer': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        // Query param: array
        script.push_str("    {\n");
        self.emit_get(script, "${BASE_URL}/conformance/params/query?tags=a&tags=b", None);
        script.push_str(
            "      check(res, { 'param:query:array': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        // Header param
        script.push_str("    {\n");
        self.emit_get(
            script,
            "${BASE_URL}/conformance/params/header",
            Some("{ 'X-Custom-Param': 'test-value' }"),
        );
        script.push_str(
            "      check(res, { 'param:header': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        // Cookie param
        script.push_str("    {\n");
        script.push_str("      let jar = http.cookieJar();\n");
        script.push_str("      jar.set(BASE_URL, 'session', 'abc123');\n");
        self.emit_get(script, "${BASE_URL}/conformance/params/cookie", None);
        script.push_str(
            "      check(res, { 'param:cookie': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        script.push_str("  });\n\n");
    }

    fn generate_request_bodies_group(&self, script: &mut String) {
        script.push_str("  group('Request Bodies', function () {\n");

        // JSON body
        script.push_str("    {\n");
        self.emit_post_like(
            script,
            "post",
            "${BASE_URL}/conformance/body/json",
            "JSON.stringify({ name: 'test', value: 42 })",
            "JSON_HEADERS",
        );
        script.push_str(
            "      check(res, { 'body:json': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        // Form-urlencoded body
        script.push_str("    {\n");
        if self.config.has_custom_headers() {
            script.push_str(
                "      let res = http.post(`${BASE_URL}/conformance/body/form`, { field1: 'value1', field2: 'value2' }, { headers: CUSTOM_HEADERS });\n",
            );
        } else {
            script.push_str(
                "      let res = http.post(`${BASE_URL}/conformance/body/form`, { field1: 'value1', field2: 'value2' });\n",
            );
        }
        script.push_str(
            "      check(res, { 'body:form-urlencoded': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        // Multipart body
        script.push_str("    {\n");
        script.push_str(
            "      let data = { field: http.file('test content', 'test.txt', 'text/plain') };\n",
        );
        if self.config.has_custom_headers() {
            script.push_str(
                "      let res = http.post(`${BASE_URL}/conformance/body/multipart`, data, { headers: CUSTOM_HEADERS });\n",
            );
        } else {
            script.push_str(
                "      let res = http.post(`${BASE_URL}/conformance/body/multipart`, data);\n",
            );
        }
        script.push_str(
            "      check(res, { 'body:multipart': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        script.push_str("  });\n\n");
    }

    fn generate_schema_types_group(&self, script: &mut String) {
        script.push_str("  group('Schema Types', function () {\n");

        let types = [
            ("string", r#"{ "value": "hello" }"#, "schema:string"),
            ("integer", r#"{ "value": 42 }"#, "schema:integer"),
            ("number", r#"{ "value": 3.14 }"#, "schema:number"),
            ("boolean", r#"{ "value": true }"#, "schema:boolean"),
            ("array", r#"{ "value": [1, 2, 3] }"#, "schema:array"),
            ("object", r#"{ "value": { "nested": "data" } }"#, "schema:object"),
        ];

        for (type_name, body, check_name) in types {
            script.push_str("    {\n");
            let url = format!("${{BASE_URL}}/conformance/schema/{}", type_name);
            let body_str = format!("'{}'", body);
            self.emit_post_like(script, "post", &url, &body_str, "JSON_HEADERS");
            script.push_str(&format!(
                "      check(res, {{ '{}': (r) => r.status >= 200 && r.status < 500 }});\n",
                check_name
            ));
            script.push_str("    }\n");
        }

        script.push_str("  });\n\n");
    }

    fn generate_composition_group(&self, script: &mut String) {
        script.push_str("  group('Composition', function () {\n");

        let compositions = [
            ("oneOf", r#"{ "type": "string", "value": "test" }"#, "composition:oneOf"),
            ("anyOf", r#"{ "value": "test" }"#, "composition:anyOf"),
            ("allOf", r#"{ "name": "test", "id": 1 }"#, "composition:allOf"),
        ];

        for (kind, body, check_name) in compositions {
            script.push_str("    {\n");
            let url = format!("${{BASE_URL}}/conformance/composition/{}", kind);
            let body_str = format!("'{}'", body);
            self.emit_post_like(script, "post", &url, &body_str, "JSON_HEADERS");
            script.push_str(&format!(
                "      check(res, {{ '{}': (r) => r.status >= 200 && r.status < 500 }});\n",
                check_name
            ));
            script.push_str("    }\n");
        }

        script.push_str("  });\n\n");
    }

    fn generate_string_formats_group(&self, script: &mut String) {
        script.push_str("  group('String Formats', function () {\n");

        let formats = [
            ("date", r#"{ "value": "2024-01-15" }"#, "format:date"),
            ("date-time", r#"{ "value": "2024-01-15T10:30:00Z" }"#, "format:date-time"),
            ("email", r#"{ "value": "test@example.com" }"#, "format:email"),
            ("uuid", r#"{ "value": "550e8400-e29b-41d4-a716-446655440000" }"#, "format:uuid"),
            ("uri", r#"{ "value": "https://example.com/path" }"#, "format:uri"),
            ("ipv4", r#"{ "value": "192.168.1.1" }"#, "format:ipv4"),
            ("ipv6", r#"{ "value": "::1" }"#, "format:ipv6"),
        ];

        for (fmt, body, check_name) in formats {
            script.push_str("    {\n");
            let url = format!("${{BASE_URL}}/conformance/formats/{}", fmt);
            let body_str = format!("'{}'", body);
            self.emit_post_like(script, "post", &url, &body_str, "JSON_HEADERS");
            script.push_str(&format!(
                "      check(res, {{ '{}': (r) => r.status >= 200 && r.status < 500 }});\n",
                check_name
            ));
            script.push_str("    }\n");
        }

        script.push_str("  });\n\n");
    }

    fn generate_constraints_group(&self, script: &mut String) {
        script.push_str("  group('Constraints', function () {\n");

        let constraints = [
            (
                "required",
                "JSON.stringify({ required_field: 'present' })",
                "constraint:required",
            ),
            ("optional", "JSON.stringify({})", "constraint:optional"),
            ("minmax", "JSON.stringify({ value: 50 })", "constraint:minmax"),
            ("pattern", "JSON.stringify({ value: 'ABC-123' })", "constraint:pattern"),
            ("enum", "JSON.stringify({ status: 'active' })", "constraint:enum"),
        ];

        for (kind, body, check_name) in constraints {
            script.push_str("    {\n");
            let url = format!("${{BASE_URL}}/conformance/constraints/{}", kind);
            self.emit_post_like(script, "post", &url, body, "JSON_HEADERS");
            script.push_str(&format!(
                "      check(res, {{ '{}': (r) => r.status >= 200 && r.status < 500 }});\n",
                check_name
            ));
            script.push_str("    }\n");
        }

        script.push_str("  });\n\n");
    }

    fn generate_response_codes_group(&self, script: &mut String) {
        script.push_str("  group('Response Codes', function () {\n");

        let codes = [
            ("200", "response:200"),
            ("201", "response:201"),
            ("204", "response:204"),
            ("400", "response:400"),
            ("404", "response:404"),
        ];

        for (code, check_name) in codes {
            script.push_str("    {\n");
            let url = format!("${{BASE_URL}}/conformance/responses/{}", code);
            self.emit_get(script, &url, None);
            script.push_str(&format!(
                "      check(res, {{ '{}': (r) => r.status === {} }});\n",
                check_name, code
            ));
            script.push_str("    }\n");
        }

        script.push_str("  });\n\n");
    }

    fn generate_http_methods_group(&self, script: &mut String) {
        script.push_str("  group('HTTP Methods', function () {\n");

        // GET
        script.push_str("    {\n");
        self.emit_get(script, "${BASE_URL}/conformance/methods", None);
        script.push_str(
            "      check(res, { 'method:GET': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        // POST
        script.push_str("    {\n");
        self.emit_post_like(
            script,
            "post",
            "${BASE_URL}/conformance/methods",
            "JSON.stringify({ action: 'create' })",
            "JSON_HEADERS",
        );
        script.push_str(
            "      check(res, { 'method:POST': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        // PUT
        script.push_str("    {\n");
        self.emit_post_like(
            script,
            "put",
            "${BASE_URL}/conformance/methods",
            "JSON.stringify({ action: 'update' })",
            "JSON_HEADERS",
        );
        script.push_str(
            "      check(res, { 'method:PUT': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        // PATCH
        script.push_str("    {\n");
        self.emit_post_like(
            script,
            "patch",
            "${BASE_URL}/conformance/methods",
            "JSON.stringify({ action: 'patch' })",
            "JSON_HEADERS",
        );
        script.push_str(
            "      check(res, { 'method:PATCH': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        // DELETE
        script.push_str("    {\n");
        self.emit_no_body(script, "del", "${BASE_URL}/conformance/methods");
        script.push_str(
            "      check(res, { 'method:DELETE': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        // HEAD
        script.push_str("    {\n");
        self.emit_no_body(script, "head", "${BASE_URL}/conformance/methods");
        script.push_str(
            "      check(res, { 'method:HEAD': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        // OPTIONS
        script.push_str("    {\n");
        self.emit_no_body(script, "options", "${BASE_URL}/conformance/methods");
        script.push_str(
            "      check(res, { 'method:OPTIONS': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        script.push_str("  });\n\n");
    }

    fn generate_content_negotiation_group(&self, script: &mut String) {
        script.push_str("  group('Content Types', function () {\n");

        script.push_str("    {\n");
        self.emit_get(
            script,
            "${BASE_URL}/conformance/content-types",
            Some("{ 'Accept': 'application/json' }"),
        );
        script.push_str(
            "      check(res, { 'content:negotiation': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        script.push_str("  });\n\n");
    }

    fn generate_security_group(&self, script: &mut String) {
        script.push_str("  group('Security', function () {\n");

        // Bearer token
        script.push_str("    {\n");
        self.emit_get(
            script,
            "${BASE_URL}/conformance/security/bearer",
            Some("{ 'Authorization': 'Bearer test-token-123' }"),
        );
        script.push_str(
            "      check(res, { 'security:bearer': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        // API Key
        let api_key = self.config.api_key.as_deref().unwrap_or("test-api-key-123");
        script.push_str("    {\n");
        let api_key_hdrs = format!("{{ 'X-API-Key': '{}' }}", api_key);
        self.emit_get(script, "${BASE_URL}/conformance/security/apikey", Some(&api_key_hdrs));
        script.push_str(
            "      check(res, { 'security:apikey': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        // Basic auth
        let basic_creds = self.config.basic_auth.as_deref().unwrap_or("user:pass");
        let encoded = base64_encode(basic_creds);
        script.push_str("    {\n");
        let basic_hdrs = format!("{{ 'Authorization': 'Basic {}' }}", encoded);
        self.emit_get(script, "${BASE_URL}/conformance/security/basic", Some(&basic_hdrs));
        script.push_str(
            "      check(res, { 'security:basic': (r) => r.status >= 200 && r.status < 500 });\n",
        );
        script.push_str("    }\n");

        script.push_str("  });\n\n");
    }

    fn generate_handle_summary(&self, script: &mut String) {
        script.push_str("export function handleSummary(data) {\n");
        script.push_str("  // Extract check results for conformance reporting\n");
        script.push_str("  let checks = {};\n");
        script.push_str("  if (data.metrics && data.metrics.checks) {\n");
        script.push_str("    // Overall check pass rate\n");
        script.push_str("    checks.overall_pass_rate = data.metrics.checks.values.rate;\n");
        script.push_str("  }\n");
        script.push_str("  // Collect per-check results from root_group\n");
        script.push_str("  let checkResults = {};\n");
        script.push_str("  function walkGroups(group) {\n");
        script.push_str("    if (group.checks) {\n");
        script.push_str("      for (let checkObj of group.checks) {\n");
        script.push_str("        checkResults[checkObj.name] = {\n");
        script.push_str("          passes: checkObj.passes,\n");
        script.push_str("          fails: checkObj.fails,\n");
        script.push_str("        };\n");
        script.push_str("      }\n");
        script.push_str("    }\n");
        script.push_str("    if (group.groups) {\n");
        script.push_str("      for (let subGroup of group.groups) {\n");
        script.push_str("        walkGroups(subGroup);\n");
        script.push_str("      }\n");
        script.push_str("    }\n");
        script.push_str("  }\n");
        script.push_str("  if (data.root_group) {\n");
        script.push_str("    walkGroups(data.root_group);\n");
        script.push_str("  }\n");
        script.push_str("  return {\n");
        script.push_str("    'conformance-report.json': JSON.stringify({ checks: checkResults, overall: checks }, null, 2),\n");
        script.push_str("    stdout: textSummary(data, { indent: '  ', enableColors: true }),\n");
        script.push_str("  };\n");
        script.push_str("}\n\n");
        script.push_str("// textSummary fallback\n");
        script.push_str("function textSummary(data, opts) {\n");
        script.push_str("  return JSON.stringify(data, null, 2);\n");
        script.push_str("}\n");
    }
}

/// Simple base64 encoding for basic auth
fn base64_encode(input: &str) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut result = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_conformance_script() {
        let config = ConformanceConfig {
            target_url: "http://localhost:8080".to_string(),
            api_key: None,
            basic_auth: None,
            skip_tls_verify: false,
            categories: None,
            base_path: None,
            custom_headers: vec![],
        };
        let generator = ConformanceGenerator::new(config);
        let script = generator.generate().unwrap();

        assert!(script.contains("import http from 'k6/http'"));
        assert!(script.contains("vus: 1"));
        assert!(script.contains("iterations: 1"));
        assert!(script.contains("group('Parameters'"));
        assert!(script.contains("group('Request Bodies'"));
        assert!(script.contains("group('Schema Types'"));
        assert!(script.contains("group('Composition'"));
        assert!(script.contains("group('String Formats'"));
        assert!(script.contains("group('Constraints'"));
        assert!(script.contains("group('Response Codes'"));
        assert!(script.contains("group('HTTP Methods'"));
        assert!(script.contains("group('Content Types'"));
        assert!(script.contains("group('Security'"));
        assert!(script.contains("handleSummary"));
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode("user:pass"), "dXNlcjpwYXNz");
        assert_eq!(base64_encode("a"), "YQ==");
        assert_eq!(base64_encode("ab"), "YWI=");
        assert_eq!(base64_encode("abc"), "YWJj");
    }

    #[test]
    fn test_conformance_script_with_custom_auth() {
        let config = ConformanceConfig {
            target_url: "https://api.example.com".to_string(),
            api_key: Some("my-api-key".to_string()),
            basic_auth: Some("admin:secret".to_string()),
            skip_tls_verify: true,
            categories: None,
            base_path: None,
            custom_headers: vec![],
        };
        let generator = ConformanceGenerator::new(config);
        let script = generator.generate().unwrap();

        assert!(script.contains("insecureSkipTLSVerify: true"));
        assert!(script.contains("my-api-key"));
        assert!(script.contains(&base64_encode("admin:secret")));
    }

    #[test]
    fn test_should_include_category_none_includes_all() {
        let config = ConformanceConfig {
            target_url: "http://localhost:8080".to_string(),
            api_key: None,
            basic_auth: None,
            skip_tls_verify: false,
            categories: None,
            base_path: None,
            custom_headers: vec![],
        };
        assert!(config.should_include_category("Parameters"));
        assert!(config.should_include_category("Security"));
        assert!(config.should_include_category("Anything"));
    }

    #[test]
    fn test_should_include_category_filtered() {
        let config = ConformanceConfig {
            target_url: "http://localhost:8080".to_string(),
            api_key: None,
            basic_auth: None,
            skip_tls_verify: false,
            categories: Some(vec!["Parameters".to_string(), "Security".to_string()]),
            base_path: None,
            custom_headers: vec![],
        };
        assert!(config.should_include_category("Parameters"));
        assert!(config.should_include_category("Security"));
        assert!(config.should_include_category("parameters")); // case-insensitive
        assert!(!config.should_include_category("Composition"));
        assert!(!config.should_include_category("Schema Types"));
    }

    #[test]
    fn test_generate_with_category_filter() {
        let config = ConformanceConfig {
            target_url: "http://localhost:8080".to_string(),
            api_key: None,
            basic_auth: None,
            skip_tls_verify: false,
            categories: Some(vec!["Parameters".to_string(), "Security".to_string()]),
            base_path: None,
            custom_headers: vec![],
        };
        let generator = ConformanceGenerator::new(config);
        let script = generator.generate().unwrap();

        assert!(script.contains("group('Parameters'"));
        assert!(script.contains("group('Security'"));
        assert!(!script.contains("group('Request Bodies'"));
        assert!(!script.contains("group('Schema Types'"));
        assert!(!script.contains("group('Composition'"));
    }

    #[test]
    fn test_effective_base_url_no_base_path() {
        let config = ConformanceConfig {
            target_url: "https://example.com".to_string(),
            api_key: None,
            basic_auth: None,
            skip_tls_verify: false,
            categories: None,
            base_path: None,
            custom_headers: vec![],
        };
        assert_eq!(config.effective_base_url(), "https://example.com");
    }

    #[test]
    fn test_effective_base_url_with_base_path() {
        let config = ConformanceConfig {
            target_url: "https://example.com".to_string(),
            api_key: None,
            basic_auth: None,
            skip_tls_verify: false,
            categories: None,
            base_path: Some("/api".to_string()),
            custom_headers: vec![],
        };
        assert_eq!(config.effective_base_url(), "https://example.com/api");
    }

    #[test]
    fn test_effective_base_url_trailing_slash_normalization() {
        let config = ConformanceConfig {
            target_url: "https://example.com/".to_string(),
            api_key: None,
            basic_auth: None,
            skip_tls_verify: false,
            categories: None,
            base_path: Some("/api".to_string()),
            custom_headers: vec![],
        };
        assert_eq!(config.effective_base_url(), "https://example.com/api");
    }

    #[test]
    fn test_generate_script_with_base_path() {
        let config = ConformanceConfig {
            target_url: "https://192.168.2.86".to_string(),
            api_key: None,
            basic_auth: None,
            skip_tls_verify: true,
            categories: None,
            base_path: Some("/api".to_string()),
            custom_headers: vec![],
        };
        let generator = ConformanceGenerator::new(config);
        let script = generator.generate().unwrap();

        assert!(script.contains("const BASE_URL = 'https://192.168.2.86/api'"));
        // Verify URLs include the base path via BASE_URL
        assert!(script.contains("${BASE_URL}/conformance/"));
    }

    #[test]
    fn test_generate_with_custom_headers() {
        let config = ConformanceConfig {
            target_url: "https://192.168.2.86".to_string(),
            api_key: None,
            basic_auth: None,
            skip_tls_verify: true,
            categories: Some(vec!["Parameters".to_string()]),
            base_path: Some("/api".to_string()),
            custom_headers: vec![
                ("X-Avi-Tenant".to_string(), "admin".to_string()),
                ("X-CSRFToken".to_string(), "real-token".to_string()),
            ],
        };
        let generator = ConformanceGenerator::new(config);
        let script = generator.generate().unwrap();

        // Should declare CUSTOM_HEADERS constant
        assert!(
            script.contains("const CUSTOM_HEADERS = "),
            "Script should declare CUSTOM_HEADERS"
        );
        assert!(script.contains("'X-Avi-Tenant': 'admin'"));
        assert!(script.contains("'X-CSRFToken': 'real-token'"));
        // GET requests should merge with CUSTOM_HEADERS
        assert!(
            script.contains("CUSTOM_HEADERS"),
            "Script should reference CUSTOM_HEADERS in requests"
        );
    }

    #[test]
    fn test_custom_headers_js_object() {
        let config = ConformanceConfig {
            target_url: "http://localhost".to_string(),
            api_key: None,
            basic_auth: None,
            skip_tls_verify: false,
            categories: None,
            base_path: None,
            custom_headers: vec![
                ("Authorization".to_string(), "Bearer abc123".to_string()),
                ("X-Custom".to_string(), "value".to_string()),
            ],
        };
        let js = config.custom_headers_js_object();
        assert!(js.contains("'Authorization': 'Bearer abc123'"));
        assert!(js.contains("'X-Custom': 'value'"));
    }
}
