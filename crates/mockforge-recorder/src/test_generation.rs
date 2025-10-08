//! AI-powered test generation from recorded API interactions
//!
//! This module provides functionality to automatically generate test cases
//! from recorded API requests and responses using AI/LLM capabilities.

use crate::{RecorderDatabase, RecorderError, Result};
use crate::models::{Protocol, RecordedRequest, RecordedResponse};
use crate::query::{QueryFilter, execute_query};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Test format to generate
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TestFormat {
    /// Rust test using reqwest
    RustReqwest,
    /// HTTP file format (.http)
    HttpFile,
    /// cURL commands
    Curl,
    /// Postman collection
    Postman,
    /// k6 load test script
    K6,
    /// Python pytest
    PythonPytest,
    /// JavaScript/TypeScript Jest
    JavaScriptJest,
    /// Go test
    GoTest,
    /// Ruby RSpec
    RubyRspec,
    /// Java JUnit
    JavaJunit,
    /// C# xUnit
    CSharpXunit,
}

/// Test generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestGenerationConfig {
    /// Test format to generate
    pub format: TestFormat,
    /// Include assertions for response validation
    pub include_assertions: bool,
    /// Include response body validation
    pub validate_body: bool,
    /// Include status code validation
    pub validate_status: bool,
    /// Include header validation
    pub validate_headers: bool,
    /// Include timing assertions
    pub validate_timing: bool,
    /// Maximum duration threshold in ms
    pub max_duration_ms: Option<u64>,
    /// Test suite name
    pub suite_name: String,
    /// Base URL for generated tests
    pub base_url: Option<String>,
    /// Use AI to generate intelligent test descriptions
    pub ai_descriptions: bool,
    /// LLM provider configuration (optional)
    pub llm_config: Option<LlmConfig>,
    /// Group tests by endpoint
    pub group_by_endpoint: bool,
    /// Include setup/teardown code
    pub include_setup_teardown: bool,
    /// Generate test data fixtures using AI
    pub generate_fixtures: bool,
    /// Suggest edge cases using AI
    pub suggest_edge_cases: bool,
    /// Perform test gap analysis
    pub analyze_test_gaps: bool,
    /// Deduplicate similar tests
    pub deduplicate_tests: bool,
    /// Optimize test execution order
    pub optimize_test_order: bool,
}

impl Default for TestGenerationConfig {
    fn default() -> Self {
        Self {
            format: TestFormat::RustReqwest,
            include_assertions: true,
            validate_body: true,
            validate_status: true,
            validate_headers: false,
            validate_timing: false,
            max_duration_ms: None,
            suite_name: "generated_tests".to_string(),
            base_url: Some("http://localhost:3000".to_string()),
            ai_descriptions: false,
            llm_config: None,
            group_by_endpoint: true,
            include_setup_teardown: true,
            generate_fixtures: false,
            suggest_edge_cases: false,
            analyze_test_gaps: false,
            deduplicate_tests: false,
            optimize_test_order: false,
        }
    }
}

/// LLM configuration for AI-powered test generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// LLM provider (openai, anthropic, ollama)
    pub provider: String,
    /// API endpoint
    pub api_endpoint: String,
    /// API key
    pub api_key: Option<String>,
    /// Model name
    pub model: String,
    /// Temperature for generation
    pub temperature: f64,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            api_endpoint: "http://localhost:11434/api/generate".to_string(),
            api_key: None,
            model: "llama2".to_string(),
            temperature: 0.3,
        }
    }
}

/// Generated test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedTest {
    /// Test name/identifier
    pub name: String,
    /// Test description
    pub description: String,
    /// Test code
    pub code: String,
    /// Original request ID
    pub request_id: String,
    /// Endpoint being tested
    pub endpoint: String,
    /// HTTP method
    pub method: String,
}

/// Test generation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestGenerationResult {
    /// Generated tests
    pub tests: Vec<GeneratedTest>,
    /// Test suite metadata
    pub metadata: TestSuiteMetadata,
    /// Full test file content
    pub test_file: String,
}

/// Test suite metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuiteMetadata {
    /// Suite name
    pub name: String,
    /// Total number of tests
    pub test_count: usize,
    /// Number of endpoints covered
    pub endpoint_count: usize,
    /// Protocols covered
    pub protocols: Vec<Protocol>,
    /// Generation timestamp
    pub generated_at: chrono::DateTime<chrono::Utc>,
    /// Format used
    pub format: TestFormat,
    /// Generated fixtures (if enabled)
    pub fixtures: Option<Vec<TestFixture>>,
    /// Edge case suggestions (if enabled)
    pub edge_cases: Option<Vec<EdgeCaseSuggestion>>,
    /// Test gap analysis (if enabled)
    pub gap_analysis: Option<TestGapAnalysis>,
}

/// Test data fixture generated by AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFixture {
    /// Fixture name
    pub name: String,
    /// Fixture description
    pub description: String,
    /// Fixture data in JSON format
    pub data: Value,
    /// Related endpoints
    pub endpoints: Vec<String>,
}

/// Edge case suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeCaseSuggestion {
    /// Endpoint being tested
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// Edge case type
    pub case_type: String,
    /// Description of the edge case
    pub description: String,
    /// Suggested test input
    pub suggested_input: Option<Value>,
    /// Expected behavior
    pub expected_behavior: String,
    /// Priority (1-5, 5 being highest)
    pub priority: u8,
}

/// Test gap analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestGapAnalysis {
    /// Endpoints without tests
    pub untested_endpoints: Vec<String>,
    /// HTTP methods not covered per endpoint
    pub missing_methods: HashMap<String, Vec<String>>,
    /// Status codes not tested
    pub missing_status_codes: HashMap<String, Vec<u16>>,
    /// Common error scenarios not tested
    pub missing_error_scenarios: Vec<String>,
    /// Coverage percentage
    pub coverage_percentage: f64,
    /// Recommendations
    pub recommendations: Vec<String>,
}

/// Test generator engine
pub struct TestGenerator {
    database: RecorderDatabase,
    config: TestGenerationConfig,
}

impl TestGenerator {
    /// Create a new test generator
    pub fn new(database: RecorderDatabase, config: TestGenerationConfig) -> Self {
        Self { database, config }
    }

    /// Create from an Arc<RecorderDatabase>
    pub fn from_arc(database: std::sync::Arc<RecorderDatabase>, config: TestGenerationConfig) -> Self {
        Self {
            database: (*database).clone(),
            config,
        }
    }

