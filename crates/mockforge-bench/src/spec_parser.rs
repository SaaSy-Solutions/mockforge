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

    /// Exclude operations matching method and path patterns
    ///
    /// This is the inverse of filter_operations - it returns all operations
    /// EXCEPT those matching the exclusion patterns.
    pub fn exclude_operations(
        &self,
        operations: Vec<ApiOperation>,
        exclude: &str,
    ) -> Result<Vec<ApiOperation>> {
        if exclude.is_empty() {
            return Ok(operations);
        }

        let exclusions: Vec<&str> = exclude.split(',').map(|s| s.trim()).collect();
        let mut result = Vec::new();

        for op in operations {
            let mut should_exclude = false;

            for exclude_str in &exclusions {
                // Support both "METHOD /path" and just "METHOD" (e.g., "DELETE")
                let parts: Vec<&str> = exclude_str.splitn(2, ' ').collect();

                let (method, path_pattern) = if parts.len() == 2 {
                    (parts[0].to_uppercase(), Some(parts[1]))
                } else {
                    // Just method name, no path - exclude all operations with this method
                    (parts[0].to_uppercase(), None)
                };

                let method_matches = op.method.to_uppercase() == method;
                let path_matches = path_pattern
                    .map(|p| Self::matches_path(&op.path, p))
                    .unwrap_or(true); // If no path specified, match all paths for this method

                if method_matches && path_matches {
                    should_exclude = true;
                    break;
                }
            }

            if !should_exclude {
                result.push(op);
            }
        }

        Ok(result)
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
        if let Some(prefix) = pattern.strip_suffix('*') {
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
    use openapiv3::Operation;

    /// Helper to create test operations
    fn create_test_operation(method: &str, path: &str, operation_id: Option<&str>) -> ApiOperation {
        ApiOperation {
            method: method.to_string(),
            path: path.to_string(),
            operation: Operation::default(),
            operation_id: operation_id.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_matches_path() {
        assert!(SpecParser::matches_path("/users", "/users"));
        assert!(SpecParser::matches_path("/users/123", "/users/*"));
        assert!(!SpecParser::matches_path("/posts", "/users"));
    }

    #[test]
    fn test_exclude_operations_by_method() {
        // Create a mock parser (we'll test the exclude_operations method directly)
        let operations = vec![
            create_test_operation("get", "/users", Some("getUsers")),
            create_test_operation("post", "/users", Some("createUser")),
            create_test_operation("delete", "/users/{id}", Some("deleteUser")),
            create_test_operation("get", "/posts", Some("getPosts")),
            create_test_operation("delete", "/posts/{id}", Some("deletePost")),
        ];

        // Test excluding all DELETE operations
        let spec = openapiv3::OpenAPI::default();
        let parser = SpecParser { spec };
        let result = parser.exclude_operations(operations.clone(), "DELETE").unwrap();

        assert_eq!(result.len(), 3);
        assert!(result.iter().all(|op| op.method.to_uppercase() != "DELETE"));
    }

    #[test]
    fn test_exclude_operations_by_method_and_path() {
        let operations = vec![
            create_test_operation("get", "/users", Some("getUsers")),
            create_test_operation("post", "/users", Some("createUser")),
            create_test_operation("delete", "/users/{id}", Some("deleteUser")),
            create_test_operation("get", "/posts", Some("getPosts")),
            create_test_operation("delete", "/posts/{id}", Some("deletePost")),
        ];

        let spec = openapiv3::OpenAPI::default();
        let parser = SpecParser { spec };

        // Test excluding specific DELETE operation by path
        let result = parser.exclude_operations(operations.clone(), "DELETE /users/{id}").unwrap();

        assert_eq!(result.len(), 4);
        // Should still have deletePost
        assert!(result.iter().any(|op| op.operation_id == Some("deletePost".to_string())));
        // Should not have deleteUser
        assert!(!result.iter().any(|op| op.operation_id == Some("deleteUser".to_string())));
    }

    #[test]
    fn test_exclude_operations_multiple_methods() {
        let operations = vec![
            create_test_operation("get", "/users", Some("getUsers")),
            create_test_operation("post", "/users", Some("createUser")),
            create_test_operation("delete", "/users/{id}", Some("deleteUser")),
            create_test_operation("put", "/users/{id}", Some("updateUser")),
        ];

        let spec = openapiv3::OpenAPI::default();
        let parser = SpecParser { spec };

        // Test excluding DELETE and POST
        let result = parser.exclude_operations(operations.clone(), "DELETE,POST").unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|op| op.method.to_uppercase() != "DELETE"));
        assert!(result.iter().all(|op| op.method.to_uppercase() != "POST"));
    }

    #[test]
    fn test_exclude_operations_empty_string() {
        let operations = vec![
            create_test_operation("get", "/users", Some("getUsers")),
            create_test_operation("delete", "/users/{id}", Some("deleteUser")),
        ];

        let spec = openapiv3::OpenAPI::default();
        let parser = SpecParser { spec };

        // Empty string should return all operations
        let result = parser.exclude_operations(operations.clone(), "").unwrap();

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_exclude_operations_with_wildcard() {
        let operations = vec![
            create_test_operation("get", "/users", Some("getUsers")),
            create_test_operation("delete", "/users/{id}", Some("deleteUser")),
            create_test_operation("delete", "/posts/{id}", Some("deletePost")),
        ];

        let spec = openapiv3::OpenAPI::default();
        let parser = SpecParser { spec };

        // Test excluding DELETE operations matching /users/*
        let result = parser.exclude_operations(operations.clone(), "DELETE /users/*").unwrap();

        assert_eq!(result.len(), 2);
        // Should still have deletePost
        assert!(result.iter().any(|op| op.operation_id == Some("deletePost".to_string())));
        // Should not have deleteUser
        assert!(!result.iter().any(|op| op.operation_id == Some("deleteUser".to_string())));
    }
}
