//! WAFBench YAML parser for importing CRS (Core Rule Set) attack patterns
//!
//! This module parses WAFBench YAML test files from the Microsoft WAFBench project
//! (<https://github.com/microsoft/WAFBench>) and converts them into security test payloads
//! compatible with MockForge's security testing framework.
//!
//! # WAFBench YAML Format
//!
//! WAFBench test files follow this structure:
//! ```yaml
//! meta:
//!   author: "author-name"
//!   description: "Tests for rule XXXXXX"
//!   enabled: true
//!   name: "XXXXXX.yaml"
//!
//! tests:
//!   - desc: "Attack scenario description"
//!     test_title: "XXXXXX-N"
//!     stages:
//!       - input:
//!           dest_addr: "127.0.0.1"
//!           headers:
//!             Host: "localhost"
//!             User-Agent: "Mozilla/5.0"
//!           method: "GET"
//!           port: 80
//!           uri: "/path?param=<script>alert(1)</script>"
//!         output:
//!           status: [200, 403, 404]
//! ```
//!
//! # Usage
//!
//! ```bash
//! mockforge bench spec.yaml --wafbench-dir ./wafbench/REQUEST-941-*
//! ```

use crate::error::{BenchError, Result};
use crate::security_payloads::{
    PayloadLocation as SecurityPayloadLocation, SecurityCategory, SecurityPayload,
};
use glob::glob;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// WAFBench test file metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WafBenchMeta {
    /// Author of the test file
    pub author: Option<String>,
    /// Description of what the tests cover
    pub description: Option<String>,
    /// Whether the tests are enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Name of the test file
    pub name: Option<String>,
}

fn default_enabled() -> bool {
    true
}

/// A single WAFBench test case
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WafBenchTest {
    /// Description of the attack scenario
    pub desc: Option<String>,
    /// Unique test identifier (e.g., "941100-1")
    pub test_title: String,
    /// Test stages (request/response pairs)
    #[serde(default)]
    pub stages: Vec<WafBenchStage>,
}

/// A test stage containing input (request) and expected output (response)
/// Supports both direct format and CRS v3.3 format with nested `stage:` wrapper
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WafBenchStage {
    /// The request configuration (direct format)
    pub input: Option<WafBenchInput>,
    /// Expected response (direct format)
    pub output: Option<WafBenchOutput>,
    /// Nested stage for CRS v3.3 format (stage: { input: ..., output: ... })
    pub stage: Option<WafBenchStageInner>,
}

/// Inner stage structure for CRS v3.3 format
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WafBenchStageInner {
    /// The request configuration
    pub input: WafBenchInput,
    /// Expected response
    pub output: Option<WafBenchOutput>,
}

impl WafBenchStage {
    /// Get the input from either direct or nested format
    pub fn get_input(&self) -> Option<&WafBenchInput> {
        // Prefer nested stage format (CRS v3.3), fall back to direct format
        if let Some(stage) = &self.stage {
            Some(&stage.input)
        } else {
            self.input.as_ref()
        }
    }

    /// Get the output from either direct or nested format
    pub fn get_output(&self) -> Option<&WafBenchOutput> {
        // Prefer nested stage format (CRS v3.3), fall back to direct format
        if let Some(stage) = &self.stage {
            stage.output.as_ref()
        } else {
            self.output.as_ref()
        }
    }
}

/// Request configuration for a WAFBench test
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WafBenchInput {
    /// Target address
    pub dest_addr: Option<String>,
    /// HTTP headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// HTTP method
    #[serde(default = "default_method")]
    pub method: String,
    /// Target port
    #[serde(default = "default_port")]
    pub port: u16,
    /// Request URI (may contain attack payloads)
    pub uri: Option<String>,
    /// Request body data
    pub data: Option<String>,
    /// Protocol version
    pub version: Option<String>,
}

fn default_method() -> String {
    "GET".to_string()
}

fn default_port() -> u16 {
    80
}

/// Expected response for a WAFBench test
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WafBenchOutput {
    /// Expected HTTP status codes (any match is valid)
    #[serde(default)]
    pub status: Vec<u16>,
    /// Expected response headers
    #[serde(default)]
    pub response_headers: HashMap<String, String>,
    /// Log contains patterns (can be string or array in different formats)
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub log_contains: Vec<String>,
    /// Log does not contain patterns (can be string or array in different formats)
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub no_log_contains: Vec<String>,
}

/// Deserialize a field that can be either a single string or a Vec of strings
fn deserialize_string_or_vec<'de, D>(deserializer: D) -> std::result::Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct StringOrVec;

    impl<'de> Visitor<'de> for StringOrVec {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("string or array of strings")
        }

        fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![value.to_string()])
        }

        fn visit_string<E>(self, value: String) -> std::result::Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![value])
        }

        fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut vec = Vec::new();
            while let Some(value) = seq.next_element::<String>()? {
                vec.push(value);
            }
            Ok(vec)
        }

        fn visit_none<E>(self) -> std::result::Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Vec::new())
        }

        fn visit_unit<E>(self) -> std::result::Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Vec::new())
        }
    }

    deserializer.deserialize_any(StringOrVec)
}

/// A single case in the SIMPLE traffic format (#987).
///
/// The WAFBench document shape (`meta` + `tests` + `stages` + `input`/`output`)
/// exists to carry CRS rule IDs and provenance. A hand-written or LLM-generated
/// traffic file has none of that, and asking authors to synthesise it buys
/// nothing. This is the shape people actually produce:
///
/// ```yaml
/// - title: utf-7 charset blocked
///   request:
///     method: POST
///     uri: /graphql
///     headers:
///       Content-Type: application/json; charset=utf-7
///     body: '...'
///   expected: 403
/// ```
///
/// Reported by Srikanth on #79: `--wafbench-dir` rejected exactly this with
/// `invalid type: sequence, expected struct WafBenchFile`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SimpleTrafficCase {
    /// Human-readable name for the case.
    pub title: Option<String>,
    /// The request to send.
    pub request: SimpleTrafficRequest,
    /// Expected status. Accepts `403`, `[403, 406]`, or `{ status: 403 }`.
    pub expected: Option<serde_yaml::Value>,
    /// WAFBench's baseline marker: send this case with the rule under test
    /// disabled. mockforge cannot honour it, because disabling a rule is
    /// WAF-side configuration and not something a traffic generator controls.
    /// Parsed only so the case can be reported and skipped instead of being
    /// silently sent as an unfalsifiable duplicate (#997).
    #[serde(default)]
    pub omit_rule: Option<bool>,
}

/// The request half of a [`SimpleTrafficCase`].
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SimpleTrafficRequest {
    /// HTTP method. Defaults to GET, matching the WAFBench input default.
    #[serde(default = "default_method")]
    pub method: String,
    /// Request URI, including any payload in the query string.
    pub uri: Option<String>,
    /// Request headers.
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Request body. Named `body` here rather than WAFBench's `data`, because
    /// `body` is what generators emit.
    pub body: Option<String>,
}

/// Pull expected status codes out of the permissive `expected` field.
///
/// Accepts a bare integer (`403`), a sequence (`[403, 406]`), or a mapping with
/// a `status` key holding either. Anything else yields no expectation rather
/// than an error: an unparsable expectation should not cost the user the case,
/// since the request itself is still perfectly valid traffic to send.
fn extract_expected_statuses(value: Option<&serde_yaml::Value>) -> Vec<u16> {
    fn as_status(v: &serde_yaml::Value) -> Option<u16> {
        v.as_u64().and_then(|n| u16::try_from(n).ok())
    }

    let Some(value) = value else {
        return Vec::new();
    };

    if let Some(one) = as_status(value) {
        return vec![one];
    }
    if let Some(seq) = value.as_sequence() {
        return seq.iter().filter_map(as_status).collect();
    }
    if let Some(map) = value.as_mapping() {
        if let Some(status) = map.get(serde_yaml::Value::String("status".to_string())) {
            if let Some(one) = as_status(status) {
                return vec![one];
            }
            if let Some(seq) = status.as_sequence() {
                return seq.iter().filter_map(as_status).collect();
            }
        }
    }
    Vec::new()
}

