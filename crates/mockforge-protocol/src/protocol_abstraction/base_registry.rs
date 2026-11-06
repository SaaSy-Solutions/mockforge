//! Generic base spec registry for protocol-agnostic fixture management
//!
//! Provides a `BaseSpecRegistry<F>` that can be used by any protocol crate to
//! manage fixtures with consistent lookup, filtering, and file-loading semantics.

use crate::Result;
use serde::de::DeserializeOwned;
use std::collections::HashMap;

/// Trait that every protocol fixture must implement so it can be stored
/// in a [`BaseSpecRegistry`].
pub trait ProtocolFixture: Send + Sync + Clone + std::fmt::Debug {
    /// Unique human-readable identifier (e.g. "get_users_200")
    fn identifier(&self) -> &str;

    /// Key used for lookup (e.g. "GET /users")
    fn lookup_key(&self) -> String;

    /// Operation type (e.g. "GET", "Query", "publish")
    fn operation_type(&self) -> &str;

    /// Optional JSON schema describing the output
    fn output_schema(&self) -> Option<&str>;

    /// Optional JSON schema describing the input
    fn input_schema(&self) -> Option<&str>;

    /// Arbitrary metadata as key-value pairs
    fn metadata(&self) -> HashMap<String, String>;
}

/// Generic registry that stores and indexes fixtures of any protocol.
#[derive(Debug, Clone)]
pub struct BaseSpecRegistry<F: ProtocolFixture> {
    fixtures: Vec<F>,
    /// Index from lookup_key -> position in `fixtures`
    key_index: HashMap<String, usize>,
    /// Index from identifier -> position in `fixtures`
    id_index: HashMap<String, usize>,
}

impl<F: ProtocolFixture> BaseSpecRegistry<F> {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            fixtures: Vec::new(),
            key_index: HashMap::new(),
            id_index: HashMap::new(),
        }
    }

    /// Add a fixture, updating indices.
    pub fn add_fixture(&mut self, fixture: F) {
        let idx = self.fixtures.len();
        self.key_index.insert(fixture.lookup_key(), idx);
        self.id_index.insert(fixture.identifier().to_string(), idx);
        self.fixtures.push(fixture);
    }

    /// Find a fixture by its lookup key.
    pub fn find_by_key(&self, key: &str) -> Option<&F> {
        self.key_index.get(key).map(|&idx| &self.fixtures[idx])
    }

    /// Find a fixture by its identifier.
    pub fn find_by_identifier(&self, identifier: &str) -> Option<&F> {
        self.id_index.get(identifier).map(|&idx| &self.fixtures[idx])
    }

    /// Iterate over all fixtures.
    pub fn fixtures(&self) -> impl Iterator<Item = &F> {
        self.fixtures.iter()
    }

    /// Return all distinct operation types present in the registry.
    pub fn operations(&self) -> Vec<super::SpecOperation> {
        self.fixtures
            .iter()
            .map(|f| super::SpecOperation {
                name: f.identifier().to_string(),
                path: f.lookup_key(),
                operation_type: f.operation_type().to_string(),
                input_schema: f.input_schema().map(String::from),
                output_schema: f.output_schema().map(String::from),
                metadata: f.metadata(),
            })
            .collect()
    }

    /// Find an operation by operation type and path.
    pub fn find_operation(&self, operation: &str, path: &str) -> Option<super::SpecOperation> {
        self.fixtures.iter().find_map(|f| {
            if f.operation_type() == operation && f.lookup_key().contains(path) {
                Some(super::SpecOperation {
                    name: f.identifier().to_string(),
                    path: f.lookup_key(),
                    operation_type: f.operation_type().to_string(),
                    input_schema: f.input_schema().map(String::from),
                    output_schema: f.output_schema().map(String::from),
                    metadata: f.metadata(),
                })
            } else {
                None
            }
        })
    }

    /// Number of fixtures in the registry.
    pub fn len(&self) -> usize {
        self.fixtures.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.fixtures.is_empty()
    }
}

impl<F: ProtocolFixture + DeserializeOwned> BaseSpecRegistry<F> {
    /// Load fixtures from a YAML or JSON file.
    ///
    /// The file must deserialize to `Vec<F>`.
    pub fn load_fixtures(&mut self, path: &std::path::Path) -> Result<usize> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            crate::Error::internal(format!("Failed to read fixture file {}: {}", path.display(), e))
        })?;

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let fixtures: Vec<F> = match ext {
            "yaml" | "yml" => serde_yaml::from_str(&content).map_err(|e| {
                crate::Error::config(format!("Failed to parse YAML fixtures: {}", e))
            })?,
            "json" => serde_json::from_str(&content).map_err(|e| {
                crate::Error::config(format!("Failed to parse JSON fixtures: {}", e))
            })?,
            _ => {
                return Err(crate::Error::internal(format!(
                    "Unsupported fixture file extension: {ext}"
                )));
            }
        };

        let count = fixtures.len();
        for f in fixtures {
            self.add_fixture(f);
        }
        Ok(count)
    }
}

