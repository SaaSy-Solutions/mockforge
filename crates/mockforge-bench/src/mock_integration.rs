//! Mock server integration for coordinated testing
//!
//! This module provides functionality to detect and integrate with
//! MockForge mock servers for stateful testing scenarios.

use crate::error::{BenchError, Result};
use serde::{Deserialize, Serialize};

/// Configuration for mock server integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockIntegrationConfig {
    /// Whether the target is a MockForge mock server
    pub is_mock_server: bool,
    /// Enable stateful mode on the mock server
    pub enable_stateful: bool,
    /// VU-based ID generation mode
    pub vu_based_ids: bool,
    /// Collect mock server metrics after test
    pub collect_metrics: bool,
}

impl Default for MockIntegrationConfig {
    fn default() -> Self {
        Self {
            is_mock_server: false,
            enable_stateful: true,
            vu_based_ids: true,
            collect_metrics: true,
        }
    }
}

impl MockIntegrationConfig {
    /// Create config for mock server target
    pub fn mock_server() -> Self {
        Self {
            is_mock_server: true,
            ..Default::default()
        }
    }

    /// Create config for real API target
    pub fn real_api() -> Self {
        Self {
            is_mock_server: false,
            enable_stateful: false,
            vu_based_ids: false,
            collect_metrics: false,
        }
    }

    /// Enable or disable stateful mode
    pub fn with_stateful(mut self, enabled: bool) -> Self {
        self.enable_stateful = enabled;
        self
    }

    /// Enable or disable VU-based ID generation
    pub fn with_vu_based_ids(mut self, enabled: bool) -> Self {
        self.vu_based_ids = enabled;
        self
    }
}

/// Mock server detection result
#[derive(Debug, Clone)]
pub struct MockServerInfo {
    /// Whether the target is a MockForge mock server
    pub is_mockforge: bool,
    /// MockForge version (if detected)
    pub version: Option<String>,
    /// Available control endpoints
    pub control_endpoints: Vec<String>,
    /// Current stateful mode status
    pub stateful_enabled: bool,
}

impl Default for MockServerInfo {
    fn default() -> Self {
        Self {
            is_mockforge: false,
            version: None,
            control_endpoints: Vec::new(),
            stateful_enabled: false,
        }
    }
}

/// Detects if a target URL is a MockForge mock server
pub struct MockServerDetector;

impl MockServerDetector {
    /// Check if a URL points to a MockForge mock server
    ///
    /// Makes a request to `/__mockforge/info` endpoint to detect MockForge servers.
    pub async fn detect(target_url: &str) -> Result<MockServerInfo> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| BenchError::Other(format!("Failed to create HTTP client: {}", e)))?;

        let info_url = format!("{}/__mockforge/info", target_url.trim_end_matches('/'));

        match client.get(&info_url).send().await {
            Ok(response) if response.status().is_success() => {
                let body: serde_json::Value = response
                    .json()
                    .await
                    .unwrap_or_else(|_| serde_json::json!({}));

                Ok(MockServerInfo {
                    is_mockforge: true,
                    version: body.get("version").and_then(|v| v.as_str()).map(String::from),
                    control_endpoints: vec![
                        "/__mockforge/config".to_string(),
                        "/__mockforge/state".to_string(),
                        "/__mockforge/metrics".to_string(),
                    ],
                    stateful_enabled: body
                        .get("stateful")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                })
            }
            _ => Ok(MockServerInfo::default()),
        }
    }

    /// Quick check if target looks like a mock server (without HTTP request)
    pub fn looks_like_mock_server(target_url: &str) -> bool {
        let url_lower = target_url.to_lowercase();
        url_lower.contains("mock")
            || url_lower.contains("localhost")
            || url_lower.contains("127.0.0.1")
            || url_lower.contains(":3000")
            || url_lower.contains(":8000")
            || url_lower.contains(":8080")
    }
}

/// Generates k6 JavaScript code for mock server integration
pub struct MockIntegrationGenerator;

