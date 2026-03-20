//! Generic fixture loading utilities for protocol crates
//!
//! Provides a shared implementation of the directory-enumerate-parse-collect pattern
//! used by most protocol crates (Kafka, MQTT, AMQP, SMTP, FTP, TCP) to load fixtures
//! from YAML/JSON files.

use crate::Result;
use serde::de::DeserializeOwned;
use std::path::Path;
use tracing::{debug, warn};

/// Load fixtures from a directory, deserializing each YAML/YML/JSON file as type `T`.
///
/// This function handles the common fixture loading pattern shared across protocol crates:
/// 1. Check if the directory exists (returns empty vec if not, or error based on `missing_dir_ok`)
/// 2. Iterate over directory entries
/// 3. Filter for `.yaml`, `.yml`, and `.json` files
/// 4. Deserialize each file as `T`
/// 5. Log warnings for files that fail to parse (but continue loading others)
///
/// # Type Parameters
/// - `T`: The fixture type to deserialize into. Must implement `DeserializeOwned`.
///
/// # Arguments
/// - `dir`: Path to the fixtures directory
/// - `missing_dir_ok`: If `true`, returns an empty vec when the directory doesn't exist.
///   If `false`, returns an error.
///
/// # Returns
/// A `Vec<T>` of successfully loaded fixtures.
///
/// # Example
/// ```rust,no_run
/// use serde::Deserialize;
/// use mockforge_core::fixture_loader::load_fixtures_from_dir;
///
/// #[derive(Debug, Deserialize)]
/// struct MyFixture {
///     identifier: String,
///     name: String,
/// }
///
/// let fixtures: Vec<MyFixture> = load_fixtures_from_dir("./fixtures", true).unwrap();
/// ```
pub fn load_fixtures_from_dir<T: DeserializeOwned>(
    dir: impl AsRef<Path>,
    missing_dir_ok: bool,
) -> Result<Vec<T>> {
    let dir = dir.as_ref();

    if !dir.exists() {
        if missing_dir_ok {
            debug!("Fixtures directory does not exist: {:?}", dir);
            return Ok(Vec::new());
        }
        return Err(crate::Error::generic(format!(
            "Fixtures directory does not exist: {}",
            dir.display()
        )));
    }

    let mut fixtures = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let extension = path.extension().and_then(|s| s.to_str());
        match extension {
            Some("yaml") | Some("yml") | Some("json") => match load_fixture_file::<T>(&path) {
                Ok(fixture) => fixtures.push(fixture),
                Err(e) => {
                    warn!("Failed to load fixture from {:?}: {}", path, e);
                }
            },
            _ => {
                debug!("Skipping non-fixture file: {:?}", path);
            }
        }
    }

    Ok(fixtures)
}

/// Load fixtures from a directory where each file contains a `Vec<T>`.
///
/// This variant is for protocols like Kafka where each YAML file contains
/// a list of fixtures rather than a single fixture.
///
/// # Arguments
/// - `dir`: Path to the fixtures directory
/// - `missing_dir_ok`: If `true`, returns an empty vec when the directory doesn't exist.
///
/// # Returns
/// A flat `Vec<T>` of all fixtures from all files.
pub fn load_fixture_list_from_dir<T: DeserializeOwned>(
    dir: impl AsRef<Path>,
    missing_dir_ok: bool,
) -> Result<Vec<T>> {
    let dir = dir.as_ref();

    if !dir.exists() {
        if missing_dir_ok {
            debug!("Fixtures directory does not exist: {:?}", dir);
            return Ok(Vec::new());
        }
        return Err(crate::Error::generic(format!(
            "Fixtures directory does not exist: {}",
            dir.display()
        )));
    }

    let mut fixtures = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let extension = path.extension().and_then(|s| s.to_str());
        match extension {
            Some("yaml") | Some("yml") | Some("json") => match load_fixture_file::<Vec<T>>(&path) {
                Ok(file_fixtures) => fixtures.extend(file_fixtures),
                Err(e) => {
                    warn!("Failed to load fixtures from {:?}: {}", path, e);
                }
            },
            _ => {
                debug!("Skipping non-fixture file: {:?}", path);
            }
        }
    }

    Ok(fixtures)
}

