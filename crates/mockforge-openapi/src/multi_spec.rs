//! Multi-spec loading and merging utilities
//!
//! This module provides functionality to load multiple OpenAPI specifications,
//! group them by version, detect conflicts, and merge them according to
//! configurable strategies.

use crate::spec::OpenApiSpec;
use mockforge_foundation::error::{Error, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Conflict resolution strategy for merging specs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictStrategy {
    /// Fail fast on conflicts (default)
    Error,
    /// First file wins
    First,
    /// Last file wins
    Last,
}

impl From<&str> for ConflictStrategy {
    fn from(s: &str) -> Self {
        match s {
            "first" => ConflictStrategy::First,
            "last" => ConflictStrategy::Last,
            _ => ConflictStrategy::Error,
        }
    }
}

/// A detected conflict between specs
#[derive(Debug, Clone)]
pub enum Conflict {
    /// Route conflict: same METHOD + PATH in multiple files
    RouteConflict {
        /// HTTP method
        method: String,
        /// API path
        path: String,
        /// Files containing this route
        files: Vec<PathBuf>,
    },
    /// Component conflict: same key with different definitions
    ComponentConflict {
        /// Type of component (schemas, responses, etc.)
        component_type: String,
        /// Component key/name
        key: String,
        /// Files containing this component
        files: Vec<PathBuf>,
    },
}

/// Error type for merge conflicts
#[derive(Debug)]
pub enum MergeConflictError {
    /// Route conflict error
    RouteConflict {
        /// HTTP method
        method: String,
        /// API path
        path: String,
        /// Files containing this route
        files: Vec<PathBuf>,
    },
    /// Component conflict error
    ComponentConflict {
        /// Type of component (schemas, responses, etc.)
        component_type: String,
        /// Component key/name
        key: String,
        /// Files containing this component
        files: Vec<PathBuf>,
    },
    /// Multiple conflicts detected
    MultipleConflicts {
        /// All detected conflicts
        conflicts: Vec<Conflict>,
    },
}

impl std::fmt::Display for MergeConflictError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MergeConflictError::MultipleConflicts { conflicts } => {
                writeln!(f, "Found {} spec conflict(s):\n", conflicts.len())?;
                for (i, conflict) in conflicts.iter().enumerate() {
                    match conflict {
                        Conflict::RouteConflict {
                            method,
                            path,
                            files,
                        } => {
                            writeln!(f, "  {}. {} {} defined in:", i + 1, method, path)?;
                            for file in files {
                                writeln!(f, "     - {}", file.display())?;
                            }
                        }
                        Conflict::ComponentConflict {
                            component_type,
                            key,
                            files,
                        } => {
                            writeln!(
                                f,
                                "  {}. components.{}.{} defined in:",
                                i + 1,
                                component_type,
                                key
                            )?;
                            for file in files {
                                writeln!(f, "     - {}", file.display())?;
                            }
                        }
                    }
                }
                writeln!(f)?;
                write!(
                    f,
                    "Resolution options:\n\
                     - Use --merge-conflicts=first to keep the first definition\n\
                     - Use --merge-conflicts=last to keep the last definition\n\
                     - Remove duplicate routes/components from conflicting spec files"
                )
            }
            MergeConflictError::RouteConflict {
                method,
                path,
                files,
            } => {
                write!(
                    f,
                    "Conflict: {} {} defined in {}",
                    method,
                    path,
                    files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(" and ")
                )
            }
            MergeConflictError::ComponentConflict {
                component_type,
                key,
                files,
            } => {
                write!(
                    f,
                    "Conflict: components.{}.{} defined differently in {}",
                    component_type,
                    key,
                    files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(" and ")
                )
            }
        }
    }
}

impl std::error::Error for MergeConflictError {}

