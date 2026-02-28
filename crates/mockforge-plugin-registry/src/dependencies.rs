//! Plugin dependency resolution and management

use crate::{RegistryError, Result, VersionEntry};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// Dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Package name
    pub name: String,

    /// Version requirement (semver)
    pub version_req: String,

    /// Optional dependency
    pub optional: bool,

    /// Features to enable
    pub features: Vec<String>,

    /// Registry source
    pub source: DependencySource,
}

/// Dependency source
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencySource {
    #[default]
    Registry,
    Git {
        url: String,
        rev: Option<String>,
    },
    Path {
        path: String,
    },
}

/// Resolved dependency
#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    pub name: String,
    pub version: Version,
    pub dependencies: Vec<Dependency>,
}

/// Dependency graph node
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct DependencyNode {
    name: String,
    version: Version,
    dependencies: Vec<String>, // List of dependent package names
}

/// Dependency resolver
pub struct DependencyResolver {
    /// Available package versions
    available_versions: HashMap<String, Vec<VersionEntry>>,
}

impl DependencyResolver {
    /// Create a new dependency resolver
    pub fn new() -> Self {
        Self {
            available_versions: HashMap::new(),
        }
    }

    /// Add available versions for a package
    pub fn add_package_versions(&mut self, name: String, versions: Vec<VersionEntry>) {
        self.available_versions.insert(name, versions);
    }

    /// Resolve dependencies for a package
    pub fn resolve(
        &self,
        root_package: &str,
        _root_version: &Version,
        dependencies: Vec<Dependency>,
    ) -> Result<Vec<ResolvedDependency>> {
        let mut resolved = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with root dependencies
        queue.push_back((root_package.to_string(), dependencies));

        while let Some((_parent, deps)) = queue.pop_front() {
            for dep in deps {
                // Skip if already resolved
                if visited.contains(&dep.name) {
                    continue;
                }

                // Parse version requirement
                let version_req = VersionReq::parse(&dep.version_req).map_err(|e| {
                    RegistryError::InvalidVersion(format!(
                        "Invalid version requirement '{}': {}",
                        dep.version_req, e
                    ))
                })?;

                // Find compatible version
                let compatible_version =
                    self.find_compatible_version(&dep.name, &version_req)?.ok_or_else(|| {
                        RegistryError::InvalidVersion(format!(
                            "No compatible version found for {} with requirement {}",
                            dep.name, dep.version_req
                        ))
                    })?;

                // Get version entry
                let version_entry =
                    self.get_version_entry(&dep.name, &compatible_version).ok_or_else(|| {
                        RegistryError::PluginNotFound(format!(
                            "{} {}",
                            dep.name, compatible_version
                        ))
                    })?;

                // Parse transitive dependencies
                let transitive_deps: Vec<Dependency> = version_entry
                    .dependencies
                    .iter()
                    .map(|(name, version_req)| Dependency {
                        name: name.clone(),
                        version_req: version_req.clone(),
                        optional: false,
                        features: vec![],
                        source: DependencySource::Registry,
                    })
                    .collect();

                // Add to resolved
                resolved.push(ResolvedDependency {
                    name: dep.name.clone(),
                    version: compatible_version.clone(),
                    dependencies: transitive_deps.clone(),
                });

                visited.insert(dep.name.clone());

                // Queue transitive dependencies
                if !transitive_deps.is_empty() {
                    queue.push_back((dep.name.clone(), transitive_deps));
                }
            }
        }

        // Check for circular dependencies
        self.check_circular_dependencies(&resolved)?;

        Ok(resolved)
    }

    /// Find a compatible version for a package
    fn find_compatible_version(
        &self,
        package: &str,
        version_req: &VersionReq,
    ) -> Result<Option<Version>> {
        let versions = self
            .available_versions
            .get(package)
            .ok_or_else(|| RegistryError::PluginNotFound(package.to_string()))?;

        // Filter out yanked versions and parse semver
        let mut compatible_versions: Vec<Version> = versions
            .iter()
            .filter(|v| !v.yanked)
            .filter_map(|v| Version::parse(&v.version).ok())
            .filter(|v| version_req.matches(v))
            .collect();

        // Sort by version (highest first)
        compatible_versions.sort();
        compatible_versions.reverse();

        Ok(compatible_versions.first().cloned())
    }

    /// Get version entry for a specific version
    fn get_version_entry(&self, package: &str, version: &Version) -> Option<&VersionEntry> {
        self.available_versions
            .get(package)?
            .iter()
            .find(|v| Version::parse(&v.version).ok().map(|v| &v == version).unwrap_or(false))
    }

