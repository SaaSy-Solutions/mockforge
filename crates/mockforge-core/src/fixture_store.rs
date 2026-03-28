//! Generic fixture loading utilities for protocol crates.
//!
//! Provides a shared `load_fixtures_from_dir` function that replaces the duplicated
//! YAML/JSON fixture loading logic across Kafka, MQTT, AMQP, SMTP, FTP, and TCP crates.

use crate::Result;
use serde::de::DeserializeOwned;
use std::path::Path;

/// How to handle parse errors when loading fixture files
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureLoadErrorMode {
    /// Stop and return the first error encountered
    FailFast,
    /// Log a warning and continue loading remaining files
    WarnAndContinue,
}

/// Whether each file contains a single fixture or an array of fixtures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureFileGranularity {
    /// Each file deserializes into exactly one `T`
    Single,
    /// Each file deserializes into a `Vec<T>`
    Array,
}

/// Supported file formats for fixture loading
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureFileFormat {
    /// YAML (.yaml, .yml)
    Yaml,
    /// JSON (.json)
    Json,
}

/// Configuration for loading fixtures from a directory
#[derive(Debug, Clone)]
pub struct FixtureLoadOptions {
    /// Which file formats to accept
    pub formats: Vec<FixtureFileFormat>,
    /// How to handle individual file parse errors
    pub error_mode: FixtureLoadErrorMode,
    /// Whether each file holds one fixture or an array
    pub granularity: FixtureFileGranularity,
}

impl FixtureLoadOptions {
    /// YAML-only, single fixture per file, warn and continue
    pub fn yaml_single() -> Self {
        Self {
            formats: vec![FixtureFileFormat::Yaml],
            error_mode: FixtureLoadErrorMode::WarnAndContinue,
            granularity: FixtureFileGranularity::Single,
        }
    }

    /// YAML + JSON, single fixture per file, warn and continue
    pub fn yaml_json_single() -> Self {
        Self {
            formats: vec![FixtureFileFormat::Yaml, FixtureFileFormat::Json],
            error_mode: FixtureLoadErrorMode::WarnAndContinue,
            granularity: FixtureFileGranularity::Single,
        }
    }

    /// YAML-only, array of fixtures per file, fail fast
    pub fn yaml_array_strict() -> Self {
        Self {
            formats: vec![FixtureFileFormat::Yaml],
            error_mode: FixtureLoadErrorMode::FailFast,
            granularity: FixtureFileGranularity::Array,
        }
    }
}

/// Load fixtures of type `T` from all matching files in a directory.
///
/// Walks the directory (non-recursively), filters by the configured file extensions,
/// and deserializes each file according to the `FixtureLoadOptions`.
///
/// # Returns
/// A `Vec<T>` of all successfully loaded fixtures. Files that fail to parse are
/// either skipped (with a warning) or cause an immediate error, depending on
/// `options.error_mode`.
///
/// # Example
/// ```ignore
/// use mockforge_core::fixture_store::{load_fixtures_from_dir, FixtureLoadOptions};
///
/// let fixtures: Vec<MyFixture> = load_fixtures_from_dir(
///     Path::new("./fixtures"),
///     &FixtureLoadOptions::yaml_json_single(),
/// )?;
/// ```
pub fn load_fixtures_from_dir<T: DeserializeOwned>(
    dir: &Path,
    options: &FixtureLoadOptions,
) -> Result<Vec<T>> {
    if !dir.exists() {
        tracing::debug!("Fixture directory does not exist: {}", dir.display());
        return Ok(Vec::new());
    }

    let entries = std::fs::read_dir(dir).map_err(|e| {
        crate::Error::io_with_context(
            format!("reading fixture directory {}", dir.display()),
            e.to_string(),
        )
    })?;

    let mut fixtures = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to read directory entry: {}", e);
                continue;
            }
        };

        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let format = match path.extension().and_then(|e| e.to_str()) {
            Some("yaml" | "yml") if options.formats.contains(&FixtureFileFormat::Yaml) => {
                FixtureFileFormat::Yaml
            }
            Some("json") if options.formats.contains(&FixtureFileFormat::Json) => {
                FixtureFileFormat::Json
            }
            _ => continue,
        };

        match load_fixture_file::<T>(&path, format, options.granularity) {
            Ok(loaded) => fixtures.extend(loaded),
            Err(e) => match options.error_mode {
                FixtureLoadErrorMode::FailFast => return Err(e),
                FixtureLoadErrorMode::WarnAndContinue => {
                    tracing::warn!("Failed to load fixture {}: {}", path.display(), e);
                }
            },
        }
    }

    tracing::debug!("Loaded {} fixtures from {}", fixtures.len(), dir.display());
    Ok(fixtures)
}