/// Load all OpenAPI spec files from a directory
///
/// Discovers all `.json`, `.yaml`, `.yml` files recursively,
/// sorts them lexicographically for deterministic ordering,
/// and loads each spec.
pub async fn load_specs_from_directory(dir: &Path) -> Result<Vec<(PathBuf, OpenApiSpec)>> {
    use globwalk::GlobWalkerBuilder;

    info!("Discovering OpenAPI specs in directory: {}", dir.display());

    if !dir.exists() {
        return Err(Error::internal(format!("Directory does not exist: {}", dir.display())));
    }

    if !dir.is_dir() {
        return Err(Error::internal(format!("Path is not a directory: {}", dir.display())));
    }

    // Discover all spec files
    let mut spec_files = Vec::new();
    let walker = GlobWalkerBuilder::from_patterns(dir, &["**/*.json", "**/*.yaml", "**/*.yml"])
        .build()
        .map_err(|e| Error::internal(format!("Failed to walk directory: {}", e)))?;

    for entry in walker {
        let entry = entry.map_err(|e| Error::io_with_context("directory entry", e.to_string()))?;
        let path = entry.path();
        if path.is_file() {
            spec_files.push(path.to_path_buf());
        }
    }

    // Sort lexicographically for deterministic ordering
    spec_files.sort();

    if spec_files.is_empty() {
        warn!("No OpenAPI spec files found in directory: {}", dir.display());
        return Ok(Vec::new());
    }

    info!("Found {} spec files, loading...", spec_files.len());

    // Load each spec file
    let mut specs = Vec::new();
    for file_path in spec_files {
        match OpenApiSpec::from_file(&file_path).await {
            Ok(spec) => {
                debug!("Loaded spec from: {}", file_path.display());
                specs.push((file_path, spec));
            }
            Err(e) => {
                warn!("Failed to load spec from {}: {}", file_path.display(), e);
                // Continue with other files
            }
        }
    }

    info!("Successfully loaded {} specs from directory", specs.len());
    Ok(specs)
}

/// Load OpenAPI specs from a list of file paths
pub async fn load_specs_from_files(files: Vec<PathBuf>) -> Result<Vec<(PathBuf, OpenApiSpec)>> {
    info!("Loading {} OpenAPI spec files", files.len());

    let mut specs = Vec::new();
    for file_path in files {
        match OpenApiSpec::from_file(&file_path).await {
            Ok(spec) => {
                debug!("Loaded spec from: {}", file_path.display());
                specs.push((file_path, spec));
            }
            Err(e) => {
                return Err(Error::internal(format!(
                    "Failed to load spec from {}: {}",
                    file_path.display(),
                    e
                )));
            }
        }
    }

    info!("Successfully loaded {} specs", specs.len());
    Ok(specs)
}