/// Load and deserialize a single fixture file (YAML or JSON).
fn load_fixture_file<T: DeserializeOwned>(path: &Path) -> Result<T> {
    debug!("Loading fixture from: {:?}", path);
    let content = std::fs::read_to_string(path)?;

    let extension = path.extension().and_then(|s| s.to_str());
    let fixture: T = if extension == Some("json") {
        serde_json::from_str(&content).map_err(|e| {
            crate::Error::generic(format!("Failed to parse JSON fixture {:?}: {}", path, e))
        })?
    } else {
        serde_yaml::from_str(&content).map_err(|e| {
            crate::Error::generic(format!("Failed to parse YAML fixture {:?}: {}", path, e))
        })?
    };

    Ok(fixture)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use tempfile::TempDir;

    #[derive(Debug, Clone, Deserialize, PartialEq)]
    struct TestFixture {
        identifier: String,
        name: String,
        value: Option<i32>,
    }

    #[test]
    fn test_load_fixtures_from_dir_empty() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures: Vec<TestFixture> = load_fixtures_from_dir(temp_dir.path(), true).unwrap();
        assert!(fixtures.is_empty());
    }

    #[test]
    fn test_load_fixtures_from_dir_nonexistent_ok() {
        let fixtures: Vec<TestFixture> = load_fixtures_from_dir("/nonexistent/path", true).unwrap();
        assert!(fixtures.is_empty());
    }

    #[test]
    fn test_load_fixtures_from_dir_nonexistent_error() {
        let result: Result<Vec<TestFixture>> = load_fixtures_from_dir("/nonexistent/path", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_fixtures_from_dir_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_content = "identifier: test-1\nname: Test Fixture\nvalue: 42\n";
        std::fs::write(temp_dir.path().join("fixture.yaml"), yaml_content).unwrap();

        let fixtures: Vec<TestFixture> = load_fixtures_from_dir(temp_dir.path(), true).unwrap();
        assert_eq!(fixtures.len(), 1);
        assert_eq!(fixtures[0].identifier, "test-1");
        assert_eq!(fixtures[0].value, Some(42));
    }

    #[test]
    fn test_load_fixtures_from_dir_yml() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_content = "identifier: test-yml\nname: YML Fixture\n";
        std::fs::write(temp_dir.path().join("fixture.yml"), yaml_content).unwrap();

        let fixtures: Vec<TestFixture> = load_fixtures_from_dir(temp_dir.path(), true).unwrap();
        assert_eq!(fixtures.len(), 1);
        assert_eq!(fixtures[0].identifier, "test-yml");
    }

    #[test]
    fn test_load_fixtures_from_dir_json() {
        let temp_dir = TempDir::new().unwrap();
        let json_content = r#"{"identifier": "test-json", "name": "JSON Fixture", "value": 99}"#;
        std::fs::write(temp_dir.path().join("fixture.json"), json_content).unwrap();

        let fixtures: Vec<TestFixture> = load_fixtures_from_dir(temp_dir.path(), true).unwrap();
        assert_eq!(fixtures.len(), 1);
        assert_eq!(fixtures[0].identifier, "test-json");
        assert_eq!(fixtures[0].value, Some(99));
    }

    #[test]
    fn test_load_fixtures_from_dir_mixed() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join("a.yaml"), "identifier: yaml-fix\nname: YAML\n")
            .unwrap();
        std::fs::write(
            temp_dir.path().join("b.json"),
            r#"{"identifier": "json-fix", "name": "JSON"}"#,
        )
        .unwrap();
        std::fs::write(temp_dir.path().join("c.txt"), "not a fixture").unwrap();

        let fixtures: Vec<TestFixture> = load_fixtures_from_dir(temp_dir.path(), true).unwrap();
        assert_eq!(fixtures.len(), 2);
    }

    #[test]
    fn test_load_fixtures_from_dir_invalid_files_skipped() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join("valid.yaml"), "identifier: valid\nname: Valid\n")
            .unwrap();
        std::fs::write(temp_dir.path().join("invalid.yaml"), "this is not valid yaml: [unclosed")
            .unwrap();
        std::fs::write(temp_dir.path().join("invalid.json"), "{invalid json}").unwrap();

        let fixtures: Vec<TestFixture> = load_fixtures_from_dir(temp_dir.path(), true).unwrap();
        assert_eq!(fixtures.len(), 1);
        assert_eq!(fixtures[0].identifier, "valid");
    }

    #[test]
    fn test_load_fixture_list_from_dir() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_content =
            "- identifier: list-1\n  name: First\n- identifier: list-2\n  name: Second\n";
        std::fs::write(temp_dir.path().join("fixtures.yaml"), yaml_content).unwrap();

        let fixtures: Vec<TestFixture> = load_fixture_list_from_dir(temp_dir.path(), true).unwrap();
        assert_eq!(fixtures.len(), 2);
        assert_eq!(fixtures[0].identifier, "list-1");
        assert_eq!(fixtures[1].identifier, "list-2");
    }

    #[test]
    fn test_load_fixture_list_from_dir_multiple_files() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join("file1.yaml"), "- identifier: a\n  name: A\n").unwrap();
        std::fs::write(temp_dir.path().join("file2.yaml"), "- identifier: b\n  name: B\n").unwrap();

        let fixtures: Vec<TestFixture> = load_fixture_list_from_dir(temp_dir.path(), true).unwrap();
        assert_eq!(fixtures.len(), 2);
    }

    #[test]
    fn test_load_fixture_list_from_dir_nonexistent_ok() {
        let fixtures: Vec<TestFixture> =
            load_fixture_list_from_dir("/nonexistent/path", true).unwrap();
        assert!(fixtures.is_empty());
    }

    #[test]
    fn test_load_fixture_list_from_dir_nonexistent_error() {
        let result: Result<Vec<TestFixture>> =
            load_fixture_list_from_dir("/nonexistent/path", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_fixtures_skips_directories() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        std::fs::write(temp_dir.path().join("fixture.yaml"), "identifier: test\nname: Test\n")
            .unwrap();

        let fixtures: Vec<TestFixture> = load_fixtures_from_dir(temp_dir.path(), true).unwrap();
        assert_eq!(fixtures.len(), 1);
    }
}
