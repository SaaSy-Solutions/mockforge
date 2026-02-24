//! Integration tests for Drift GitOps feature
//!
//! Tests PR generation, OpenAPI/fixture updates, and GitOps configuration.

use mockforge_core::drift_gitops::handler::DriftGitOpsConfig;
use mockforge_core::drift_gitops::DriftGitOpsHandler;
use mockforge_core::incidents::types::{
    DriftIncident, IncidentSeverity, IncidentStatus, IncidentType,
};
use mockforge_core::pr_generation::{
    PRFileChange, PRFileChangeType, PRGenerationConfig, PRProvider,
};
use serde_json::json;
use std::collections::HashMap;

/// Test GitOps configuration validation
#[tokio::test]
async fn test_gitops_config_validation() {
    // Test disabled config
    let config = DriftGitOpsConfig {
        enabled: false,
        ..Default::default()
    };
    let handler = DriftGitOpsHandler::new(config).unwrap();
    // Handler created successfully with disabled config
    let _ = &handler;

    // Test enabled config without PR config (should fail)
    let config = DriftGitOpsConfig {
        enabled: true,
        pr_config: None,
        ..Default::default()
    };
    // Handler can be created but PR generation will fail
    let handler = DriftGitOpsHandler::new(config).unwrap();
    // Handler created successfully with enabled config
    let _ = &handler;
}

/// Test GitOps config with PR generation settings
#[tokio::test]
async fn test_gitops_config_with_pr_settings() {
    let pr_config = PRGenerationConfig {
        enabled: true,
        provider: PRProvider::GitHub,
        owner: "test-owner".to_string(),
        repo: "test-repo".to_string(),
        token: Some("test-token".to_string()),
        base_branch: "main".to_string(),
        branch_prefix: "mockforge/".to_string(),
        auto_merge: false,
        reviewers: vec![],
        labels: vec![],
    };

    let config = DriftGitOpsConfig {
        enabled: true,
        pr_config: Some(pr_config),
        update_openapi_specs: true,
        update_fixtures: true,
        regenerate_clients: false,
        run_tests: false,
        openapi_spec_dir: Some("specs".to_string()),
        fixtures_dir: Some("fixtures".to_string()),
        clients_dir: Some("clients".to_string()),
        branch_prefix: "mockforge/drift-fix".to_string(),
    };

    // Handler creation should succeed with valid config
    let handler = DriftGitOpsHandler::new(config).unwrap();
    // Handler created successfully with enabled config
    let _ = &handler;
}