impl SimpleTrafficCase {
    /// Convert into the internal WAFBench representation so the rest of the
    /// security-test pipeline is untouched. The mapping is 1:1 except for
    /// `body` -> `data` and the synthesised title.
    fn into_wafbench_test(self, index: usize) -> WafBenchTest {
        let statuses = extract_expected_statuses(self.expected.as_ref());
        let title = self.title.unwrap_or_else(|| format!("case-{}", index + 1));

        WafBenchTest {
            desc: Some(title.clone()),
            test_title: title,
            stages: vec![WafBenchStage {
                input: Some(WafBenchInput {
                    dest_addr: None,
                    headers: self.request.headers,
                    method: self.request.method,
                    port: default_port(),
                    uri: self.request.uri,
                    data: self.request.body,
                    version: None,
                }),
                output: Some(WafBenchOutput {
                    status: statuses,
                    response_headers: HashMap::new(),
                    log_contains: Vec::new(),
                    no_log_contains: Vec::new(),
                }),
                stage: None,
            }],
        }
    }
}

/// Turn traffic cases into request templates that are sent EXACTLY as written
/// (#994).
///
/// The normal WAFBench path treats a case's `uri` as "a CRS attack string
/// hidden in a query parameter": `extract_uri_payload` keeps only the FIRST
/// parameter's value, discards the path, and the survivor is re-attached to a
/// spec-derived endpoint as `?test=<payload>`. That is right for CRS files,
/// where one attack string is the whole test and fuzzing it across a real API
/// is the point.
///
/// It is wrong when the RELATIONSHIP BETWEEN PARAMETERS is the test. Reported
/// by Srikanth on #79: a rule chained on `ARGS:redirect_uri` and
/// `ARGS:response_type` could never fire, because
///
///   /oauth/authorize?response_type=totally-unsupported&redirect_uri=https%3A%2F%2Fevil...
///
/// collapsed to the bare string `totally-unsupported`, sent to a spec endpoint
/// under the parameter name `test`. `redirect_uri` never reached the wire, so a
/// green run looked like the WAF passing when nothing had been exercised.
///
/// Verbatim mode does none of that. The raw `uri` becomes the request path with
/// `query_params` left EMPTY on purpose: `RequestTemplate::generate_path`
/// rejoins query params as `k=v`, which would re-encode values that are already
/// percent-encoded. Keeping the URI whole in `path` preserves it byte for byte,
/// which matters when the encoding IS the payload.
pub fn traffic_cases_to_templates(
    cases: &[WafBenchTestCase],
) -> Vec<crate::request_gen::RequestTemplate> {
    use crate::request_gen::RequestTemplate;
    use crate::spec_parser::ApiOperation;

    let mut templates = Vec::new();

    for case in cases {
        // The Uri payload carries the RAW uri: extraction into a bare attack
        // string only happens later, when building SecurityPayloads (see
        // `extract_uri_payload`). Verbatim mode reads it before that.
        let Some(uri) = case
            .payloads
            .iter()
            .find(|p| p.location == PayloadLocation::Uri)
            .map(|p| p.value.clone())
            .filter(|u| !u.is_empty())
        else {
            continue;
        };

        let headers: HashMap<String, String> = case
            .payloads
            .iter()
            .filter(|p| p.location == PayloadLocation::Header)
            .filter_map(|p| p.header_name.clone().map(|n| (n, p.value.clone())))
            .collect();

        let body = case.payloads.iter().find(|p| p.location == PayloadLocation::Body).map(|p| {
            // Keep the body as written. Parse as JSON only so a JSON body
            // renders as JSON rather than a quoted string; anything else
            // (form data, raw XML, a deliberately malformed payload) passes
            // through untouched.
            serde_json::from_str::<serde_json::Value>(&p.value)
                .unwrap_or_else(|_| serde_json::Value::String(p.value.clone()))
        });

        templates.push(RequestTemplate {
            operation: ApiOperation {
                method: case.method.clone(),
                path: uri,
                operation: Default::default(),
                operation_id: Some(case.description.clone()),
            },
            path_params: HashMap::new(),
            // Left EMPTY on purpose. `RequestTemplate::generate_path` rejoins
            // query params as `k=v`, which would re-encode values that are
            // already percent-encoded. Keeping the whole URI in `path`
            // preserves it byte for byte, and the encoding is often the payload.
            query_params: HashMap::new(),
            headers,
            body,
        });
    }

    templates
}

/// Parse a traffic file in either supported shape.
///
/// Tries the WAFBench document first, then the simple sequence. On failure the
/// error names BOTH accepted shapes: the previous message reported only
/// `invalid type: sequence, expected struct WafBenchFile`, which is accurate
/// about what failed but silent about what would have worked.
pub fn parse_traffic_file(content: &str, source: &str) -> Result<WafBenchFile> {
    match serde_yaml::from_str::<WafBenchFile>(content) {
        Ok(file) => Ok(file),
        Err(wafbench_err) => match serde_yaml::from_str::<Vec<SimpleTrafficCase>>(content) {
            Ok(cases) => {
                let parsed = cases.len();
                let (omitted, cases): (Vec<_>, Vec<_>) =
                    cases.into_iter().partition(|c| c.omit_rule == Some(true));

                for case in &omitted {
                    tracing::warn!(
                        "{source}: skipping `{}` -- it sets `omit_rule: true`, which asks for the \
                         request to be sent with the rule under test disabled. mockforge sends \
                         traffic; it cannot toggle your WAF's rules. Sending it anyway would put \
                         a byte-identical request on the wire as its non-omitted twin while \
                         asserting the opposite status, so exactly one of the pair would always \
                         fail regardless of whether the rule works. Run the same file against a \
                         WAF configuration with that rule disabled to get the baseline.",
                        case.title.as_deref().unwrap_or("<untitled>")
                    );
                }

                tracing::info!(
                    "{source}: parsed {parsed} case(s) in the simple request/expected format{}",
                    if omitted.is_empty() {
                        String::new()
                    } else {
                        format!(
                            "; {} skipped for `omit_rule: true`, {} will be sent",
                            omitted.len(),
                            cases.len()
                        )
                    }
                );
                Ok(WafBenchFile {
                    meta: WafBenchMeta {
                        author: None,
                        description: Some(format!("simple traffic file: {source}")),
                        enabled: true,
                        name: Some(source.to_string()),
                    },
                    tests: cases
                        .into_iter()
                        .enumerate()
                        .map(|(i, c)| c.into_wafbench_test(i))
                        .collect(),
                    // #79: Srikanth wants omitted cases in the per-file
                    // breakdown, not only in a tracing line he may miss.
                    omitted_count: omitted.len(),
                })
            }
            Err(simple_err) => Err(BenchError::Other(format!(
                "Failed to parse traffic file {source}. Two shapes are accepted:\n  \
                 (1) WAFBench document -- a `meta:` mapping plus a `tests:` list. Parse error: {wafbench_err}\n  \
                 (2) simple list -- `- title: .. / request: {{method, uri, headers, body}} / expected: 403`. Parse error: {simple_err}"
            ))),
        },
    }
}

/// Complete WAFBench test file structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WafBenchFile {
    /// Test file metadata
    pub meta: WafBenchMeta,
    /// Test cases
    #[serde(default)]
    pub tests: Vec<WafBenchTest>,
    /// Cases dropped for `omit_rule: true`. Not in the YAML; filled by
    /// `parse_traffic_file` so the per-file breakdown can report them.
    #[serde(default, skip)]
    pub omitted_count: usize,
}

/// A parsed WAFBench test case ready for use in security testing
#[derive(Debug, Clone)]
pub struct WafBenchTestCase {
    /// Test identifier
    pub test_id: String,
    /// Description
    pub description: String,
    /// CRS rule ID (e.g., 941100)
    pub rule_id: String,
    /// Security category
    pub category: SecurityCategory,
    /// HTTP method
    pub method: String,
    /// Attack payloads extracted from the test
    pub payloads: Vec<WafBenchPayload>,
    /// Expected to be blocked (403)
    pub expects_block: bool,
}