    /// Generate tests from a query filter
    pub async fn generate_from_filter(
        &self,
        filter: QueryFilter,
    ) -> Result<TestGenerationResult> {
        // Execute query to get recordings
        let query_result = execute_query(&self.database, filter).await?;

        if query_result.exchanges.is_empty() {
            return Err(RecorderError::InvalidFilter(
                "No recordings found matching the filter".to_string(),
            ));
        }

        // Generate tests from exchanges
        let mut tests = Vec::new();
        let mut endpoints = std::collections::HashSet::new();
        let mut protocols = std::collections::HashSet::new();

        for exchange in &query_result.exchanges {
            let request = &exchange.request;

            // Skip exchanges without responses
            let Some(response) = &exchange.response else {
                continue;
            };

            endpoints.insert(format!("{} {}", request.method, request.path));
            protocols.insert(request.protocol.clone());

            let test = self.generate_test_for_exchange(request, response).await?;
            tests.push(test);
        }

        // Deduplicate tests if configured
        if self.config.deduplicate_tests {
            tests = self.deduplicate_tests(tests);
        }

        // Optimize test order if configured
        if self.config.optimize_test_order {
            tests = self.optimize_test_order(tests);
        }

        // Group tests by endpoint if configured
        if self.config.group_by_endpoint {
            tests.sort_by(|a, b| a.endpoint.cmp(&b.endpoint));
        }

        // Generate advanced AI features
        let fixtures = if self.config.generate_fixtures {
            Some(self.generate_test_fixtures(&query_result.exchanges).await?)
        } else {
            None
        };

        let edge_cases = if self.config.suggest_edge_cases {
            Some(self.suggest_edge_cases(&query_result.exchanges).await?)
        } else {
            None
        };

        let gap_analysis = if self.config.analyze_test_gaps {
            Some(self.analyze_test_gaps(&query_result.exchanges, &tests).await?)
        } else {
            None
        };

        // Generate full test file
        let test_file = self.generate_test_file(&tests)?;

        // Create metadata
        let metadata = TestSuiteMetadata {
            name: self.config.suite_name.clone(),
            test_count: tests.len(),
            endpoint_count: endpoints.len(),
            protocols: protocols.into_iter().collect(),
            generated_at: chrono::Utc::now(),
            format: self.config.format.clone(),
            fixtures,
            edge_cases,
            gap_analysis,
        };

        Ok(TestGenerationResult {
            tests,
            metadata,
            test_file,
        })
    }

    /// Generate a single test from an exchange
    async fn generate_test_for_exchange(
        &self,
        request: &RecordedRequest,
        response: &RecordedResponse,
    ) -> Result<GeneratedTest> {
        let test_name = self.generate_test_name(request);
        let description = if self.config.ai_descriptions {
            self.generate_ai_description(request, response).await?
        } else {
            format!("Test {} {}", request.method, request.path)
        };

        let code = match self.config.format {
            TestFormat::RustReqwest => self.generate_rust_test(request, response)?,
            TestFormat::HttpFile => self.generate_http_file(request, response)?,
            TestFormat::Curl => self.generate_curl(request, response)?,
            TestFormat::Postman => self.generate_postman(request, response)?,
            TestFormat::K6 => self.generate_k6(request, response)?,
            TestFormat::PythonPytest => self.generate_python_test(request, response)?,
            TestFormat::JavaScriptJest => self.generate_javascript_test(request, response)?,
            TestFormat::GoTest => self.generate_go_test(request, response)?,
            TestFormat::RubyRspec => self.generate_ruby_test(request, response)?,
            TestFormat::JavaJunit => self.generate_java_test(request, response)?,
            TestFormat::CSharpXunit => self.generate_csharp_test(request, response)?,
        };

        Ok(GeneratedTest {
            name: test_name,
            description,
            code,
            request_id: request.id.clone(),
            endpoint: request.path.clone(),
            method: request.method.clone(),
        })
    }

    /// Generate test name from request
    fn generate_test_name(&self, request: &RecordedRequest) -> String {
        let method = request.method.to_lowercase();
        let path = request.path
            .trim_start_matches('/')
            .replace('/', "_")
            .replace('-', "_")
            .replace("{", "")
            .replace("}", "");

        format!("test_{}_{}", method, path)
    }

    /// Generate AI-powered test description
    async fn generate_ai_description(
        &self,
        request: &RecordedRequest,
        response: &RecordedResponse,
    ) -> Result<String> {
        if let Some(llm_config) = &self.config.llm_config {
            // Use LLM to generate meaningful description
            let prompt = format!(
                "Generate a concise test description for this API call:\n\
                Method: {}\n\
                Path: {}\n\
                Status: {}\n\
                \n\
                Describe what this endpoint does and what the test validates in one sentence.",
                request.method,
                request.path,
                response.status_code
            );

            match self.call_llm(llm_config, &prompt).await {
                Ok(description) => Ok(description),
                Err(_) => Ok(format!("Test {} {}", request.method, request.path)),
            }
        } else {
            Ok(format!("Test {} {}", request.method, request.path))
        }
    }

    /// Call LLM for generation
    async fn call_llm(&self, config: &LlmConfig, prompt: &str) -> Result<String> {
        let client = reqwest::Client::new();

        match config.provider.as_str() {
            "ollama" => {
                let body = serde_json::json!({
                    "model": config.model,
                    "prompt": prompt,
                    "stream": false,
                    "options": {
                        "temperature": config.temperature
                    }
                });

                let response = client
                    .post(&config.api_endpoint)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| RecorderError::Replay(format!("LLM request failed: {}", e)))?;

                let result: Value = response
                    .json()
                    .await
                    .map_err(|e| RecorderError::Replay(format!("Failed to parse JSON response: {}", e)))?;

                result
                    .get("response")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .ok_or_else(|| RecorderError::Replay("Invalid LLM response".to_string()))
            }
            "openai" => {
                let body = serde_json::json!({
                    "model": config.model,
                    "messages": [
                        {"role": "system", "content": "You are a helpful assistant that generates concise test descriptions."},
                        {"role": "user", "content": prompt}
                    ],
                    "temperature": config.temperature,
                    "max_tokens": 100
                });

                let mut request_builder = client
                    .post(&config.api_endpoint)
                    .json(&body);

                if let Some(api_key) = &config.api_key {
                    request_builder = request_builder.header("Authorization", format!("Bearer {}", api_key));
                }

                let response = request_builder
                    .send()
                    .await
                    .map_err(|e| RecorderError::Replay(format!("LLM request failed: {}", e)))?;

                let result: Value = response
                    .json()
                    .await
                    .map_err(|e| RecorderError::Replay(format!("Failed to parse JSON response: {}", e)))?;

                result
                    .get("choices")
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.get("message"))
                    .and_then(|v| v.get("content"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .ok_or_else(|| RecorderError::Replay("Invalid LLM response".to_string()))
            }
            _ => Err(RecorderError::Replay(format!("Unsupported LLM provider: {}", config.provider))),
        }
    }