impl MockIntegrationGenerator {
    /// Generate k6 setup code for mock server
    pub fn generate_setup(config: &MockIntegrationConfig) -> String {
        if !config.is_mock_server {
            return "// Real API target - no mock server setup needed\n".to_string();
        }

        let mut code = String::new();

        code.push_str("// MockForge mock server integration\n");
        code.push_str("export function setup() {\n");
        code.push_str("  const configUrl = `${BASE_URL}/__mockforge/config`;\n");
        code.push_str("  \n");

        if config.enable_stateful {
            code.push_str("  // Enable stateful mode for CRUD testing\n");
            code.push_str("  const statefulConfig = {\n");
            code.push_str("    stateful: true,\n");
            if config.vu_based_ids {
                code.push_str("    vuBasedIds: true,\n");
            }
            code.push_str("  };\n");
            code.push_str("  \n");
            code.push_str("  const configRes = http.post(configUrl, JSON.stringify(statefulConfig), {\n");
            code.push_str("    headers: { 'Content-Type': 'application/json' }\n");
            code.push_str("  });\n");
            code.push_str("  \n");
            code.push_str("  if (configRes.status !== 200) {\n");
            code.push_str("    console.warn('Failed to configure mock server:', configRes.status);\n");
            code.push_str("  }\n");
        }

        code.push_str("  \n");
        code.push_str("  return { mockServerConfigured: true };\n");
        code.push_str("}\n");

        code
    }

    /// Generate k6 teardown code for collecting mock server metrics
    pub fn generate_teardown(config: &MockIntegrationConfig) -> String {
        if !config.is_mock_server || !config.collect_metrics {
            return "// No mock server teardown needed\n".to_string();
        }

        let mut code = String::new();

        code.push_str("// Collect mock server metrics after test\n");
        code.push_str("export function teardown(data) {\n");
        code.push_str("  if (!data.mockServerConfigured) return;\n");
        code.push_str("  \n");
        code.push_str("  const metricsUrl = `${BASE_URL}/__mockforge/metrics`;\n");
        code.push_str("  const metricsRes = http.get(metricsUrl);\n");
        code.push_str("  \n");
        code.push_str("  if (metricsRes.status === 200) {\n");
        code.push_str("    try {\n");
        code.push_str("      const metrics = metricsRes.json();\n");
        code.push_str("      console.log('\\n=== Mock Server Metrics ===');\n");
        code.push_str("      console.log(`Total Requests: ${metrics.totalRequests || 0}`);\n");
        code.push_str("      console.log(`Matched Routes: ${metrics.matchedRoutes || 0}`);\n");
        code.push_str("      console.log(`Unmatched Routes: ${metrics.unmatchedRoutes || 0}`);\n");
        code.push_str("      if (metrics.statefulOperations) {\n");
        code.push_str("        console.log(`Stateful Creates: ${metrics.statefulOperations.creates || 0}`);\n");
        code.push_str("        console.log(`Stateful Reads: ${metrics.statefulOperations.reads || 0}`);\n");
        code.push_str("        console.log(`Stateful Updates: ${metrics.statefulOperations.updates || 0}`);\n");
        code.push_str("        console.log(`Stateful Deletes: ${metrics.statefulOperations.deletes || 0}`);\n");
        code.push_str("      }\n");
        code.push_str("      console.log('===========================\\n');\n");
        code.push_str("    } catch (e) {\n");
        code.push_str("      console.warn('Failed to parse mock server metrics:', e);\n");
        code.push_str("    }\n");
        code.push_str("  }\n");
        code.push_str("  \n");

        if config.enable_stateful {
            code.push_str("  // Reset mock server state\n");
            code.push_str("  const resetUrl = `${BASE_URL}/__mockforge/state/reset`;\n");
            code.push_str("  http.post(resetUrl);\n");
        }

        code.push_str("}\n");

        code
    }

    /// Generate k6 code for VU-based consistent IDs
    pub fn generate_vu_id_helper() -> String {
        r#"// Generate consistent VU-based ID for mock server
function getVuBasedId(prefix = 'resource') {
  return `${prefix}-vu${__VU}-${__ITER}`;
}

// Store created resources for cleanup
const createdResources = [];

function trackResource(id) {
  createdResources.push(id);
}
"#
        .to_string()
    }