/// A specific payload from a WAFBench test
#[derive(Debug, Clone)]
pub struct WafBenchPayload {
    /// The payload location (uri, header, body)
    pub location: PayloadLocation,
    /// The actual payload string
    pub value: String,
    /// Header name if location is Header
    pub header_name: Option<String>,
}

/// Where the payload is injected
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadLocation {
    /// Payload in URI/query string
    Uri,
    /// Payload in HTTP header
    Header,
    /// Payload in request body
    Body,
}

impl std::fmt::Display for PayloadLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Uri => write!(f, "uri"),
            Self::Header => write!(f, "header"),
            Self::Body => write!(f, "body"),
        }
    }
}

/// WAFBench loader and parser
pub struct WafBenchLoader {
    /// Loaded test cases
    test_cases: Vec<WafBenchTestCase>,
    /// Statistics
    stats: WafBenchStats,
}

/// Per-file counts so a multi-YAML run can say how many attacks vs
/// baseline vs omitted cases each file contributed (#79).
#[derive(Debug, Clone, Default)]
pub struct TrafficFileSummary {
    /// Path as loaded (`--wafbench-dir` entry or glob match).
    pub file: String,
    /// Cases that will actually be sent.
    pub sent: usize,
    /// Sent cases whose expected status includes 403 (treated as attacks).
    pub attack: usize,
    /// Sent cases whose expected status includes 200 and not 403.
    pub normal: usize,
    /// Cases skipped for `omit_rule: true`.
    pub omitted: usize,
    /// Sent cases with neither 200 nor 403 in `expected`.
    pub other: usize,
}

/// Statistics about loaded WAFBench tests
#[derive(Debug, Clone, Default)]
pub struct WafBenchStats {
    /// Number of files processed
    pub files_processed: usize,
    /// Number of test cases loaded
    pub test_cases_loaded: usize,
    /// Number of payloads extracted
    pub payloads_extracted: usize,
    /// Tests by category
    pub by_category: HashMap<SecurityCategory, usize>,
    /// Files that failed to parse
    pub parse_errors: Vec<String>,
    /// One row per loaded traffic file.
    pub per_file: Vec<TrafficFileSummary>,
}

/// Classify a parsed file into the attack / normal / omitted buckets
/// Srikanth asked for: expected 403 = attack, expected 200 = normal.
pub fn summarize_traffic_file(file: &WafBenchFile, source: &str) -> TrafficFileSummary {
    let mut attack = 0;
    let mut normal = 0;
    let mut other = 0;
    for test in &file.tests {
        let mut has_403 = false;
        let mut has_200 = false;
        for stage in &test.stages {
            if let Some(output) = stage.get_output() {
                has_403 |= output.status.contains(&403);
                has_200 |= output.status.contains(&200);
            }
        }
        if has_403 {
            attack += 1;
        } else if has_200 {
            normal += 1;
        } else {
            other += 1;
        }
    }
    TrafficFileSummary {
        file: source.to_string(),
        sent: file.tests.len(),
        attack,
        normal,
        omitted: file.omitted_count,
        other,
    }
}

impl WafBenchLoader {
    /// Create a new empty loader
    pub fn new() -> Self {
        Self {
            test_cases: Vec::new(),
            stats: WafBenchStats::default(),
        }
    }

    /// Load WAFBench tests from a directory pattern (supports glob)
    ///
    /// # Arguments
    /// * `pattern` - Glob pattern like `./wafbench/REQUEST-941-*` or a direct path
    ///
    /// # Example
    /// ```ignore
    /// let loader = WafBenchLoader::new();
    /// loader.load_from_pattern("./wafbench/REQUEST-941-APPLICATION-ATTACK-XSS/**/*.yaml")?;
    /// ```
    pub fn load_from_pattern(&mut self, pattern: &str) -> Result<()> {
        // If pattern doesn't contain wildcards, check if it's a file or directory
        if !pattern.contains('*') && !pattern.contains('?') {
            let path = Path::new(pattern);
            if path.is_file() {
                // Load single file directly
                return self.load_file(path);
            } else if path.is_dir() {
                return self.load_from_directory(path);
            } else {
                return Err(BenchError::Other(format!(
                    "WAFBench path does not exist: {}",
                    pattern
                )));
            }
        }

        // Use glob to find matching files
        let entries = glob(pattern).map_err(|e| {
            BenchError::Other(format!("Invalid WAFBench pattern '{}': {}", pattern, e))
        })?;

        for entry in entries {
            match entry {
                Ok(path) => {
                    if path.is_file()
                        && path.extension().is_some_and(|ext| ext == "yaml" || ext == "yml")
                    {
                        if let Err(e) = self.load_file(&path) {
                            self.stats.parse_errors.push(format!("{}: {}", path.display(), e));
                        }
                    } else if path.is_dir() {
                        if let Err(e) = self.load_from_directory(&path) {
                            self.stats.parse_errors.push(format!("{}: {}", path.display(), e));
                        }
                    }
                }
                Err(e) => {
                    self.stats.parse_errors.push(format!("Glob error: {}", e));
                }
            }
        }

        Ok(())
    }

    /// Load WAFBench tests from a directory (recursive)
    pub fn load_from_directory(&mut self, dir: &Path) -> Result<()> {
        if !dir.is_dir() {
            return Err(BenchError::Other(format!(
                "WAFBench path is not a directory: {}",
                dir.display()
            )));
        }

        self.load_directory_recursive(dir)?;
        Ok(())
    }

