//! Integration tests for promotion workflow
//!
//! Tests the complete promotion workflow including:
//! - Creating promotion requests
//! - Approving/rejecting promotions
//! - GitOps integration
//! - Promotion history tracking

use mockforge_collab::promotion::{PromotionGitOpsConfig, PromotionService};
use mockforge_core::workspace::{
    mock_environment::MockEnvironmentName,
    scenario_promotion::{
        ApprovalRules, PromotionEntityType, PromotionRequest, PromotionStatus,
        ScenarioPromotionWorkflow,
    },
};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::collections::HashMap;
use uuid::Uuid;

/// Setup test database
async fn setup_test_db() -> Pool<Sqlite> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

/// Create a test user in the database
async fn create_test_user(pool: &Pool<Sqlite>) -> Uuid {
    let user_id = Uuid::new_v4();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"INSERT INTO users (id, username, email, password_hash, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?)"#,
    )
    .bind(user_id.to_string())
    .bind("testuser")
    .bind("test@example.com")
    .bind("hash")
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .expect("Failed to create test user");

    user_id
}

/// Create a test workspace in the database
async fn create_test_workspace(pool: &Pool<Sqlite>, owner_id: Uuid) -> Uuid {
    let workspace_id = Uuid::new_v4();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"INSERT INTO workspaces (id, name, owner_id, config, version, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(workspace_id.to_string())
    .bind("Test Workspace")
    .bind(owner_id.to_string())
    .bind("{}")
    .bind(1)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .expect("Failed to create test workspace");

    workspace_id
}

#[tokio::test]
async fn test_create_promotion() {
    let db = setup_test_db().await;
    let user_id = create_test_user(&db).await;
    let workspace_id = create_test_workspace(&db, user_id).await;
    let service = PromotionService::new(db);

    let workspace_id_str = workspace_id.to_string();

    let request = PromotionRequest {
        entity_type: PromotionEntityType::Scenario,
        entity_id: "scenario-456".to_string(),
        entity_version: Some("v1.0.0".to_string()),
        workspace_id: workspace_id_str.clone(),
        from_environment: MockEnvironmentName::Dev,
        to_environment: MockEnvironmentName::Test,
        requires_approval: true,
        approval_required_reason: None,
        comments: Some("Ready for QA testing".to_string()),
        metadata: HashMap::new(),
    };

    let promotion_id = service
        .record_promotion(&request, user_id, PromotionStatus::Pending, None)
        .await
        .expect("Failed to create promotion");

    // Verify promotion was created
    let history = service
        .get_promotion_history(&workspace_id_str, PromotionEntityType::Scenario, "scenario-456")
        .await
        .expect("Failed to get promotion history");

    assert_eq!(history.promotions.len(), 1);
    assert_eq!(history.promotions[0].promotion_id, promotion_id.to_string());
    assert_eq!(history.promotions[0].status, PromotionStatus::Pending);
    assert_eq!(history.promotions[0].from_environment, MockEnvironmentName::Dev);
    assert_eq!(history.promotions[0].to_environment, MockEnvironmentName::Test);
}

#[tokio::test]
async fn test_approve_promotion() {
    let db = setup_test_db().await;
    let user_id = create_test_user(&db).await;
    let workspace_id = create_test_workspace(&db, user_id).await;
    let service = PromotionService::new(db);

    let workspace_id_str = workspace_id.to_string();

    // Create promotion
    let request = PromotionRequest {
        entity_type: PromotionEntityType::Scenario,
        entity_id: "scenario-789".to_string(),
        entity_version: None,
        workspace_id: workspace_id_str.clone(),
        from_environment: MockEnvironmentName::Test,
        to_environment: MockEnvironmentName::Prod,
        requires_approval: true,
        approval_required_reason: None,
        comments: Some("Ready for production".to_string()),
        metadata: HashMap::new(),
    };

    let promotion_id = service
        .record_promotion(&request, user_id, PromotionStatus::Pending, None)
        .await
        .expect("Failed to create promotion");

    // Approve promotion (use same user as approver for simplicity)
    service
        .update_promotion_status(promotion_id, PromotionStatus::Approved, Some(user_id))
        .await
        .expect("Failed to approve promotion");

    // Verify status was updated
    let history = service
        .get_promotion_history(&workspace_id_str, PromotionEntityType::Scenario, "scenario-789")
        .await
        .expect("Failed to get promotion history");

    assert_eq!(history.promotions.len(), 1);
    assert_eq!(history.promotions[0].status, PromotionStatus::Approved);
    assert_eq!(history.promotions[0].approved_by, Some(user_id.to_string()));
}

