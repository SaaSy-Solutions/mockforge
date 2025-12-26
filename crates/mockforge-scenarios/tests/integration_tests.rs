//! Integration tests for the scenarios marketplace

use mockforge_scenarios::*;
use tempfile::TempDir;

#[tokio::test]
async fn test_scenario_manifest_validation() {
    let manifest = ScenarioManifest::new(
        "test-scenario".to_string(),
        "1.0.0".to_string(),
        "Test Scenario".to_string(),
        "A test scenario".to_string(),
    );

    assert!(manifest.validate().is_ok());
    assert_eq!(manifest.id(), "test-scenario@1.0.0");
}

#[tokio::test]
async fn test_scenario_package_loading() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create minimal valid package
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(
        root.join("scenario.yaml"),
        r#"
manifest_version: "1.0"
name: test-scenario
version: "1.0.0"
title: Test Scenario
description: A test scenario
author: test
category: other
compatibility:
  min_version: "0.2.0"
files: []
"#,
    )
    .unwrap();

    let package = ScenarioPackage::from_directory(root).unwrap();
    assert_eq!(package.manifest.name, "test-scenario");
    assert_eq!(package.manifest.version, "1.0.0");

    let validation = package.validate().unwrap();
    assert!(validation.is_valid);
}

#[tokio::test]
async fn test_scenario_source_parsing() {
    // Test local path
    let source = ScenarioSource::parse("./scenarios/my-scenario").unwrap();
    assert!(matches!(source, ScenarioSource::Local(_)));

    // Test URL
    let source = ScenarioSource::parse("https://example.com/scenario.zip").unwrap();
    assert!(matches!(source, ScenarioSource::Url { .. }));

    // Test Git URL
    let source = ScenarioSource::parse("https://github.com/user/repo").unwrap();
    assert!(matches!(source, ScenarioSource::Git { .. }));

    // Test Git URL with branch
    let source = ScenarioSource::parse("https://github.com/user/repo#main").unwrap();
    match source {
        ScenarioSource::Git { reference, .. } => {
            assert_eq!(reference, Some("main".to_string()));
        }
        _ => panic!("Expected Git source"),
    }

    // Test Git URL with subdirectory
    let source =
        ScenarioSource::parse("https://github.com/user/repo#main:scenarios/my-scenario").unwrap();
    match source {
        ScenarioSource::Git {
            reference,
            subdirectory,
            ..
        } => {
            assert_eq!(reference, Some("main".to_string()));
            assert_eq!(subdirectory, Some("scenarios/my-scenario".to_string()));
        }
        _ => panic!("Expected Git source"),
    }

    // Test registry
    let source = ScenarioSource::parse("ecommerce-store").unwrap();
    assert!(matches!(source, ScenarioSource::Registry { .. }));

    // Test registry with version
    let source = ScenarioSource::parse("ecommerce-store@1.0.0").unwrap();
    match source {
        ScenarioSource::Registry { name, version } => {
            assert_eq!(name, "ecommerce-store");
            assert_eq!(version, Some("1.0.0".to_string()));
        }
        _ => panic!("Expected Registry source"),
    }
}

#[tokio::test]
async fn test_scenario_storage() {
    let temp_dir = TempDir::new().unwrap();
    let storage = ScenarioStorage::with_dir(temp_dir.path()).unwrap();
    storage.init().await.unwrap();

    assert!(storage.list().is_empty());
}

#[tokio::test]
async fn test_scenario_installer_creation() {
    let temp_dir = TempDir::new().unwrap();
    let installer = ScenarioInstaller::with_dir(temp_dir.path()).unwrap();
    assert!(installer.list_installed().is_empty());
}

#[tokio::test]
async fn test_scenario_installer_init() {
    let temp_dir = TempDir::new().unwrap();
    let mut installer = ScenarioInstaller::with_dir(temp_dir.path()).unwrap();
    installer.init().await.unwrap();
    assert!(installer.list_installed().is_empty());
}
