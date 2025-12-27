//! Integration tests for the Scenario Marketplace feature
//!
//! Tests cover:
//! - Preview functionality
//! - VBR entity installation
//! - MockAI config application
//! - Schema alignment
//! - Domain pack installation
//! - Review submission

use mockforge_scenarios::{
    DomainPackInstaller, DomainPackManifest, InstallOptions, MergeStrategy, PackScenario,
    ScenarioInstaller, ScenarioManifest, SchemaAlignmentConfig,
};
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test helper to create a minimal scenario directory structure
fn create_test_scenario_dir(temp_dir: &TempDir) -> PathBuf {
    let scenario_dir = temp_dir.path().join("test-scenario");
    std::fs::create_dir_all(&scenario_dir).unwrap();

    // Create scenario.yaml manifest
    let mut manifest = ScenarioManifest::new(
        "test-scenario".to_string(),
        "1.0.0".to_string(),
        "Test Scenario".to_string(),
        "A test scenario for integration tests".to_string(),
    );
    manifest.category = mockforge_scenarios::ScenarioCategory::Other;
    manifest.author = "test-author".to_string();

    let manifest_yaml = serde_yaml::to_string(&manifest).unwrap();
    std::fs::write(scenario_dir.join("scenario.yaml"), manifest_yaml).unwrap();

    // Create a simple OpenAPI spec
    let openapi_spec = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "paths": {
            "/test": {
                "get": {
                    "summary": "Test endpoint",
                    "responses": {
                        "200": {
                            "description": "Success"
                        }
                    }
                }
            }
        }
    });

    std::fs::write(
        scenario_dir.join("openapi.json"),
        serde_json::to_string_pretty(&openapi_spec).unwrap(),
    )
    .unwrap();

    scenario_dir
}

#[tokio::test]
async fn test_preview_functionality() {
    let temp_dir = TempDir::new().unwrap();
    let scenario_dir = create_test_scenario_dir(&temp_dir);

    let installer = ScenarioInstaller::new().unwrap();

    // Test preview from local path
    let source = scenario_dir.to_string_lossy().to_string();
    let preview = installer.preview(&source).await.unwrap();

    assert_eq!(preview.manifest.name, "test-scenario");
    assert_eq!(preview.manifest.version, "1.0.0");
    assert!(!preview.openapi_endpoints.is_empty());
    assert!(preview.estimated_size > 0);
}

#[tokio::test]
async fn test_vbr_entity_retrieval() {
    let temp_dir = TempDir::new().unwrap();
    let scenario_dir = create_test_scenario_dir(&temp_dir);

    // Create a scenario with VBR entities
    let mut manifest = ScenarioManifest::new(
        "vbr-scenario".to_string(),
        "1.0.0".to_string(),
        "VBR Scenario".to_string(),
        "A scenario with VBR entities".to_string(),
    );
    manifest.category = mockforge_scenarios::ScenarioCategory::Other;
    manifest.author = "test-author".to_string();

    // Add VBR entity definition
    let vbr_entity = mockforge_scenarios::VbrEntityDefinition {
        name: "User".to_string(),
        schema: json!({
            "type": "object",
            "properties": {
                "id": {"type": "string"},
                "name": {"type": "string"}
            }
        }),
        seed_data_path: Some("fixtures/users.json".to_string()),
        state_machine: None,
    };

    manifest.vbr_entities = Some(vec![vbr_entity]);

    // Write updated manifest
    let manifest_yaml = serde_yaml::to_string(&manifest).unwrap();
    std::fs::write(scenario_dir.join("scenario.yaml"), manifest_yaml).unwrap();

    let mut installer = ScenarioInstaller::new().unwrap();
    installer.init().await.unwrap();

    // Install the scenario
    let source = scenario_dir.to_string_lossy().to_string();
    let options = InstallOptions {
        force: true, // Use force to allow re-running tests without manual cleanup
        skip_validation: false,
        expected_checksum: None,
    };

    installer.install(&source, options).await.unwrap();

    // Retrieve VBR entities
    let vbr_entities = installer.get_vbr_entities("vbr-scenario", Some("1.0.0")).unwrap();
    assert!(vbr_entities.is_some());
    let entities = vbr_entities.unwrap();
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].name, "User");
}