#[tokio::test]
async fn test_list_workspace_promotions() {
    let db = setup_test_db().await;
    let user_id = create_test_user(&db).await;
    let workspace_id = create_test_workspace(&db, user_id).await;
    let service = PromotionService::new(db);

    let workspace_id_str = workspace_id.to_string();

    // Create multiple promotions
    for i in 0..5 {
        let request = PromotionRequest {
            entity_type: PromotionEntityType::Scenario,
            entity_id: format!("scenario-{}", i),
            entity_version: None,
            workspace_id: workspace_id_str.clone(),
            from_environment: MockEnvironmentName::Dev,
            to_environment: MockEnvironmentName::Test,
            requires_approval: true,
            approval_required_reason: None,
            comments: None,
            metadata: HashMap::new(),
        };

        service
            .record_promotion(&request, user_id, PromotionStatus::Pending, None)
            .await
            .expect("Failed to create promotion");
    }

    // List promotions
    let promotions = service
        .get_workspace_promotions(&workspace_id_str, Some(10))
        .await
        .expect("Failed to list promotions");

    assert_eq!(promotions.len(), 5);
}

#[tokio::test]
async fn test_list_pending_promotions() {
    let db = setup_test_db().await;
    let user_id = create_test_user(&db).await;
    let workspace_id = create_test_workspace(&db, user_id).await;
    let service = PromotionService::new(db);

    let workspace_id_str = workspace_id.to_string();

    // Create pending promotions
    for i in 0..3 {
        let request = PromotionRequest {
            entity_type: PromotionEntityType::Scenario,
            entity_id: format!("scenario-pending-{}", i),
            entity_version: None,
            workspace_id: workspace_id_str.clone(),
            from_environment: MockEnvironmentName::Dev,
            to_environment: MockEnvironmentName::Test,
            requires_approval: true,
            approval_required_reason: None,
            comments: None,
            metadata: HashMap::new(),
        };

        service
            .record_promotion(&request, user_id, PromotionStatus::Pending, None)
            .await
            .expect("Failed to create promotion");
    }

    // Create and approve one promotion
    let request = PromotionRequest {
        entity_type: PromotionEntityType::Scenario,
        entity_id: "scenario-approved".to_string(),
        entity_version: None,
        workspace_id: workspace_id_str.clone(),
        from_environment: MockEnvironmentName::Dev,
        to_environment: MockEnvironmentName::Test,
        requires_approval: true,
        approval_required_reason: None,
        comments: None,
        metadata: HashMap::new(),
    };

    let promotion_id = service
        .record_promotion(&request, user_id, PromotionStatus::Pending, None)
        .await
        .expect("Failed to create promotion");

    service
        .update_promotion_status(promotion_id, PromotionStatus::Approved, Some(user_id))
        .await
        .expect("Failed to approve promotion");

    // List pending promotions
    let pending = service
        .get_pending_promotions(Some(&workspace_id_str))
        .await
        .expect("Failed to list pending promotions");

    assert_eq!(pending.len(), 3);
    assert!(pending.iter().all(|p| p.status == PromotionStatus::Pending));
}

#[tokio::test]
async fn test_promotion_history_for_entity() {
    let db = setup_test_db().await;
    let user_id = create_test_user(&db).await;
    let workspace_id = create_test_workspace(&db, user_id).await;
    let service = PromotionService::new(db);

    let workspace_id_str = workspace_id.to_string();
    let entity_id = "scenario-history-test";

    // Create multiple promotions for the same entity
    let promotions = vec![
        (MockEnvironmentName::Dev, MockEnvironmentName::Test),
        (MockEnvironmentName::Test, MockEnvironmentName::Prod),
    ];

    for (from, to) in promotions {
        let request = PromotionRequest {
            entity_type: PromotionEntityType::Scenario,
            entity_id: entity_id.to_string(),
            entity_version: None,
            workspace_id: workspace_id_str.clone(),
            from_environment: from,
            to_environment: to,
            requires_approval: true,
            approval_required_reason: None,
            comments: None,
            metadata: HashMap::new(),
        };

        service
            .record_promotion(&request, user_id, PromotionStatus::Pending, None)
            .await
            .expect("Failed to create promotion");
    }

    // Get promotion history
    let history = service
        .get_promotion_history(&workspace_id_str, PromotionEntityType::Scenario, entity_id)
        .await
        .expect("Failed to get promotion history");

    assert_eq!(history.promotions.len(), 2);
    assert_eq!(history.entity_id, entity_id);
    assert_eq!(history.entity_type, PromotionEntityType::Scenario);
}

#[tokio::test]
async fn test_promotion_with_metadata() {
    let db = setup_test_db().await;
    let user_id = create_test_user(&db).await;
    let workspace_id = create_test_workspace(&db, user_id).await;
    let service = PromotionService::new(db);

    let workspace_id_str = workspace_id.to_string();

    let mut metadata = HashMap::new();
    metadata.insert("test_key".to_string(), serde_json::json!("test_value"));
    metadata.insert("number".to_string(), serde_json::json!(42));

    let request = PromotionRequest {
        entity_type: PromotionEntityType::Config,
        entity_id: "config-123".to_string(),
        entity_version: None,
        workspace_id: workspace_id_str.clone(),
        from_environment: MockEnvironmentName::Dev,
        to_environment: MockEnvironmentName::Test,
        requires_approval: false,
        approval_required_reason: None,
        comments: None,
        metadata: metadata.clone(),
    };

    let _promotion_id = service
        .record_promotion(&request, user_id, PromotionStatus::Pending, None)
        .await
        .expect("Failed to create promotion");

    // Verify metadata was stored
    let history = service
        .get_promotion_history(&workspace_id_str, PromotionEntityType::Config, "config-123")
        .await
        .expect("Failed to get promotion history");

    assert_eq!(history.promotions.len(), 1);
    assert_eq!(
        history.promotions[0].metadata.get("test_key"),
        Some(&serde_json::json!("test_value"))
    );
    assert_eq!(history.promotions[0].metadata.get("number"), Some(&serde_json::json!(42)));
}

