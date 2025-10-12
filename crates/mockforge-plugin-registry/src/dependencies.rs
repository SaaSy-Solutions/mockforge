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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencySource {
    Registry,
    Git { url: String, rev: Option<String> },
    Path { path: String },
}

impl Default for DependencySource {
    fn default() -> Self {
        Self::Registry
    }
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

    #[test]
    fn test_dependency_resolution() {
        let mut resolver = DependencyResolver::new();

        // Add package A with versions
        resolver.add_package_versions(
            "package-a".to_string(),
            vec![
                VersionEntry {
                    version: "1.0.0".to_string(),
                    download_url: "https://example.com/a-1.0.0".to_string(),
                    checksum: "abc".to_string(),
                    size: 1000,
                    published_at: "2025-01-01".to_string(),
                    yanked: false,
                    min_mockforge_version: None,
                    dependencies: HashMap::new(),
                },
                VersionEntry {
                    version: "1.1.0".to_string(),
                    download_url: "https://example.com/a-1.1.0".to_string(),
                    checksum: "def".to_string(),
                    size: 1100,
                    published_at: "2025-01-02".to_string(),
                    yanked: false,
                    min_mockforge_version: None,
                    dependencies: HashMap::new(),
                },
            ],
        );

        let deps = vec![Dependency {
            name: "package-a".to_string(),
            version_req: "^1.0".to_string(),
            optional: false,
            features: vec![],
            source: DependencySource::Registry,
        }];

        let root_version = Version::parse("1.0.0").unwrap();
        let resolved = resolver.resolve("root", &root_version, deps);

        assert!(resolved.is_ok());
        let resolved = resolved.unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].name, "package-a");
        assert_eq!(resolved[0].version, Version::parse("1.1.0").unwrap());
    }

    #[test]
    fn test_circular_dependency_detection() {
        let resolver = DependencyResolver::new();

        // Create circular dependency: A -> B -> A
        let resolved = vec![
            ResolvedDependency {
                name: "package-a".to_string(),
                version: Version::parse("1.0.0").unwrap(),
                dependencies: vec![Dependency {
                    name: "package-b".to_string(),
                    version_req: "1.0".to_string(),
                    optional: false,
                    features: vec![],
                    source: DependencySource::Registry,
                }],
            },
            ResolvedDependency {
                name: "package-b".to_string(),
                version: Version::parse("1.0.0").unwrap(),
                dependencies: vec![Dependency {
                    name: "package-a".to_string(),
                    version_req: "1.0".to_string(),
                    optional: false,
                    features: vec![],
                    source: DependencySource::Registry,
                }],
            },
        ];

        let result = resolver.check_circular_dependencies(&resolved);
        assert!(result.is_err());
    }
}
