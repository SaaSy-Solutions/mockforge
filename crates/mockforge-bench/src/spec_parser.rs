//! OpenAPI specification parsing for load testing

use crate::error::{BenchError, Result};
use mockforge_core::openapi::spec::OpenApiSpec;
use openapiv3::{OpenAPI, Operation, PathItem, ReferenceOr};
use std::path::Path;

/// An API operation extracted from an OpenAPI spec
#[derive(Debug, Clone)]
pub struct ApiOperation {
    pub method: String,
    pub path: String,
    pub operation: Operation,
    pub operation_id: Option<String>,
}

impl ApiOperation {
    /// Get a display name for this operation
    pub fn display_name(&self) -> String {
        self.operation_id
            .clone()
            .unwrap_or_else(|| format!("{} {}", self.method.to_uppercase(), self.path))
    }
}

/// Parse OpenAPI specification and extract operations
pub struct SpecParser {
    spec: OpenAPI,
}

impl SpecParser {
    /// Load and parse an OpenAPI spec from a file
    pub async fn from_file(path: &Path) -> Result<Self> {
        let spec = OpenApiSpec::from_file(path)
            .await
            .map_err(|e| BenchError::SpecParseError(e.to_string()))?;

        Ok(Self {
            spec: spec.spec.clone(),
        })
    }

    /// Get all operations from the spec
    pub fn get_operations(&self) -> Vec<ApiOperation> {
        let mut operations = Vec::new();

        for (path, path_item) in &self.spec.paths.paths {
            if let ReferenceOr::Item(item) = path_item {
                self.extract_operations_from_path(path, item, &mut operations);
            }
        }

        operations
    }

    /// Filter operations by method and path pattern
    pub fn filter_operations(&self, filter: &str) -> Result<Vec<ApiOperation>> {
        let all_ops = self.get_operations();

        if filter.is_empty() {
            return Ok(all_ops);
        }

        let filters: Vec<&str> = filter.split(',').map(|s| s.trim()).collect();
        let mut filtered = Vec::new();

        for filter_str in filters {
            let parts: Vec<&str> = filter_str.splitn(2, ' ').collect();
            if parts.len() != 2 {
                return Err(BenchError::Other(format!(
                    "Invalid operation filter format: '{}'. Expected 'METHOD /path'",
                    filter_str
                )));
            }

            let method = parts[0].to_uppercase();
            let path_pattern = parts[1];

            for op in &all_ops {
                if op.method.to_uppercase() == method && Self::matches_path(&op.path, path_pattern)
                {
                    filtered.push(op.clone());
                }
            }
        }

        if filtered.is_empty() {
            return Err(BenchError::OperationNotFound(filter.to_string()));
        }

        Ok(filtered)
    }

    /// Extract operations from a path item
    fn extract_operations_from_path(
        &self,
        path: &str,
        path_item: &PathItem,
        operations: &mut Vec<ApiOperation>,
    ) {
        let method_ops = vec![
            ("get", &path_item.get),
            ("post", &path_item.post),
            ("put", &path_item.put),
            ("delete", &path_item.delete),
            ("patch", &path_item.patch),
            ("head", &path_item.head),
            ("options", &path_item.options),
        ];

        for (method, op_ref) in method_ops {
            if let Some(op) = op_ref {
                operations.push(ApiOperation {
                    method: method.to_string(),
                    path: path.to_string(),
                    operation: op.clone(),
                    operation_id: op.operation_id.clone(),
                });
            }
        }
    }

    /// Check if a path matches a pattern (supports wildcards)
    fn matches_path(path: &str, pattern: &str) -> bool {
        if pattern == path {
            return true;
        }

        // Simple wildcard matching
        if pattern.ends_with('*') {
            let prefix = &pattern[..pattern.len() - 1];
            return path.starts_with(prefix);
        }

        false
    }

    /// Get the base URL from the spec (if available)
    pub fn get_base_url(&self) -> Option<String> {
        self.spec.servers.first().map(|server| server.url.clone())
    }

    /// Get API info
    pub fn get_info(&self) -> (String, String) {
        (self.spec.info.title.clone(), self.spec.info.version.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_path() {
        assert!(SpecParser::matches_path("/users", "/users"));
        assert!(SpecParser::matches_path("/users/123", "/users/*"));
        assert!(!SpecParser::matches_path("/posts", "/users"));
    }
}