fn load_fixture_file<T: DeserializeOwned>(
    path: &Path,
    format: FixtureFileFormat,
    granularity: FixtureFileGranularity,
) -> Result<Vec<T>> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        crate::Error::io_with_context(format!("reading fixture {}", path.display()), e.to_string())
    })?;

    match (format, granularity) {
        (FixtureFileFormat::Yaml, FixtureFileGranularity::Single) => {
            let fixture: T = serde_yaml::from_str(&content)?;
            Ok(vec![fixture])
        }
        (FixtureFileFormat::Yaml, FixtureFileGranularity::Array) => {
            let fixtures: Vec<T> = serde_yaml::from_str(&content)?;
            Ok(fixtures)
        }
        (FixtureFileFormat::Json, FixtureFileGranularity::Single) => {
            let fixture: T = serde_json::from_str(&content)?;
            Ok(vec![fixture])
        }
        (FixtureFileFormat::Json, FixtureFileGranularity::Array) => {
            let fixtures: Vec<T> = serde_json::from_str(&content)?;
            Ok(fixtures)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::fs;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestFixture {
        name: String,
        value: i32,
    }

    #[test]
    fn test_load_yaml_single() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("test.yaml"), "name: hello\nvalue: 42\n").unwrap();

        let fixtures: Vec<TestFixture> =
            load_fixtures_from_dir(dir.path(), &FixtureLoadOptions::yaml_single()).unwrap();

        assert_eq!(fixtures.len(), 1);
        assert_eq!(fixtures[0].name, "hello");
        assert_eq!(fixtures[0].value, 42);
    }

    #[test]
    fn test_load_json_single() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("test.json"), r#"{"name": "world", "value": 99}"#).unwrap();

        let fixtures: Vec<TestFixture> =
            load_fixtures_from_dir(dir.path(), &FixtureLoadOptions::yaml_json_single()).unwrap();

        assert_eq!(fixtures.len(), 1);
        assert_eq!(fixtures[0].name, "world");
    }

    #[test]
    fn test_load_yaml_array() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("items.yaml"), "- name: a\n  value: 1\n- name: b\n  value: 2\n")
            .unwrap();

        let fixtures: Vec<TestFixture> =
            load_fixtures_from_dir(dir.path(), &FixtureLoadOptions::yaml_array_strict()).unwrap();

        assert_eq!(fixtures.len(), 2);
    }

    #[test]
    fn test_nonexistent_dir_returns_empty() {
        let fixtures: Vec<TestFixture> = load_fixtures_from_dir(
            Path::new("/nonexistent/path"),
            &FixtureLoadOptions::yaml_single(),
        )
        .unwrap();

        assert!(fixtures.is_empty());
    }

    #[test]
    fn test_warn_and_continue_skips_bad_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("good.yaml"), "name: ok\nvalue: 1\n").unwrap();
        fs::write(dir.path().join("bad.yaml"), "not valid yaml: [[[").unwrap();

        let fixtures: Vec<TestFixture> =
            load_fixtures_from_dir(dir.path(), &FixtureLoadOptions::yaml_single()).unwrap();

        assert_eq!(fixtures.len(), 1);
        assert_eq!(fixtures[0].name, "ok");
    }

    #[test]
    fn test_fail_fast_propagates_error() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("bad.yaml"), "not valid yaml: [[[").unwrap();

        let result: Result<Vec<TestFixture>> =
            load_fixtures_from_dir(dir.path(), &FixtureLoadOptions::yaml_array_strict());

        assert!(result.is_err());
    }

    #[test]
    fn test_ignores_non_matching_extensions() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("readme.txt"), "not a fixture").unwrap();
        fs::write(dir.path().join("data.yaml"), "name: x\nvalue: 0\n").unwrap();

        let fixtures: Vec<TestFixture> =
            load_fixtures_from_dir(dir.path(), &FixtureLoadOptions::yaml_single()).unwrap();

        assert_eq!(fixtures.len(), 1);
    }
}