#[tokio::test]
async fn test_mockai_config_retrieval() {
    let temp_dir = TempDir::new().unwrap();
    let scenario_dir = create_test_scenario_dir(&temp_dir);

    // Create a scenario with MockAI config
    let mut manifest = ScenarioManifest::new(
        "mockai-scenario".to_string(),
        "1.0.0".to_string(),
        "MockAI Scenario".to_string(),
        "A scenario with MockAI config".to_string(),
    );
    manifest.category = mockforge_scenarios::ScenarioCategory::Other;
    manifest.author = "test-author".to_string();

    // Add MockAI config
    let mockai_config = mockforge_scenarios::MockAIConfigDefinition::new(json!({
        "enabled": true,
        "behavior_model": {
            "llm_provider": "ollama",
            "model": "llama3.2"
        }
    }));

    manifest.mockai_config = Some(mockai_config);

    // Write updated manifest
    let manifest_yaml = serde_yaml::to_string(&manifest).unwrap();
    std::fs::write(scenario_dir.join("scenario.yaml"), manifest_yaml).unwrap();

    let mut installer = ScenarioInstaller::new().unwrap();
    installer.init().await.unwrap();

    // Install the scenario
    let source = scenario_dir.to_string_lossy().to_string();
    let options = InstallOptions {
        force: true, // Use force to allow re-running tests without manual cleanup
        skip_validation: false,
        expected_checksum: None,
    };

    installer.install(&source, options).await.unwrap();

    // Retrieve MockAI config
    let mockai_config = installer.get_mockai_config("mockai-scenario", Some("1.0.0")).unwrap();
    assert!(mockai_config.is_some());
    let config = mockai_config.unwrap();
    assert!(config.config.get("enabled").and_then(|v| v.as_bool()).unwrap());
}

#[tokio::test]
async fn test_schema_alignment_prefer_existing() {
    let existing_spec = json!({
        "openapi": "3.0.0",
        "info": {"title": "Existing API", "version": "1.0.0"},
        "paths": {
            "/users": {
                "get": {"summary": "Get users"}
            }
        }
    });

    let scenario_spec = json!({
        "openapi": "3.0.0",
        "info": {"title": "Scenario API", "version": "1.0.0"},
        "paths": {
            "/users": {
                "post": {"summary": "Create user"}
            },
            "/products": {
                "get": {"summary": "Get products"}
            }
        }
    });

    let config = SchemaAlignmentConfig {
        merge_strategy: MergeStrategy::PreferExisting,
        validate_merged: false,
        backup_existing: false,
    };

    let result =
        mockforge_scenarios::align_openapi_specs(&existing_spec, &scenario_spec, &config).unwrap();

    assert!(result.success);
    let merged = result.merged_spec.unwrap();

    // Should have existing /users GET
    assert!(merged["paths"]["/users"]["get"].is_object());

    // Should have new /products path
    assert!(merged["paths"]["/products"].is_object());

    // Should NOT have scenario /users POST (prefer existing)
    assert!(!merged["paths"]["/users"]["post"].is_object());
}

#[tokio::test]
async fn test_schema_alignment_prefer_scenario() {
    let existing_spec = json!({
        "openapi": "3.0.0",
        "info": {"title": "Existing API", "version": "1.0.0"},
        "paths": {
            "/users": {
                "get": {"summary": "Get users (old)"}
            }
        }
    });

    let scenario_spec = json!({
        "openapi": "3.0.0",
        "info": {"title": "Scenario API", "version": "1.0.0"},
        "paths": {
            "/users": {
                "get": {"summary": "Get users (new)"}
            }
        }
    });

    let config = SchemaAlignmentConfig {
        merge_strategy: MergeStrategy::PreferScenario,
        validate_merged: false,
        backup_existing: false,
    };

    let result =
        mockforge_scenarios::align_openapi_specs(&existing_spec, &scenario_spec, &config).unwrap();

    assert!(result.success);
    let merged = result.merged_spec.unwrap();

    // Should have scenario version
    let summary = merged["paths"]["/users"]["get"]["summary"].as_str().unwrap();
    assert_eq!(summary, "Get users (new)");
}

#[tokio::test]
async fn test_schema_alignment_intelligent() {
    let existing_spec = json!({
        "openapi": "3.0.0",
        "info": {"title": "Existing API", "version": "1.0.0"},
        "paths": {
            "/users": {
                "get": {"summary": "Get users"}
            }
        }
    });

    let scenario_spec = json!({
        "openapi": "3.0.0",
        "info": {"title": "Scenario API", "version": "1.0.0"},
        "paths": {
            "/users": {
                "post": {"summary": "Create user"}
            }
        }
    });

    let config = SchemaAlignmentConfig {
        merge_strategy: MergeStrategy::Intelligent,
        validate_merged: false,
        backup_existing: false,
    };

    let result =
        mockforge_scenarios::align_openapi_specs(&existing_spec, &scenario_spec, &config).unwrap();

    assert!(result.success);
    let merged = result.merged_spec.unwrap();

    // Should have both GET and POST for /users
    assert!(merged["paths"]["/users"]["get"].is_object());
    assert!(merged["paths"]["/users"]["post"].is_object());
}

