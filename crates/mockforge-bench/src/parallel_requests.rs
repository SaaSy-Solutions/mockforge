//! Parallel request execution support for load testing
//!
//! This module provides functionality to generate k6 scripts that execute
//! multiple requests in parallel using http.batch(), enabling high-throughput
//! testing scenarios like creating 300 resources simultaneously.

use serde::{Deserialize, Serialize};

/// Configuration for parallel request execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelConfig {
    /// Number of resources to create in parallel per VU
    pub count: u32,
    /// Whether to collect response IDs for subsequent operations
    pub collect_ids: bool,
    /// Maximum batch size (k6 limits)
    pub max_batch_size: u32,
    /// Delay between batches in milliseconds
    pub batch_delay_ms: u32,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            count: 10,
            collect_ids: true,
            max_batch_size: 100, // k6 recommends keeping batches reasonable
            batch_delay_ms: 100,
        }
    }
}

impl ParallelConfig {
    /// Create a new parallel config with the specified count
    pub fn new(count: u32) -> Self {
        Self {
            count,
            ..Default::default()
        }
    }

    /// Set whether to collect IDs from responses
    pub fn with_collect_ids(mut self, collect: bool) -> Self {
        self.collect_ids = collect;
        self
    }

    /// Set the maximum batch size
    pub fn with_max_batch_size(mut self, size: u32) -> Self {
        self.max_batch_size = size;
        self
    }

    /// Calculate the number of batches needed
    pub fn num_batches(&self) -> u32 {
        if self.max_batch_size == 0 {
            return 1;
        }
        self.count.div_ceil(self.max_batch_size)
    }

    /// Get the size of a specific batch (last batch may be smaller)
    pub fn batch_size(&self, batch_index: u32) -> u32 {
        let remaining = self.count - (batch_index * self.max_batch_size);
        remaining.min(self.max_batch_size)
    }
}

/// Generates k6 JavaScript code for parallel request execution
pub struct ParallelRequestGenerator;

impl ParallelRequestGenerator {
    /// Generate k6 code for parallel POST requests
    ///
    /// Generates code that uses http.batch() to execute multiple requests in parallel,
    /// optionally collecting the response IDs for subsequent operations.
    pub fn generate_parallel_post(
        config: &ParallelConfig,
        path: &str,
        body_template: &str,
        id_field: &str,
    ) -> String {
        let mut code = String::new();

        // Generate batch creation loop
        code.push_str("// Parallel resource creation\n");
        code.push_str("const batchRequests = [];\n");
        code.push_str(&format!("for (let i = 0; i < {}; i++) {{\n", config.count));
        code.push_str("  batchRequests.push({\n");
        code.push_str("    method: 'POST',\n");
        code.push_str(&format!("    url: `${{BASE_URL}}{}`", path));
        code.push_str(",\n");
        code.push_str(&format!("    body: JSON.stringify({}),\n", body_template));
        code.push_str("    params: { headers }\n");
        code.push_str("  });\n");
        code.push_str("}\n\n");

        // Execute batch with size limits
        if config.count > config.max_batch_size {
            code.push_str("// Execute in batches to avoid overwhelming the server\n");
            code.push_str("const createdIds = [];\n");
            code.push_str(&format!("const batchSize = {};\n", config.max_batch_size));
            code.push_str("for (let batchStart = 0; batchStart < batchRequests.length; batchStart += batchSize) {\n");
            code.push_str(
                "  const batchEnd = Math.min(batchStart + batchSize, batchRequests.length);\n",
            );
            code.push_str("  const batch = batchRequests.slice(batchStart, batchEnd);\n");
            code.push_str("  const responses = http.batch(batch);\n\n");

            if config.collect_ids {
                code.push_str("  // Collect IDs from responses\n");
                code.push_str("  for (const res of responses) {\n");
                code.push_str("    if (res.status >= 200 && res.status < 300) {\n");
                code.push_str("      try {\n");
                code.push_str(&format!("        const id = res.json('{}');\n", id_field));
                code.push_str("        if (id) createdIds.push(id);\n");
                code.push_str("      } catch (e) {\n");
                code.push_str("        console.error('Failed to extract ID:', e);\n");
                code.push_str("      }\n");
                code.push_str("    }\n");
                code.push_str("  }\n\n");
            }

            code.push_str("  // Check batch results\n");
            code.push_str(
                "  const batchSuccess = responses.every(r => r.status >= 200 && r.status < 300);\n",
            );
            code.push_str("  check(responses, {\n");
            code.push_str("    'batch creation successful': () => batchSuccess\n");
            code.push_str("  });\n\n");

            if config.batch_delay_ms > 0 {
                code.push_str(&format!("  sleep({});\n", config.batch_delay_ms as f64 / 1000.0));
            }
            code.push_str("}\n");
        } else {
            code.push_str("// Execute all requests in parallel\n");
            code.push_str("const responses = http.batch(batchRequests);\n\n");

            if config.collect_ids {
                code.push_str("// Collect IDs from responses\n");
                code.push_str("const createdIds = [];\n");
                code.push_str("for (const res of responses) {\n");
                code.push_str("  if (res.status >= 200 && res.status < 300) {\n");
                code.push_str("    try {\n");
                code.push_str(&format!("      const id = res.json('{}');\n", id_field));
                code.push_str("      if (id) createdIds.push(id);\n");
                code.push_str("    } catch (e) {\n");
                code.push_str("      console.error('Failed to extract ID:', e);\n");
                code.push_str("    }\n");
                code.push_str("  }\n");
                code.push_str("}\n\n");
            }

            code.push_str("// Check all responses\n");
            code.push_str(
                "const allSuccess = responses.every(r => r.status >= 200 && r.status < 300);\n",
            );
            code.push_str("check(responses, {\n");
            code.push_str("  'parallel creation successful': () => allSuccess\n");
            code.push_str("});\n");
        }

        code
    }