    /// Check for circular dependencies
    fn check_circular_dependencies(&self, resolved: &[ResolvedDependency]) -> Result<()> {
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        // Build adjacency list
        for dep in resolved {
            let deps: Vec<String> = dep.dependencies.iter().map(|d| d.name.clone()).collect();
            graph.insert(dep.name.clone(), deps);
        }

        // DFS to detect cycles
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node in graph.keys() {
            if Self::has_cycle_impl(&graph, node, &mut visited, &mut rec_stack) {
                return Err(RegistryError::InvalidManifest(format!(
                    "Circular dependency detected involving package: {}",
                    node
                )));
            }
        }

        Ok(())
    }

    /// Check if there's a cycle starting from a node
    fn has_cycle_impl(
        graph: &HashMap<String, Vec<String>>,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        if rec_stack.contains(node) {
            return true;
        }

        if visited.contains(node) {
            return false;
        }

        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if Self::has_cycle_impl(graph, neighbor, visited, rec_stack) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }

    /// Calculate installation order (topological sort)
    pub fn calculate_install_order(&self, resolved: &[ResolvedDependency]) -> Result<Vec<String>> {
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        // Build graph
        for dep in resolved {
            in_degree.entry(dep.name.clone()).or_insert(0);

            for child_dep in &dep.dependencies {
                graph.entry(dep.name.clone()).or_default().push(child_dep.name.clone());

                *in_degree.entry(child_dep.name.clone()).or_insert(0) += 1;
            }
        }

        // Kahn's algorithm for topological sort
        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(name, _)| name.clone())
            .collect();

        let mut order = Vec::new();

        while let Some(node) = queue.pop_front() {
            order.push(node.clone());

            if let Some(neighbors) = graph.get(&node) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }
        }

        // Check if all nodes are in the order (no cycles)
        if order.len() != in_degree.len() {
            return Err(RegistryError::InvalidManifest(
                "Circular dependency detected during install order calculation".to_string(),
            ));
        }

        // Reverse to get correct install order (dependencies first)
        order.reverse();

        Ok(order)
    }
}

impl Default for DependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Dependency conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConflict {
    pub package: String,
    pub required_by: Vec<ConflictRequirement>,
}