    /// Generate k6 code for mock server health check
    pub fn generate_health_check() -> String {
        r#"// Check mock server health before starting
function checkMockServerHealth() {
  const healthUrl = `${BASE_URL}/__mockforge/health`;
  const res = http.get(healthUrl, { timeout: '5s' });

  if (res.status !== 200) {
    console.error('Mock server health check failed:', res.status);
    return false;
  }

  return true;
}
"#
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_integration_config_default() {
        let config = MockIntegrationConfig::default();
        assert!(!config.is_mock_server);
        assert!(config.enable_stateful);
        assert!(config.vu_based_ids);
        assert!(config.collect_metrics);
    }

    #[test]
    fn test_mock_integration_config_mock_server() {
        let config = MockIntegrationConfig::mock_server();
        assert!(config.is_mock_server);
        assert!(config.enable_stateful);
    }

    #[test]
    fn test_mock_integration_config_real_api() {
        let config = MockIntegrationConfig::real_api();
        assert!(!config.is_mock_server);
        assert!(!config.enable_stateful);
        assert!(!config.vu_based_ids);
        assert!(!config.collect_metrics);
    }

    #[test]
    fn test_mock_integration_config_builders() {
        let config = MockIntegrationConfig::mock_server()
            .with_stateful(false)
            .with_vu_based_ids(false);

        assert!(config.is_mock_server);
        assert!(!config.enable_stateful);
        assert!(!config.vu_based_ids);
    }

    #[test]
    fn test_looks_like_mock_server() {
        assert!(MockServerDetector::looks_like_mock_server("http://localhost:3000"));
        assert!(MockServerDetector::looks_like_mock_server("http://127.0.0.1:8080"));
        assert!(MockServerDetector::looks_like_mock_server("http://mock-api.local"));
        assert!(!MockServerDetector::looks_like_mock_server("https://api.example.com"));
    }

    #[test]
    fn test_generate_setup_real_api() {
        let config = MockIntegrationConfig::real_api();
        let code = MockIntegrationGenerator::generate_setup(&config);
        assert!(code.contains("no mock server setup"));
    }

    #[test]
    fn test_generate_setup_mock_server() {
        let config = MockIntegrationConfig::mock_server();
        let code = MockIntegrationGenerator::generate_setup(&config);
        assert!(code.contains("export function setup()"));
        assert!(code.contains("__mockforge/config"));
        assert!(code.contains("stateful: true"));
    }

    #[test]
    fn test_generate_setup_with_vu_based_ids() {
        let config = MockIntegrationConfig::mock_server().with_vu_based_ids(true);
        let code = MockIntegrationGenerator::generate_setup(&config);
        assert!(code.contains("vuBasedIds: true"));
    }

    #[test]
    fn test_generate_teardown_real_api() {
        let config = MockIntegrationConfig::real_api();
        let code = MockIntegrationGenerator::generate_teardown(&config);
        assert!(code.contains("No mock server teardown"));
    }

    #[test]
    fn test_generate_teardown_mock_server() {
        let config = MockIntegrationConfig::mock_server();
        let code = MockIntegrationGenerator::generate_teardown(&config);
        assert!(code.contains("export function teardown"));
        assert!(code.contains("__mockforge/metrics"));
        assert!(code.contains("Mock Server Metrics"));
    }

    #[test]
    fn test_generate_teardown_with_state_reset() {
        let config = MockIntegrationConfig::mock_server().with_stateful(true);
        let code = MockIntegrationGenerator::generate_teardown(&config);
        assert!(code.contains("__mockforge/state/reset"));
    }

    #[test]
    fn test_generate_vu_id_helper() {
        let code = MockIntegrationGenerator::generate_vu_id_helper();
        assert!(code.contains("getVuBasedId"));
        assert!(code.contains("__VU"));
        assert!(code.contains("__ITER"));
        assert!(code.contains("createdResources"));
    }

    #[test]
    fn test_generate_health_check() {
        let code = MockIntegrationGenerator::generate_health_check();
        assert!(code.contains("checkMockServerHealth"));
        assert!(code.contains("__mockforge/health"));
    }

    #[test]
    fn test_mock_server_info_default() {
        let info = MockServerInfo::default();
        assert!(!info.is_mockforge);
        assert!(info.version.is_none());
        assert!(info.control_endpoints.is_empty());
        assert!(!info.stateful_enabled);
    }
}
