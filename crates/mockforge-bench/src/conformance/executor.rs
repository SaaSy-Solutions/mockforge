//! Native Rust conformance test executor
//!
//! Replaces the k6-based execution path with direct HTTP requests via reqwest,
//! producing the same `ConformanceReport` output. This removes the k6 binary
//! dependency and enables API/SDK integration.

use super::custom::{CustomCheck, CustomConformanceConfig};
use super::generator::ConformanceConfig;
use super::report::{ConformanceReport, FailureDetail, FailureRequest, FailureResponse};
use super::spec::ConformanceFeature;
use super::spec_driven::{AnnotatedOperation, ApiKeyLocation, SecuritySchemeInfo};
use crate::error::{BenchError, Result};
use reqwest::{Client, Method};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::sync::mpsc;

/// A field-level schema validation violation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SchemaViolation {
    /// JSON path to the field that failed validation (e.g., "/name", "/items/0/age")
    pub field_path: String,
    /// Type of violation (e.g., "type", "required", "additionalProperties")
    pub violation_type: String,
    /// What the schema expected
    pub expected: String,
    /// What was actually found
    pub actual: String,
}

/// A single conformance check to execute
#[derive(Debug, Clone)]
pub struct ConformanceCheck {
    /// Check name (e.g., "param:path:string" or "param:path:string:/users/{id}")
    pub name: String,
    /// HTTP method
    pub method: Method,
    /// Relative path (appended to base_url)
    pub path: String,
    /// Request headers
    pub headers: Vec<(String, String)>,
    /// Optional request body
    pub body: Option<CheckBody>,
    /// How to validate the response
    pub validation: CheckValidation,
}

/// Request body variants
#[derive(Debug, Clone)]
pub enum CheckBody {
    /// JSON body
    Json(serde_json::Value),
    /// Form-urlencoded body
    FormUrlencoded(Vec<(String, String)>),
    /// Raw string body with content type
    Raw {
        content: String,
        content_type: String,
    },
}

/// How to validate a conformance check response
#[derive(Debug, Clone)]
pub enum CheckValidation {
    /// status >= min && status < max_exclusive
    StatusRange { min: u16, max_exclusive: u16 },
    /// status === code
    ExactStatus(u16),
    /// Schema validation: status in range + JSON body matches schema
    SchemaValidation {
        status_min: u16,
        status_max: u16,
        schema: serde_json::Value,
    },
    /// Custom: exact status + optional header regex + optional body field type checks
    Custom {
        expected_status: u16,
        expected_headers: Vec<(String, String)>,
        expected_body_fields: Vec<(String, String)>,
    },
}

/// Progress event for SSE streaming
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum ConformanceProgress {
    /// Test run started
    #[serde(rename = "started")]
    Started { total_checks: usize },
    /// A single check completed
    #[serde(rename = "check_completed")]
    CheckCompleted {
        name: String,
        passed: bool,
        checks_done: usize,
    },
    /// All checks finished
    #[serde(rename = "finished")]
    Finished,
    /// An error occurred
    #[serde(rename = "error")]
    Error { message: String },
}

/// Result of executing a single conformance check
#[derive(Debug)]
struct CheckResult {
    name: String,
    passed: bool,
    failure_detail: Option<FailureDetail>,
}

/// Native conformance executor using reqwest
pub struct NativeConformanceExecutor {
    config: ConformanceConfig,
    client: Client,
    checks: Vec<ConformanceCheck>,
}

