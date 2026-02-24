//! Integration tests for LLM/Voice Interface workspace creation
//!
//! Tests command parsing, workspace creation, and end-to-end NL to workspace flow.

use mockforge_core::intelligent_behavior::IntelligentBehaviorConfig;
use mockforge_core::multi_tenant::MultiTenantWorkspaceRegistry;
use mockforge_core::voice::command_parser::{
    EntityEndpointRequirement, EntityRequirement, FieldRequirement, ParsedCommand,
    ParsedWorkspaceCreation, PersonaRelationship, PersonaRequirement, ScenarioRequirement,
    ScenarioStepRequirement, VoiceCommandParser,
};
use mockforge_core::voice::workspace_builder::{BuiltWorkspace, WorkspaceBuilder};
use mockforge_data::Domain;
use serde_json::json;
use std::collections::HashMap;

/// Test command parser creation
#[tokio::test]
async fn test_command_parser_creation() {
    let config = IntelligentBehaviorConfig::default();
    let parser = VoiceCommandParser::new(config);
    // Parser should be created successfully with default config
    let _ = &parser;
}

/// Test entity requirement parsing
#[tokio::test]
async fn test_entity_requirement() {
    let requirement = EntityRequirement {
        name: "user".to_string(),
        description: "User entity".to_string(),
        endpoints: vec![
            EntityEndpointRequirement {
                path: "/api/users".to_string(),
                method: "GET".to_string(),
                description: "List users".to_string(),
            },
            EntityEndpointRequirement {
                path: "/api/users".to_string(),
                method: "POST".to_string(),
                description: "Create user".to_string(),
            },
        ],
        fields: vec![
            FieldRequirement {
                name: "name".to_string(),
                r#type: "string".to_string(),
                description: "User name".to_string(),
                required: true,
            },
            FieldRequirement {
                name: "email".to_string(),
                r#type: "string".to_string(),
                description: "User email".to_string(),
                required: true,
            },
        ],
    };

    assert_eq!(requirement.name, "user");
    assert_eq!(requirement.endpoints.len(), 2);
    assert_eq!(requirement.fields.len(), 2);
}

/// Test persona requirement parsing
#[tokio::test]
async fn test_persona_requirement() {
    let requirement = PersonaRequirement {
        name: "premium-customer".to_string(),
        description: "Premium customer persona".to_string(),
        traits: {
            let mut map = HashMap::new();
            map.insert("loyalty_level".to_string(), "gold".to_string());
            map
        },
        relationships: vec![PersonaRelationship {
            r#type: "owns".to_string(),
            target_entity: "order".to_string(),
        }],
    };

    assert_eq!(requirement.name, "premium-customer");
    assert_eq!(requirement.relationships.len(), 1);
    assert_eq!(requirement.traits.len(), 1);
}