#[tokio::test]
async fn test_pillar_tag_approval_detection() {
    use mockforge_core::pillars::Pillar;

    let rules = ApprovalRules::default();

    // Test [Cloud][Contracts][Reality] combination requires approval
    let tags = vec!["[Cloud][Contracts][Reality]".to_string()];
    let (requires, reason) =
        ScenarioPromotionWorkflow::requires_approval(&tags, MockEnvironmentName::Test, &rules);
    assert!(requires, "Should require approval for Cloud+Contracts+Reality combination");
    assert!(reason.is_some());
    assert!(reason.unwrap().contains("pillar tag combination"));

    // Test single pillar tag doesn't require approval by default
    let tags2 = vec!["[Cloud]".to_string()];
    let (requires2, _) =
        ScenarioPromotionWorkflow::requires_approval(&tags2, MockEnvironmentName::Test, &rules);
    assert!(!requires2, "Single pillar tag should not require approval by default");

    // Test partial combination doesn't match pattern
    let tags3 = vec!["[Cloud][Contracts]".to_string()];
    let (requires3, _) =
        ScenarioPromotionWorkflow::requires_approval(&tags3, MockEnvironmentName::Test, &rules);
    assert!(!requires3, "Partial pillar combination should not require approval");
}

#[tokio::test]
async fn test_pillar_tags_preserved_in_promotion() {
    let db = setup_test_db().await;
    let user_id = create_test_user(&db).await;
    let workspace_id = create_test_workspace(&db, user_id).await;
    let service = PromotionService::new(db);

    let workspace_id_str = workspace_id.to_string();

    let mut metadata = HashMap::new();
    metadata.insert(
        "scenario_tags".to_string(),
        serde_json::json!(vec!["[Cloud][Contracts][Reality]", "checkout", "payment"]),
    );

    let request = PromotionRequest {
        entity_type: PromotionEntityType::Scenario,
        entity_id: "scenario-pillar-tags".to_string(),
        entity_version: None,
        workspace_id: workspace_id_str.clone(),
        from_environment: MockEnvironmentName::Dev,
        to_environment: MockEnvironmentName::Test,
        requires_approval: true,
        approval_required_reason: Some(
            "High-impact pillar tag combination [Cloud][Contracts][Reality] requires approval"
                .to_string(),
        ),
        comments: None,
        metadata: metadata.clone(),
    };

    let _promotion_id = service
        .record_promotion(&request, user_id, PromotionStatus::Pending, None)
        .await
        .expect("Failed to create promotion");

    // Verify tags were preserved in promotion history
    let history = service
        .get_promotion_history(
            &workspace_id_str,
            PromotionEntityType::Scenario,
            "scenario-pillar-tags",
        )
        .await
        .expect("Failed to get promotion history");

    assert_eq!(history.promotions.len(), 1);
    let stored_tags = history.promotions[0]
        .metadata
        .get("scenario_tags")
        .and_then(|v| v.as_array())
        .cloned();
    assert!(stored_tags.is_some());
    let tags_array = stored_tags.unwrap();
    assert_eq!(tags_array.len(), 3);
    assert!(tags_array.contains(&serde_json::json!("[Cloud][Contracts][Reality]")));
    assert!(tags_array.contains(&serde_json::json!("checkout")));
    assert!(tags_array.contains(&serde_json::json!("payment")));
}

#[tokio::test]
async fn test_prod_promotion_with_pillar_tags() {
    use mockforge_core::pillars::Pillar;

    let rules = ApprovalRules::default();

    // Test that prod promotions always require approval, even without pillar tags
    let tags = vec!["normal".to_string()];
    let (requires, reason) =
        ScenarioPromotionWorkflow::requires_approval(&tags, MockEnvironmentName::Prod, &rules);
    assert!(requires, "Prod promotions should always require approval");
    assert!(reason.is_some());
    assert!(reason.unwrap().contains("Production promotions require approval"));

    // Test that pillar tags add additional context to prod approval
    let tags2 = vec!["[Cloud][Contracts][Reality]".to_string()];
    let (requires2, reason2) =
        ScenarioPromotionWorkflow::requires_approval(&tags2, MockEnvironmentName::Prod, &rules);
    assert!(requires2, "Prod promotions with pillar tags should require approval");
    assert!(reason2.is_some());
    // Should mention production (prod always requires approval)
    let reason_str = reason2.unwrap();
    assert!(reason_str.contains("production") || reason_str.contains("Production"));
}