impl<F: ProtocolFixture> Default for BaseSpecRegistry<F> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct TestFixture {
        id: String,
        key: String,
        op_type: String,
    }

    impl ProtocolFixture for TestFixture {
        fn identifier(&self) -> &str {
            &self.id
        }
        fn lookup_key(&self) -> String {
            self.key.clone()
        }
        fn operation_type(&self) -> &str {
            &self.op_type
        }
        fn output_schema(&self) -> Option<&str> {
            None
        }
        fn input_schema(&self) -> Option<&str> {
            None
        }
        fn metadata(&self) -> HashMap<String, String> {
            HashMap::new()
        }
    }

    fn sample_fixture(id: &str, key: &str, op: &str) -> TestFixture {
        TestFixture {
            id: id.to_string(),
            key: key.to_string(),
            op_type: op.to_string(),
        }
    }

    #[test]
    fn test_new_registry_is_empty() {
        let reg = BaseSpecRegistry::<TestFixture>::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn test_add_and_find_by_key() {
        let mut reg = BaseSpecRegistry::new();
        reg.add_fixture(sample_fixture("get_users", "GET /users", "GET"));
        assert_eq!(reg.len(), 1);
        assert!(!reg.is_empty());

        let found = reg.find_by_key("GET /users");
        assert!(found.is_some());
        assert_eq!(found.map(|f| f.identifier()), Some("get_users"));
    }

    #[test]
    fn test_find_by_identifier() {
        let mut reg = BaseSpecRegistry::new();
        reg.add_fixture(sample_fixture("get_users", "GET /users", "GET"));
        let found = reg.find_by_identifier("get_users");
        assert!(found.is_some());
    }

    #[test]
    fn test_find_missing_returns_none() {
        let reg = BaseSpecRegistry::<TestFixture>::new();
        assert!(reg.find_by_key("nonexistent").is_none());
        assert!(reg.find_by_identifier("nonexistent").is_none());
    }

    #[test]
    fn test_fixtures_iterator() {
        let mut reg = BaseSpecRegistry::new();
        reg.add_fixture(sample_fixture("a", "GET /a", "GET"));
        reg.add_fixture(sample_fixture("b", "POST /b", "POST"));
        let ids: Vec<&str> = reg.fixtures().map(|f| f.identifier()).collect();
        assert_eq!(ids, vec!["a", "b"]);
    }

    #[test]
    fn test_operations() {
        let mut reg = BaseSpecRegistry::new();
        reg.add_fixture(sample_fixture("get_users", "GET /users", "GET"));
        reg.add_fixture(sample_fixture("post_users", "POST /users", "POST"));
        let ops = reg.operations();
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].operation_type, "GET");
        assert_eq!(ops[1].operation_type, "POST");
    }

    #[test]
    fn test_find_operation() {
        let mut reg = BaseSpecRegistry::new();
        reg.add_fixture(sample_fixture("get_users", "GET /users", "GET"));
        let op = reg.find_operation("GET", "/users");
        assert!(op.is_some());
        assert_eq!(op.as_ref().map(|o| o.name.as_str()), Some("get_users"));

        let missing = reg.find_operation("DELETE", "/users");
        assert!(missing.is_none());
    }

    #[test]
    fn test_load_fixtures_json() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let path = dir.path().join("fixtures.json");
        let fixtures = vec![
            sample_fixture("a", "GET /a", "GET"),
            sample_fixture("b", "POST /b", "POST"),
        ];
        std::fs::write(&path, serde_json::to_string(&fixtures).expect("serialize")).expect("write");

        let mut reg = BaseSpecRegistry::<TestFixture>::new();
        let count = reg.load_fixtures(&path).expect("load");
        assert_eq!(count, 2);
        assert_eq!(reg.len(), 2);
    }

    #[test]
    fn test_load_fixtures_yaml() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let path = dir.path().join("fixtures.yaml");
        let fixtures = vec![sample_fixture("x", "GET /x", "GET")];
        std::fs::write(&path, serde_yaml::to_string(&fixtures).expect("serialize")).expect("write");

        let mut reg = BaseSpecRegistry::<TestFixture>::new();
        let count = reg.load_fixtures(&path).expect("load");
        assert_eq!(count, 1);
        assert!(reg.find_by_identifier("x").is_some());
    }

    #[test]
    fn test_load_fixtures_unsupported_extension() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let path = dir.path().join("fixtures.txt");
        std::fs::write(&path, "irrelevant").expect("write");

        let mut reg = BaseSpecRegistry::<TestFixture>::new();
        assert!(reg.load_fixtures(&path).is_err());
    }

    #[test]
    fn test_default_is_empty() {
        let reg = BaseSpecRegistry::<TestFixture>::default();
        assert!(reg.is_empty());
    }
}