/// Conflict requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictRequirement {
    pub package: String,
    pub version_req: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_version_entry(version: &str, yanked: bool) -> VersionEntry {
        VersionEntry {
            version: version.to_string(),
            download_url: format!("https://example.com/pkg-{}.tar.gz", version),
            checksum: "abc123".to_string(),
            size: 1000,
            published_at: "2025-01-01".to_string(),
            yanked,
            min_mockforge_version: None,
            dependencies: HashMap::new(),
        }
    }

    fn create_dependency(name: &str, version_req: &str) -> Dependency {
        Dependency {
            name: name.to_string(),
            version_req: version_req.to_string(),
            optional: false,
            features: vec![],
            source: DependencySource::Registry,
        }
    }

    // Dependency struct tests
    #[test]
    fn test_dependency_clone() {
        let dep = Dependency {
            name: "test-dep".to_string(),
            version_req: "^1.0.0".to_string(),
            optional: true,
            features: vec!["feature1".to_string(), "feature2".to_string()],
            source: DependencySource::Registry,
        };

        let cloned = dep.clone();
        assert_eq!(dep.name, cloned.name);
        assert_eq!(dep.version_req, cloned.version_req);
        assert_eq!(dep.optional, cloned.optional);
        assert_eq!(dep.features.len(), cloned.features.len());
    }

    #[test]
    fn test_dependency_debug() {
        let dep = create_dependency("my-dep", "^2.0");
        let debug = format!("{:?}", dep);
        assert!(debug.contains("Dependency"));
        assert!(debug.contains("my-dep"));
    }

    #[test]
    fn test_dependency_serialize() {
        let dep = create_dependency("test-dep", ">=1.0.0");
        let json = serde_json::to_string(&dep).unwrap();
        assert!(json.contains("\"name\":\"test-dep\""));
        assert!(json.contains("\"version_req\":\">=1.0.0\""));
    }

    #[test]
    fn test_dependency_deserialize() {
        let json = r#"{
            "name": "parsed-dep",
            "version_req": "~1.2.0",
            "optional": true,
            "features": ["async"],
            "source": "registry"
        }"#;

        let dep: Dependency = serde_json::from_str(json).unwrap();
        assert_eq!(dep.name, "parsed-dep");
        assert_eq!(dep.version_req, "~1.2.0");
        assert!(dep.optional);
        assert_eq!(dep.features, vec!["async"]);
    }

    #[test]
    fn test_dependency_with_features() {
        let dep = Dependency {
            name: "feature-dep".to_string(),
            version_req: "1.0.0".to_string(),
            optional: false,
            features: vec!["serde".to_string(), "async".to_string(), "full".to_string()],
            source: DependencySource::Registry,
        };

        assert_eq!(dep.features.len(), 3);
        assert!(dep.features.contains(&"serde".to_string()));
    }

    // DependencySource tests
    #[test]
    fn test_dependency_source_default() {
        let source = DependencySource::default();
        assert!(matches!(source, DependencySource::Registry));
    }

    #[test]
    fn test_dependency_source_registry_serialize() {
        let source = DependencySource::Registry;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"registry\"");
    }

    #[test]
    fn test_dependency_source_git_serialize() {
        let source = DependencySource::Git {
            url: "https://github.com/test/repo".to_string(),
            rev: Some("main".to_string()),
        };
        let json = serde_json::to_string(&source).unwrap();
        assert!(json.contains("git"));
        assert!(json.contains("github.com"));
        assert!(json.contains("main"));
    }

    #[test]
    fn test_dependency_source_git_without_rev() {
        let source = DependencySource::Git {
            url: "https://github.com/test/repo".to_string(),
            rev: None,
        };
        let json = serde_json::to_string(&source).unwrap();
        assert!(json.contains("git"));
    }

    #[test]
    fn test_dependency_source_path_serialize() {
        let source = DependencySource::Path {
            path: "/local/path/to/dep".to_string(),
        };
        let json = serde_json::to_string(&source).unwrap();
        assert!(json.contains("path"));
        assert!(json.contains("/local/path"));
    }

    #[test]
    fn test_dependency_source_deserialize_registry() {
        let source: DependencySource = serde_json::from_str("\"registry\"").unwrap();
        assert!(matches!(source, DependencySource::Registry));
    }

    #[test]
    fn test_dependency_source_deserialize_git() {
        let json = r#"{"git": {"url": "https://github.com/test/repo", "rev": "v1.0.0"}}"#;
        let source: DependencySource = serde_json::from_str(json).unwrap();
        match source {
            DependencySource::Git { url, rev } => {
                assert!(url.contains("github.com"));
                assert_eq!(rev, Some("v1.0.0".to_string()));
            }
            _ => panic!("Expected Git source"),
        }
    }

    #[test]
    fn test_dependency_source_clone() {
        let source = DependencySource::Git {
            url: "https://test.com".to_string(),
            rev: Some("abc123".to_string()),
        };
        let cloned = source.clone();
        match cloned {
            DependencySource::Git { url, rev } => {
                assert_eq!(url, "https://test.com");
                assert_eq!(rev, Some("abc123".to_string()));
            }
            _ => panic!("Expected Git source"),
        }
    }

    #[test]
    fn test_dependency_source_debug() {
        let source = DependencySource::Path {
            path: "./local".to_string(),
        };
        let debug = format!("{:?}", source);
        assert!(debug.contains("Path"));
    }

    // ResolvedDependency tests
    #[test]
    fn test_resolved_dependency_clone() {
        let resolved = ResolvedDependency {
            name: "resolved-pkg".to_string(),
            version: Version::parse("1.2.3").unwrap(),
            dependencies: vec![create_dependency("dep-a", "^1.0")],
        };

        let cloned = resolved.clone();
        assert_eq!(resolved.name, cloned.name);
        assert_eq!(resolved.version, cloned.version);
        assert_eq!(resolved.dependencies.len(), cloned.dependencies.len());
    }

    #[test]
    fn test_resolved_dependency_debug() {
        let resolved = ResolvedDependency {
            name: "test-pkg".to_string(),
            version: Version::parse("2.0.0").unwrap(),
            dependencies: vec![],
        };

        let debug = format!("{:?}", resolved);
        assert!(debug.contains("ResolvedDependency"));
        assert!(debug.contains("test-pkg"));
    }

    // DependencyResolver tests
    #[test]
    fn test_dependency_resolver_new() {
        let _resolver = DependencyResolver::new();
        // DependencyResolver created successfully
    }

    #[test]
    fn test_dependency_resolver_default() {
        let _resolver = DependencyResolver::default();
        // DependencyResolver::default() works
    }

    #[test]
    fn test_dependency_resolver_add_package_versions() {
        let mut resolver = DependencyResolver::new();

        resolver.add_package_versions(
            "my-package".to_string(),
            vec![
                create_version_entry("1.0.0", false),
                create_version_entry("1.1.0", false),
            ],
        );

        // Can add another package
        resolver.add_package_versions(
            "other-package".to_string(),
            vec![create_version_entry("2.0.0", false)],
        );
    }

    #[test]
    fn test_dependency_resolution() {
        let mut resolver = DependencyResolver::new();

        // Add package A with versions
        resolver.add_package_versions(
            "package-a".to_string(),
            vec![
                create_version_entry("1.0.0", false),
                create_version_entry("1.1.0", false),
            ],
        );

        let deps = vec![create_dependency("package-a", "^1.0")];

        let root_version = Version::parse("1.0.0").unwrap();
        let resolved = resolver.resolve("root", &root_version, deps);

        assert!(resolved.is_ok());
        let resolved = resolved.unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].name, "package-a");
        assert_eq!(resolved[0].version, Version::parse("1.1.0").unwrap());
    }

    #[test]
    fn test_dependency_resolution_exact_version() {
        let mut resolver = DependencyResolver::new();

        resolver.add_package_versions(
            "exact-pkg".to_string(),
            vec![
                create_version_entry("1.0.0", false),
                create_version_entry("1.1.0", false),
                create_version_entry("2.0.0", false),
            ],
        );

        let deps = vec![create_dependency("exact-pkg", "=1.0.0")];
        let root_version = Version::parse("1.0.0").unwrap();
        let resolved = resolver.resolve("root", &root_version, deps).unwrap();

        assert_eq!(resolved[0].version, Version::parse("1.0.0").unwrap());
    }

    #[test]
    fn test_dependency_resolution_yanked_excluded() {
        let mut resolver = DependencyResolver::new();

        resolver.add_package_versions(
            "yanked-pkg".to_string(),
            vec![
                create_version_entry("1.0.0", false),
                create_version_entry("1.1.0", true), // yanked
                create_version_entry("1.0.5", false),
            ],
        );

        let deps = vec![create_dependency("yanked-pkg", "^1.0")];
        let root_version = Version::parse("1.0.0").unwrap();
        let resolved = resolver.resolve("root", &root_version, deps).unwrap();

        // Should pick 1.0.5 (highest non-yanked that matches)
        assert_eq!(resolved[0].version, Version::parse("1.0.5").unwrap());
    }

    #[test]
    fn test_dependency_resolution_package_not_found() {
        let resolver = DependencyResolver::new();

        let deps = vec![create_dependency("nonexistent", "^1.0")];
        let root_version = Version::parse("1.0.0").unwrap();
        let result = resolver.resolve("root", &root_version, deps);

        assert!(result.is_err());
    }

    #[test]
    fn test_dependency_resolution_no_compatible_version() {
        let mut resolver = DependencyResolver::new();

        resolver.add_package_versions(
            "old-pkg".to_string(),
            vec![create_version_entry("0.5.0", false)],
        );

        let deps = vec![create_dependency("old-pkg", "^1.0")];
        let root_version = Version::parse("1.0.0").unwrap();
        let result = resolver.resolve("root", &root_version, deps);

        assert!(result.is_err());
    }

    #[test]
    fn test_dependency_resolution_invalid_version_req() {
        let mut resolver = DependencyResolver::new();

        resolver
            .add_package_versions("pkg".to_string(), vec![create_version_entry("1.0.0", false)]);

        let deps = vec![create_dependency("pkg", "invalid-req")];
        let root_version = Version::parse("1.0.0").unwrap();
        let result = resolver.resolve("root", &root_version, deps);

        assert!(result.is_err());
    }

    #[test]
    fn test_dependency_resolution_multiple_deps() {
        let mut resolver = DependencyResolver::new();

        resolver
            .add_package_versions("pkg-a".to_string(), vec![create_version_entry("1.0.0", false)]);
        resolver
            .add_package_versions("pkg-b".to_string(), vec![create_version_entry("2.0.0", false)]);
        resolver
            .add_package_versions("pkg-c".to_string(), vec![create_version_entry("3.0.0", false)]);

        let deps = vec![
            create_dependency("pkg-a", "^1.0"),
            create_dependency("pkg-b", "^2.0"),
            create_dependency("pkg-c", "^3.0"),
        ];

        let root_version = Version::parse("1.0.0").unwrap();
        let resolved = resolver.resolve("root", &root_version, deps).unwrap();

        assert_eq!(resolved.len(), 3);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let resolver = DependencyResolver::new();

        // Create circular dependency: A -> B -> A
        let resolved = vec![
            ResolvedDependency {
                name: "package-a".to_string(),
                version: Version::parse("1.0.0").unwrap(),
                dependencies: vec![create_dependency("package-b", "1.0")],
            },
            ResolvedDependency {
                name: "package-b".to_string(),
                version: Version::parse("1.0.0").unwrap(),
                dependencies: vec![create_dependency("package-a", "1.0")],
            },
        ];

        let result = resolver.check_circular_dependencies(&resolved);
        assert!(result.is_err());
    }

    #[test]
    fn test_no_circular_dependency() {
        let resolver = DependencyResolver::new();

        // Linear dependency: A -> B -> C (no cycle)
        let resolved = vec![
            ResolvedDependency {
                name: "package-a".to_string(),
                version: Version::parse("1.0.0").unwrap(),
                dependencies: vec![create_dependency("package-b", "1.0")],
            },
            ResolvedDependency {
                name: "package-b".to_string(),
                version: Version::parse("1.0.0").unwrap(),
                dependencies: vec![create_dependency("package-c", "1.0")],
            },
            ResolvedDependency {
                name: "package-c".to_string(),
                version: Version::parse("1.0.0").unwrap(),
                dependencies: vec![],
            },
        ];

        let result = resolver.check_circular_dependencies(&resolved);
        assert!(result.is_ok());
    }

    #[test]
    fn test_circular_dependency_three_node_cycle() {
        let resolver = DependencyResolver::new();

        // A -> B -> C -> A
        let resolved = vec![
            ResolvedDependency {
                name: "a".to_string(),
                version: Version::parse("1.0.0").unwrap(),
                dependencies: vec![create_dependency("b", "1.0")],
            },
            ResolvedDependency {
                name: "b".to_string(),
                version: Version::parse("1.0.0").unwrap(),
                dependencies: vec![create_dependency("c", "1.0")],
            },
            ResolvedDependency {
                name: "c".to_string(),
                version: Version::parse("1.0.0").unwrap(),
                dependencies: vec![create_dependency("a", "1.0")],
            },
        ];

        let result = resolver.check_circular_dependencies(&resolved);
        assert!(result.is_err());
    }

    // calculate_install_order tests
    #[test]
    fn test_calculate_install_order_simple() {
        let resolver = DependencyResolver::new();

        let resolved = vec![
            ResolvedDependency {
                name: "root".to_string(),
                version: Version::parse("1.0.0").unwrap(),
                dependencies: vec![create_dependency("leaf", "1.0")],
            },
            ResolvedDependency {
                name: "leaf".to_string(),
                version: Version::parse("1.0.0").unwrap(),
                dependencies: vec![],
            },
        ];

        let order = resolver.calculate_install_order(&resolved).unwrap();
        assert_eq!(order.len(), 2);
        // leaf should come before root (dependencies first)
        let leaf_pos = order.iter().position(|x| x == "leaf").unwrap();
        let root_pos = order.iter().position(|x| x == "root").unwrap();
        assert!(leaf_pos < root_pos);
    }

    #[test]
    fn test_calculate_install_order_chain() {
        let resolver = DependencyResolver::new();

        // A depends on B, B depends on C
        let resolved = vec![
            ResolvedDependency {
                name: "a".to_string(),
                version: Version::parse("1.0.0").unwrap(),
                dependencies: vec![create_dependency("b", "1.0")],
            },
            ResolvedDependency {
                name: "b".to_string(),
                version: Version::parse("1.0.0").unwrap(),
                dependencies: vec![create_dependency("c", "1.0")],
            },
            ResolvedDependency {
                name: "c".to_string(),
                version: Version::parse("1.0.0").unwrap(),
                dependencies: vec![],
            },
        ];

        let order = resolver.calculate_install_order(&resolved).unwrap();
        let c_pos = order.iter().position(|x| x == "c").unwrap();
        let b_pos = order.iter().position(|x| x == "b").unwrap();
        let a_pos = order.iter().position(|x| x == "a").unwrap();

        assert!(c_pos < b_pos);
        assert!(b_pos < a_pos);
    }

    #[test]
    fn test_calculate_install_order_no_deps() {
        let resolver = DependencyResolver::new();

        let resolved = vec![ResolvedDependency {
            name: "standalone".to_string(),
            version: Version::parse("1.0.0").unwrap(),
            dependencies: vec![],
        }];

        let order = resolver.calculate_install_order(&resolved).unwrap();
        assert_eq!(order, vec!["standalone"]);
    }

    #[test]
    fn test_calculate_install_order_empty() {
        let resolver = DependencyResolver::new();
        let resolved: Vec<ResolvedDependency> = vec![];

        let order = resolver.calculate_install_order(&resolved).unwrap();
        assert!(order.is_empty());
    }

    // DependencyConflict tests
    #[test]
    fn test_dependency_conflict_clone() {
        let conflict = DependencyConflict {
            package: "conflicting-pkg".to_string(),
            required_by: vec![
                ConflictRequirement {
                    package: "pkg-a".to_string(),
                    version_req: "^1.0".to_string(),
                },
                ConflictRequirement {
                    package: "pkg-b".to_string(),
                    version_req: "^2.0".to_string(),
                },
            ],
        };

        let cloned = conflict.clone();
        assert_eq!(conflict.package, cloned.package);
        assert_eq!(conflict.required_by.len(), cloned.required_by.len());
    }

    #[test]
    fn test_dependency_conflict_debug() {
        let conflict = DependencyConflict {
            package: "test-conflict".to_string(),
            required_by: vec![],
        };

        let debug = format!("{:?}", conflict);
        assert!(debug.contains("DependencyConflict"));
        assert!(debug.contains("test-conflict"));
    }

    #[test]
    fn test_dependency_conflict_serialize() {
        let conflict = DependencyConflict {
            package: "pkg".to_string(),
            required_by: vec![ConflictRequirement {
                package: "requirer".to_string(),
                version_req: "^1.0".to_string(),
            }],
        };

        let json = serde_json::to_string(&conflict).unwrap();
        assert!(json.contains("\"package\":\"pkg\""));
        assert!(json.contains("requirer"));
    }

    // ConflictRequirement tests
    #[test]
    fn test_conflict_requirement_clone() {
        let req = ConflictRequirement {
            package: "req-pkg".to_string(),
            version_req: ">=1.0.0".to_string(),
        };

        let cloned = req.clone();
        assert_eq!(req.package, cloned.package);
        assert_eq!(req.version_req, cloned.version_req);
    }

    #[test]
    fn test_conflict_requirement_debug() {
        let req = ConflictRequirement {
            package: "debug-pkg".to_string(),
            version_req: "~1.2".to_string(),
        };

        let debug = format!("{:?}", req);
        assert!(debug.contains("ConflictRequirement"));
    }

    #[test]
    fn test_conflict_requirement_serialize() {
        let req = ConflictRequirement {
            package: "ser-pkg".to_string(),
            version_req: "*".to_string(),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"package\":\"ser-pkg\""));
        assert!(json.contains("\"version_req\":\"*\""));
    }

    #[test]
    fn test_conflict_requirement_deserialize() {
        let json = r#"{"package": "de-pkg", "version_req": "^3.0"}"#;
        let req: ConflictRequirement = serde_json::from_str(json).unwrap();
        assert_eq!(req.package, "de-pkg");
        assert_eq!(req.version_req, "^3.0");
    }

    // Transitive dependency tests
    #[test]
    fn test_transitive_dependency_resolution() {
        let mut resolver = DependencyResolver::new();

        // pkg-a depends on pkg-b (via dependencies field in VersionEntry)
        let mut deps_map = HashMap::new();
        deps_map.insert("pkg-b".to_string(), "^1.0".to_string());

        resolver.add_package_versions(
            "pkg-a".to_string(),
            vec![VersionEntry {
                version: "1.0.0".to_string(),
                download_url: "https://example.com/a.tar.gz".to_string(),
                checksum: "abc".to_string(),
                size: 1000,
                published_at: "2025-01-01".to_string(),
                yanked: false,
                min_mockforge_version: None,
                dependencies: deps_map,
            }],
        );

        resolver
            .add_package_versions("pkg-b".to_string(), vec![create_version_entry("1.0.0", false)]);

        let deps = vec![create_dependency("pkg-a", "^1.0")];
        let root_version = Version::parse("1.0.0").unwrap();
        let resolved = resolver.resolve("root", &root_version, deps).unwrap();

        // Should resolve both pkg-a and pkg-b
        assert_eq!(resolved.len(), 2);
        let names: Vec<&str> = resolved.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"pkg-a"));
        assert!(names.contains(&"pkg-b"));
    }
}