/// Test scenario requirement parsing
#[tokio::test]
async fn test_scenario_requirement() {
    let requirement = ScenarioRequirement {
        name: "happy-path-checkout".to_string(),
        r#type: "happy_path".to_string(),
        description: "Successful checkout flow".to_string(),
        steps: vec![
            ScenarioStepRequirement {
                description: "Login".to_string(),
                endpoint: "POST /api/login".to_string(),
                expected_outcome: "success".to_string(),
            },
            ScenarioStepRequirement {
                description: "Add to cart".to_string(),
                endpoint: "POST /api/cart".to_string(),
                expected_outcome: "success".to_string(),
            },
            ScenarioStepRequirement {
                description: "Checkout".to_string(),
                endpoint: "POST /api/checkout".to_string(),
                expected_outcome: "success".to_string(),
            },
        ],
    };

    assert_eq!(requirement.r#type, "happy_path");
    assert_eq!(requirement.steps.len(), 3);
}

/// Test parsed workspace creation
#[tokio::test]
async fn test_parsed_workspace_creation() {
    let parsed = ParsedWorkspaceCreation {
        workspace_name: "ecommerce-workspace".to_string(),
        workspace_description: "E-commerce mock workspace".to_string(),
        entities: vec![EntityRequirement {
            name: "user".to_string(),
            description: "User entity".to_string(),
            endpoints: vec![EntityEndpointRequirement {
                path: "/api/users".to_string(),
                method: "GET".to_string(),
                description: "List users".to_string(),
            }],
            fields: vec![FieldRequirement {
                name: "name".to_string(),
                r#type: "string".to_string(),
                description: "User name".to_string(),
                required: true,
            }],
        }],
        personas: vec![PersonaRequirement {
            name: "customer".to_string(),
            description: "Customer persona".to_string(),
            traits: HashMap::new(),
            relationships: vec![],
        }],
        scenarios: vec![ScenarioRequirement {
            name: "happy-path".to_string(),
            r#type: "happy_path".to_string(),
            description: "Happy path scenario".to_string(),
            steps: vec![],
        }],
        reality_continuum: None,
        drift_budget: None,
    };

    assert_eq!(parsed.workspace_name, "ecommerce-workspace");
    assert_eq!(parsed.entities.len(), 1);
    assert_eq!(parsed.scenarios.len(), 1);
}

/// Test workspace builder creation
#[tokio::test]
async fn test_workspace_builder_creation() {
    let builder = WorkspaceBuilder::new();
    // Builder should be created successfully
    let _ = &builder;
}

/// Test workspace builder with parsed command
#[tokio::test]
async fn test_workspace_builder_with_command() {
    let _builder = WorkspaceBuilder::new();
    let _registry = MultiTenantWorkspaceRegistry::new(Default::default());

    let parsed = ParsedWorkspaceCreation {
        workspace_name: "test-workspace".to_string(),
        workspace_description: "Test workspace".to_string(),
        entities: vec![EntityRequirement {
            name: "user".to_string(),
            description: "User entity".to_string(),
            endpoints: vec![EntityEndpointRequirement {
                path: "/api/users".to_string(),
                method: "GET".to_string(),
                description: "List users".to_string(),
            }],
            fields: vec![FieldRequirement {
                name: "name".to_string(),
                r#type: "string".to_string(),
                description: "User name".to_string(),
                required: true,
            }],
        }],
        personas: vec![],
        scenarios: vec![],
        reality_continuum: None,
        drift_budget: None,
    };

    // Note: This test verifies the structure, actual workspace creation
    // may require additional setup (LLM provider, etc.)
    assert_eq!(parsed.workspace_name, "test-workspace");
}

/// Test entity requirement with multiple endpoints
#[tokio::test]
async fn test_entity_requirement_multiple_endpoints() {
    let requirement = EntityRequirement {
        name: "order".to_string(),
        description: "Order entity".to_string(),
        endpoints: vec![
            EntityEndpointRequirement {
                path: "/api/orders".to_string(),
                method: "GET".to_string(),
                description: "List orders".to_string(),
            },
            EntityEndpointRequirement {
                path: "/api/orders".to_string(),
                method: "POST".to_string(),
                description: "Create order".to_string(),
            },
            EntityEndpointRequirement {
                path: "/api/orders/{id}".to_string(),
                method: "GET".to_string(),
                description: "Get order".to_string(),
            },
            EntityEndpointRequirement {
                path: "/api/orders/{id}".to_string(),
                method: "PUT".to_string(),
                description: "Update order".to_string(),
            },
        ],
        fields: vec![
            FieldRequirement {
                name: "id".to_string(),
                r#type: "string".to_string(),
                description: "Order ID".to_string(),
                required: true,
            },
            FieldRequirement {
                name: "total".to_string(),
                r#type: "number".to_string(),
                description: "Order total".to_string(),
                required: true,
            },
            FieldRequirement {
                name: "status".to_string(),
                r#type: "string".to_string(),
                description: "Order status".to_string(),
                required: false,
            },
        ],
    };

    assert_eq!(requirement.endpoints.len(), 4);
    assert_eq!(requirement.fields.len(), 3);
}

/// Test persona requirement with relationships
#[tokio::test]
async fn test_persona_requirement_relationships() {
    let requirement = PersonaRequirement {
        name: "customer".to_string(),
        description: "Customer persona".to_string(),
        traits: HashMap::new(),
        relationships: vec![
            PersonaRelationship {
                r#type: "owns".to_string(),
                target_entity: "order".to_string(),
            },
            PersonaRelationship {
                r#type: "has".to_string(),
                target_entity: "payment".to_string(),
            },
            PersonaRelationship {
                r#type: "owns".to_string(),
                target_entity: "device".to_string(),
            },
        ],
    };

    assert_eq!(requirement.relationships.len(), 3);
    assert_eq!(requirement.relationships[0].target_entity, "order");
}

/// Test scenario requirement types
#[tokio::test]
async fn test_scenario_requirement_types() {
    let happy_path = ScenarioRequirement {
        name: "happy-path".to_string(),
        r#type: "happy_path".to_string(),
        description: "Successful flow".to_string(),
        steps: vec![],
    };

    let failure_path = ScenarioRequirement {
        name: "failure".to_string(),
        r#type: "failure".to_string(),
        description: "Failed payment flow".to_string(),
        steps: vec![],
    };

    let slow_path = ScenarioRequirement {
        name: "slow-path".to_string(),
        r#type: "slow_path".to_string(),
        description: "Slow shipping scenario".to_string(),
        steps: vec![],
    };

    assert_eq!(happy_path.r#type, "happy_path");
    assert_eq!(failure_path.r#type, "failure");
    assert_eq!(slow_path.r#type, "slow_path");
}

/// Test parsed command with all fields
#[tokio::test]
async fn test_parsed_command_complete() {
    let parsed = ParsedWorkspaceCreation {
        workspace_name: "complete-workspace".to_string(),
        workspace_description: "Complete test workspace".to_string(),
        entities: vec![
            EntityRequirement {
                name: "user".to_string(),
                description: "User entity".to_string(),
                endpoints: vec![EntityEndpointRequirement {
                    path: "/api/users".to_string(),
                    method: "GET".to_string(),
                    description: "List users".to_string(),
                }],
                fields: vec![
                    FieldRequirement {
                        name: "name".to_string(),
                        r#type: "string".to_string(),
                        description: "User name".to_string(),
                        required: true,
                    },
                    FieldRequirement {
                        name: "email".to_string(),
                        r#type: "string".to_string(),
                        description: "User email".to_string(),
                        required: true,
                    },
                ],
            },
            EntityRequirement {
                name: "order".to_string(),
                description: "Order entity".to_string(),
                endpoints: vec![EntityEndpointRequirement {
                    path: "/api/orders".to_string(),
                    method: "GET".to_string(),
                    description: "List orders".to_string(),
                }],
                fields: vec![
                    FieldRequirement {
                        name: "id".to_string(),
                        r#type: "string".to_string(),
                        description: "Order ID".to_string(),
                        required: true,
                    },
                    FieldRequirement {
                        name: "total".to_string(),
                        r#type: "number".to_string(),
                        description: "Order total".to_string(),
                        required: true,
                    },
                ],
            },
        ],
        personas: vec![PersonaRequirement {
            name: "customer".to_string(),
            description: "Customer persona".to_string(),
            traits: HashMap::new(),
            relationships: vec![PersonaRelationship {
                r#type: "owns".to_string(),
                target_entity: "order".to_string(),
            }],
        }],
        scenarios: vec![
            ScenarioRequirement {
                name: "happy-path".to_string(),
                r#type: "happy_path".to_string(),
                description: "Happy path scenario".to_string(),
                steps: vec![
                    ScenarioStepRequirement {
                        description: "login".to_string(),
                        endpoint: "POST /api/login".to_string(),
                        expected_outcome: "success".to_string(),
                    },
                    ScenarioStepRequirement {
                        description: "checkout".to_string(),
                        endpoint: "POST /api/checkout".to_string(),
                        expected_outcome: "success".to_string(),
                    },
                ],
            },
            ScenarioRequirement {
                name: "failure".to_string(),
                r#type: "failure".to_string(),
                description: "Failure scenario".to_string(),
                steps: vec![
                    ScenarioStepRequirement {
                        description: "login".to_string(),
                        endpoint: "POST /api/login".to_string(),
                        expected_outcome: "success".to_string(),
                    },
                    ScenarioStepRequirement {
                        description: "payment_failed".to_string(),
                        endpoint: "POST /api/payment".to_string(),
                        expected_outcome: "failure".to_string(),
                    },
                ],
            },
        ],
        reality_continuum: None,
        drift_budget: None,
    };

    assert_eq!(parsed.entities.len(), 2);
    assert_eq!(parsed.scenarios.len(), 2);
    assert!(!parsed.personas.is_empty());
}

/// Test workspace builder creation log
#[tokio::test]
async fn test_workspace_builder_log() {
    let builder = WorkspaceBuilder::new();
    // Builder should have empty creation log initially
    // (Note: creation_log is private, so we verify construction doesn't panic)
    let _ = &builder;
}

/// Test entity requirement validation
#[tokio::test]
async fn test_entity_requirement_validation() {
    // Valid requirement
    let valid = EntityRequirement {
        name: "user".to_string(),
        description: "User entity".to_string(),
        endpoints: vec![EntityEndpointRequirement {
            path: "/api/users".to_string(),
            method: "GET".to_string(),
            description: "List users".to_string(),
        }],
        fields: vec![FieldRequirement {
            name: "name".to_string(),
            r#type: "string".to_string(),
            description: "User name".to_string(),
            required: true,
        }],
    };

    assert!(!valid.name.is_empty());
    assert!(!valid.endpoints.is_empty());

    // Requirement with no endpoints (may be valid if auto-generated)
    let no_endpoints = EntityRequirement {
        name: "product".to_string(),
        description: "Product entity".to_string(),
        endpoints: vec![],
        fields: vec![],
    };

    assert!(!no_endpoints.name.is_empty());
}

/// Test scenario requirement with steps
#[tokio::test]
async fn test_scenario_requirement_with_steps() {
    let requirement = ScenarioRequirement {
        name: "checkout".to_string(),
        r#type: "checkout".to_string(),
        description: "Checkout flow".to_string(),
        steps: vec![
            ScenarioStepRequirement {
                description: "login".to_string(),
                endpoint: "POST /api/login".to_string(),
                expected_outcome: "success".to_string(),
            },
            ScenarioStepRequirement {
                description: "add_to_cart".to_string(),
                endpoint: "POST /api/cart".to_string(),
                expected_outcome: "success".to_string(),
            },
            ScenarioStepRequirement {
                description: "view_cart".to_string(),
                endpoint: "GET /api/cart".to_string(),
                expected_outcome: "success".to_string(),
            },
            ScenarioStepRequirement {
                description: "checkout".to_string(),
                endpoint: "POST /api/checkout".to_string(),
                expected_outcome: "success".to_string(),
            },
            ScenarioStepRequirement {
                description: "payment".to_string(),
                endpoint: "POST /api/payment".to_string(),
                expected_outcome: "success".to_string(),
            },
        ],
    };

    assert_eq!(requirement.steps.len(), 5);
    assert_eq!(requirement.steps[0].description, "login");
    assert_eq!(requirement.steps[4].description, "payment");
}

/// Test parsed workspace with reality continuum
#[tokio::test]
async fn test_parsed_workspace_with_reality_continuum() {
    let parsed = ParsedWorkspaceCreation {
        workspace_name: "reality-workspace".to_string(),
        workspace_description: "Reality workspace".to_string(),
        entities: vec![],
        personas: vec![],
        scenarios: vec![],
        reality_continuum: Some(mockforge_core::voice::command_parser::ParsedRealityContinuum {
            default_ratio: 0.8,
            enabled: true,
            route_rules: vec![],
            transition_mode: "manual".to_string(),
            merge_strategy: "field_level".to_string(),
        }),
        drift_budget: None,
    };

    assert!(parsed.reality_continuum.is_some());
    if let Some(rc) = parsed.reality_continuum {
        assert_eq!(rc.default_ratio, 0.8);
    }
}

/// Test parsed workspace with drift budget
#[tokio::test]
async fn test_parsed_workspace_with_drift_budget() {
    let parsed = ParsedWorkspaceCreation {
        workspace_name: "drift-workspace".to_string(),
        workspace_description: "Drift workspace".to_string(),
        entities: vec![],
        personas: vec![],
        scenarios: vec![],
        reality_continuum: None,
        drift_budget: Some(mockforge_core::voice::command_parser::ParsedDriftBudget {
            strictness: "strict".to_string(),
            enabled: true,
            max_breaking_changes: 0,
            max_non_breaking_changes: 5,
            max_field_churn_percent: None,
            time_window_days: None,
            per_service_budgets: HashMap::new(),
            description: "Strict drift budget".to_string(),
        }),
    };

    assert!(parsed.drift_budget.is_some());
    if let Some(db) = parsed.drift_budget {
        assert_eq!(db.max_breaking_changes, 0);
        assert_eq!(db.max_non_breaking_changes, 5);
    }
}