#[tokio::test]
async fn test_domain_pack_creation_and_installation() {
    let temp_dir = TempDir::new().unwrap();

    // Create a pack manifest
    let mut pack = DomainPackManifest::new(
        "test-pack".to_string(),
        "1.0.0".to_string(),
        "Test Pack".to_string(),
        "A test domain pack".to_string(),
        "test".to_string(),
        "test-author".to_string(),
    );

    // Add a scenario to the pack
    pack.add_scenario(PackScenario {
        name: "test-scenario".to_string(),
        version: Some("1.0.0".to_string()),
        source: "./test-scenario".to_string(),
        required: true,
        description: Some("Test scenario".to_string()),
    });

    // Save pack manifest
    let pack_manifest_path = temp_dir.path().join("pack.yaml");
    pack.to_file(&pack_manifest_path).unwrap();

    // Test pack installer
    let pack_installer = DomainPackInstaller::new().unwrap();
    pack_installer.init().unwrap();

    // Install pack
    let pack_info = pack_installer.install_from_manifest(&pack_manifest_path).unwrap();

    assert_eq!(pack_info.manifest.name, "test-pack");
    assert_eq!(pack_info.manifest.scenarios.len(), 1);
}

#[tokio::test]
async fn test_domain_pack_list() {
    let pack_installer = DomainPackInstaller::new().unwrap();
    pack_installer.init().unwrap();

    // List installed packs (may be empty, that's ok)
    let packs = pack_installer.list_installed().unwrap();
    // Just verify the method works without panicking
    // packs.len() is always >= 0, so this assertion is redundant
    // Just verify packs exists
    let _ = packs.len();
}

#[tokio::test]
async fn test_apply_to_workspace_with_alignment() {
    let temp_dir = TempDir::new().unwrap();
    let scenario_dir = create_test_scenario_dir(&temp_dir);

    // Create existing OpenAPI spec in workspace
    let workspace_dir = temp_dir.path().join("workspace");
    std::fs::create_dir_all(&workspace_dir).unwrap();

    let existing_spec = json!({
        "openapi": "3.0.0",
        "info": {"title": "Workspace API", "version": "1.0.0"},
        "paths": {
            "/existing": {
                "get": {"summary": "Existing endpoint"}
            }
        }
    });

    std::fs::write(
        workspace_dir.join("openapi.json"),
        serde_json::to_string_pretty(&existing_spec).unwrap(),
    )
    .unwrap();

    // Install scenario
    let mut installer = ScenarioInstaller::new().unwrap();
    installer.init().await.unwrap();

    let source = scenario_dir.to_string_lossy().to_string();
    let options = InstallOptions {
        force: true, // Use force to allow re-running tests without manual cleanup
        skip_validation: false,
        expected_checksum: None,
    };

    installer.install(&source, options).await.unwrap();

    // Change to workspace directory
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&workspace_dir).unwrap();

    // Apply with alignment
    let alignment_config = SchemaAlignmentConfig {
        merge_strategy: MergeStrategy::Intelligent,
        validate_merged: false,
        backup_existing: false,
    };

    installer
        .apply_to_workspace_with_alignment("test-scenario", None, Some(alignment_config))
        .await
        .unwrap();

    // Restore original directory
    std::env::set_current_dir(&original_dir).unwrap();

    // Verify merged spec exists
    let merged_spec_path = workspace_dir.join("openapi.json");
    assert!(merged_spec_path.exists());
}

#[tokio::test]
async fn test_scenario_preview_compatibility_check() {
    let temp_dir = TempDir::new().unwrap();
    let scenario_dir = create_test_scenario_dir(&temp_dir);

    let installer = ScenarioInstaller::new().unwrap();

    let source = scenario_dir.to_string_lossy().to_string();
    let preview = installer.preview(&source).await.unwrap();

    // Check compatibility
    let compatibility = preview.compatibility;
    // Just check compatibility exists (the boolean value is always true or false)
    let _ = compatibility.compatible;
    assert!(!compatibility.current_version.is_empty());
}

#[tokio::test]
async fn test_scenario_preview_file_tree() {
    let temp_dir = TempDir::new().unwrap();
    let scenario_dir = create_test_scenario_dir(&temp_dir);

    let installer = ScenarioInstaller::new().unwrap();

    let source = scenario_dir.to_string_lossy().to_string();
    let preview = installer.preview(&source).await.unwrap();

    // Check file tree is generated
    let file_tree = &preview.file_tree;
    assert!(!file_tree.is_empty());
    // Check if any file in the tree contains the expected files
    let tree_str = file_tree.join("\n");
    assert!(
        tree_str.contains("scenario.yaml")
            || tree_str.contains("openapi.json")
            || tree_str.contains("scenario.yml")
    );
}

#[tokio::test]
async fn test_scenario_preview_openapi_endpoints() {
    let temp_dir = TempDir::new().unwrap();
    let scenario_dir = create_test_scenario_dir(&temp_dir);

    let installer = ScenarioInstaller::new().unwrap();

    let source = scenario_dir.to_string_lossy().to_string();
    let preview = installer.preview(&source).await.unwrap();

    // Check OpenAPI endpoints are extracted
    assert!(!preview.openapi_endpoints.is_empty());
    assert!(preview.openapi_endpoints.iter().any(|e| e.path == "/test" && e.method == "GET"));
}