/// Group specs by OpenAPI document version (the `openapi` field)
///
/// Returns a map from OpenAPI version (e.g., "3.0.0") to lists of (path, spec) tuples.
pub fn group_specs_by_openapi_version(
    specs: Vec<(PathBuf, OpenApiSpec)>,
) -> HashMap<String, Vec<(PathBuf, OpenApiSpec)>> {
    let mut groups: HashMap<String, Vec<(PathBuf, OpenApiSpec)>> = HashMap::new();

    for (path, spec) in specs {
        // Extract OpenAPI version from the spec
        let version = spec
            .raw_document
            .as_ref()
            .and_then(|doc| doc.get("openapi"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        groups.entry(version.clone()).or_default().push((path, spec));
    }

    info!("Grouped specs into {} OpenAPI version groups", groups.len());
    for (version, specs_in_group) in &groups {
        info!("  OpenAPI {}: {} specs", version, specs_in_group.len());
    }

    groups
}

/// Group specs by API version (the `info.version` field)
///
/// Returns a map from API version (e.g., "1.0", "2.0") to lists of (path, spec) tuples.
/// Specs without `info.version` are grouped under "unknown".
pub fn group_specs_by_api_version(
    specs: Vec<(PathBuf, OpenApiSpec)>,
) -> HashMap<String, Vec<(PathBuf, OpenApiSpec)>> {
    let mut groups: HashMap<String, Vec<(PathBuf, OpenApiSpec)>> = HashMap::new();

    for (path, spec) in specs {
        // Extract API version from info.version
        let api_version = spec
            .raw_document
            .as_ref()
            .and_then(|doc| doc.get("info"))
            .and_then(|info| info.get("version"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        groups.entry(api_version.clone()).or_default().push((path, spec));
    }

    info!("Grouped specs into {} API version groups", groups.len());
    for (version, specs_in_group) in &groups {
        info!("  API version {}: {} specs", version, specs_in_group.len());
    }

    groups
}

/// Detect conflicts between specs
///
/// Returns a list of all detected conflicts (route and component conflicts).
pub fn detect_conflicts(specs: &[(PathBuf, OpenApiSpec)]) -> Vec<Conflict> {
    let mut conflicts = Vec::new();

    // Detect route conflicts (same METHOD + PATH)
    let mut routes: HashMap<(String, String), Vec<PathBuf>> = HashMap::new();
    for (path, spec) in specs {
        for (route_path, path_item_ref) in &spec.spec.paths.paths {
            if let openapiv3::ReferenceOr::Item(path_item) = path_item_ref {
                // Check all HTTP methods
                let methods = vec![
                    ("GET", path_item.get.as_ref()),
                    ("POST", path_item.post.as_ref()),
                    ("PUT", path_item.put.as_ref()),
                    ("DELETE", path_item.delete.as_ref()),
                    ("PATCH", path_item.patch.as_ref()),
                    ("HEAD", path_item.head.as_ref()),
                    ("OPTIONS", path_item.options.as_ref()),
                ];

                for (method, operation) in methods {
                    if operation.is_some() {
                        let key = (method.to_string(), route_path.clone());
                        routes.entry(key).or_default().push(path.clone());
                    }
                }
            }
        }
    }

    // Find route conflicts (same route in multiple files)
    for ((method, route_path), files) in routes {
        if files.len() > 1 {
            conflicts.push(Conflict::RouteConflict {
                method,
                path: route_path,
                files,
            });
        }
    }

    // Detect component conflicts
    for component_type in &[
        "schemas",
        "parameters",
        "responses",
        "requestBodies",
        "headers",
        "examples",
        "links",
        "callbacks",
    ] {
        let mut components: HashMap<String, Vec<PathBuf>> = HashMap::new();

        for (path, spec) in specs {
            if let Some(components_obj) = spec
                .raw_document
                .as_ref()
                .and_then(|doc| doc.get("components"))
                .and_then(|c| c.get(component_type))
            {
                if let Some(components_map) = components_obj.as_object() {
                    for key in components_map.keys() {
                        components.entry(key.clone()).or_default().push(path.clone());
                    }
                }
            }
        }

        // Check for conflicts (same key in multiple files with potentially different definitions)
        for (key, files) in components {
            if files.len() > 1 {
                // Check if definitions are identical
                let mut definitions = Vec::new();
                for (file_path, spec) in specs {
                    if files.contains(file_path) {
                        if let Some(def) = spec
                            .raw_document
                            .as_ref()
                            .and_then(|doc| doc.get("components"))
                            .and_then(|c| c.get(component_type))
                            .and_then(|ct| ct.get(&key))
                        {
                            definitions.push((file_path.clone(), def.clone()));
                        }
                    }
                }

                // Check if all definitions are byte-for-byte identical
                let first_def = &definitions[0].1;
                let all_identical = definitions.iter().all(|(_, def)| {
                    serde_json::to_string(def).ok() == serde_json::to_string(first_def).ok()
                });

                if !all_identical {
                    conflicts.push(Conflict::ComponentConflict {
                        component_type: component_type.to_string(),
                        key,
                        files,
                    });
                }
            }
        }
    }

    conflicts
}

/// Merge multiple OpenAPI specs according to the conflict strategy
///
/// This function merges paths and components from all specs.
/// Conflicts are handled according to the provided strategy.
pub fn merge_specs(
    specs: Vec<(PathBuf, OpenApiSpec)>,
    conflict_strategy: ConflictStrategy,
) -> std::result::Result<OpenApiSpec, MergeConflictError> {
    if specs.is_empty() {
        return Err(MergeConflictError::ComponentConflict {
            component_type: "general".to_string(),
            key: "no_specs".to_string(),
            files: Vec::new(),
        });
    }

    if specs.len() == 1 {
        // No merging needed — safe because we just checked len() == 1
        return specs.into_iter().next().map(|(_, spec)| spec).ok_or_else(|| {
            MergeConflictError::ComponentConflict {
                component_type: "general".to_string(),
                key: "no_specs".to_string(),
                files: Vec::new(),
            }
        });
    }

    // Detect conflicts first
    let conflicts = detect_conflicts(&specs);

    // Handle conflicts based on strategy
    match conflict_strategy {
        ConflictStrategy::Error => {
            if !conflicts.is_empty() {
                // Return all conflicts as an error for comprehensive feedback
                return Err(MergeConflictError::MultipleConflicts {
                    conflicts: conflicts.clone(),
                });
            }
        }
        ConflictStrategy::First | ConflictStrategy::Last => {
            // Log warnings for conflicts
            for conflict in &conflicts {
                match conflict {
                    Conflict::RouteConflict {
                        method,
                        path,
                        files,
                    } => {
                        warn!(
                            "Route conflict: {} {} defined in multiple files: {:?}. Using {} definition.",
                            method, path, files,
                            if conflict_strategy == ConflictStrategy::First { "first" } else { "last" }
                        );
                    }
                    Conflict::ComponentConflict {
                        component_type,
                        key,
                        files,
                    } => {
                        warn!(
                            "Component conflict: components.{} defined in multiple files: {}. Using {} definition (strategy: {}).",
                            component_type, key, files.iter().map(|f| f.display().to_string()).collect::<Vec<_>>().join(", "),
                            if conflict_strategy == ConflictStrategy::First { "first" } else { "last" }
                        );
                    }
                }
            }
        }
    }

    // Collect file paths before processing (needed for error messages)
    let all_file_paths: Vec<PathBuf> = specs.iter().map(|(p, _)| p.clone()).collect();

    // Start with the first spec as the base
    let base_spec = specs.first().map(|(_, spec)| spec.clone()).ok_or_else(|| {
        MergeConflictError::ComponentConflict {
            component_type: "general".to_string(),
            key: "no_specs".to_string(),
            files: Vec::new(),
        }
    })?;
    let mut base_doc = base_spec
        .raw_document
        .as_ref()
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    // Skip the first spec (used as base) and merge the rest
    let specs_to_merge: Vec<&(PathBuf, OpenApiSpec)> = specs.iter().skip(1).collect();

    // Merge each subsequent spec
    for (_file_path, spec) in specs_to_merge {
        let spec_doc = spec.raw_document.as_ref().cloned().unwrap_or_else(|| serde_json::json!({}));

        // Merge paths
        if let Some(paths) = spec_doc.get("paths").and_then(|p| p.as_object()) {
            if base_doc.get("paths").is_none() {
                base_doc["paths"] = serde_json::json!({});
            }
            let base_paths = base_doc["paths"].as_object_mut().ok_or_else(|| {
                MergeConflictError::ComponentConflict {
                    component_type: "paths".to_string(),
                    key: "invalid_type".to_string(),
                    files: all_file_paths.clone(),
                }
            })?;
            for (path, path_item) in paths {
                if base_paths.contains_key(path) {
                    // Conflict - handle based on strategy
                    if conflict_strategy == ConflictStrategy::Last {
                        base_paths.insert(path.clone(), path_item.clone());
                    }
                    // For First and Error, we already handled it above
                } else {
                    base_paths.insert(path.clone(), path_item.clone());
                }
            }
        }

        // Merge components
        if let Some(components) = spec_doc.get("components").and_then(|c| c.as_object()) {
            if base_doc.get("components").is_none() {
                base_doc["components"] = serde_json::json!({});
            }
            let base_components = base_doc["components"].as_object_mut().ok_or_else(|| {
                MergeConflictError::ComponentConflict {
                    component_type: "components".to_string(),
                    key: "invalid_type".to_string(),
                    files: all_file_paths.clone(),
                }
            })?;
            for (component_type, component_obj) in components {
                if let Some(component_map) = component_obj.as_object() {
                    let base_component_map = base_components
                        .entry(component_type.clone())
                        .or_insert_with(|| serde_json::json!({}))
                        .as_object_mut()
                        .ok_or_else(|| MergeConflictError::ComponentConflict {
                            component_type: component_type.clone(),
                            key: "invalid_type".to_string(),
                            files: all_file_paths.clone(),
                        })?;

                    for (key, value) in component_map {
                        if let Some(existing) = base_component_map.get(key) {
                            // Check if identical
                            if serde_json::to_string(existing).ok()
                                != serde_json::to_string(value).ok()
                            {
                                // Different - handle based on strategy
                                if conflict_strategy == ConflictStrategy::Last {
                                    base_component_map.insert(key.clone(), value.clone());
                                }
                                // For First and Error, we already handled it above
                            }
                            // If identical, no action needed
                        } else {
                            base_component_map.insert(key.clone(), value.clone());
                        }
                    }
                }
            }
        }
    }

    // Re-parse the merged document
    let merged_spec: openapiv3::OpenAPI =
        serde_json::from_value(base_doc.clone()).map_err(|e| {
            MergeConflictError::ComponentConflict {
                component_type: "parsing".to_string(),
                key: format!("merge_error: {}", e),
                files: all_file_paths,
            }
        })?;

    Ok(OpenApiSpec {
        spec: merged_spec,
        file_path: None, // Merged spec has no single file path
        raw_document: Some(base_doc),
    })
}

#[cfg(test)]
mod multi_spec_tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    /// Build a minimal valid spec whose raw document carries the given
    /// paths/components verbatim (conflict detection reads raw documents).
    fn spec_with_document(document: serde_json::Value) -> OpenApiSpec {
        OpenApiSpec::from_json(document).expect("valid fixture spec")
    }

    fn simple_document(
        paths: serde_json::Value,
        components: Option<serde_json::Value>,
    ) -> serde_json::Value {
        let mut doc = json!({
            "openapi": "3.0.0",
            "info": {"title": "Fixture", "version": "1.0"},
            "paths": paths,
        });
        if let Some(components) = components {
            doc["components"] = components;
        }
        doc
    }

    // ─── ConflictStrategy parsing ───────────────────────────────────────────

    #[test]
    fn strategy_parses_names_and_defaults_to_error() {
        assert_eq!(ConflictStrategy::from("first"), ConflictStrategy::First);
        assert_eq!(ConflictStrategy::from("last"), ConflictStrategy::Last);
        assert_eq!(ConflictStrategy::from("error"), ConflictStrategy::Error);
        // Anything unrecognized falls back to fail-fast.
        assert_eq!(ConflictStrategy::from("bogus"), ConflictStrategy::Error);
        assert_eq!(ConflictStrategy::from(""), ConflictStrategy::Error);
    }

    // ─── Grouping ───────────────────────────────────────────────────────────

    #[test]
    fn groups_specs_by_openapi_document_version() {
        let mut v0302 = spec_with_document(simple_document(json!({}), None));
        v0302.raw_document =
            Some(json!({"openapi": "3.0.2", "info": {"title": "t", "version": "1"}}));
        let mut unknown = spec_with_document(simple_document(json!({}), None));
        unknown.raw_document = None;

        let groups = group_specs_by_openapi_version(vec![
            (PathBuf::from("a.json"), spec_with_document(simple_document(json!({}), None))),
            (PathBuf::from("b.json"), spec_with_document(simple_document(json!({}), None))),
            (PathBuf::from("c.json"), v0302),
            (PathBuf::from("d.json"), unknown),
        ]);

        assert_eq!(groups.len(), 3);
        assert_eq!(groups["3.0.0"].len(), 2);
        assert_eq!(groups["3.0.2"].len(), 1);
        assert_eq!(groups["unknown"].len(), 1);
    }

    #[test]
    fn unversioned_specs_group_under_unknown() {
        let mut no_openapi = spec_with_document(simple_document(json!({}), None));
        no_openapi.raw_document = Some(json!({"info": {"title": "t"}}));

        let groups = group_specs_by_openapi_version(vec![(PathBuf::from("a.json"), no_openapi)]);
        assert_eq!(groups.len(), 1);
        assert!(groups.contains_key("unknown"));
    }

    #[test]
    fn groups_specs_by_api_version() {
        let mut v1 = spec_with_document(simple_document(json!({}), None));
        v1.raw_document =
            Some(json!({"openapi": "3.0.0", "info": {"title": "t", "version": "1.0"}}));
        let mut v2 = spec_with_document(simple_document(json!({}), None));
        v2.raw_document =
            Some(json!({"openapi": "3.0.0", "info": {"title": "t", "version": "2.0"}}));
        let mut no_version = spec_with_document(simple_document(json!({}), None));
        no_version.raw_document = Some(json!({}));

        let groups = group_specs_by_api_version(vec![
            (PathBuf::from("a.json"), v1),
            (PathBuf::from("b.json"), v2),
            (PathBuf::from("c.json"), no_version),
        ]);

        assert_eq!(groups.len(), 3);
        assert!(groups.contains_key("1.0"));
        assert!(groups.contains_key("2.0"));
        assert!(groups.contains_key("unknown"));
    }

    // ─── Conflict detection ─────────────────────────────────────────────────

    #[test]
    fn disjoint_specs_have_no_conflicts() {
        let a = (
            PathBuf::from("a.json"),
            spec_with_document(simple_document(
                json!({"/a": {"get": {"operationId": "getA"}}}),
                Some(json!({"schemas": {"A": {"type": "string"}}})),
            )),
        );
        let b = (
            PathBuf::from("b.json"),
            spec_with_document(simple_document(
                json!({"/b": {"post": {"operationId": "postB"}}}),
                Some(json!({"schemas": {"B": {"type": "string"}}})),
            )),
        );
        assert!(detect_conflicts(&[a, b]).is_empty());
    }

    #[test]
    fn same_method_and_path_is_a_route_conflict() {
        let route = json!({"/users": {"get": {"operationId": "getUsers"}}});
        let conflicts = detect_conflicts(&[
            (
                PathBuf::from("a.json"),
                spec_with_document(simple_document(route.clone(), None)),
            ),
            (PathBuf::from("b.json"), spec_with_document(simple_document(route, None))),
        ]);

        assert_eq!(conflicts.len(), 1);
        match &conflicts[0] {
            Conflict::RouteConflict {
                method,
                path,
                files,
            } => {
                assert_eq!(method, "GET");
                assert_eq!(path, "/users");
                let names: Vec<_> = files.iter().map(|f| f.display().to_string()).collect();
                assert!(names.contains(&"a.json".to_string()));
                assert!(names.contains(&"b.json".to_string()));
            }
            _ => panic!("expected a route conflict"),
        }
    }

    #[test]
    fn same_path_with_different_methods_is_not_a_conflict() {
        let a = (
            PathBuf::from("a.json"),
            spec_with_document(simple_document(json!({"/users": {"get": {}}}), None)),
        );
        let b = (
            PathBuf::from("b.json"),
            spec_with_document(simple_document(json!({"/users": {"post": {}}}), None)),
        );
        assert!(detect_conflicts(&[a, b]).is_empty());
    }

    #[test]
    fn differing_components_conflict_but_identical_ones_do_not() {
        let user_v1 = json!({"schemas": {"User": {"type": "object", "properties": {"name": {"type": "string"}}}}});
        let user_v2 = json!({"schemas": {"User": {"type": "object", "properties": {"id": {"type": "integer"}}}}});

        // Different definitions for the same key → conflict.
        let conflicts = detect_conflicts(&[
            (
                PathBuf::from("a.json"),
                spec_with_document(simple_document(json!({}), Some(user_v1.clone()))),
            ),
            (
                PathBuf::from("b.json"),
                spec_with_document(simple_document(json!({}), Some(user_v2))),
            ),
        ]);
        assert_eq!(conflicts.len(), 1);
        match &conflicts[0] {
            Conflict::ComponentConflict {
                component_type,
                key,
                ..
            } => {
                assert_eq!(component_type, "schemas");
                assert_eq!(key, "User");
            }
            _ => panic!("expected a component conflict"),
        }

        // Byte-identical definitions → no conflict.
        let conflicts = detect_conflicts(&[
            (
                PathBuf::from("a.json"),
                spec_with_document(simple_document(json!({}), Some(user_v1.clone()))),
            ),
            (
                PathBuf::from("b.json"),
                spec_with_document(simple_document(json!({}), Some(user_v1))),
            ),
        ]);
        assert!(conflicts.is_empty());
    }

    // ─── Merging ────────────────────────────────────────────────────────────

    #[test]
    fn empty_spec_list_is_rejected() {
        let err = merge_specs(vec![], ConflictStrategy::Error).unwrap_err();
        assert!(
            matches!(err, MergeConflictError::ComponentConflict { ref key, .. } if key == "no_specs")
        );
    }

    #[test]
    fn single_spec_is_returned_unchanged() {
        let spec = spec_with_document(simple_document(
            json!({"/a": {"get": {"operationId": "getA"}}}),
            None,
        ));
        let merged = merge_specs(vec![(PathBuf::from("a.json"), spec)], ConflictStrategy::Error)
            .expect("single spec merges trivially");
        assert_eq!(merged.raw_document.as_ref().unwrap()["info"]["title"], "Fixture");
    }

    #[test]
    fn disjoint_specs_merge_paths_and_components() {
        let a = (
            PathBuf::from("a.json"),
            spec_with_document(simple_document(
                json!({"/a": {"get": {"operationId": "getA"}}}),
                Some(json!({"schemas": {"A": {"type": "string"}}})),
            )),
        );
        let b = (
            PathBuf::from("b.json"),
            spec_with_document(simple_document(
                json!({"/b": {"post": {"operationId": "postB"}}}),
                Some(json!({"schemas": {"B": {"type": "integer"}}})),
            )),
        );

        let merged = merge_specs(vec![a, b], ConflictStrategy::Error).expect("merge succeeds");
        let doc = merged.raw_document.as_ref().expect("raw document preserved");
        assert!(doc["paths"].get("/a").is_some());
        assert!(doc["paths"].get("/b").is_some());
        assert!(doc["components"]["schemas"].get("A").is_some());
        assert!(doc["components"]["schemas"].get("B").is_some());
    }

    #[test]
    fn error_strategy_reports_all_conflicts() {
        let route = json!({"/users": {"get": {"operationId": "getUsers"}}});
        let err = merge_specs(
            vec![
                (
                    PathBuf::from("a.json"),
                    spec_with_document(simple_document(route.clone(), None)),
                ),
                (PathBuf::from("b.json"), spec_with_document(simple_document(route, None))),
            ],
            ConflictStrategy::Error,
        )
        .unwrap_err();

        match err {
            MergeConflictError::MultipleConflicts { conflicts } => {
                assert_eq!(conflicts.len(), 1);
            }
            _ => panic!("expected MultipleConflicts"),
        }

        // The rendered error explains the conflicts and the resolution flags.
        let rendered = merge_specs(
            vec![
                (
                    PathBuf::from("a.json"),
                    spec_with_document(simple_document(
                        json!({"/users": {"get": {"operationId": "getUsers"}}}),
                        None,
                    )),
                ),
                (
                    PathBuf::from("b.json"),
                    spec_with_document(simple_document(
                        json!({"/users": {"get": {"operationId": "getUsers"}}}),
                        None,
                    )),
                ),
            ],
            ConflictStrategy::Error,
        )
        .unwrap_err()
        .to_string();
        assert!(rendered.contains("GET"), "{rendered}");
        assert!(rendered.contains("/users"), "{rendered}");
        assert!(rendered.contains("Resolution options"), "{rendered}");
        assert!(rendered.contains("--merge-conflicts=first"), "{rendered}");
    }

    #[tokio::test]
    async fn first_strategy_keeps_base_definition() {
        let a = (
            PathBuf::from("a.json"),
            spec_with_document(simple_document(
                json!({"/users": {"get": {"operationId": "firstVersion"}}}),
                Some(json!({"schemas": {"User": {"type": "string"}}})),
            )),
        );
        let b = (
            PathBuf::from("b.json"),
            spec_with_document(simple_document(
                json!({"/users": {"get": {"operationId": "secondVersion"}}}),
                Some(json!({"schemas": {"User": {"type": "integer"}}})),
            )),
        );

        let merged = merge_specs(vec![a, b], ConflictStrategy::First).expect("first merges");
        let doc = merged.raw_document.as_ref().unwrap();
        assert_eq!(doc["paths"]["/users"]["get"]["operationId"], "firstVersion");
        assert_eq!(doc["components"]["schemas"]["User"], json!({"type": "string"}));
    }

    #[tokio::test]
    async fn last_strategy_prefers_later_definition() {
        let a = (
            PathBuf::from("a.json"),
            spec_with_document(simple_document(
                json!({"/users": {"get": {"operationId": "firstVersion"}}}),
                Some(json!({"schemas": {"User": {"type": "string"}}})),
            )),
        );
        let b = (
            PathBuf::from("b.json"),
            spec_with_document(simple_document(
                json!({"/users": {"get": {"operationId": "secondVersion"}}}),
                Some(json!({"schemas": {"User": {"type": "integer"}}})),
            )),
        );

        let merged = merge_specs(vec![a, b], ConflictStrategy::Last).expect("last merges");
        let doc = merged.raw_document.as_ref().unwrap();
        assert_eq!(doc["paths"]["/users"]["get"]["operationId"], "secondVersion");
        assert_eq!(doc["components"]["schemas"]["User"], json!({"type": "integer"}));
    }

    // ─── Directory loading ──────────────────────────────────────────────────

    fn valid_spec_json(title: &str) -> String {
        json!({
            "openapi": "3.0.0",
            "info": {"title": title, "version": "1.0"},
            "paths": {}
        })
        .to_string()
    }

    #[tokio::test]
    async fn loads_json_yaml_and_yml_files_recursively_sorted() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("b.yaml"),
            "openapi: 3.0.0\ninfo:\n  title: B\n  version: \"1.0\"\npaths: {}\n",
        )
        .expect("write b.yaml");
        std::fs::write(dir.path().join("a.json"), valid_spec_json("A")).expect("write a.json");
        std::fs::create_dir(dir.path().join("nested")).expect("mkdir");
        std::fs::write(
            dir.path().join("nested/c.yml"),
            "openapi: 3.0.0\ninfo:\n  title: C\n  version: \"1.0\"\npaths: {}\n",
        )
        .expect("write c.yml");
        std::fs::write(dir.path().join("notes.txt"), "not a spec").expect("write notes.txt");
        std::fs::write(dir.path().join("broken.json"), "{ not json").expect("write broken.json");

        let specs = load_specs_from_directory(dir.path()).await.expect("load specs");
        let titles: Vec<String> =
            specs.iter().map(|(path, _)| path.display().to_string()).collect();

        // .txt is ignored, broken JSON is skipped with a warning, and the
        // remaining files come back in lexicographic order.
        assert_eq!(titles.len(), 3, "{titles:?}");
        assert!(titles[0].ends_with("a.json"), "{titles:?}");
        assert!(titles[1].ends_with("b.yaml"), "{titles:?}");
        assert!(titles[2].ends_with("c.yml"), "{titles:?}");
    }

    #[tokio::test]
    async fn empty_directory_loads_no_specs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let specs = load_specs_from_directory(dir.path()).await.expect("load specs");
        assert!(specs.is_empty());
    }

    #[tokio::test]
    async fn missing_directory_is_an_error() {
        let err = load_specs_from_directory(Path::new("/nonexistent/mockforge/specs"))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("does not exist"), "{err}");
    }

    #[tokio::test]
    async fn loading_from_files_fails_fast_on_bad_paths() {
        let dir = tempfile::tempdir().expect("tempdir");
        let good = dir.path().join("a.json");
        std::fs::write(&good, valid_spec_json("A")).expect("write a.json");

        let specs = load_specs_from_files(vec![good.clone()]).await.expect("load specs");
        assert_eq!(specs.len(), 1);

        let err = load_specs_from_files(vec![dir.path().join("missing.json")]).await.unwrap_err();
        assert!(err.to_string().contains("missing.json"), "{err}");
    }
}