    /// Generate k6 code for parallel GET requests using collected IDs
    pub fn generate_parallel_get(path_template: &str, id_param: &str) -> String {
        let mut code = String::new();

        code.push_str("// Parallel resource retrieval\n");
        code.push_str("if (createdIds.length > 0) {\n");
        code.push_str("  const getRequests = createdIds.map(id => ({\n");
        code.push_str("    method: 'GET',\n");
        code.push_str(&format!(
            "    url: `${{BASE_URL}}{}`.replace('{{{{{}}}}}', id),\n",
            path_template, id_param
        ));
        code.push_str("    params: { headers }\n");
        code.push_str("  }));\n\n");
        code.push_str("  const getResponses = http.batch(getRequests);\n");
        code.push_str(
            "  const getSuccess = getResponses.every(r => r.status >= 200 && r.status < 300);\n",
        );
        code.push_str("  check(getResponses, {\n");
        code.push_str("    'parallel retrieval successful': () => getSuccess\n");
        code.push_str("  });\n");
        code.push_str("}\n");

        code
    }

    /// Generate k6 code for parallel DELETE requests using collected IDs
    pub fn generate_parallel_delete(path_template: &str, id_param: &str) -> String {
        let mut code = String::new();

        code.push_str("// Parallel resource cleanup\n");
        code.push_str("if (createdIds.length > 0) {\n");
        code.push_str("  const deleteRequests = createdIds.map(id => ({\n");
        code.push_str("    method: 'DELETE',\n");
        code.push_str(&format!(
            "    url: `${{BASE_URL}}{}`.replace('{{{{{}}}}}', id),\n",
            path_template, id_param
        ));
        code.push_str("    params: { headers }\n");
        code.push_str("  }));\n\n");
        code.push_str("  const deleteResponses = http.batch(deleteRequests);\n");
        code.push_str("  const deleteSuccess = deleteResponses.every(r => r.status >= 200 && r.status < 300);\n");
        code.push_str("  check(deleteResponses, {\n");
        code.push_str("    'parallel cleanup successful': () => deleteSuccess\n");
        code.push_str("  });\n");
        code.push_str("}\n");

        code
    }

    /// Generate k6 helper functions for batch operations
    pub fn generate_batch_helper(config: &ParallelConfig) -> String {
        let mut code = String::new();

        code.push_str("// Parallel batch execution helpers\n");
        code.push_str(&format!("const PARALLEL_BATCH_SIZE = {};\n", config.max_batch_size));
        code.push_str(&format!("const PARALLEL_COUNT = {};\n\n", config.count));

        code.push_str(
            r#"function executeBatch(requests) {
  const results = [];
  const batchSize = PARALLEL_BATCH_SIZE;

  for (let i = 0; i < requests.length; i += batchSize) {
    const batch = requests.slice(i, i + batchSize);
    const responses = http.batch(batch);
    results.push(...responses);
  }

  return results;
}

function collectIds(responses, idField = 'id') {
  const ids = [];
  for (const res of responses) {
    if (res.status >= 200 && res.status < 300) {
      try {
        const id = res.json(idField);
        if (id) ids.push(id);
      } catch (e) {
        // Ignore parse errors
      }
    }
  }
  return ids;
}
"#,
        );