    /// Generate Rust test code
    fn generate_rust_test(
        &self,
        request: &RecordedRequest,
        response: &RecordedResponse,
    ) -> Result<String> {
        let base_url = self.config.base_url.as_deref().unwrap_or("http://localhost:3000");
        let url = format!("{}{}", base_url, request.path);

        let mut code = String::new();
        let test_name = self.generate_test_name(request);

        code.push_str(&format!("#[tokio::test]\n"));
        code.push_str(&format!("async fn {}() {{\n", test_name));
        code.push_str("    let client = reqwest::Client::new();\n");
        code.push_str(&format!("    let response = client.{}(\"{}\")\n",
            request.method.to_lowercase(), url));

        // Add headers
        if let Ok(headers) = serde_json::from_str::<HashMap<String, String>>(&request.headers) {
            for (key, value) in headers.iter() {
                if key.to_lowercase() != "host" {
                    code.push_str(&format!("        .header(\"{}\", \"{}\")\n", key, value));
                }
            }
        }

        // Add body if present
        if let Some(body) = &request.body {
            if !body.is_empty() {
                code.push_str(&format!("        .body(r#\"{}\"#)\n", body));
            }
        }

        code.push_str("        .send()\n");
        code.push_str("        .await\n");
        code.push_str("        .expect(\"Failed to send request\");\n\n");

        // Add assertions
        if self.config.validate_status {
            code.push_str(&format!("    assert_eq!(response.status().as_u16(), {});\n",
                response.status_code));
        }

        if self.config.validate_body && response.body.is_some() {
            code.push_str("    let body = response.text().await.expect(\"Failed to read body\");\n");
            if let Some(body) = &response.body {
                // Try to parse as JSON for better validation
                if let Ok(_json) = serde_json::from_str::<Value>(body) {
                    code.push_str(&format!("    let json: serde_json::Value = serde_json::from_str(&body).expect(\"Invalid JSON\");\n"));
                    code.push_str(&format!("    // Validate response structure\n"));
                    code.push_str(&format!("    assert!(json.is_object() || json.is_array());\n"));
                }
            }
        }

        if self.config.validate_timing {
            if let Some(max_duration) = self.config.max_duration_ms {
                code.push_str(&format!("    // Note: Add timing validation (max {} ms)\n", max_duration));
            }
        }

        code.push_str("}\n");

        Ok(code)
    }

    /// Generate HTTP file format
    fn generate_http_file(
        &self,
        request: &RecordedRequest,
        _response: &RecordedResponse,
    ) -> Result<String> {
        let base_url = self.config.base_url.as_deref().unwrap_or("http://localhost:3000");
        let mut code = String::new();

        code.push_str(&format!("### {} {}\n", request.method, request.path));
        code.push_str(&format!("{} {}{}\n", request.method, base_url, request.path));

        // Add headers
        if let Ok(headers) = serde_json::from_str::<HashMap<String, String>>(&request.headers) {
            for (key, value) in headers.iter() {
                if key.to_lowercase() != "host" {
                    code.push_str(&format!("{}: {}\n", key, value));
                }
            }
        }

        // Add body
        if let Some(body) = &request.body {
            if !body.is_empty() {
                code.push_str("\n");
                code.push_str(body);
                code.push_str("\n");
            }
        }

        code.push_str("\n");
        Ok(code)
    }

    /// Generate cURL command
    fn generate_curl(
        &self,
        request: &RecordedRequest,
        _response: &RecordedResponse,
    ) -> Result<String> {
        let base_url = self.config.base_url.as_deref().unwrap_or("http://localhost:3000");
        let url = format!("{}{}", base_url, request.path);

        let mut code = format!("curl -X {} '{}'", request.method, url);

        // Add headers
        if let Ok(headers) = serde_json::from_str::<HashMap<String, String>>(&request.headers) {
            for (key, value) in headers.iter() {
                if key.to_lowercase() != "host" {
                    code.push_str(&format!(" \\\n  -H '{}: {}'", key, value));
                }
            }
        }

        // Add body
        if let Some(body) = &request.body {
            if !body.is_empty() {
                let escaped_body = body.replace('\'', "'\\''");
                code.push_str(&format!(" \\\n  -d '{}'", escaped_body));
            }
        }

        Ok(code)
    }

    /// Generate Postman collection item
    fn generate_postman(
        &self,
        request: &RecordedRequest,
        _response: &RecordedResponse,
    ) -> Result<String> {
        let base_url = self.config.base_url.as_deref().unwrap_or("http://localhost:3000");

        let mut headers_vec = Vec::new();
        if let Ok(headers) = serde_json::from_str::<HashMap<String, String>>(&request.headers) {
            for (key, value) in headers.iter() {
                if key.to_lowercase() != "host" {
                    headers_vec.push(serde_json::json!({
                        "key": key,
                        "value": value
                    }));
                }
            }
        }

        let item = serde_json::json!({
            "name": format!("{} {}", request.method, request.path),
            "request": {
                "method": request.method,
                "header": headers_vec,
                "url": {
                    "raw": format!("{}{}", base_url, request.path),
                    "protocol": "http",
                    "host": ["localhost"],
                    "port": "3000",
                    "path": request.path.split('/').filter(|s| !s.is_empty()).collect::<Vec<_>>()
                },
                "body": if let Some(body) = &request.body {
                    serde_json::json!({
                        "mode": "raw",
                        "raw": body
                    })
                } else {
                    serde_json::json!({})
                }
            }
        });

        serde_json::to_string_pretty(&item)
            .map_err(|e| RecorderError::Serialization(e))
    }