    fn load_directory_recursive(&mut self, dir: &Path) -> Result<()> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| BenchError::Other(format!("Failed to read WAFBench directory: {}", e)))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Recurse into subdirectories
                self.load_directory_recursive(&path)?;
            } else if path.extension().is_some_and(|ext| ext == "yaml" || ext == "yml") {
                if let Err(e) = self.load_file(&path) {
                    self.stats.parse_errors.push(format!("{}: {}", path.display(), e));
                }
            }
        }

        Ok(())
    }

    /// Load a single WAFBench YAML file
    pub fn load_file(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            BenchError::Other(format!("Failed to read WAFBench file {}: {}", path.display(), e))
        })?;

        let source = path.display().to_string();
        let wafbench_file = parse_traffic_file(&content, &source)?;

        // Skip disabled test files
        if !wafbench_file.meta.enabled {
            return Ok(());
        }

        self.stats.per_file.push(summarize_traffic_file(&wafbench_file, &source));
        self.stats.files_processed += 1;

        // Determine the rule category from the file path or name
        let category = self.detect_category(path, &wafbench_file.meta);

        // Parse each test case
        for test in wafbench_file.tests {
            if let Some(test_case) = self.parse_test_case(&test, category) {
                self.stats.payloads_extracted += test_case.payloads.len();
                *self.stats.by_category.entry(category).or_insert(0) += 1;
                self.test_cases.push(test_case);
                self.stats.test_cases_loaded += 1;
            }
        }

        Ok(())
    }

    /// Detect the security category from the file path
    fn detect_category(&self, path: &Path, _meta: &WafBenchMeta) -> SecurityCategory {
        let path_str = path.to_string_lossy().to_uppercase();

        if path_str.contains("XSS") || path_str.contains("941") {
            SecurityCategory::Xss
        } else if path_str.contains("SQLI") || path_str.contains("942") {
            SecurityCategory::SqlInjection
        } else if path_str.contains("RCE") || path_str.contains("932") {
            SecurityCategory::CommandInjection
        } else if path_str.contains("LFI") || path_str.contains("930") {
            SecurityCategory::PathTraversal
        } else if path_str.contains("LDAP") {
            SecurityCategory::LdapInjection
        } else if path_str.contains("XXE") || path_str.contains("XML") {
            SecurityCategory::Xxe
        } else if path_str.contains("TEMPLATE") || path_str.contains("SSTI") {
            SecurityCategory::Ssti
        } else {
            // Default to XSS as it's the most common in WAFBench
            SecurityCategory::Xss
        }
    }

    /// Parse a single test case into our format
    fn parse_test_case(
        &self,
        test: &WafBenchTest,
        category: SecurityCategory,
    ) -> Option<WafBenchTestCase> {
        // Extract rule ID from test_title (e.g., "941100-1" -> "941100")
        let rule_id = test.test_title.split('-').next().unwrap_or(&test.test_title).to_string();

        let mut payloads = Vec::new();
        let mut method = "GET".to_string();
        let mut expects_block = false;

        for stage in &test.stages {
            // Get input from either direct or nested format (CRS v3.3 compatibility)
            let Some(input) = stage.get_input() else {
                continue;
            };

            method = input.method.clone();

            // Check if this test expects a block (403)
            if let Some(output) = stage.get_output() {
                if output.status.contains(&403) {
                    expects_block = true;
                }
            }

            // Extract payload from URI — CRS test files are attack payloads by
            // definition, so we accept all values without filtering. Previously
            // a narrow looks_like_attack() check discarded exotic payloads like
            // VML, VBScript, UTF-7, JSFuck, and bracket-notation XSS.
            if let Some(uri) = &input.uri {
                if !uri.is_empty() {
                    payloads.push(WafBenchPayload {
                        location: PayloadLocation::Uri,
                        value: uri.clone(),
                        header_name: None,
                    });
                }
            }

            // Extract payloads from headers
            for (header_name, header_value) in &input.headers {
                if !header_value.is_empty() {
                    payloads.push(WafBenchPayload {
                        location: PayloadLocation::Header,
                        value: header_value.clone(),
                        header_name: Some(header_name.clone()),
                    });
                }
            }

            // Extract payload from body
            if let Some(data) = &input.data {
                if !data.is_empty() {
                    payloads.push(WafBenchPayload {
                        location: PayloadLocation::Body,
                        value: data.clone(),
                        header_name: None,
                    });
                }
            }
        }

        // If no payloads found, still include the test but with full URI as payload
        if payloads.is_empty() {
            if let Some(stage) = test.stages.first() {
                if let Some(input) = stage.get_input() {
                    if let Some(uri) = &input.uri {
                        payloads.push(WafBenchPayload {
                            location: PayloadLocation::Uri,
                            value: uri.clone(),
                            header_name: None,
                        });
                    }
                }
            }
        }

        if payloads.is_empty() {
            return None;
        }

        let description = test.desc.clone().unwrap_or_else(|| format!("CRS Rule {} test", rule_id));

        Some(WafBenchTestCase {
            test_id: test.test_title.clone(),
            description,
            rule_id,
            category,
            method,
            payloads,
            expects_block,
        })
    }

    /// Check if a string looks like an attack payload (used in tests)
    #[cfg(test)]
    fn looks_like_attack(&self, s: &str) -> bool {
        // Common attack patterns
        let attack_patterns = [
            "<script",
            "javascript:",
            "onerror=",
            "onload=",
            "onclick=",
            "onfocus=",
            "onmouseover=",
            "eval(",
            "alert(",
            "document.",
            "window.",
            "'--",
            "' OR ",
            "' AND ",
            "1=1",
            "UNION SELECT",
            "CONCAT(",
            "CHAR(",
            "../",
            "..\\",
            "/etc/passwd",
            "cmd.exe",
            "powershell",
            "; ls",
            "| cat",
            "${",
            "{{",
            "<%",
            "<?",
            "<!ENTITY",
            "SYSTEM \"",
        ];

        let lower = s.to_lowercase();
        attack_patterns.iter().any(|p| lower.contains(&p.to_lowercase()))
    }

    /// Get all loaded test cases
    pub fn test_cases(&self) -> &[WafBenchTestCase] {
        &self.test_cases
    }

    /// Get statistics about loaded tests
    pub fn stats(&self) -> &WafBenchStats {
        &self.stats
    }

    /// Decode a form-URL-encoded body payload.
    /// Replaces `+` with space (form-encoding convention), then decodes `%XX` sequences.
    /// Strips form field name prefix (e.g., `var=;;dd foo bar` → `;;dd foo bar`)
    /// since JSON injection puts the value in a field, not the form key.
    fn decode_form_encoded_body(value: &str) -> String {
        // Replace + with space first (form-encoding convention)
        let plus_decoded = value.replace('+', " ");
        // Then decode %XX sequences
        let decoded = urlencoding::decode(&plus_decoded)
            .map(|s| s.into_owned())
            .unwrap_or(plus_decoded);
        // Strip form field name prefix (e.g., "var=value" → "value")
        // CRS test data like "var=;;dd foo bar" has the form key included,
        // but we inject only the value into a JSON field.
        Self::strip_form_key(&decoded)
    }

    /// Strip a single leading form key from a form-encoded value.
    /// `"var=;;dd foo bar"` → `";;dd foo bar"`
    /// `"pay=exec (@\n"` → `"exec (@\n"`
    /// Values without `=` or starting with special chars are returned as-is.
    fn strip_form_key(value: &str) -> String {
        // Only strip if the prefix before the first = looks like a form field name
        // (alphanumeric/underscore chars). Don't strip if the = is part of the attack.
        if let Some(eq_pos) = value.find('=') {
            let key = &value[..eq_pos];
            // Form field names are alphanumeric with underscores
            if !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return value[eq_pos + 1..].to_string();
            }
        }
        value.to_string()
    }

    /// Normalize a form body value to valid `application/x-www-form-urlencoded` format.
    ///
    /// CRS YAML `data` fields may be pre-encoded (`var=%3B%3Bdd+foo+bar`) or decoded
    /// (`var=;;dd foo bar`). This function ensures the output is always properly encoded
    /// so WAFs can parse it into ARGS and fire rules like 942432.
    ///
    /// Strategy: decode fully first (handling `+` as space and `%XX` sequences), then
    /// re-encode. Pre-encoded input round-trips correctly; decoded input gets encoded.
    fn ensure_form_encoded(value: &str) -> String {
        value
            .split('&')
            .map(|pair| {
                if let Some(eq_pos) = pair.find('=') {
                    let key = &pair[..eq_pos];
                    let val = &pair[eq_pos + 1..];
                    // Decode: + → space, then %XX → chars
                    let key_plus = key.replace('+', " ");
                    let val_plus = val.replace('+', " ");
                    let decoded_key = urlencoding::decode(&key_plus).unwrap_or(key.into());
                    let decoded_val = urlencoding::decode(&val_plus).unwrap_or(val.into());
                    // Re-encode with form-encoding (spaces as +)
                    let enc_key = urlencoding::encode(&decoded_key).replace("%20", "+");
                    let enc_val = urlencoding::encode(&decoded_val).replace("%20", "+");
                    format!("{enc_key}={enc_val}")
                } else {
                    // No key=value structure — encode the whole thing
                    let pair_plus = pair.replace('+', " ");
                    let decoded = urlencoding::decode(&pair_plus).unwrap_or(pair.into());
                    urlencoding::encode(&decoded).replace("%20", "+").to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("&")
    }

    /// Convert loaded tests to SecurityPayload format for use with existing security testing
    pub fn to_security_payloads(&self) -> Vec<SecurityPayload> {
        let mut payloads = Vec::new();

        for test_case in &self.test_cases {
            // Assign group_id when a test case has multiple payloads
            let group_id = if test_case.payloads.len() > 1 {
                Some(test_case.test_id.clone())
            } else {
                None
            };

            for payload in &test_case.payloads {
                // Extract just the attack payload part if possible
                let payload_str = match payload.location {
                    PayloadLocation::Body => {
                        // Form-URL-decode body payloads so WAFs see the real characters
                        Self::decode_form_encoded_body(&payload.value)
                    }
                    PayloadLocation::Uri => {
                        // Extract attack payload from URI, URL-decode, strip path prefix
                        self.extract_uri_payload(&payload.value)
                    }
                    PayloadLocation::Header => {
                        // Headers are used as-is (Cookie values, User-Agent, etc.)
                        payload.value.clone()
                    }
                };

                // Convert local PayloadLocation to SecurityPayloadLocation
                let location = match payload.location {
                    PayloadLocation::Uri => SecurityPayloadLocation::Uri,
                    PayloadLocation::Header => SecurityPayloadLocation::Header,
                    PayloadLocation::Body => SecurityPayloadLocation::Body,
                };

                let mut sec_payload = SecurityPayload::new(
                    payload_str,
                    test_case.category,
                    format!(
                        "[WAFBench {}] {} ({})",
                        test_case.rule_id, test_case.description, payload.location
                    ),
                )
                .high_risk()
                .with_location(location);

                // Add header name for header payloads
                if let Some(header_name) = &payload.header_name {
                    sec_payload = sec_payload.with_header_name(header_name.clone());
                }

                // Add group ID for multi-part test cases
                if let Some(gid) = &group_id {
                    sec_payload = sec_payload.with_group_id(gid.clone());
                }

                // URI payloads without '?' are path-only attacks (e.g., 942101: POST /1234%20OR%201=1)
                // These need to replace the request path so WAF inspects via REQUEST_FILENAME
                if payload.location == PayloadLocation::Uri && !payload.value.contains('?') {
                    sec_payload = sec_payload.with_inject_as_path();
                }

                // Body payloads: normalize to valid form-encoded format for WAF ARGS parsing
                // (e.g., 942432: data "var=%3B%3Bdd+foo+bar" or decoded "var=;;dd foo bar")
                if payload.location == PayloadLocation::Body {
                    sec_payload = sec_payload
                        .with_form_encoded_body(Self::ensure_form_encoded(&payload.value));
                }

                payloads.push(sec_payload);
            }
        }

        payloads
    }

    /// Extract the actual attack payload from a URI.
    ///
    /// For URIs with query parameters (e.g., `/?var=EXECUTE%20IMMEDIATE%20%22`),
    /// extracts and URL-decodes the first parameter value.
    ///
    /// For path-only URIs (e.g., `/1234%20OR%201=1`), URL-decodes the path and
    /// strips the leading `/` which is a URI artifact, not part of the attack.
    fn extract_uri_payload(&self, value: &str) -> String {
        // If it's a URI with query params, extract the first parameter value
        // (URL-decoded). CRS test files put the attack in query params.
        if value.contains('?') {
            if let Some(query) = value.split('?').nth(1) {
                for param in query.split('&') {
                    if let Some(val) = param.split('=').nth(1) {
                        let decoded = urlencoding::decode(val).unwrap_or_else(|_| val.into());
                        if !decoded.is_empty() {
                            return decoded.to_string();
                        }
                    }
                }
            }
        }

        // For path-only URIs, URL-decode and strip leading /
        // e.g., /1234%20OR%201=1 → 1234 OR 1=1
        let decoded = urlencoding::decode(value)
            .map(|s| s.into_owned())
            .unwrap_or_else(|_| value.to_string());
        let trimmed = decoded.trim_start_matches('/');
        if trimmed.is_empty() {
            // Don't return empty string for bare "/" paths
            return decoded;
        }
        trimmed.to_string()
    }
}

impl Default for WafBenchLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wafbench_yaml() {
        let yaml = r#"
meta:
  author: test
  description: Test XSS rules
  enabled: true
  name: test.yaml

tests:
  - desc: "XSS in URI parameter"
    test_title: "941100-1"
    stages:
      - input:
          dest_addr: "127.0.0.1"
          headers:
            Host: "localhost"
            User-Agent: "Mozilla/5.0"
          method: "GET"
          port: 80
          uri: "/test?param=<script>alert(1)</script>"
        output:
          status: [403]
"#;

        let file: WafBenchFile = serde_yaml::from_str(yaml).unwrap();
        assert!(file.meta.enabled);
        assert_eq!(file.tests.len(), 1);
        assert_eq!(file.tests[0].test_title, "941100-1");
    }

    #[test]
    fn test_detect_category() {
        let loader = WafBenchLoader::new();
        let meta = WafBenchMeta {
            author: None,
            description: None,
            enabled: true,
            name: None,
        };

        assert_eq!(
            loader.detect_category(Path::new("/wafbench/REQUEST-941-XSS/test.yaml"), &meta),
            SecurityCategory::Xss
        );

        assert_eq!(
            loader.detect_category(Path::new("/wafbench/REQUEST-942-SQLI/test.yaml"), &meta),
            SecurityCategory::SqlInjection
        );
    }

    #[test]
    fn test_looks_like_attack() {
        let loader = WafBenchLoader::new();

        assert!(loader.looks_like_attack("<script>alert(1)</script>"));
        assert!(loader.looks_like_attack("' OR '1'='1"));
        assert!(loader.looks_like_attack("../../../etc/passwd"));
        assert!(loader.looks_like_attack("; ls -la"));
        assert!(!loader.looks_like_attack("normal text"));
        assert!(!loader.looks_like_attack("hello world"));
    }

    #[test]
    fn test_extract_uri_payload_with_query_params() {
        let loader = WafBenchLoader::new();

        // URI with query params: extracts and decodes the parameter value
        let uri = "/test?param=%3Cscript%3Ealert(1)%3C/script%3E";
        let payload = loader.extract_uri_payload(uri);
        assert_eq!(payload, "<script>alert(1)</script>");
    }

    #[test]
    fn test_extract_uri_payload_path_only() {
        let loader = WafBenchLoader::new();

        // Path-only URI: URL-decodes and strips leading /
        let uri = "/1234%20OR%201=1";
        let payload = loader.extract_uri_payload(uri);
        assert_eq!(payload, "1234 OR 1=1");

        // Path with quotes and special chars
        let uri2 = "/foo')waitfor%20delay'5%3a0%3a20'--";
        let payload2 = loader.extract_uri_payload(uri2);
        assert_eq!(payload2, "foo')waitfor delay'5:0:20'--");

        // Bare slash returns "/" (not empty)
        let uri3 = "/";
        let payload3 = loader.extract_uri_payload(uri3);
        assert_eq!(payload3, "/");
    }

    #[test]
    fn test_group_id_assigned_for_multi_part_test_cases() {
        let yaml = r#"
meta:
  author: test
  description: Multi-part test
  enabled: true
  name: test.yaml

tests:
  - desc: "Multi-part attack with URI and header"
    test_title: "942290-1"
    stages:
      - input:
          dest_addr: "127.0.0.1"
          headers:
            Host: "localhost"
            User-Agent: "ModSecurity CRS 3 Tests"
          method: "GET"
          port: 80
          uri: "/test?param=attack"
        output:
          status: [403]
"#;

        let file: WafBenchFile = serde_yaml::from_str(yaml).unwrap();
        let mut loader = WafBenchLoader::new();
        loader.stats.files_processed += 1;

        let category = SecurityCategory::SqlInjection;
        for test in &file.tests {
            if let Some(test_case) = loader.parse_test_case(test, category) {
                loader.test_cases.push(test_case);
            }
        }

        let payloads = loader.to_security_payloads();
        // This test has URI + 2 headers = 3 payloads, all should share a group_id
        assert!(payloads.len() >= 2, "Should have at least 2 payloads");
        let group_ids: Vec<_> = payloads.iter().map(|p| p.group_id.clone()).collect();
        assert!(
            group_ids.iter().all(|g| g.is_some()),
            "All payloads in multi-part test should have group_id"
        );
        assert!(
            group_ids.iter().all(|g| g.as_deref() == Some("942290-1")),
            "All payloads should share the same group_id"
        );
    }

    #[test]
    fn test_single_payload_no_group_id() {
        let yaml = r#"
meta:
  author: test
  description: Single payload test
  enabled: true
  name: test.yaml

tests:
  - desc: "Simple XSS"
    test_title: "941100-1"
    stages:
      - input:
          dest_addr: "127.0.0.1"
          headers: {}
          method: "GET"
          port: 80
          uri: "/test?param=<script>alert(1)</script>"
        output:
          status: [403]
"#;

        let file: WafBenchFile = serde_yaml::from_str(yaml).unwrap();
        let mut loader = WafBenchLoader::new();
        loader.stats.files_processed += 1;

        let category = SecurityCategory::Xss;
        for test in &file.tests {
            if let Some(test_case) = loader.parse_test_case(test, category) {
                loader.test_cases.push(test_case);
            }
        }

        let payloads = loader.to_security_payloads();
        assert_eq!(payloads.len(), 1, "Should have exactly 1 payload");
        assert!(payloads[0].group_id.is_none(), "Single-payload test should NOT have group_id");
    }

    #[test]
    fn test_body_payload_form_url_decoded() {
        let yaml = r#"
meta:
  author: test
  description: Body payload test
  enabled: true
  name: test.yaml

tests:
  - desc: "SQL injection in body"
    test_title: "942240-1"
    stages:
      - stage:
          input:
            dest_addr: 127.0.0.1
            headers:
              Host: localhost
            method: POST
            port: 80
            uri: "/"
            data: "%22+WAITFOR+DELAY+%270%3A0%3A5%27"
          output:
            log_contains: id "942240"
"#;

        let file: WafBenchFile = serde_yaml::from_str(yaml).unwrap();
        let mut loader = WafBenchLoader::new();
        loader.stats.files_processed += 1;

        let category = SecurityCategory::SqlInjection;
        for test in &file.tests {
            if let Some(test_case) = loader.parse_test_case(test, category) {
                loader.test_cases.push(test_case);
            }
        }

        let payloads = loader.to_security_payloads();
        // Find the body payload
        let body_payload = payloads
            .iter()
            .find(|p| p.location == SecurityPayloadLocation::Body)
            .expect("Should have a body payload");

        // The body payload should be form-URL-decoded
        assert!(
            body_payload.payload.contains('"'),
            "Body payload should have decoded %22 to double-quote: {}",
            body_payload.payload
        );
        assert!(
            body_payload.payload.contains(' '),
            "Body payload should have decoded + to space: {}",
            body_payload.payload
        );
        assert!(
            !body_payload.payload.contains("%22"),
            "Body payload should NOT contain literal %22: {}",
            body_payload.payload
        );
    }

    #[test]
    fn test_decode_form_encoded_body() {
        // Basic decoding
        assert_eq!(
            WafBenchLoader::decode_form_encoded_body("%22+WAITFOR+DELAY+%27%0A"),
            "\" WAITFOR DELAY '\n"
        );
        assert_eq!(WafBenchLoader::decode_form_encoded_body("normal+text"), "normal text");
        assert_eq!(
            WafBenchLoader::decode_form_encoded_body("no+encoding+needed"),
            "no encoding needed"
        );
        // Form key stripping: var=value → value
        assert_eq!(
            WafBenchLoader::decode_form_encoded_body("var%3D%3B%3Bdd+foo+bar"),
            ";;dd foo bar"
        );
        // Form key stripping: pay=exec → exec
        assert_eq!(WafBenchLoader::decode_form_encoded_body("pay%3Dexec+%28%40%0A"), "exec (@\n");
        // No form key: starts with special char → returned as-is
        assert_eq!(WafBenchLoader::decode_form_encoded_body("%22+WAITFOR"), "\" WAITFOR");
    }

    #[test]
    fn test_strip_form_key() {
        // Standard form key=value
        assert_eq!(WafBenchLoader::strip_form_key("var=;;dd foo bar"), ";;dd foo bar");
        assert_eq!(WafBenchLoader::strip_form_key("pay=exec (@\n"), "exec (@\n");
        assert_eq!(WafBenchLoader::strip_form_key("pay=DECLARE/**/@x\n"), "DECLARE/**/@x\n");
        // No form key (starts with special char)
        assert_eq!(WafBenchLoader::strip_form_key("\" WAITFOR DELAY '\n"), "\" WAITFOR DELAY '\n");
        // = inside attack payload, key is not alphanumeric
        assert_eq!(WafBenchLoader::strip_form_key("' OR 1=1"), "' OR 1=1");
        // Empty input
        assert_eq!(WafBenchLoader::strip_form_key(""), "");
        // Only key, no value
        assert_eq!(WafBenchLoader::strip_form_key("var="), "");
    }

    #[test]
    fn test_ensure_form_encoded() {
        // Pre-encoded input round-trips correctly
        assert_eq!(
            WafBenchLoader::ensure_form_encoded("var=%3B%3Bdd+foo+bar"),
            "var=%3B%3Bdd+foo+bar"
        );
        // Decoded input gets properly encoded
        assert_eq!(WafBenchLoader::ensure_form_encoded("var=;;dd foo bar"), "var=%3B%3Bdd+foo+bar");
        // Multi-field form
        assert_eq!(
            WafBenchLoader::ensure_form_encoded("var=-------------------&var2=whatever"),
            "var=-------------------&var2=whatever"
        );
        // Already-encoded multi-field
        assert_eq!(
            WafBenchLoader::ensure_form_encoded("key=%22value%22&other=test+data"),
            "key=%22value%22&other=test+data"
        );
        // Decoded multi-field
        assert_eq!(
            WafBenchLoader::ensure_form_encoded("key=\"value\"&other=test data"),
            "key=%22value%22&other=test+data"
        );
        // No key=value structure
        assert_eq!(WafBenchLoader::ensure_form_encoded("plain text"), "plain+text");
        // Empty string
        assert_eq!(WafBenchLoader::ensure_form_encoded(""), "");
    }

    #[test]
    fn test_uri_path_only_gets_inject_as_path() {
        let yaml = r#"
meta:
  author: test
  description: Path injection test
  enabled: true
  name: test.yaml

tests:
  - desc: "Path-based SQL injection"
    test_title: "942101-1"
    stages:
      - stage:
          input:
            dest_addr: 127.0.0.1
            headers:
              Host: localhost
            method: POST
            port: 80
            uri: "/1234%20OR%201=1"
          output:
            log_contains: id "942101"
"#;

        let file: WafBenchFile = serde_yaml::from_str(yaml).unwrap();
        let mut loader = WafBenchLoader::new();
        loader.stats.files_processed += 1;

        let category = SecurityCategory::SqlInjection;
        for test in &file.tests {
            if let Some(test_case) = loader.parse_test_case(test, category) {
                loader.test_cases.push(test_case);
            }
        }

        let payloads = loader.to_security_payloads();
        let uri_payload = payloads
            .iter()
            .find(|p| p.location == SecurityPayloadLocation::Uri)
            .expect("Should have URI payload");

        assert_eq!(
            uri_payload.inject_as_path,
            Some(true),
            "Path-only URI should have inject_as_path=true"
        );
    }

    #[test]
    fn test_uri_with_query_no_inject_as_path() {
        let yaml = r#"
meta:
  author: test
  description: Query param test
  enabled: true
  name: test.yaml

tests:
  - desc: "Query-param SQL injection"
    test_title: "942100-1"
    stages:
      - stage:
          input:
            dest_addr: 127.0.0.1
            headers: {}
            method: GET
            port: 80
            uri: "/test?param=1+OR+1%3D1"
          output:
            log_contains: id "942100"
"#;

        let file: WafBenchFile = serde_yaml::from_str(yaml).unwrap();
        let mut loader = WafBenchLoader::new();
        loader.stats.files_processed += 1;

        let category = SecurityCategory::SqlInjection;
        for test in &file.tests {
            if let Some(test_case) = loader.parse_test_case(test, category) {
                loader.test_cases.push(test_case);
            }
        }

        let payloads = loader.to_security_payloads();
        let uri_payload = payloads
            .iter()
            .find(|p| p.location == SecurityPayloadLocation::Uri)
            .expect("Should have URI payload");

        assert!(
            uri_payload.inject_as_path.is_none(),
            "URI with query params should NOT have inject_as_path"
        );
    }

    #[test]
    fn test_body_payload_gets_form_encoded_body() {
        let yaml = r#"
meta:
  author: test
  description: Form body test
  enabled: true
  name: test.yaml

tests:
  - desc: "Form-encoded body attack"
    test_title: "942432-1"
    stages:
      - stage:
          input:
            dest_addr: 127.0.0.1
            headers:
              Host: localhost
            method: POST
            port: 80
            uri: "/"
            data: "var=%3B%3Bdd+foo+bar"
          output:
            log_contains: id "942432"
"#;

        let file: WafBenchFile = serde_yaml::from_str(yaml).unwrap();
        let mut loader = WafBenchLoader::new();
        loader.stats.files_processed += 1;

        let category = SecurityCategory::SqlInjection;
        for test in &file.tests {
            if let Some(test_case) = loader.parse_test_case(test, category) {
                loader.test_cases.push(test_case);
            }
        }

        let payloads = loader.to_security_payloads();
        let body_payload = payloads
            .iter()
            .find(|p| p.location == SecurityPayloadLocation::Body)
            .expect("Should have body payload");

        assert!(
            body_payload.form_encoded_body.is_some(),
            "Body payload should have form_encoded_body set"
        );
        // Pre-encoded CRS YAML value round-trips through ensure_form_encoded
        assert_eq!(
            body_payload.form_encoded_body.as_deref().unwrap(),
            "var=%3B%3Bdd+foo+bar",
            "form_encoded_body should be properly URL-encoded"
        );
    }

    #[test]
    fn test_body_payload_decoded_yaml_gets_encoded() {
        // CRS YAML with already-decoded data value (some CRS distributions)
        let yaml = r#"
meta:
  author: test
  description: Form body test (decoded)
  enabled: true
  name: test.yaml

tests:
  - desc: "Form-encoded body attack (decoded)"
    test_title: "942432-2"
    stages:
      - stage:
          input:
            dest_addr: 127.0.0.1
            headers:
              Host: localhost
            method: POST
            port: 80
            uri: "/"
            data: "var=;;dd foo bar"
          output:
            log_contains: id "942432"
"#;

        let file: WafBenchFile = serde_yaml::from_str(yaml).unwrap();
        let mut loader = WafBenchLoader::new();
        loader.stats.files_processed += 1;

        let category = SecurityCategory::SqlInjection;
        for test in &file.tests {
            if let Some(test_case) = loader.parse_test_case(test, category) {
                loader.test_cases.push(test_case);
            }
        }

        let payloads = loader.to_security_payloads();
        let body_payload = payloads
            .iter()
            .find(|p| p.location == SecurityPayloadLocation::Body)
            .expect("Should have body payload");

        assert!(
            body_payload.form_encoded_body.is_some(),
            "Body payload should have form_encoded_body set"
        );
        // Decoded input must be re-encoded for WAF ARGS parsing
        let encoded = body_payload.form_encoded_body.as_deref().unwrap();
        assert!(
            encoded.contains("%3B%3B") || encoded.contains("%3b%3b"),
            "Semicolons must be URL-encoded: {encoded}"
        );
        assert!(!encoded.contains(' '), "Spaces must be encoded as + in form body: {encoded}");
        assert!(encoded.starts_with("var="), "Form key must be preserved: {encoded}");
    }

    #[test]
    fn test_parse_crs_v33_format() {
        // CRS v3.3/master uses a nested stage: wrapper
        let yaml = r#"
meta:
  author: "Christian Folini"
  description: Various SQL injection tests
  enabled: true
  name: 942100.yaml

tests:
  - test_title: 942100-1
    desc: "Simple SQL Injection"
    stages:
      - stage:
          input:
            dest_addr: 127.0.0.1
            headers:
              Host: localhost
            method: POST
            port: 80
            uri: "/"
            data: "var=1234 OR 1=1"
            version: HTTP/1.0
          output:
            log_contains: id "942100"
"#;

        let file: WafBenchFile = serde_yaml::from_str(yaml).unwrap();
        assert!(file.meta.enabled);
        assert_eq!(file.tests.len(), 1);
        assert_eq!(file.tests[0].test_title, "942100-1");

        // Verify we can get the input from nested format
        let stage = &file.tests[0].stages[0];
        let input = stage.get_input().expect("Should have input");
        assert_eq!(input.method, "POST");
        assert_eq!(input.data.as_deref(), Some("var=1234 OR 1=1"));
    }
}

#[cfg(test)]
mod simple_traffic_tests {
    use super::*;

    /// Srikanth's actual file shape from #79 (r65 item b). Before #987 this
    /// failed with `invalid type: sequence, expected struct WafBenchFile`.
    const SRIKANTH_YAML: &str = r#"
- title: utf-7 charset blocked
  request:
    method: POST
    uri: /graphql
    headers:
      Content-Type: application/json; charset=utf-7
    body: '+AHsAIgBxAHUAZQByAHkAIgA6ACIAewBfAF8AdAB5AHAAZQBuAGEAbQBlAH0AIgB9-'
  expected: 403
- title: utf-8 charset allowed
  request:
    method: POST
    uri: /graphql
    headers:
      Content-Type: application/json; charset=utf-8
    body: '{"query":"{__typename}"}'
  expected: 200
"#;

    #[test]
    fn simple_sequence_parses_and_maps_every_field() {
        let file = parse_traffic_file(SRIKANTH_YAML, "test_cases.yaml").expect("should parse");
        assert_eq!(file.tests.len(), 2);
        assert!(file.meta.enabled, "synthesised meta must not disable the file");

        let first = &file.tests[0];
        assert_eq!(first.test_title, "utf-7 charset blocked");
        let stage = &first.stages[0];
        let input = stage.input.as_ref().expect("input mapped");
        assert_eq!(input.method, "POST");
        assert_eq!(input.uri.as_deref(), Some("/graphql"));
        assert_eq!(
            input.headers.get("Content-Type").map(String::as_str),
            Some("application/json; charset=utf-7")
        );
        assert!(
            input.data.as_deref().unwrap().starts_with("+AHsAIgBxAHUAZQ"),
            "`body` must map onto WAFBench's `data`, payload intact"
        );
        assert_eq!(stage.output.as_ref().unwrap().status, vec![403]);
    }

    /// The WAFBench shape must keep working unchanged.
    #[test]
    fn wafbench_document_still_parses() {
        let yaml = r#"
meta:
  author: crs
  enabled: true
  name: 941100
tests:
  - test_title: 941100-1
    stages:
      - stage:
          input:
            method: GET
            uri: /?x=<script>alert(1)</script>
          output:
            status: [403]
"#;
        let file = parse_traffic_file(yaml, "941100.yaml").expect("should parse");
        assert_eq!(file.tests.len(), 1);
        assert_eq!(file.meta.author.as_deref(), Some("crs"));
    }

    /// `expected` is permissive because generators are inconsistent about it.
    #[test]
    fn expected_accepts_scalar_sequence_and_mapping() {
        let scalar: serde_yaml::Value = serde_yaml::from_str("403").unwrap();
        assert_eq!(extract_expected_statuses(Some(&scalar)), vec![403]);

        let seq: serde_yaml::Value = serde_yaml::from_str("[403, 406]").unwrap();
        assert_eq!(extract_expected_statuses(Some(&seq)), vec![403, 406]);

        let map: serde_yaml::Value = serde_yaml::from_str("status: 403").unwrap();
        assert_eq!(extract_expected_statuses(Some(&map)), vec![403]);

        let map_seq: serde_yaml::Value = serde_yaml::from_str("status: [403, 406]").unwrap();
        assert_eq!(extract_expected_statuses(Some(&map_seq)), vec![403, 406]);
    }

    /// An unparsable expectation must not cost the case: the request is still
    /// valid traffic to send.
    #[test]
    fn unusable_expected_yields_no_statuses_not_an_error() {
        let junk: serde_yaml::Value = serde_yaml::from_str("'not a status'").unwrap();
        assert!(extract_expected_statuses(Some(&junk)).is_empty());
        assert!(extract_expected_statuses(None).is_empty());

        let yaml = "- request:\n    uri: /a\n";
        let file = parse_traffic_file(yaml, "x.yaml").expect("case without expected still parses");
        assert_eq!(file.tests.len(), 1);
        assert_eq!(
            file.tests[0].stages[0].input.as_ref().unwrap().method,
            "GET",
            "method defaults"
        );
        assert_eq!(file.tests[0].test_title, "case-1", "title is synthesised when absent");
    }

    /// The error must name BOTH accepted shapes. The old message reported only
    /// the struct that failed, which told the user nothing about what to write.
    #[test]
    fn parse_error_names_both_accepted_shapes() {
        let err = parse_traffic_file("just a string", "bad.yaml").unwrap_err().to_string();
        assert!(err.contains("meta"), "must mention the WAFBench shape: {err}");
        assert!(err.contains("tests"), "must mention the WAFBench shape: {err}");
        assert!(err.contains("expected"), "must mention the simple shape: {err}");
        assert!(err.contains("request"), "must mention the simple shape: {err}");
    }
}

#[cfg(test)]
mod verbatim_tests {
    use super::*;

    fn load(yaml: &str) -> Vec<crate::request_gen::RequestTemplate> {
        let file = parse_traffic_file(yaml, "t.yaml").expect("parses");
        let mut loader = WafBenchLoader::new();
        for test in &file.tests {
            if let Some(case) = loader.parse_test_case(test, SecurityCategory::Xss) {
                loader.test_cases.push(case);
            }
        }
        traffic_cases_to_templates(loader.test_cases())
    }

    /// Srikanth's OAuth case from #79. The default path collapses this to the
    /// bare string `totally-unsupported` and sends it to a spec endpoint as
    /// `?test=`, so a rule chained on `ARGS:redirect_uri` can never fire.
    /// Verbatim mode must put every parameter on the wire.
    #[test]
    fn verbatim_preserves_every_query_parameter() {
        let t = load(
            "- title: oauth open redirect\n  request:\n    method: GET\n    uri: /oauth/authorize?response_type=totally-unsupported&redirect_uri=https%3A%2F%2Fevil.example%2Flanding&state=s1\n  expected: 403\n",
        );
        assert_eq!(t.len(), 1);
        let path = t[0].generate_path();
        assert!(path.starts_with("/oauth/authorize"), "path must survive, got {path}");
        assert!(path.contains("redirect_uri="), "redirect_uri must reach the wire: {path}");
        assert!(path.contains("response_type="), "response_type must reach the wire: {path}");
        assert!(path.contains("state=s1"), "trailing params must survive: {path}");
        assert_eq!(t[0].operation.method, "GET");
    }

    /// The encoding IS the payload for charset and traversal tests, so the URI
    /// must survive byte for byte. This is why `query_params` is left empty
    /// rather than parsed out and rejoined.
    #[test]
    fn verbatim_does_not_reencode_the_uri() {
        let t = load(
            "- title: encoded\n  request:\n    method: GET\n    uri: /a?u=https%3A%2F%2Fevil.example%2Fx&b=%2E%2E%2F\n",
        );
        let path = t[0].generate_path();
        assert!(path.contains("https%3A%2F%2Fevil.example%2Fx"), "must not decode: {path}");
        assert!(path.contains("%2E%2E%2F"), "must not normalise traversal: {path}");
        assert!(!path.contains("://evil.example"), "must not decode to a real URL: {path}");
    }

    #[test]
    fn verbatim_carries_headers_and_body() {
        let t = load(
            "- title: post\n  request:\n    method: POST\n    uri: /graphql\n    headers:\n      Content-Type: application/json; charset=utf-7\n    body: '{\"query\":\"{__typename}\"}'\n",
        );
        assert_eq!(t[0].operation.method, "POST");
        assert_eq!(
            t[0].headers.get("Content-Type").map(String::as_str),
            Some("application/json; charset=utf-7"),
            "the charset is the attack; it must not be normalised"
        );
        assert!(t[0].body.is_some(), "body must be carried");
    }

    /// A non-JSON body must pass through rather than being dropped or mangled.
    #[test]
    fn verbatim_passes_through_non_json_body() {
        let t = load("- title: form\n  request:\n    method: POST\n    uri: /f\n    body: 'a=1&b=<script>'\n");
        let body = t[0].body.as_ref().expect("body carried");
        assert_eq!(body.as_str(), Some("a=1&b=<script>"), "raw body preserved verbatim");
    }

    /// Cases with no URI cannot be sent, and must be skipped rather than
    /// producing a request against the base URL.
    #[test]
    fn verbatim_skips_cases_without_a_uri() {
        let t = load("- title: no uri\n  request:\n    method: GET\n");
        assert!(t.is_empty(), "a case with no uri yields no request");
    }

    /// #997: a case marked `omit_rule: true` asks for the request to be sent
    /// with the rule under test disabled. mockforge cannot do that, and sending
    /// it anyway produces a byte-identical twin of the non-omitted case with the
    /// opposite expectation, so one of the pair always fails no matter what the
    /// WAF does. It must be dropped, not silently sent.
    #[test]
    fn omit_rule_cases_are_skipped_not_silently_sent() {
        let yaml = r#"
- title: unsupported response_type blocked
  omit_rule: false
  request:
    method: GET
    uri: /oauth/authorize?response_type=totally-unsupported&redirect_uri=https%3A%2F%2Fevil.example%2Flanding
  expected: 403
- title: "baseline: same payload with rule omitted"
  omit_rule: true
  request:
    method: GET
    uri: /oauth/authorize?response_type=totally-unsupported&redirect_uri=https%3A%2F%2Fevil.example%2Flanding
  expected: 200
- title: legitimate flow allowed
  request:
    method: GET
    uri: /oauth/authorize?response_type=code&client_id=abc123
  expected: 200
"#;
        let parsed = parse_traffic_file(yaml, "oauth.yaml").expect("simple format should parse");

        let titles: Vec<_> = parsed.tests.iter().map(|t| t.test_title.clone()).collect();
        assert_eq!(
            parsed.tests.len(),
            2,
            "the omit_rule:true case must be dropped, got {titles:?}"
        );
        assert!(
            !titles.iter().any(|t| t.contains("rule omitted")),
            "baseline case leaked into the run: {titles:?}"
        );
        // omit_rule:false is an ordinary case and must survive.
        assert!(
            titles.iter().any(|t| t.contains("unsupported response_type blocked")),
            "omit_rule:false must not be treated as omitted: {titles:?}"
        );
        assert_eq!(parsed.omitted_count, 1, "omitted_count must match the dropped baseline");
    }

    /// #79: per-file breakdown so a directory of YAML files reports
    /// attack (403) vs normal (200) vs omitted before k6 starts.
    #[test]
    fn traffic_file_summary_splits_attack_normal_omitted() {
        let yaml = r#"
- title: blocked
  request:
    method: GET
    uri: /oauth/authorize?response_type=totally-unsupported
  expected: 403
- title: allowed
  request:
    method: GET
    uri: /oauth/authorize?response_type=code
  expected: 200
- title: baseline
  omit_rule: true
  request:
    method: GET
    uri: /oauth/authorize?response_type=totally-unsupported
  expected: 200
"#;
        let parsed = parse_traffic_file(yaml, "oauth.yaml").expect("parse");
        let summary = summarize_traffic_file(&parsed, "oauth.yaml");
        assert_eq!(summary.sent, 2);
        assert_eq!(summary.attack, 1);
        assert_eq!(summary.normal, 1);
        assert_eq!(summary.omitted, 1);
        assert_eq!(summary.other, 0);
    }

    #[test]
    fn missing_wafbench_path_is_an_error() {
        let mut loader = WafBenchLoader::new();
        let err = loader
            .load_from_pattern("definitely-not-a-real-file-apisix.yaml")
            .expect_err("missing path must not look like an empty payload pool");
        let msg = err.to_string();
        assert!(msg.contains("does not exist"), "error must name the missing path, got: {msg}");
    }
}