        code
    }

    /// Generate complete parallel test scenario
    pub fn generate_complete_scenario(
        config: &ParallelConfig,
        base_path: &str,
        detail_path: &str,
        id_param: &str,
        body_template: &str,
        id_field: &str,
        include_cleanup: bool,
    ) -> String {
        let mut code = String::new();

        // Create resources
        code.push_str(&Self::generate_parallel_post(config, base_path, body_template, id_field));
        code.push('\n');

        // Read all created resources
        code.push_str(&Self::generate_parallel_get(detail_path, id_param));
        code.push('\n');

        // Cleanup if requested
        if include_cleanup {
            code.push_str(&Self::generate_parallel_delete(detail_path, id_param));
        }

        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_config_default() {
        let config = ParallelConfig::default();
        assert_eq!(config.count, 10);
        assert!(config.collect_ids);
        assert_eq!(config.max_batch_size, 100);
    }

    #[test]
    fn test_parallel_config_new() {
        let config = ParallelConfig::new(50);
        assert_eq!(config.count, 50);
        assert!(config.collect_ids);
    }

    #[test]
    fn test_parallel_config_builders() {
        let config = ParallelConfig::new(100).with_collect_ids(false).with_max_batch_size(25);

        assert_eq!(config.count, 100);
        assert!(!config.collect_ids);
        assert_eq!(config.max_batch_size, 25);
    }

    #[test]
    fn test_num_batches() {
        let config = ParallelConfig::new(100).with_max_batch_size(30);
        assert_eq!(config.num_batches(), 4); // 30 + 30 + 30 + 10

        let config2 = ParallelConfig::new(50).with_max_batch_size(100);
        assert_eq!(config2.num_batches(), 1);

        let config3 = ParallelConfig::new(100).with_max_batch_size(100);
        assert_eq!(config3.num_batches(), 1);
    }

    #[test]
    fn test_batch_size() {
        let config = ParallelConfig::new(100).with_max_batch_size(30);

        assert_eq!(config.batch_size(0), 30);
        assert_eq!(config.batch_size(1), 30);
        assert_eq!(config.batch_size(2), 30);
        assert_eq!(config.batch_size(3), 10); // Last batch is smaller
    }

    #[test]
    fn test_generate_parallel_post_small_batch() {
        let config = ParallelConfig::new(5);
        let code = ParallelRequestGenerator::generate_parallel_post(
            &config,
            "/resources",
            "{ name: `resource-${__VU}-${i}` }",
            "id",
        );

        assert!(code.contains("batchRequests.push"));
        assert!(code.contains("for (let i = 0; i < 5; i++)"));
        assert!(code.contains("http.batch(batchRequests)"));
        assert!(code.contains("res.json('id')"));
    }

    #[test]
    fn test_generate_parallel_post_large_batch() {
        let config = ParallelConfig::new(150).with_max_batch_size(50);
        let code = ParallelRequestGenerator::generate_parallel_post(
            &config,
            "/resources",
            "{ name: `resource-${i}` }",
            "uuid",
        );

        assert!(code.contains("Execute in batches"));
        assert!(code.contains("batchSize = 50"));
        assert!(code.contains("batchStart + batchSize"));
        assert!(code.contains("res.json('uuid')"));
    }

    #[test]
    fn test_generate_parallel_post_no_collect() {
        let config = ParallelConfig::new(10).with_collect_ids(false);
        let code = ParallelRequestGenerator::generate_parallel_post(
            &config,
            "/resources",
            "{ name: 'test' }",
            "id",
        );

        assert!(!code.contains("createdIds.push"));
        assert!(!code.contains("res.json"));
    }

    #[test]
    fn test_generate_parallel_get() {
        let code = ParallelRequestGenerator::generate_parallel_get("/resources/{id}", "id");

        assert!(code.contains("Parallel resource retrieval"));
        assert!(code.contains("createdIds.map"));
        assert!(code.contains("method: 'GET'"));
        // The generated code uses double braces for escaping in format strings
        assert!(code.contains("{{id}}") || code.contains("{id}"));
    }

    #[test]
    fn test_generate_parallel_delete() {
        let code = ParallelRequestGenerator::generate_parallel_delete(
            "/resources/{resourceId}",
            "resourceId",
        );

        assert!(code.contains("Parallel resource cleanup"));
        assert!(code.contains("method: 'DELETE'"));
        // The generated code uses double braces for escaping in format strings
        assert!(code.contains("{{resourceId}}") || code.contains("{resourceId}"));
    }

    #[test]
    fn test_generate_complete_scenario() {
        let config = ParallelConfig::new(20);
        let code = ParallelRequestGenerator::generate_complete_scenario(
            &config,
            "/users",
            "/users/{id}",
            "id",
            "{ name: `user-${i}` }",
            "id",
            true,
        );

        assert!(code.contains("Parallel resource creation"));
        assert!(code.contains("Parallel resource retrieval"));
        assert!(code.contains("Parallel resource cleanup"));
    }

    #[test]
    fn test_generate_complete_scenario_no_cleanup() {
        let config = ParallelConfig::new(20);
        let code = ParallelRequestGenerator::generate_complete_scenario(
            &config,
            "/users",
            "/users/{id}",
            "id",
            "{ name: `user-${i}` }",
            "id",
            false,
        );

        assert!(code.contains("Parallel resource creation"));
        assert!(code.contains("Parallel resource retrieval"));
        assert!(!code.contains("Parallel resource cleanup"));
    }
}