    /// Generate k6 test script
    fn generate_k6(
        &self,
        request: &RecordedRequest,
        response: &RecordedResponse,
    ) -> Result<String> {
        let base_url = self.config.base_url.as_deref().unwrap_or("http://localhost:3000");
        let url = format!("{}{}", base_url, request.path);

        let mut code = String::new();
        code.push_str(&format!("  // {} {}\n", request.method, request.path));
        code.push_str("  {\n");

        let method = request.method.to_lowercase();

        // Build params object
        code.push_str("    const params = {\n");
        code.push_str("      headers: {\n");
        if let Ok(headers) = serde_json::from_str::<HashMap<String, String>>(&request.headers) {
            for (key, value) in headers.iter() {
                if key.to_lowercase() != "host" {
                    code.push_str(&format!("        '{}': '{}',\n", key, value));
                }
            }
        }
        code.push_str("      },\n");
        code.push_str("    };\n");

        // Make request
        if let Some(body) = &request.body {
            if !body.is_empty() {
                code.push_str(&format!("    const payload = `{}`;\n", body));
                code.push_str(&format!("    const res = http.{}('{}', payload, params);\n", method, url));
            } else {
                code.push_str(&format!("    const res = http.{}('{}', null, params);\n", method, url));
            }
        } else {
            code.push_str(&format!("    const res = http.{}('{}', null, params);\n", method, url));
        }

        // Add checks
        if self.config.validate_status {
            code.push_str(&format!("    check(res, {{\n"));
            code.push_str(&format!("      'status is {}': (r) => r.status === {},\n",
                response.status_code, response.status_code));
            code.push_str("    });\n");
        }

        code.push_str("  }\n");
        Ok(code)
    }

    /// Generate Python pytest
    fn generate_python_test(
        &self,
        request: &RecordedRequest,
        response: &RecordedResponse,
    ) -> Result<String> {
        let base_url = self.config.base_url.as_deref().unwrap_or("http://localhost:3000");
        let url = format!("{}{}", base_url, request.path);
        let test_name = self.generate_test_name(request);

        let mut code = String::new();
        code.push_str(&format!("def {}():\n", test_name));

        // Build headers
        code.push_str("    headers = {\n");
        if let Ok(headers) = serde_json::from_str::<HashMap<String, String>>(&request.headers) {
            for (key, value) in headers.iter() {
                if key.to_lowercase() != "host" {
                    code.push_str(&format!("        '{}': '{}',\n", key, value));
                }
            }
        }
        code.push_str("    }\n");

        // Make request
        let method = request.method.to_lowercase();
        if let Some(body) = &request.body {
            if !body.is_empty() {
                code.push_str(&format!("    data = r'''{}'''\n", body));
                code.push_str(&format!("    response = requests.{}('{}', headers=headers, data=data)\n",
                    method, url));
            } else {
                code.push_str(&format!("    response = requests.{}('{}', headers=headers)\n", method, url));
            }
        } else {
            code.push_str(&format!("    response = requests.{}('{}', headers=headers)\n", method, url));
        }

        // Assertions
        if self.config.validate_status {
            code.push_str(&format!("    assert response.status_code == {}\n", response.status_code));
        }

        Ok(code)
    }

    /// Generate JavaScript/Jest test
    fn generate_javascript_test(
        &self,
        request: &RecordedRequest,
        response: &RecordedResponse,
    ) -> Result<String> {
        let base_url = self.config.base_url.as_deref().unwrap_or("http://localhost:3000");
        let url = format!("{}{}", base_url, request.path);
        let test_name = format!("{} {}", request.method, request.path);

        let mut code = String::new();
        code.push_str(&format!("test('{}', async () => {{\n", test_name));

        // Build fetch options
        code.push_str("  const options = {\n");
        code.push_str(&format!("    method: '{}',\n", request.method));
        code.push_str("    headers: {\n");
        if let Ok(headers) = serde_json::from_str::<HashMap<String, String>>(&request.headers) {
            for (key, value) in headers.iter() {
                if key.to_lowercase() != "host" {
                    code.push_str(&format!("      '{}': '{}',\n", key, value));
                }
            }
        }
        code.push_str("    },\n");

        if let Some(body) = &request.body {
            if !body.is_empty() {
                code.push_str(&format!("    body: `{}`,\n", body));
            }
        }

        code.push_str("  };\n");

        code.push_str(&format!("  const response = await fetch('{}', options);\n", url));

        if self.config.validate_status {
            code.push_str(&format!("  expect(response.status).toBe({});\n", response.status_code));
        }

        if self.config.validate_body && response.body.is_some() {
            code.push_str("  const data = await response.json();\n");
            code.push_str("  expect(data).toBeDefined();\n");
        }

        code.push_str("});\n");
        Ok(code)
    }