impl NativeConformanceExecutor {
    /// Create a new executor from a `ConformanceConfig`
    pub fn new(config: ConformanceConfig) -> Result<Self> {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10));

        if config.skip_tls_verify {
            builder = builder.danger_accept_invalid_certs(true);
        }

        let client = builder
            .build()
            .map_err(|e| BenchError::Other(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Self {
            config,
            client,
            checks: Vec::new(),
        })
    }

    /// Populate checks from hardcoded reference endpoints (`/conformance/*`).
    /// Used when no `--spec` is provided.
    #[must_use]
    pub fn with_reference_checks(mut self) -> Self {
        // --- Parameters ---
        if self.config.should_include_category("Parameters") {
            self.add_ref_get("param:path:string", "/conformance/params/hello");
            self.add_ref_get("param:path:integer", "/conformance/params/42");
            self.add_ref_get("param:query:string", "/conformance/params/query?name=test");
            self.add_ref_get("param:query:integer", "/conformance/params/query?count=10");
            self.add_ref_get("param:query:array", "/conformance/params/query?tags=a&tags=b");
            self.checks.push(ConformanceCheck {
                name: "param:header".to_string(),
                method: Method::GET,
                path: "/conformance/params/header".to_string(),
                headers: self
                    .merge_headers(vec![("X-Custom-Param".to_string(), "test-value".to_string())]),
                body: None,
                validation: CheckValidation::StatusRange {
                    min: 200,
                    max_exclusive: 500,
                },
            });
            self.checks.push(ConformanceCheck {
                name: "param:cookie".to_string(),
                method: Method::GET,
                path: "/conformance/params/cookie".to_string(),
                headers: self
                    .merge_headers(vec![("Cookie".to_string(), "session=abc123".to_string())]),
                body: None,
                validation: CheckValidation::StatusRange {
                    min: 200,
                    max_exclusive: 500,
                },
            });
        }

        // --- Request Bodies ---
        if self.config.should_include_category("Request Bodies") {
            self.checks.push(ConformanceCheck {
                name: "body:json".to_string(),
                method: Method::POST,
                path: "/conformance/body/json".to_string(),
                headers: self.merge_headers(vec![(
                    "Content-Type".to_string(),
                    "application/json".to_string(),
                )]),
                body: Some(CheckBody::Json(serde_json::json!({"name": "test", "value": 42}))),
                validation: CheckValidation::StatusRange {
                    min: 200,
                    max_exclusive: 500,
                },
            });
            self.checks.push(ConformanceCheck {
                name: "body:form-urlencoded".to_string(),
                method: Method::POST,
                path: "/conformance/body/form".to_string(),
                headers: self.custom_headers_only(),
                body: Some(CheckBody::FormUrlencoded(vec![
                    ("field1".to_string(), "value1".to_string()),
                    ("field2".to_string(), "value2".to_string()),
                ])),
                validation: CheckValidation::StatusRange {
                    min: 200,
                    max_exclusive: 500,
                },
            });
            self.checks.push(ConformanceCheck {
                name: "body:multipart".to_string(),
                method: Method::POST,
                path: "/conformance/body/multipart".to_string(),
                headers: self.custom_headers_only(),
                body: Some(CheckBody::Raw {
                    content: "test content".to_string(),
                    content_type: "text/plain".to_string(),
                }),
                validation: CheckValidation::StatusRange {
                    min: 200,
                    max_exclusive: 500,
                },
            });
        }

        // --- Schema Types ---
        if self.config.should_include_category("Schema Types") {
            let types = [
                ("string", r#"{"value": "hello"}"#, "schema:string"),
                ("integer", r#"{"value": 42}"#, "schema:integer"),
                ("number", r#"{"value": 3.14}"#, "schema:number"),
                ("boolean", r#"{"value": true}"#, "schema:boolean"),
                ("array", r#"{"value": [1, 2, 3]}"#, "schema:array"),
                ("object", r#"{"value": {"nested": "data"}}"#, "schema:object"),
            ];
            for (type_name, body_str, check_name) in types {
                self.checks.push(ConformanceCheck {
                    name: check_name.to_string(),
                    method: Method::POST,
                    path: format!("/conformance/schema/{}", type_name),
                    headers: self.merge_headers(vec![(
                        "Content-Type".to_string(),
                        "application/json".to_string(),
                    )]),
                    body: Some(CheckBody::Json(
                        serde_json::from_str(body_str).expect("valid JSON"),
                    )),
                    validation: CheckValidation::StatusRange {
                        min: 200,
                        max_exclusive: 500,
                    },
                });
            }
        }

        // --- Composition ---
        if self.config.should_include_category("Composition") {
            let compositions = [
                ("oneOf", r#"{"type": "string", "value": "test"}"#, "composition:oneOf"),
                ("anyOf", r#"{"value": "test"}"#, "composition:anyOf"),
                ("allOf", r#"{"name": "test", "id": 1}"#, "composition:allOf"),
            ];
            for (kind, body_str, check_name) in compositions {
                self.checks.push(ConformanceCheck {
                    name: check_name.to_string(),
                    method: Method::POST,
                    path: format!("/conformance/composition/{}", kind),
                    headers: self.merge_headers(vec![(
                        "Content-Type".to_string(),
                        "application/json".to_string(),
                    )]),
                    body: Some(CheckBody::Json(
                        serde_json::from_str(body_str).expect("valid JSON"),
                    )),
                    validation: CheckValidation::StatusRange {
                        min: 200,
                        max_exclusive: 500,
                    },
                });
            }
        }

        // --- String Formats ---
        if self.config.should_include_category("String Formats") {
            let formats = [
                ("date", r#"{"value": "2024-01-15"}"#, "format:date"),
                ("date-time", r#"{"value": "2024-01-15T10:30:00Z"}"#, "format:date-time"),
                ("email", r#"{"value": "test@example.com"}"#, "format:email"),
                ("uuid", r#"{"value": "550e8400-e29b-41d4-a716-446655440000"}"#, "format:uuid"),
                ("uri", r#"{"value": "https://example.com/path"}"#, "format:uri"),
                ("ipv4", r#"{"value": "192.168.1.1"}"#, "format:ipv4"),
                ("ipv6", r#"{"value": "::1"}"#, "format:ipv6"),
            ];
            for (fmt, body_str, check_name) in formats {
                self.checks.push(ConformanceCheck {
                    name: check_name.to_string(),
                    method: Method::POST,
                    path: format!("/conformance/formats/{}", fmt),
                    headers: self.merge_headers(vec![(
                        "Content-Type".to_string(),
                        "application/json".to_string(),
                    )]),
                    body: Some(CheckBody::Json(
                        serde_json::from_str(body_str).expect("valid JSON"),
                    )),
                    validation: CheckValidation::StatusRange {
                        min: 200,
                        max_exclusive: 500,
                    },
                });
            }
        }

        // --- Constraints ---
        if self.config.should_include_category("Constraints") {
            let constraints = [
                ("required", r#"{"required_field": "present"}"#, "constraint:required"),
                ("optional", r#"{}"#, "constraint:optional"),
                ("minmax", r#"{"value": 50}"#, "constraint:minmax"),
                ("pattern", r#"{"value": "ABC-123"}"#, "constraint:pattern"),
                ("enum", r#"{"status": "active"}"#, "constraint:enum"),
            ];
            for (kind, body_str, check_name) in constraints {
                self.checks.push(ConformanceCheck {
                    name: check_name.to_string(),
                    method: Method::POST,
                    path: format!("/conformance/constraints/{}", kind),
                    headers: self.merge_headers(vec![(
                        "Content-Type".to_string(),
                        "application/json".to_string(),
                    )]),
                    body: Some(CheckBody::Json(
                        serde_json::from_str(body_str).expect("valid JSON"),
                    )),
                    validation: CheckValidation::StatusRange {
                        min: 200,
                        max_exclusive: 500,
                    },
                });
            }
        }

        // --- Response Codes ---
        if self.config.should_include_category("Response Codes") {
            for (code_str, check_name) in [
                ("200", "response:200"),
                ("201", "response:201"),
                ("204", "response:204"),
                ("400", "response:400"),
                ("404", "response:404"),
            ] {
                let code: u16 = code_str.parse().unwrap();
                self.checks.push(ConformanceCheck {
                    name: check_name.to_string(),
                    method: Method::GET,
                    path: format!("/conformance/responses/{}", code_str),
                    headers: self.custom_headers_only(),
                    body: None,
                    validation: CheckValidation::ExactStatus(code),
                });
            }
        }

        // --- HTTP Methods ---
        if self.config.should_include_category("HTTP Methods") {
            self.add_ref_get("method:GET", "/conformance/methods");
            for (method, check_name) in [
                (Method::POST, "method:POST"),
                (Method::PUT, "method:PUT"),
                (Method::PATCH, "method:PATCH"),
            ] {
                self.checks.push(ConformanceCheck {
                    name: check_name.to_string(),
                    method,
                    path: "/conformance/methods".to_string(),
                    headers: self.merge_headers(vec![(
                        "Content-Type".to_string(),
                        "application/json".to_string(),
                    )]),
                    body: Some(CheckBody::Json(serde_json::json!({"action": "test"}))),
                    validation: CheckValidation::StatusRange {
                        min: 200,
                        max_exclusive: 500,
                    },
                });
            }
            for (method, check_name) in [
                (Method::DELETE, "method:DELETE"),
                (Method::HEAD, "method:HEAD"),
                (Method::OPTIONS, "method:OPTIONS"),
            ] {
                self.checks.push(ConformanceCheck {
                    name: check_name.to_string(),
                    method,
                    path: "/conformance/methods".to_string(),
                    headers: self.custom_headers_only(),
                    body: None,
                    validation: CheckValidation::StatusRange {
                        min: 200,
                        max_exclusive: 500,
                    },
                });
            }
        }

        // --- Content Types ---
        if self.config.should_include_category("Content Types") {
            self.checks.push(ConformanceCheck {
                name: "content:negotiation".to_string(),
                method: Method::GET,
                path: "/conformance/content-types".to_string(),
                headers: self
                    .merge_headers(vec![("Accept".to_string(), "application/json".to_string())]),
                body: None,
                validation: CheckValidation::StatusRange {
                    min: 200,
                    max_exclusive: 500,
                },
            });
        }

        // --- Security ---
        if self.config.should_include_category("Security") {
            // Bearer
            self.checks.push(ConformanceCheck {
                name: "security:bearer".to_string(),
                method: Method::GET,
                path: "/conformance/security/bearer".to_string(),
                headers: self.merge_headers(vec![(
                    "Authorization".to_string(),
                    "Bearer test-token-123".to_string(),
                )]),
                body: None,
                validation: CheckValidation::StatusRange {
                    min: 200,
                    max_exclusive: 500,
                },
            });

            // API Key
            let api_key = self.config.api_key.as_deref().unwrap_or("test-api-key-123");
            self.checks.push(ConformanceCheck {
                name: "security:apikey".to_string(),
                method: Method::GET,
                path: "/conformance/security/apikey".to_string(),
                headers: self.merge_headers(vec![("X-API-Key".to_string(), api_key.to_string())]),
                body: None,
                validation: CheckValidation::StatusRange {
                    min: 200,
                    max_exclusive: 500,
                },
            });

            // Basic auth
            let basic_creds = self.config.basic_auth.as_deref().unwrap_or("user:pass");
            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD.encode(basic_creds.as_bytes());
            self.checks.push(ConformanceCheck {
                name: "security:basic".to_string(),
                method: Method::GET,
                path: "/conformance/security/basic".to_string(),
                headers: self.merge_headers(vec![(
                    "Authorization".to_string(),
                    format!("Basic {}", encoded),
                )]),
                body: None,
                validation: CheckValidation::StatusRange {
                    min: 200,
                    max_exclusive: 500,
                },
            });
        }

        self
    }

    /// Populate checks from annotated spec operations (spec-driven mode)
    #[must_use]
    pub fn with_spec_driven_checks(mut self, operations: &[AnnotatedOperation]) -> Self {
        // Track which features have been seen to deduplicate in default mode
        let mut feature_seen: HashSet<&'static str> = HashSet::new();

        for op in operations {
            for feature in &op.features {
                let category = feature.category();
                if !self.config.should_include_category(category) {
                    continue;
                }

                let check_name_base = feature.check_name();

                if self.config.all_operations {
                    // All-operations mode: test every operation with path-qualified names
                    let check_name = format!("{}:{}", check_name_base, op.path);
                    let check = self.build_spec_check(&check_name, op, feature);
                    self.checks.push(check);
                } else {
                    // Default mode: one representative operation per feature
                    if feature_seen.insert(check_name_base) {
                        let check_name = format!("{}:{}", check_name_base, op.path);
                        let check = self.build_spec_check(&check_name, op, feature);
                        self.checks.push(check);
                    }
                }
            }
        }

        self
    }

    /// Load custom checks from the configured YAML file
    pub fn with_custom_checks(mut self) -> Result<Self> {
        let path = match &self.config.custom_checks_file {
            Some(p) => p.clone(),
            None => return Ok(self),
        };
        let custom_config = CustomConformanceConfig::from_file(&path)?;
        for check in &custom_config.custom_checks {
            self.add_custom_check(check);
        }
        Ok(self)
    }

    /// Return the number of checks that will be executed
    pub fn check_count(&self) -> usize {
        self.checks.len()
    }

    /// Execute all checks and return a `ConformanceReport`
    pub async fn execute(&self) -> Result<ConformanceReport> {
        let mut results = Vec::with_capacity(self.checks.len());
        let delay = self.config.request_delay_ms;

        for (i, check) in self.checks.iter().enumerate() {
            if delay > 0 && i > 0 {
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }
            let result = self.execute_check(check).await;
            results.push(result);
        }

        Ok(Self::aggregate(results))
    }

    /// Execute all checks with progress events sent to the channel
    pub async fn execute_with_progress(
        &self,
        tx: mpsc::Sender<ConformanceProgress>,
    ) -> Result<ConformanceReport> {
        let total = self.checks.len();
        let delay = self.config.request_delay_ms;
        let _ = tx
            .send(ConformanceProgress::Started {
                total_checks: total,
            })
            .await;

        let mut results = Vec::with_capacity(total);

        for (i, check) in self.checks.iter().enumerate() {
            if delay > 0 && i > 0 {
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }
            let result = self.execute_check(check).await;
            let passed = result.passed;
            let name = result.name.clone();
            results.push(result);

            let _ = tx
                .send(ConformanceProgress::CheckCompleted {
                    name,
                    passed,
                    checks_done: i + 1,
                })
                .await;
        }

        let _ = tx.send(ConformanceProgress::Finished).await;
        Ok(Self::aggregate(results))
    }

    /// Execute a single check
    async fn execute_check(&self, check: &ConformanceCheck) -> CheckResult {
        let base_url = self.config.effective_base_url();
        let url = format!("{}{}", base_url.trim_end_matches('/'), check.path);

        let mut request = self.client.request(check.method.clone(), &url);

        // Add headers
        for (name, value) in &check.headers {
            request = request.header(name.as_str(), value.as_str());
        }

        // Add body
        match &check.body {
            Some(CheckBody::Json(value)) => {
                request = request.json(value);
            }
            Some(CheckBody::FormUrlencoded(fields)) => {
                request = request.form(fields);
            }
            Some(CheckBody::Raw {
                content,
                content_type,
            }) => {
                // For multipart, use the multipart API
                if content_type == "text/plain" && check.path.contains("multipart") {
                    let part = reqwest::multipart::Part::bytes(content.as_bytes().to_vec())
                        .file_name("test.txt")
                        .mime_str(content_type)
                        .unwrap_or_else(|_| {
                            reqwest::multipart::Part::bytes(content.as_bytes().to_vec())
                        });
                    let form = reqwest::multipart::Form::new().part("field", part);
                    request = request.multipart(form);
                } else {
                    request =
                        request.header("Content-Type", content_type.as_str()).body(content.clone());
                }
            }
            None => {}
        }

        let response = match request.send().await {
            Ok(resp) => resp,
            Err(e) => {
                return CheckResult {
                    name: check.name.clone(),
                    passed: false,
                    failure_detail: Some(FailureDetail {
                        check: check.name.clone(),
                        request: FailureRequest {
                            method: check.method.to_string(),
                            url: url.clone(),
                            headers: HashMap::new(),
                            body: String::new(),
                        },
                        response: FailureResponse {
                            status: 0,
                            headers: HashMap::new(),
                            body: format!("Request failed: {}", e),
                        },
                        expected: format!("{:?}", check.validation),
                        schema_violations: Vec::new(),
                    }),
                };
            }
        };

        let status = response.status().as_u16();
        let resp_headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let resp_body = response.text().await.unwrap_or_default();

        let (passed, schema_violations) =
            self.validate_response(&check.validation, status, &resp_headers, &resp_body);

        let failure_detail = if !passed {
            Some(FailureDetail {
                check: check.name.clone(),
                request: FailureRequest {
                    method: check.method.to_string(),
                    url,
                    headers: check.headers.iter().cloned().collect(),
                    body: match &check.body {
                        Some(CheckBody::Json(v)) => v.to_string(),
                        Some(CheckBody::FormUrlencoded(f)) => f
                            .iter()
                            .map(|(k, v)| format!("{}={}", k, v))
                            .collect::<Vec<_>>()
                            .join("&"),
                        Some(CheckBody::Raw { content, .. }) => content.clone(),
                        None => String::new(),
                    },
                },
                response: FailureResponse {
                    status,
                    headers: resp_headers,
                    body: if resp_body.len() > 500 {
                        format!("{}...", &resp_body[..500])
                    } else {
                        resp_body
                    },
                },
                expected: Self::describe_validation(&check.validation),
                schema_violations,
            })
        } else {
            None
        };

        CheckResult {
            name: check.name.clone(),
            passed,
            failure_detail,
        }
    }

    /// Validate a response against the check's validation rules.
    ///
    /// Returns `(passed, schema_violations)` where `schema_violations` contains
    /// field-level details when a `SchemaValidation` check fails.
    fn validate_response(
        &self,
        validation: &CheckValidation,
        status: u16,
        headers: &HashMap<String, String>,
        body: &str,
    ) -> (bool, Vec<SchemaViolation>) {
        match validation {
            CheckValidation::StatusRange { min, max_exclusive } => {
                (status >= *min && status < *max_exclusive, Vec::new())
            }
            CheckValidation::ExactStatus(expected) => (status == *expected, Vec::new()),
            CheckValidation::SchemaValidation {
                status_min,
                status_max,
                schema,
            } => {
                if status < *status_min || status >= *status_max {
                    return (false, Vec::new());
                }
                // Parse body as JSON and validate against schema
                let Ok(body_value) = serde_json::from_str::<serde_json::Value>(body) else {
                    return (
                        false,
                        vec![SchemaViolation {
                            field_path: "/".to_string(),
                            violation_type: "parse_error".to_string(),
                            expected: "valid JSON".to_string(),
                            actual: "non-JSON response body".to_string(),
                        }],
                    );
                };
                match jsonschema::validator_for(schema) {
                    Ok(validator) => {
                        let errors: Vec<_> = validator.iter_errors(&body_value).collect();
                        if errors.is_empty() {
                            (true, Vec::new())
                        } else {
                            let violations = errors
                                .iter()
                                .map(|err| {
                                    let field_path = err.instance_path.to_string();
                                    let field_path = if field_path.is_empty() {
                                        "/".to_string()
                                    } else {
                                        field_path
                                    };
                                    SchemaViolation {
                                        field_path,
                                        violation_type: format!("{:?}", err.kind)
                                            .split('(')
                                            .next()
                                            .unwrap_or("unknown")
                                            .split('{')
                                            .next()
                                            .unwrap_or("unknown")
                                            .split(' ')
                                            .next()
                                            .unwrap_or("unknown")
                                            .trim()
                                            .to_string(),
                                        expected: format!("{}", err.schema_path),
                                        actual: format!("{}", err),
                                    }
                                })
                                .collect();
                            (false, violations)
                        }
                    }
                    Err(_) => {
                        // Schema compilation failed — fall back to is_valid behavior
                        (
                            false,
                            vec![SchemaViolation {
                                field_path: "/".to_string(),
                                violation_type: "schema_compile_error".to_string(),
                                expected: "valid JSON schema".to_string(),
                                actual: "schema failed to compile".to_string(),
                            }],
                        )
                    }
                }
            }
            CheckValidation::Custom {
                expected_status,
                expected_headers,
                expected_body_fields,
            } => {
                if status != *expected_status {
                    return (false, Vec::new());
                }
                // Check headers with regex
                for (header_name, pattern) in expected_headers {
                    let header_val = headers
                        .get(header_name)
                        .or_else(|| headers.get(&header_name.to_lowercase()))
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    if let Ok(re) = regex::Regex::new(pattern) {
                        if !re.is_match(header_val) {
                            return (false, Vec::new());
                        }
                    }
                }
                // Check body field types
                if !expected_body_fields.is_empty() {
                    let Ok(body_value) = serde_json::from_str::<serde_json::Value>(body) else {
                        return (false, Vec::new());
                    };
                    for (field_name, field_type) in expected_body_fields {
                        let field = &body_value[field_name];
                        let ok = match field_type.as_str() {
                            "string" => field.is_string(),
                            "integer" => field.is_i64() || field.is_u64(),
                            "number" => field.is_number(),
                            "boolean" => field.is_boolean(),
                            "array" => field.is_array(),
                            "object" => field.is_object(),
                            _ => !field.is_null(),
                        };
                        if !ok {
                            return (false, Vec::new());
                        }
                    }
                }
                (true, Vec::new())
            }
        }
    }

    /// Human-readable validation description for failure reports
    fn describe_validation(validation: &CheckValidation) -> String {
        match validation {
            CheckValidation::StatusRange { min, max_exclusive } => {
                format!("status >= {} && status < {}", min, max_exclusive)
            }
            CheckValidation::ExactStatus(code) => format!("status === {}", code),
            CheckValidation::SchemaValidation {
                status_min,
                status_max,
                ..
            } => {
                format!("status >= {} && status < {} + schema validation", status_min, status_max)
            }
            CheckValidation::Custom {
                expected_status, ..
            } => {
                format!("status === {}", expected_status)
            }
        }
    }

    /// Aggregate check results into a `ConformanceReport`
    fn aggregate(results: Vec<CheckResult>) -> ConformanceReport {
        let mut check_results: HashMap<String, (u64, u64)> = HashMap::new();
        let mut failure_details = Vec::new();

        for result in results {
            let entry = check_results.entry(result.name.clone()).or_insert((0, 0));
            if result.passed {
                entry.0 += 1;
            } else {
                entry.1 += 1;
            }
            if let Some(detail) = result.failure_detail {
                failure_details.push(detail);
            }
        }

        ConformanceReport::from_results(check_results, failure_details)
    }

    // --- Helper methods ---

    /// Build a spec-driven check from an annotated operation and feature
    fn build_spec_check(
        &self,
        check_name: &str,
        op: &AnnotatedOperation,
        feature: &ConformanceFeature,
    ) -> ConformanceCheck {
        // Build URL path with parameters substituted
        let mut url_path = op.path.clone();
        for (name, value) in &op.path_params {
            url_path = url_path.replace(&format!("{{{}}}", name), value);
        }
        // Append query params
        if !op.query_params.is_empty() {
            let qs: Vec<String> =
                op.query_params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            url_path = format!("{}?{}", url_path, qs.join("&"));
        }

        // Build effective headers
        let mut effective_headers = self.effective_headers(&op.header_params);

        // For non-default response codes, add mock server header
        if matches!(feature, ConformanceFeature::Response400 | ConformanceFeature::Response404) {
            let code = match feature {
                ConformanceFeature::Response400 => "400",
                ConformanceFeature::Response404 => "404",
                _ => unreachable!(),
            };
            effective_headers.push(("X-Mockforge-Response-Status".to_string(), code.to_string()));
        }

        // Inject auth headers for security checks or secured endpoints
        let needs_auth = matches!(
            feature,
            ConformanceFeature::SecurityBearer
                | ConformanceFeature::SecurityBasic
                | ConformanceFeature::SecurityApiKey
        ) || !op.security_schemes.is_empty();

        if needs_auth {
            self.inject_security_headers(&op.security_schemes, &mut effective_headers);
        }

        // Determine method
        let method = match op.method.as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "PUT" => Method::PUT,
            "PATCH" => Method::PATCH,
            "DELETE" => Method::DELETE,
            "HEAD" => Method::HEAD,
            "OPTIONS" => Method::OPTIONS,
            _ => Method::GET,
        };

        // Determine body
        let body = match method {
            Method::POST | Method::PUT | Method::PATCH => {
                if let Some(sample) = &op.sample_body {
                    // Add Content-Type if not present
                    let content_type =
                        op.request_body_content_type.as_deref().unwrap_or("application/json");
                    if !effective_headers
                        .iter()
                        .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                    {
                        effective_headers
                            .push(("Content-Type".to_string(), content_type.to_string()));
                    }
                    match content_type {
                        "application/x-www-form-urlencoded" => {
                            // Parse as form fields
                            let fields: Vec<(String, String)> = serde_json::from_str::<
                                serde_json::Value,
                            >(
                                sample
                            )
                            .ok()
                            .and_then(|v| {
                                v.as_object().map(|obj| {
                                    obj.iter()
                                        .map(|(k, v)| {
                                            (k.clone(), v.as_str().unwrap_or("").to_string())
                                        })
                                        .collect()
                                })
                            })
                            .unwrap_or_default();
                            Some(CheckBody::FormUrlencoded(fields))
                        }
                        _ => {
                            // Try JSON, fall back to raw
                            match serde_json::from_str::<serde_json::Value>(sample) {
                                Ok(v) => Some(CheckBody::Json(v)),
                                Err(_) => Some(CheckBody::Raw {
                                    content: sample.clone(),
                                    content_type: content_type.to_string(),
                                }),
                            }
                        }
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        // Determine validation
        let validation = self.determine_validation(feature, op);

        ConformanceCheck {
            name: check_name.to_string(),
            method,
            path: url_path,
            headers: effective_headers,
            body,
            validation,
        }
    }

    /// Determine validation strategy based on the conformance feature
    fn determine_validation(
        &self,
        feature: &ConformanceFeature,
        op: &AnnotatedOperation,
    ) -> CheckValidation {
        match feature {
            ConformanceFeature::Response200 => CheckValidation::ExactStatus(200),
            ConformanceFeature::Response201 => CheckValidation::ExactStatus(201),
            ConformanceFeature::Response204 => CheckValidation::ExactStatus(204),
            ConformanceFeature::Response400 => CheckValidation::ExactStatus(400),
            ConformanceFeature::Response404 => CheckValidation::ExactStatus(404),
            ConformanceFeature::SecurityBearer
            | ConformanceFeature::SecurityBasic
            | ConformanceFeature::SecurityApiKey => CheckValidation::StatusRange {
                min: 200,
                max_exclusive: 400,
            },
            ConformanceFeature::ResponseValidation => {
                if let Some(schema) = &op.response_schema {
                    // Convert openapiv3 Schema to JSON Schema value for jsonschema crate
                    let schema_json = openapi_schema_to_json_schema(schema);
                    CheckValidation::SchemaValidation {
                        status_min: 200,
                        status_max: 500,
                        schema: schema_json,
                    }
                } else {
                    CheckValidation::StatusRange {
                        min: 200,
                        max_exclusive: 500,
                    }
                }
            }
            _ => CheckValidation::StatusRange {
                min: 200,
                max_exclusive: 500,
            },
        }
    }

    /// Add a simple GET reference check with default status range validation
    fn add_ref_get(&mut self, name: &str, path: &str) {
        self.checks.push(ConformanceCheck {
            name: name.to_string(),
            method: Method::GET,
            path: path.to_string(),
            headers: self.custom_headers_only(),
            body: None,
            validation: CheckValidation::StatusRange {
                min: 200,
                max_exclusive: 500,
            },
        });
    }

    /// Merge spec-derived headers with custom headers (custom overrides spec)
    fn effective_headers(&self, spec_headers: &[(String, String)]) -> Vec<(String, String)> {
        let mut headers = Vec::new();
        for (k, v) in spec_headers {
            // Skip if custom headers override this one
            if self.config.custom_headers.iter().any(|(ck, _)| ck.eq_ignore_ascii_case(k)) {
                continue;
            }
            headers.push((k.clone(), v.clone()));
        }
        // Append custom headers
        headers.extend(self.config.custom_headers.clone());
        headers
    }

    /// Merge provided headers with custom headers
    fn merge_headers(&self, mut headers: Vec<(String, String)>) -> Vec<(String, String)> {
        for (k, v) in &self.config.custom_headers {
            if !headers.iter().any(|(hk, _)| hk.eq_ignore_ascii_case(k)) {
                headers.push((k.clone(), v.clone()));
            }
        }
        headers
    }

    /// Return only custom headers (for checks that don't have spec-derived headers)
    fn custom_headers_only(&self) -> Vec<(String, String)> {
        self.config.custom_headers.clone()
    }

    /// Inject security headers based on resolved security schemes.
    /// If the user provides a Cookie header via --conformance-header, skip automatic
    /// Authorization headers (Bearer/Basic) since the user manages their own auth.
    fn inject_security_headers(
        &self,
        schemes: &[SecuritySchemeInfo],
        headers: &mut Vec<(String, String)>,
    ) {
        // If user provides Cookie header, they're using session-based auth — skip auto auth
        let has_cookie_auth =
            self.config.custom_headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("Cookie"));
        let mut to_add: Vec<(String, String)> = Vec::new();

        for scheme in schemes {
            match scheme {
                SecuritySchemeInfo::Bearer => {
                    if !has_cookie_auth
                        && !Self::header_present(
                            "Authorization",
                            headers,
                            &self.config.custom_headers,
                        )
                    {
                        to_add.push((
                            "Authorization".to_string(),
                            "Bearer mockforge-conformance-test-token".to_string(),
                        ));
                    }
                }
                SecuritySchemeInfo::Basic => {
                    if !has_cookie_auth
                        && !Self::header_present(
                            "Authorization",
                            headers,
                            &self.config.custom_headers,
                        )
                    {
                        let creds = self.config.basic_auth.as_deref().unwrap_or("test:test");
                        use base64::Engine;
                        let encoded =
                            base64::engine::general_purpose::STANDARD.encode(creds.as_bytes());
                        to_add.push(("Authorization".to_string(), format!("Basic {}", encoded)));
                    }
                }
                SecuritySchemeInfo::ApiKey { location, name } => match location {
                    ApiKeyLocation::Header => {
                        if !Self::header_present(name, headers, &self.config.custom_headers) {
                            let key = self
                                .config
                                .api_key
                                .as_deref()
                                .unwrap_or("mockforge-conformance-test-key");
                            to_add.push((name.clone(), key.to_string()));
                        }
                    }
                    ApiKeyLocation::Cookie => {
                        if !Self::header_present("Cookie", headers, &self.config.custom_headers) {
                            to_add.push((
                                "Cookie".to_string(),
                                format!("{}=mockforge-conformance-test-session", name),
                            ));
                        }
                    }
                    ApiKeyLocation::Query => {
                        // Handled in URL, not headers
                    }
                },
            }
        }

        headers.extend(to_add);
    }

    /// Check if a header name is present in either the existing headers or custom headers
    fn header_present(
        name: &str,
        headers: &[(String, String)],
        custom_headers: &[(String, String)],
    ) -> bool {
        headers.iter().any(|(h, _)| h.eq_ignore_ascii_case(name))
            || custom_headers.iter().any(|(h, _)| h.eq_ignore_ascii_case(name))
    }

    /// Add a custom check from YAML config
    fn add_custom_check(&mut self, check: &CustomCheck) {
        let method = match check.method.to_uppercase().as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "PUT" => Method::PUT,
            "PATCH" => Method::PATCH,
            "DELETE" => Method::DELETE,
            "HEAD" => Method::HEAD,
            "OPTIONS" => Method::OPTIONS,
            _ => Method::GET,
        };

        // Build headers
        let mut headers: Vec<(String, String)> =
            check.headers.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        // Add global custom headers (check-specific take priority)
        for (k, v) in &self.config.custom_headers {
            if !check.headers.contains_key(k) {
                headers.push((k.clone(), v.clone()));
            }
        }
        // Add Content-Type for JSON body if not present
        if check.body.is_some()
            && !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        {
            headers.push(("Content-Type".to_string(), "application/json".to_string()));
        }

        // Body
        let body = check
            .body
            .as_ref()
            .and_then(|b| serde_json::from_str::<serde_json::Value>(b).ok().map(CheckBody::Json));

        // Build expected headers for validation
        let expected_headers: Vec<(String, String)> =
            check.expected_headers.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        // Build expected body fields
        let expected_body_fields: Vec<(String, String)> = check
            .expected_body_fields
            .iter()
            .map(|f| (f.name.clone(), f.field_type.clone()))
            .collect();

        // Primary status check
        self.checks.push(ConformanceCheck {
            name: check.name.clone(),
            method,
            path: check.path.clone(),
            headers,
            body,
            validation: CheckValidation::Custom {
                expected_status: check.expected_status,
                expected_headers,
                expected_body_fields,
            },
        });
    }
}

/// Convert an `openapiv3::Schema` to a JSON Schema `serde_json::Value`
/// suitable for use with the `jsonschema` crate.
fn openapi_schema_to_json_schema(schema: &openapiv3::Schema) -> serde_json::Value {
    use openapiv3::{SchemaKind, Type};

    match &schema.schema_kind {
        SchemaKind::Type(Type::Object(obj)) => {
            let mut props = serde_json::Map::new();
            for (name, prop_ref) in &obj.properties {
                if let openapiv3::ReferenceOr::Item(prop_schema) = prop_ref {
                    props.insert(name.clone(), openapi_schema_to_json_schema(prop_schema));
                }
            }
            let mut schema_obj = serde_json::json!({
                "type": "object",
                "properties": props,
            });
            if !obj.required.is_empty() {
                schema_obj["required"] = serde_json::Value::Array(
                    obj.required.iter().map(|s| serde_json::json!(s)).collect(),
                );
            }
            schema_obj
        }
        SchemaKind::Type(Type::Array(arr)) => {
            let mut schema_obj = serde_json::json!({"type": "array"});
            if let Some(openapiv3::ReferenceOr::Item(item_schema)) = &arr.items {
                schema_obj["items"] = openapi_schema_to_json_schema(item_schema);
            }
            schema_obj
        }
        SchemaKind::Type(Type::String(s)) => {
            let mut obj = serde_json::json!({"type": "string"});
            if let Some(min) = s.min_length {
                obj["minLength"] = serde_json::json!(min);
            }
            if let Some(max) = s.max_length {
                obj["maxLength"] = serde_json::json!(max);
            }
            if let Some(pattern) = &s.pattern {
                obj["pattern"] = serde_json::json!(pattern);
            }
            if !s.enumeration.is_empty() {
                obj["enum"] = serde_json::Value::Array(
                    s.enumeration
                        .iter()
                        .filter_map(|v| v.as_ref().map(|s| serde_json::json!(s)))
                        .collect(),
                );
            }
            obj
        }
        SchemaKind::Type(Type::Integer(_)) => serde_json::json!({"type": "integer"}),
        SchemaKind::Type(Type::Number(_)) => serde_json::json!({"type": "number"}),
        SchemaKind::Type(Type::Boolean(_)) => serde_json::json!({"type": "boolean"}),
        _ => serde_json::json!({}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reference_check_count() {
        let config = ConformanceConfig {
            target_url: "http://localhost:3000".to_string(),
            ..Default::default()
        };
        let executor = NativeConformanceExecutor::new(config).unwrap().with_reference_checks();
        // 7 params + 3 bodies + 6 schema + 3 composition + 7 formats + 5 constraints
        // + 5 response codes + 7 methods + 1 content + 3 security = 47
        assert_eq!(executor.check_count(), 47);
    }

    #[test]
    fn test_reference_checks_with_category_filter() {
        let config = ConformanceConfig {
            target_url: "http://localhost:3000".to_string(),
            categories: Some(vec!["Parameters".to_string()]),
            ..Default::default()
        };
        let executor = NativeConformanceExecutor::new(config).unwrap().with_reference_checks();
        assert_eq!(executor.check_count(), 7);
    }

    #[test]
    fn test_validate_status_range() {
        let config = ConformanceConfig {
            target_url: "http://localhost:3000".to_string(),
            ..Default::default()
        };
        let executor = NativeConformanceExecutor::new(config).unwrap();
        let headers = HashMap::new();

        assert!(
            executor
                .validate_response(
                    &CheckValidation::StatusRange {
                        min: 200,
                        max_exclusive: 500,
                    },
                    200,
                    &headers,
                    "",
                )
                .0
        );
        assert!(
            executor
                .validate_response(
                    &CheckValidation::StatusRange {
                        min: 200,
                        max_exclusive: 500,
                    },
                    404,
                    &headers,
                    "",
                )
                .0
        );
        assert!(
            !executor
                .validate_response(
                    &CheckValidation::StatusRange {
                        min: 200,
                        max_exclusive: 500,
                    },
                    500,
                    &headers,
                    "",
                )
                .0
        );
    }

    #[test]
    fn test_validate_exact_status() {
        let config = ConformanceConfig {
            target_url: "http://localhost:3000".to_string(),
            ..Default::default()
        };
        let executor = NativeConformanceExecutor::new(config).unwrap();
        let headers = HashMap::new();

        assert!(
            executor
                .validate_response(&CheckValidation::ExactStatus(200), 200, &headers, "")
                .0
        );
        assert!(
            !executor
                .validate_response(&CheckValidation::ExactStatus(200), 201, &headers, "")
                .0
        );
    }

    #[test]
    fn test_validate_schema() {
        let config = ConformanceConfig {
            target_url: "http://localhost:3000".to_string(),
            ..Default::default()
        };
        let executor = NativeConformanceExecutor::new(config).unwrap();
        let headers = HashMap::new();

        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            },
            "required": ["name"]
        });

        let (passed, violations) = executor.validate_response(
            &CheckValidation::SchemaValidation {
                status_min: 200,
                status_max: 300,
                schema: schema.clone(),
            },
            200,
            &headers,
            r#"{"name": "test", "age": 25}"#,
        );
        assert!(passed);
        assert!(violations.is_empty());

        // Missing required field
        let (passed, violations) = executor.validate_response(
            &CheckValidation::SchemaValidation {
                status_min: 200,
                status_max: 300,
                schema: schema.clone(),
            },
            200,
            &headers,
            r#"{"age": 25}"#,
        );
        assert!(!passed);
        assert!(!violations.is_empty());
        assert_eq!(violations[0].violation_type, "Required");
    }

    #[test]
    fn test_validate_custom() {
        let config = ConformanceConfig {
            target_url: "http://localhost:3000".to_string(),
            ..Default::default()
        };
        let executor = NativeConformanceExecutor::new(config).unwrap();
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());

        assert!(
            executor
                .validate_response(
                    &CheckValidation::Custom {
                        expected_status: 200,
                        expected_headers: vec![(
                            "content-type".to_string(),
                            "application/json".to_string(),
                        )],
                        expected_body_fields: vec![("name".to_string(), "string".to_string())],
                    },
                    200,
                    &headers,
                    r#"{"name": "test"}"#,
                )
                .0
        );

        // Wrong status
        assert!(
            !executor
                .validate_response(
                    &CheckValidation::Custom {
                        expected_status: 200,
                        expected_headers: vec![],
                        expected_body_fields: vec![],
                    },
                    404,
                    &headers,
                    "",
                )
                .0
        );
    }

    #[test]
    fn test_aggregate_results() {
        let results = vec![
            CheckResult {
                name: "check1".to_string(),
                passed: true,
                failure_detail: None,
            },
            CheckResult {
                name: "check2".to_string(),
                passed: false,
                failure_detail: Some(FailureDetail {
                    check: "check2".to_string(),
                    request: FailureRequest {
                        method: "GET".to_string(),
                        url: "http://example.com".to_string(),
                        headers: HashMap::new(),
                        body: String::new(),
                    },
                    response: FailureResponse {
                        status: 500,
                        headers: HashMap::new(),
                        body: "error".to_string(),
                    },
                    expected: "status >= 200 && status < 500".to_string(),
                    schema_violations: Vec::new(),
                }),
            },
        ];

        let report = NativeConformanceExecutor::aggregate(results);
        let raw = report.raw_check_results();
        assert_eq!(raw.get("check1"), Some(&(1, 0)));
        assert_eq!(raw.get("check2"), Some(&(0, 1)));
    }

    #[test]
    fn test_custom_check_building() {
        let config = ConformanceConfig {
            target_url: "http://localhost:3000".to_string(),
            ..Default::default()
        };
        let mut executor = NativeConformanceExecutor::new(config).unwrap();

        let custom = CustomCheck {
            name: "custom:test-get".to_string(),
            path: "/api/test".to_string(),
            method: "GET".to_string(),
            expected_status: 200,
            body: None,
            expected_headers: std::collections::HashMap::new(),
            expected_body_fields: vec![],
            headers: std::collections::HashMap::new(),
        };

        executor.add_custom_check(&custom);
        assert_eq!(executor.check_count(), 1);
        assert_eq!(executor.checks[0].name, "custom:test-get");
    }

    #[test]
    fn test_openapi_schema_to_json_schema_object() {
        use openapiv3::{ObjectType, Schema, SchemaData, SchemaKind, Type};

        let schema = Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::Type(Type::Object(ObjectType {
                required: vec!["name".to_string()],
                ..Default::default()
            })),
        };

        let json = openapi_schema_to_json_schema(&schema);
        assert_eq!(json["type"], "object");
        assert_eq!(json["required"][0], "name");
    }
}