/// Test PR generation from empty incidents (should return None)
#[tokio::test]
async fn test_pr_generation_empty_incidents() {
    let config = DriftGitOpsConfig {
        enabled: true,
        pr_config: None, // No PR config, but handler can still be created
        ..Default::default()
    };

    let handler = DriftGitOpsHandler::new(config).unwrap();
    let incidents = vec![];

    // Should return None for empty incidents
    let result = handler.generate_pr_from_incidents(&incidents).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

/// Test PR generation with disabled GitOps
#[tokio::test]
async fn test_pr_generation_disabled() {
    let config = DriftGitOpsConfig {
        enabled: false,
        ..Default::default()
    };

    let handler = DriftGitOpsHandler::new(config).unwrap();

    let incident = DriftIncident::new(
        "incident-1".to_string(),
        "/api/users".to_string(),
        "POST".to_string(),
        IncidentType::BreakingChange,
        IncidentSeverity::Critical,
        json!({
            "breaking_changes": 2,
            "description": "Test incident"
        }),
    );

    let incidents = vec![incident];

    // Should return None when disabled
    let result = handler.generate_pr_from_incidents(&incidents).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

/// Test incident creation for PR generation
#[tokio::test]
async fn test_incident_creation() {
    let incident = DriftIncident::new(
        "incident-1".to_string(),
        "/api/users".to_string(),
        "POST".to_string(),
        IncidentType::BreakingChange,
        IncidentSeverity::Critical,
        json!({
            "breaking_changes": 2,
            "description": "Required field 'email' was removed"
        }),
    );

    assert_eq!(incident.id, "incident-1");
    assert_eq!(incident.endpoint, "/api/users");
    assert_eq!(incident.method, "POST");
    assert_eq!(incident.incident_type, IncidentType::BreakingChange);
    assert_eq!(incident.severity, IncidentSeverity::Critical);
    assert_eq!(incident.status, IncidentStatus::Open);
}

/// Test incident with before/after samples
#[tokio::test]
async fn test_incident_with_samples() {
    let mut incident = DriftIncident::new(
        "incident-1".to_string(),
        "/api/users".to_string(),
        "POST".to_string(),
        IncidentType::BreakingChange,
        IncidentSeverity::High,
        json!({
            "breaking_changes": 1
        }),
    );

    incident.before_sample = Some(json!({
        "schema": {
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "email": {"type": "string"}
            },
            "required": ["name", "email"]
        }
    }));

    incident.after_sample = Some(json!({
        "schema": {
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        },
        "corrections": [
            {
                "op": "add",
                "path": "/properties/email",
                "value": {"type": "string"}
            }
        ]
    }));

    assert!(incident.before_sample.is_some());
    assert!(incident.after_sample.is_some());
}

/// Test GitOps config with custom paths
#[tokio::test]
async fn test_gitops_config_custom_paths() {
    let config = DriftGitOpsConfig {
        enabled: true,
        update_openapi_specs: true,
        update_fixtures: true,
        openapi_spec_dir: Some("api/specs".to_string()),
        fixtures_dir: Some("test/fixtures".to_string()),
        clients_dir: Some("generated/clients".to_string()),
        branch_prefix: "fix/drift".to_string(),
        ..Default::default()
    };

    assert_eq!(config.openapi_spec_dir, Some("api/specs".to_string()));
    assert_eq!(config.fixtures_dir, Some("test/fixtures".to_string()));
    assert_eq!(config.clients_dir, Some("generated/clients".to_string()));
    assert_eq!(config.branch_prefix, "fix/drift".to_string());
}

/// Test multiple incidents for PR generation
#[tokio::test]
async fn test_multiple_incidents() {
    let incident1 = DriftIncident::new(
        "incident-1".to_string(),
        "/api/users".to_string(),
        "POST".to_string(),
        IncidentType::BreakingChange,
        IncidentSeverity::Critical,
        json!({"breaking_changes": 1}),
    );

    let incident2 = DriftIncident::new(
        "incident-2".to_string(),
        "/api/orders".to_string(),
        "GET".to_string(),
        IncidentType::ThresholdExceeded,
        IncidentSeverity::High,
        json!({"non_breaking_changes": 5}),
    );

    let incidents = vec![incident1, incident2];
    assert_eq!(incidents.len(), 2);
}

/// Test incident status transitions
#[tokio::test]
async fn test_incident_status() {
    let mut incident = DriftIncident::new(
        "incident-1".to_string(),
        "/api/users".to_string(),
        "POST".to_string(),
        IncidentType::BreakingChange,
        IncidentSeverity::Critical,
        json!({}),
    );

    assert_eq!(incident.status, IncidentStatus::Open);

    // Simulate status change
    incident.status = IncidentStatus::Acknowledged;
    assert_eq!(incident.status, IncidentStatus::Acknowledged);

    incident.status = IncidentStatus::Resolved;
    assert_eq!(incident.status, IncidentStatus::Resolved);
}

/// Test incident with workspace context
#[tokio::test]
async fn test_incident_with_workspace() {
    let mut incident = DriftIncident::new(
        "incident-1".to_string(),
        "/api/users".to_string(),
        "POST".to_string(),
        IncidentType::BreakingChange,
        IncidentSeverity::High,
        json!({}),
    );

    incident.workspace_id = Some("workspace-1".to_string());
    incident.budget_id = Some("budget-1".to_string());

    assert_eq!(incident.workspace_id, Some("workspace-1".to_string()));
    assert_eq!(incident.budget_id, Some("budget-1".to_string()));
}

/// Test incident with sync cycle ID
#[tokio::test]
async fn test_incident_with_sync_cycle() {
    let mut incident = DriftIncident::new(
        "incident-1".to_string(),
        "/api/users".to_string(),
        "POST".to_string(),
        IncidentType::BreakingChange,
        IncidentSeverity::Medium,
        json!({}),
    );

    incident.sync_cycle_id = Some("sync-cycle-123".to_string());
    incident.contract_diff_id = Some("diff-456".to_string());

    assert_eq!(incident.sync_cycle_id, Some("sync-cycle-123".to_string()));
    assert_eq!(incident.contract_diff_id, Some("diff-456".to_string()));
}

/// Test GitOps config default values
#[tokio::test]
async fn test_gitops_config_defaults() {
    let config = DriftGitOpsConfig::default();

    assert!(!config.enabled);
    assert!(config.update_openapi_specs);
    assert!(config.update_fixtures);
    assert!(!config.regenerate_clients);
    assert!(!config.run_tests);
    assert_eq!(config.branch_prefix, "mockforge/drift-fix");
}

/// Test incident type variants
#[tokio::test]
async fn test_incident_types() {
    let breaking_incident = DriftIncident::new(
        "incident-1".to_string(),
        "/api/users".to_string(),
        "POST".to_string(),
        IncidentType::BreakingChange,
        IncidentSeverity::Critical,
        json!({}),
    );

    let threshold_incident = DriftIncident::new(
        "incident-2".to_string(),
        "/api/orders".to_string(),
        "GET".to_string(),
        IncidentType::ThresholdExceeded,
        IncidentSeverity::High,
        json!({}),
    );

    assert_eq!(breaking_incident.incident_type, IncidentType::BreakingChange);
    assert_eq!(threshold_incident.incident_type, IncidentType::ThresholdExceeded);
}

/// Test incident severity levels
#[tokio::test]
async fn test_incident_severity() {
    let severities = vec![
        IncidentSeverity::Critical,
        IncidentSeverity::High,
        IncidentSeverity::Medium,
        IncidentSeverity::Low,
    ];

    for severity in severities {
        let incident = DriftIncident::new(
            "incident-1".to_string(),
            "/api/users".to_string(),
            "POST".to_string(),
            IncidentType::BreakingChange,
            severity,
            json!({}),
        );
        assert_eq!(incident.severity, severity);
    }
}

/// Test PR file change types
#[tokio::test]
async fn test_pr_file_change_types() {
    let openapi_change = PRFileChange {
        path: "specs/api.yaml".to_string(),
        change_type: PRFileChangeType::Update,
        content: "updated content".to_string(),
    };

    let fixture_change = PRFileChange {
        path: "fixtures/users.json".to_string(),
        change_type: PRFileChangeType::Create,
        content: "new content".to_string(),
    };

    assert_eq!(openapi_change.change_type, PRFileChangeType::Update);
    assert_eq!(fixture_change.change_type, PRFileChangeType::Create);
}

/// Test GitOps handler with minimal config
#[tokio::test]
async fn test_gitops_handler_minimal_config() {
    let config = DriftGitOpsConfig {
        enabled: false,
        ..Default::default()
    };

    let handler = DriftGitOpsHandler::new(config).unwrap();
    // Handler created successfully with disabled config
    let _ = &handler;
}