    /// Generate Go test
    fn generate_go_test(
        &self,
        request: &RecordedRequest,
        response: &RecordedResponse,
    ) -> Result<String> {
        let base_url = self.config.base_url.as_deref().unwrap_or("http://localhost:3000");
        let url = format!("{}{}", base_url, request.path);
        let test_name = self.generate_test_name(request)
            .split('_')
            .map(|s| {
                let mut c = s.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<String>();

        let mut code = String::new();
        code.push_str(&format!("func {}(t *testing.T) {{\n", test_name));

        // Create request
        if let Some(body) = &request.body {
            if !body.is_empty() {
                code.push_str(&format!("    body := strings.NewReader(`{}`)\n", body));
                code.push_str(&format!("    req, err := http.NewRequest(\"{}\", \"{}\", body)\n",
                    request.method, url));
            } else {
                code.push_str(&format!("    req, err := http.NewRequest(\"{}\", \"{}\", nil)\n",
                    request.method, url));
            }
        } else {
            code.push_str(&format!("    req, err := http.NewRequest(\"{}\", \"{}\", nil)\n",
                request.method, url));
        }

        code.push_str("    if err != nil {\n");
        code.push_str("        t.Fatal(err)\n");
        code.push_str("    }\n");

        // Add headers
        if let Ok(headers) = serde_json::from_str::<HashMap<String, String>>(&request.headers) {
            for (key, value) in headers.iter() {
                if key.to_lowercase() != "host" {
                    code.push_str(&format!("    req.Header.Set(\"{}\", \"{}\")\n", key, value));
                }
            }
        }

        // Send request
        code.push_str("    client := &http.Client{}\n");
        code.push_str("    resp, err := client.Do(req)\n");
        code.push_str("    if err != nil {\n");
        code.push_str("        t.Fatal(err)\n");
        code.push_str("    }\n");
        code.push_str("    defer resp.Body.Close()\n\n");

        // Assertions
        if self.config.validate_status {
            code.push_str(&format!("    if resp.StatusCode != {} {{\n", response.status_code));
            code.push_str(&format!("        t.Errorf(\"Expected status {}, got %d\", resp.StatusCode)\n",
                response.status_code));
            code.push_str("    }\n");
        }

        code.push_str("}\n");
        Ok(code)
    }

    /// Generate Ruby RSpec test
    fn generate_ruby_test(
        &self,
        request: &RecordedRequest,
        response: &RecordedResponse,
    ) -> Result<String> {
        let base_url = self.config.base_url.as_deref().unwrap_or("http://localhost:3000");
        let url = format!("{}{}", base_url, request.path);
        let test_name = request.path
            .trim_start_matches('/')
            .replace('/', " ")
            .replace('-', " ")
            .replace("{", "")
            .replace("}", "");

        let mut code = String::new();
        code.push_str(&format!("  it \"should {} {}\" do\n", request.method, test_name));

        // Build request parameters
        let mut request_params = vec![format!("method: :{}", request.method.to_lowercase())];

        // Add headers
        if let Ok(headers) = serde_json::from_str::<HashMap<String, String>>(&request.headers) {
            let header_items: Vec<String> = headers.iter()
                .filter(|(k, _)| k.to_lowercase() != "host")
                .map(|(k, v)| format!("'{}' => '{}'", k, v))
                .collect();
            if !header_items.is_empty() {
                request_params.push(format!("headers: {{ {} }}", header_items.join(", ")));
            }
        }

        // Add body
        if let Some(body) = &request.body {
            if !body.is_empty() {
                let escaped_body = body.replace('\'', "\\'").replace('\n', "\\n");
                request_params.push(format!("body: '{}'", escaped_body));
            }
        }

        code.push_str(&format!("    response = HTTParty.{}('{}', {})\n",
            request.method.to_lowercase(),
            url,
            request_params.join(", ")));

        // Add assertions
        if self.config.validate_status {
            code.push_str(&format!("    expect(response.code).to eq({})\n", response.status_code));
        }

        if self.config.validate_body && response.body.is_some() {
            if let Some(body) = &response.body {
                if serde_json::from_str::<Value>(body).is_ok() {
                    code.push_str("    expect(response.parsed_response).not_to be_nil\n");
                }
            }
        }

        code.push_str("  end\n");
        Ok(code)
    }

    /// Generate Java JUnit test
    fn generate_java_test(
        &self,
        request: &RecordedRequest,
        response: &RecordedResponse,
    ) -> Result<String> {
        let base_url = self.config.base_url.as_deref().unwrap_or("http://localhost:3000");
        let url = format!("{}{}", base_url, request.path);
        let test_name = self.generate_test_name(request)
            .split('_')
            .enumerate()
            .map(|(i, s)| {
                if i == 0 { s.to_string() } else {
                    let mut c = s.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    }
                }
            })
            .collect::<String>();

        let mut code = String::new();
        code.push_str("    @Test\n");
        code.push_str(&format!("    public void {}() throws Exception {{\n", test_name));

        // Create request
        code.push_str(&format!("        HttpRequest request = HttpRequest.newBuilder()\n"));
        code.push_str(&format!("            .uri(URI.create(\"{}\"))\n", url));
        code.push_str(&format!("            .method(\"{}\", ", request.method));

        if let Some(body) = &request.body {
            if !body.is_empty() {
                let escaped_body = body.replace('"', "\\\"").replace('\n', "\\n");
                code.push_str(&format!("HttpRequest.BodyPublishers.ofString(\"{}\"))\n", escaped_body));
            } else {
                code.push_str("HttpRequest.BodyPublishers.noBody())\n");
            }
        } else {
            code.push_str("HttpRequest.BodyPublishers.noBody())\n");
        }

        // Add headers
        if let Ok(headers) = serde_json::from_str::<HashMap<String, String>>(&request.headers) {
            for (key, value) in headers.iter() {
                if key.to_lowercase() != "host" {
                    code.push_str(&format!("            .header(\"{}\", \"{}\")\n", key, value));
                }
            }
        }

        code.push_str("            .build();\n\n");

        // Send request
        code.push_str("        HttpClient client = HttpClient.newHttpClient();\n");
        code.push_str("        HttpResponse<String> response = client.send(request, HttpResponse.BodyHandlers.ofString());\n\n");

        // Assertions
        if self.config.validate_status {
            code.push_str(&format!("        assertEquals({}, response.statusCode());\n", response.status_code));
        }

        if self.config.validate_body && response.body.is_some() {
            if let Some(body) = &response.body {
                if serde_json::from_str::<Value>(body).is_ok() {
                    code.push_str("        assertNotNull(response.body());\n");
                }
            }
        }

        code.push_str("    }\n");
        Ok(code)
    }

    /// Generate C# xUnit test
    fn generate_csharp_test(
        &self,
        request: &RecordedRequest,
        response: &RecordedResponse,
    ) -> Result<String> {
        let base_url = self.config.base_url.as_deref().unwrap_or("http://localhost:3000");
        let url = format!("{}{}", base_url, request.path);
        let test_name = self.generate_test_name(request)
            .split('_')
            .map(|s| {
                let mut c = s.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<String>();

        let mut code = String::new();
        code.push_str("        [Fact]\n");
        code.push_str(&format!("        public async Task {}Async()\n", test_name));
        code.push_str("        {\n");

        // Create client and request
        code.push_str("            using var client = new HttpClient();\n");

        // Create request message
        code.push_str(&format!("            var request = new HttpRequestMessage(HttpMethod.{}, \"{}\");\n",
            request.method.chars().next().unwrap().to_uppercase().collect::<String>() + &request.method[1..].to_lowercase(),
            url));

        // Add headers
        if let Ok(headers) = serde_json::from_str::<HashMap<String, String>>(&request.headers) {
            for (key, value) in headers.iter() {
                if key.to_lowercase() != "host" && key.to_lowercase() != "content-type" {
                    code.push_str(&format!("            request.Headers.Add(\"{}\", \"{}\");\n", key, value));
                }
            }
        }

        // Add body
        if let Some(body) = &request.body {
            if !body.is_empty() {
                let escaped_body = body.replace('"', "\\\"").replace('\n', "\\n");
                code.push_str(&format!("            request.Content = new StringContent(\"{}\", Encoding.UTF8, \"application/json\");\n",
                    escaped_body));
            }
        }

        // Send request
        code.push_str("            var response = await client.SendAsync(request);\n\n");

        // Assertions
        if self.config.validate_status {
            code.push_str(&format!("            Assert.Equal({}, (int)response.StatusCode);\n", response.status_code));
        }

        if self.config.validate_body && response.body.is_some() {
            code.push_str("            var content = await response.Content.ReadAsStringAsync();\n");
            code.push_str("            Assert.NotNull(content);\n");
            code.push_str("            Assert.NotEmpty(content);\n");
        }

        code.push_str("        }\n");
        Ok(code)
    }

    /// Generate complete test file with imports and structure
    fn generate_test_file(&self, tests: &[GeneratedTest]) -> Result<String> {
        let mut file = String::new();

        match self.config.format {
            TestFormat::RustReqwest => {
                file.push_str("// Generated test file\n");
                file.push_str("// Run with: cargo test\n\n");
                if self.config.include_setup_teardown {
                    file.push_str("use reqwest;\n");
                    file.push_str("use serde_json::Value;\n\n");
                }

                for test in tests {
                    file.push_str(&test.code);
                    file.push_str("\n");
                }
            }
            TestFormat::HttpFile => {
                for test in tests {
                    file.push_str(&test.code);
                    file.push_str("\n");
                }
            }
            TestFormat::Curl => {
                file.push_str("#!/bin/bash\n");
                file.push_str("# Generated cURL commands\n\n");
                for test in tests {
                    file.push_str(&format!("# {} {}\n", test.method, test.endpoint));
                    file.push_str(&test.code);
                    file.push_str("\n\n");
                }
            }
            TestFormat::Postman => {
                let collection = serde_json::json!({
                    "info": {
                        "name": self.config.suite_name,
                        "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
                    },
                    "item": tests.iter().map(|t| {
                        serde_json::from_str::<Value>(&t.code).unwrap_or(Value::Null)
                    }).collect::<Vec<_>>()
                });
                file = serde_json::to_string_pretty(&collection)
                    .map_err(|e| RecorderError::Serialization(e))?;
            }
            TestFormat::K6 => {
                file.push_str("import http from 'k6/http';\n");
                file.push_str("import { check, sleep } from 'k6';\n\n");
                file.push_str("export const options = {\n");
                file.push_str("  vus: 10,\n");
                file.push_str("  duration: '30s',\n");
                file.push_str("};\n\n");
                file.push_str("export default function() {\n");
                for test in tests {
                    file.push_str(&test.code);
                }
                file.push_str("  sleep(1);\n");
                file.push_str("}\n");
            }
            TestFormat::PythonPytest => {
                file.push_str("# Generated test file\n");
                file.push_str("# Run with: pytest\n\n");
                file.push_str("import requests\n");
                file.push_str("import pytest\n\n");
                for test in tests {
                    file.push_str(&test.code);
                    file.push_str("\n");
                }
            }
            TestFormat::JavaScriptJest => {
                file.push_str("// Generated test file\n");
                file.push_str("// Run with: npm test\n\n");
                file.push_str(&format!("describe('{}', () => {{\n", self.config.suite_name));
                for test in tests {
                    file.push_str("  ");
                    file.push_str(&test.code.replace("\n", "\n  "));
                    file.push_str("\n");
                }
                file.push_str("});\n");
            }
            TestFormat::GoTest => {
                file.push_str("package main\n\n");
                file.push_str("import (\n");
                file.push_str("    \"net/http\"\n");
                file.push_str("    \"strings\"\n");
                file.push_str("    \"testing\"\n");
                file.push_str(")\n\n");
                for test in tests {
                    file.push_str(&test.code);
                    file.push_str("\n");
                }
            }
            TestFormat::RubyRspec => {
                file.push_str("# Generated test file\n");
                file.push_str("# Run with: rspec spec/api_spec.rb\n\n");
                file.push_str("require 'httparty'\n");
                file.push_str("require 'rspec'\n\n");
                file.push_str(&format!("RSpec.describe '{}' do\n", self.config.suite_name));
                for test in tests {
                    file.push_str(&test.code);
                    file.push_str("\n");
                }
                file.push_str("end\n");
            }
            TestFormat::JavaJunit => {
                file.push_str("// Generated test file\n");
                file.push_str("// Run with: mvn test or gradle test\n\n");
                file.push_str("import org.junit.jupiter.api.Test;\n");
                file.push_str("import static org.junit.jupiter.api.Assertions.*;\n");
                file.push_str("import java.net.URI;\n");
                file.push_str("import java.net.http.HttpClient;\n");
                file.push_str("import java.net.http.HttpRequest;\n");
                file.push_str("import java.net.http.HttpResponse;\n\n");
                file.push_str(&format!("public class {} {{\n", self.config.suite_name.replace("-", "_")));
                for test in tests {
                    file.push_str(&test.code);
                    file.push_str("\n");
                }
                file.push_str("}\n");
            }
            TestFormat::CSharpXunit => {
                file.push_str("// Generated test file\n");
                file.push_str("// Run with: dotnet test\n\n");
                file.push_str("using System;\n");
                file.push_str("using System.Net.Http;\n");
                file.push_str("using System.Text;\n");
                file.push_str("using System.Threading.Tasks;\n");
                file.push_str("using Xunit;\n\n");
                file.push_str(&format!("namespace {}\n", self.config.suite_name.replace("-", "_")));
                file.push_str("{\n");
                file.push_str("    public class ApiTests\n");
                file.push_str("    {\n");
                for test in tests {
                    file.push_str(&test.code);
                    file.push_str("\n");
                }
                file.push_str("    }\n");
                file.push_str("}\n");
            }
        }

        Ok(file)
    }

    /// Deduplicate similar tests
    fn deduplicate_tests(&self, tests: Vec<GeneratedTest>) -> Vec<GeneratedTest> {
        let mut unique_tests = Vec::new();
        let mut seen_signatures = std::collections::HashSet::new();

        for test in tests {
            // Create a signature based on method + endpoint + code structure
            let signature = format!("{}:{}:{}", test.method, test.endpoint, test.code.len());

            if !seen_signatures.contains(&signature) {
                seen_signatures.insert(signature);
                unique_tests.push(test);
            }
        }

        unique_tests
    }

    /// Optimize test execution order
    fn optimize_test_order(&self, mut tests: Vec<GeneratedTest>) -> Vec<GeneratedTest> {
        // Sort tests by:
        // 1. GET requests first (read-only, fast)
        // 2. POST/PUT requests (may modify state)
        // 3. DELETE requests last
        tests.sort_by(|a, b| {
            let order_a = match a.method.as_str() {
                "GET" | "HEAD" => 0,
                "POST" | "PUT" | "PATCH" => 1,
                "DELETE" => 2,
                _ => 3,
            };
            let order_b = match b.method.as_str() {
                "GET" | "HEAD" => 0,
                "POST" | "PUT" | "PATCH" => 1,
                "DELETE" => 2,
                _ => 3,
            };
            order_a.cmp(&order_b).then_with(|| a.endpoint.cmp(&b.endpoint))
        });

        tests
    }

    /// Generate test data fixtures using AI
    async fn generate_test_fixtures(
        &self,
        exchanges: &[crate::models::RecordedExchange],
    ) -> Result<Vec<TestFixture>> {
        if self.config.llm_config.is_none() {
            return Ok(Vec::new());
        }

        let llm_config = self.config.llm_config.as_ref().unwrap();
        let mut fixtures = Vec::new();

        // Group exchanges by endpoint
        let mut endpoint_data: HashMap<String, Vec<&crate::models::RecordedExchange>> = HashMap::new();
        for exchange in exchanges {
            let endpoint = format!("{} {}", exchange.request.method, exchange.request.path);
            endpoint_data.entry(endpoint).or_insert_with(Vec::new).push(exchange);
        }

        // Generate fixtures for each endpoint
        for (endpoint, endpoint_exchanges) in endpoint_data.iter().take(5) {
            // Collect sample request bodies
            let mut sample_bodies = Vec::new();
            for exchange in endpoint_exchanges.iter().take(3) {
                if let Some(body) = &exchange.request.body {
                    if !body.is_empty() {
                        if let Ok(json) = serde_json::from_str::<Value>(body) {
                            sample_bodies.push(json);
                        }
                    }
                }
            }

            if sample_bodies.is_empty() {
                continue;
            }

            let prompt = format!(
                "Based on these sample API request bodies for endpoint '{}', generate a reusable test fixture in JSON format:\n{}\n\nProvide a clean JSON object with varied test data including edge cases.",
                endpoint,
                serde_json::to_string_pretty(&sample_bodies).unwrap_or_default()
            );

            if let Ok(response) = self.call_llm(llm_config, &prompt).await {
                // Try to parse the LLM response as JSON
                if let Ok(data) = serde_json::from_str::<Value>(&response) {
                    fixtures.push(TestFixture {
                        name: format!("fixture_{}", endpoint.replace(' ', "_").replace('/', "_")),
                        description: format!("Test fixture for {}", endpoint),
                        data,
                        endpoints: vec![endpoint.clone()],
                    });
                }
            }
        }

        Ok(fixtures)
    }

    /// Suggest edge cases using AI
    async fn suggest_edge_cases(
        &self,
        exchanges: &[crate::models::RecordedExchange],
    ) -> Result<Vec<EdgeCaseSuggestion>> {
        if self.config.llm_config.is_none() {
            return Ok(Vec::new());
        }

        let llm_config = self.config.llm_config.as_ref().unwrap();
        let mut edge_cases = Vec::new();

        // Group by endpoint
        let mut endpoint_data: HashMap<String, Vec<&crate::models::RecordedExchange>> = HashMap::new();
        for exchange in exchanges {
            let key = format!("{} {}", exchange.request.method, exchange.request.path);
            endpoint_data.entry(key).or_insert_with(Vec::new).push(exchange);
        }

        for (endpoint_key, endpoint_exchanges) in endpoint_data.iter().take(5) {
            let parts: Vec<&str> = endpoint_key.splitn(2, ' ').collect();
            if parts.len() != 2 {
                continue;
            }
            let (method, endpoint) = (parts[0], parts[1]);

            // Collect sample data
            let sample_exchange = endpoint_exchanges.first();
            let sample_body = sample_exchange
                .and_then(|e| e.request.body.as_ref())
                .map(|s| s.as_str())
                .unwrap_or("{}");

            let prompt = format!(
                "Suggest 3 critical edge cases to test for this API endpoint:\n\
                Method: {}\n\
                Path: {}\n\
                Sample Request: {}\n\n\
                For each edge case, provide:\n\
                1. Type (e.g., 'validation', 'boundary', 'security')\n\
                2. Description\n\
                3. Expected behavior\n\
                4. Priority (1-5)\n\n\
                Format: type|description|behavior|priority",
                method, endpoint, sample_body
            );

            if let Ok(response) = self.call_llm(llm_config, &prompt).await {
                // Parse LLM response
                for line in response.lines().take(3) {
                    let parts: Vec<&str> = line.split('|').collect();
                    if parts.len() >= 4 {
                        let priority = parts[3].trim().parse::<u8>().unwrap_or(3);
                        edge_cases.push(EdgeCaseSuggestion {
                            endpoint: endpoint.to_string(),
                            method: method.to_string(),
                            case_type: parts[0].trim().to_string(),
                            description: parts[1].trim().to_string(),
                            suggested_input: None,
                            expected_behavior: parts[2].trim().to_string(),
                            priority,
                        });
                    }
                }
            }
        }

        Ok(edge_cases)
    }

    /// Analyze test gaps
    async fn analyze_test_gaps(
        &self,
        exchanges: &[crate::models::RecordedExchange],
        tests: &[GeneratedTest],
    ) -> Result<TestGapAnalysis> {
        // Collect all unique endpoints from exchanges
        let mut all_endpoints = std::collections::HashSet::new();
        let mut method_by_endpoint: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
        let mut status_codes_by_endpoint: HashMap<String, std::collections::HashSet<u16>> = HashMap::new();

        for exchange in exchanges {
            let endpoint = exchange.request.path.clone();
            let method = exchange.request.method.clone();
            all_endpoints.insert(endpoint.clone());
            method_by_endpoint
                .entry(endpoint.clone())
                .or_insert_with(std::collections::HashSet::new)
                .insert(method);

            if let Some(response) = &exchange.response {
                let status_code = response.status_code as u16;
                status_codes_by_endpoint
                    .entry(endpoint.clone())
                    .or_insert_with(std::collections::HashSet::new)
                    .insert(status_code);
            }
        }

        // Collect tested endpoints from generated tests
        let mut tested_endpoints = std::collections::HashSet::new();
        for test in tests {
            tested_endpoints.insert(test.endpoint.clone());
        }

        // Find gaps
        let untested_endpoints: Vec<String> = all_endpoints
            .difference(&tested_endpoints)
            .cloned()
            .collect();

        let mut missing_methods: HashMap<String, Vec<String>> = HashMap::new();
        for (endpoint, methods) in &method_by_endpoint {
            let tested_methods: std::collections::HashSet<String> = tests
                .iter()
                .filter(|t| &t.endpoint == endpoint)
                .map(|t| t.method.clone())
                .collect();

            let missing: Vec<String> = methods
                .difference(&tested_methods)
                .cloned()
                .collect();

            if !missing.is_empty() {
                missing_methods.insert(endpoint.clone(), missing);
            }
        }

        let mut missing_status_codes: HashMap<String, Vec<u16>> = HashMap::new();
        for (endpoint, codes) in &status_codes_by_endpoint {
            // For now, we'll just note if error codes (4xx, 5xx) are missing
            let has_error_tests = codes.iter().any(|c| *c >= 400);
            if has_error_tests {
                missing_status_codes.insert(
                    endpoint.clone(),
                    codes.iter().filter(|c| **c >= 400).copied().collect(),
                );
            }
        }

        let missing_error_scenarios = vec![
            "401 Unauthorized scenarios".to_string(),
            "403 Forbidden scenarios".to_string(),
            "404 Not Found scenarios".to_string(),
            "429 Rate Limiting scenarios".to_string(),
            "500 Internal Server Error scenarios".to_string(),
        ];

        let coverage_percentage = if all_endpoints.is_empty() {
            100.0
        } else {
            (tested_endpoints.len() as f64 / all_endpoints.len() as f64) * 100.0
        };

        let mut recommendations = Vec::new();
        if !untested_endpoints.is_empty() {
            recommendations.push(format!(
                "Add tests for {} untested endpoints",
                untested_endpoints.len()
            ));
        }
        if !missing_methods.is_empty() {
            recommendations.push(format!(
                "Add tests for missing HTTP methods on {} endpoints",
                missing_methods.len()
            ));
        }
        if coverage_percentage < 80.0 {
            recommendations.push("Increase test coverage to at least 80%".to_string());
        }

        Ok(TestGapAnalysis {
            untested_endpoints,
            missing_methods,
            missing_status_codes,
            missing_error_scenarios,
            coverage_percentage,
            recommendations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{RecordedRequest, RecordedResponse, Protocol};

    #[tokio::test]
    async fn test_generate_test_name() {
        let database = RecorderDatabase::new_in_memory().await.unwrap();
        let config = TestGenerationConfig::default();
        let generator = TestGenerator::new(database, config);

        let request = RecordedRequest {
            id: "test".to_string(),
            protocol: Protocol::Http,
            timestamp: chrono::Utc::now(),
            method: "GET".to_string(),
            path: "/api/users/123".to_string(),
            headers: "{}".to_string(),
            body: None,
            status_code: Some(200),
            duration_ms: Some(50),
            client_ip: None,
            trace_id: None,
            span_id: None,
        };

        let name = generator.generate_test_name(&request);
        assert_eq!(name, "test_get_api_users_123");
    }

    #[test]
    fn test_default_config() {
        let config = TestGenerationConfig::default();
        assert_eq!(config.format, TestFormat::RustReqwest);
        assert!(config.include_assertions);
        assert!(config.validate_body);
        assert!(config.validate_status);
    }

    #[tokio::test]
    async fn test_generate_rust_test() {
        let database = RecorderDatabase::new_in_memory().await.unwrap();
        let config = TestGenerationConfig::default();
        let generator = TestGenerator::new(database, config);

        let request = RecordedRequest {
            id: "test-1".to_string(),
            protocol: Protocol::Http,
            timestamp: chrono::Utc::now(),
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            headers: r#"{"content-type":"application/json"}"#.to_string(),
            body: None,
            status_code: Some(200),
            duration_ms: Some(45),
            client_ip: Some("127.0.0.1".to_string()),
            trace_id: None,
            span_id: None,
        };

        let response = RecordedResponse {
            request_id: "test-1".to_string(),
            status_code: 200,
            headers: r#"{"content-type":"application/json"}"#.to_string(),
            body: Some(r#"{"users":[]}"#.to_string()),
            body_encoding: Some("utf-8".to_string()),
            size_bytes: Some(12),
            timestamp: chrono::Utc::now(),
        };

        let code = generator.generate_rust_test(&request, &response).unwrap();

        assert!(code.contains("#[tokio::test]"));
        assert!(code.contains("async fn test_get_api_users()"));
        assert!(code.contains("reqwest::Client::new()"));
        assert!(code.contains("assert_eq!(response.status().as_u16(), 200)"));
    }

    #[tokio::test]
    async fn test_generate_curl() {
        let database = RecorderDatabase::new_in_memory().await.unwrap();
        let config = TestGenerationConfig::default();
        let generator = TestGenerator::new(database, config);

        let request = RecordedRequest {
            id: "test-2".to_string(),
            protocol: Protocol::Http,
            timestamp: chrono::Utc::now(),
            method: "POST".to_string(),
            path: "/api/users".to_string(),
            headers: r#"{"content-type":"application/json"}"#.to_string(),
            body: Some(r#"{"name":"John"}"#.to_string()),
            status_code: Some(201),
            duration_ms: Some(80),
            client_ip: None,
            trace_id: None,
            span_id: None,
        };

        let response = RecordedResponse {
            request_id: "test-2".to_string(),
            status_code: 201,
            headers: r#"{}"#.to_string(),
            body: None,
            body_encoding: None,
            size_bytes: Some(0),
            timestamp: chrono::Utc::now(),
        };

        let code = generator.generate_curl(&request, &response).unwrap();

        assert!(code.contains("curl -X POST"));
        assert!(code.contains("/api/users"));
        assert!(code.contains("-H 'content-type: application/json'"));
        assert!(code.contains(r#"-d '{"name":"John"}'"#));
    }

    #[tokio::test]
    async fn test_generate_http_file() {
        let database = RecorderDatabase::new_in_memory().await.unwrap();
        let config = TestGenerationConfig::default();
        let generator = TestGenerator::new(database, config);

        let request = RecordedRequest {
            id: "test-3".to_string(),
            protocol: Protocol::Http,
            timestamp: chrono::Utc::now(),
            method: "DELETE".to_string(),
            path: "/api/users/123".to_string(),
            headers: r#"{}"#.to_string(),
            body: None,
            status_code: Some(204),
            duration_ms: Some(30),
            client_ip: None,
            trace_id: None,
            span_id: None,
        };

        let response = RecordedResponse {
            request_id: "test-3".to_string(),
            status_code: 204,
            headers: r#"{}"#.to_string(),
            body: None,
            body_encoding: None,
            size_bytes: Some(0),
            timestamp: chrono::Utc::now(),
        };

        let code = generator.generate_http_file(&request, &response).unwrap();

        assert!(code.contains("### DELETE /api/users/123"));
        assert!(code.contains("DELETE http://localhost:3000/api/users/123"));
    }

    #[test]
    fn test_llm_config_defaults() {
        let config = LlmConfig::default();
        assert_eq!(config.provider, "ollama");
        assert_eq!(config.model, "llama2");
        assert_eq!(config.temperature, 0.3);
    }

    #[test]
    fn test_test_format_variants() {
        assert_eq!(TestFormat::RustReqwest, TestFormat::RustReqwest);
        assert_ne!(TestFormat::RustReqwest, TestFormat::Curl);
        assert_ne!(TestFormat::PythonPytest, TestFormat::JavaScriptJest);
    }
}
