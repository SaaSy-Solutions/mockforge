//! Shared test fixture builders for MockForge crates.
//!
//! Provides reusable utilities for constructing test data that would
//! otherwise be duplicated across every crate's test modules.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Create a minimal valid OpenAPI 3.0 spec as a JSON value.
///
/// Useful when you need a valid spec but don't care about specific
/// paths or operations.
pub fn minimal_openapi_spec() -> Value {
    json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "paths": {}
    })
}

/// Create an OpenAPI spec with a single GET endpoint that returns JSON.
///
/// # Arguments
/// * `path` — The API path (e.g., `"/users"`)
/// * `status` — The HTTP status code for the response (e.g., `200`)
pub fn openapi_spec_with_get(path: &str, status: u16) -> Value {
    json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "paths": {
            path: {
                "get": {
                    "operationId": format!("get_{}", path.trim_start_matches('/')),
                    "responses": {
                        status.to_string(): {
                            "description": "Success",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

/// Create an OpenAPI spec with CRUD operations (GET, POST, PUT, DELETE) for a resource.
///
/// # Arguments
/// * `resource` — The resource name (e.g., `"users"`)
pub fn openapi_spec_crud(resource: &str) -> Value {
    let collection_path = format!("/{resource}");
    let item_path = format!("/{resource}/{{id}}");

    json!({
        "openapi": "3.0.0",
        "info": {
            "title": format!("{} API", capitalize(resource)),
            "version": "1.0.0"
        },
        "paths": {
            collection_path: {
                "get": {
                    "operationId": format!("list_{resource}"),
                    "responses": {
                        "200": {
                            "description": "List all",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": { "type": "object" }
                                    }
                                }
                            }
                        }
                    }
                },
                "post": {
                    "operationId": format!("create_{resource}"),
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": { "type": "object" }
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "Created",
                            "content": {
                                "application/json": {
                                    "schema": { "type": "object" }
                                }
                            }
                        }
                    }
                }
            },
            item_path: {
                "get": {
                    "operationId": format!("get_{resource}"),
                    "parameters": [{
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }],
                    "responses": {
                        "200": {
                            "description": "Found",
                            "content": {
                                "application/json": {
                                    "schema": { "type": "object" }
                                }
                            }
                        }
                    }
                },
                "put": {
                    "operationId": format!("update_{resource}"),
                    "parameters": [{
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }],
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": { "type": "object" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Updated",
                            "content": {
                                "application/json": {
                                    "schema": { "type": "object" }
                                }
                            }
                        }
                    }
                },
                "delete": {
                    "operationId": format!("delete_{resource}"),
                    "parameters": [{
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }],
                    "responses": {
                        "204": {
                            "description": "Deleted"
                        }
                    }
                }
            }
        }
    })
}

/// Write a JSON value as an OpenAPI spec file to a temporary directory.
///
/// Returns `(TempDir, PathBuf)` — keep `TempDir` alive for the test duration.
///
/// # Panics
/// Panics if the temporary directory or file cannot be created.
pub fn write_temp_spec(spec: &Value) -> (tempfile::TempDir, PathBuf) {
    let temp_dir = tempfile::TempDir::new().expect("create temp dir for test spec");
    let spec_path = temp_dir.path().join("spec.json");
    std::fs::write(&spec_path, serde_json::to_string_pretty(spec).expect("serialize spec"))
        .expect("write spec file");
    (temp_dir, spec_path)
}

/// Write a JSON value as an OpenAPI spec file to a specific path.
///
/// # Panics
/// Panics if the file cannot be written.
pub fn write_spec_to(spec: &Value, path: &Path) {
    std::fs::write(path, serde_json::to_string_pretty(spec).expect("serialize spec"))
        .expect("write spec file");
}

/// Resolve a fixture path relative to the calling crate's `CARGO_MANIFEST_DIR`.
///
/// Typically used as: `fixture_path(env!("CARGO_MANIFEST_DIR"), "tests/fixtures/my_spec.json")`
pub fn fixture_path(manifest_dir: &str, relative: &str) -> PathBuf {
    PathBuf::from(manifest_dir).join(relative)
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().chain(chars).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_spec_is_valid() {
        let spec = minimal_openapi_spec();
        assert_eq!(spec["openapi"], "3.0.0");
        assert!(spec["paths"].is_object());
    }

    #[test]
    fn test_spec_with_get() {
        let spec = openapi_spec_with_get("/users", 200);
        assert!(spec["paths"]["/users"]["get"].is_object());
        assert!(spec["paths"]["/users"]["get"]["responses"]["200"].is_object());
    }

    #[test]
    fn test_crud_spec() {
        let spec = openapi_spec_crud("users");
        assert!(spec["paths"]["/users"]["get"].is_object());
        assert!(spec["paths"]["/users"]["post"].is_object());
        assert!(spec["paths"]["/users/{id}"]["get"].is_object());
        assert!(spec["paths"]["/users/{id}"]["put"].is_object());
        assert!(spec["paths"]["/users/{id}"]["delete"].is_object());
    }

    #[test]
    fn test_write_temp_spec() {
        let spec = minimal_openapi_spec();
        let (_dir, path) = write_temp_spec(&spec);
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["openapi"], "3.0.0");
    }
}
