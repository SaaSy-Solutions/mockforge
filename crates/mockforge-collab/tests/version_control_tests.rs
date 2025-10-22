//! Integration tests for version control endpoints

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{auth_header, TestContext};
use mockforge_collab::models::UserRole;
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn test_create_commit_success() {
    let ctx = TestContext::new().await;
    let (user, token) = ctx.create_test_user("editor", "editor@example.com").await;
    let workspace_id = ctx.create_test_workspace(user.id, "Test Workspace").await;

    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/commits", workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&token).0, auth_header(&token).1)
                .body(Body::from(
                    json!({
                        "message": "Initial commit",
                        "changes": {
                            "added": ["endpoint1", "endpoint2"],
                            "modified": [],
                            "deleted": []
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["message"], "Initial commit");
    assert_eq!(json["version"], 1);
    assert!(json["parent_id"].is_null());
    assert!(json["id"].is_string());
}

#[tokio::test]
async fn test_create_commit_viewer_forbidden() {
    let ctx = TestContext::new().await;
    let (owner, _) = ctx.create_test_user("owner", "owner@example.com").await;
    let (viewer, viewer_token) = ctx.create_test_user("viewer", "viewer@example.com").await;
    let workspace_id = ctx.create_test_workspace(owner.id, "Test Workspace").await;

    // Add viewer
    ctx.add_workspace_member(workspace_id, owner.id, viewer.id, UserRole::Viewer)
        .await;

    // Try to create commit as viewer
    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/commits", workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&viewer_token).0, auth_header(&viewer_token).1)
                .body(Body::from(
                    json!({
                        "message": "Viewer commit",
                        "changes": {}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_create_commit_empty_message_validation() {
    let ctx = TestContext::new().await;
    let (user, token) = ctx.create_test_user("editor", "editor@example.com").await;
    let workspace_id = ctx.create_test_workspace(user.id, "Test Workspace").await;

    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/commits", workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&token).0, auth_header(&token).1)
                .body(Body::from(
                    json!({
                        "message": "",
                        "changes": {}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_commit_message_too_long() {
    let ctx = TestContext::new().await;
    let (user, token) = ctx.create_test_user("editor", "editor@example.com").await;
    let workspace_id = ctx.create_test_workspace(user.id, "Test Workspace").await;

    let long_message = "a".repeat(501);

    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/commits", workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&token).0, auth_header(&token).1)
                .body(Body::from(
                    json!({
                        "message": long_message,
                        "changes": {}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_commits_with_pagination() {
    let ctx = TestContext::new().await;
    let (user, token) = ctx.create_test_user("editor", "editor@example.com").await;
    let workspace_id = ctx.create_test_workspace(user.id, "Test Workspace").await;

    // Create multiple commits
    for i in 1..=5 {
        ctx.history
            .create_commit(
                workspace_id,
                user.id,
                format!("Commit {}", i),
                None,
                i,
                json!({"version": i}),
                json!({"change": i}),
            )
            .await
            .unwrap();
    }

    // List with pagination
    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/workspaces/{}/commits?limit=3&offset=0", workspace_id))
                .header(auth_header(&token).0, auth_header(&token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json["commits"].is_array());
    assert_eq!(json["commits"].as_array().unwrap().len(), 3);
    assert_eq!(json["pagination"]["limit"], 3);
    assert_eq!(json["pagination"]["offset"], 0);
}

#[tokio::test]
async fn test_get_commit() {
    let ctx = TestContext::new().await;
    let (user, token) = ctx.create_test_user("editor", "editor@example.com").await;
    let workspace_id = ctx.create_test_workspace(user.id, "Test Workspace").await;

    // Create a commit
    let commit = ctx
        .history
        .create_commit(
            workspace_id,
            user.id,
            "Test commit".to_string(),
            None,
            1,
            json!({"state": "test"}),
            json!({"added": ["file1"]}),
        )
        .await
        .unwrap();

    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/workspaces/{}/commits/{}", workspace_id, commit.id))
                .header(auth_header(&token).0, auth_header(&token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["message"], "Test commit");
    assert_eq!(json["version"], 1);
}

#[tokio::test]
async fn test_get_commit_wrong_workspace() {
    let ctx = TestContext::new().await;
    let (user, token) = ctx.create_test_user("editor", "editor@example.com").await;
    let workspace1_id = ctx.create_test_workspace(user.id, "Workspace 1").await;
    let workspace2_id = ctx.create_test_workspace(user.id, "Workspace 2").await;

    // Create a commit in workspace 1
    let commit = ctx
        .history
        .create_commit(
            workspace1_id,
            user.id,
            "Test commit".to_string(),
            None,
            1,
            json!({}),
            json!({}),
        )
        .await
        .unwrap();

    // Try to get it from workspace 2
    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/workspaces/{}/commits/{}", workspace2_id, commit.id))
                .header(auth_header(&token).0, auth_header(&token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_restore_to_commit() {
    let ctx = TestContext::new().await;
    let (user, token) = ctx.create_test_user("admin", "admin@example.com").await;
    let workspace_id = ctx.create_test_workspace(user.id, "Test Workspace").await;

    // Create a commit
    let commit = ctx
        .history
        .create_commit(
            workspace_id,
            user.id,
            "Snapshot state".to_string(),
            None,
            1,
            json!({"important": "state"}),
            json!({}),
        )
        .await
        .unwrap();

    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/restore/{}", workspace_id, commit.id))
                .header(auth_header(&token).0, auth_header(&token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["workspace_id"], workspace_id.to_string());
    assert_eq!(json["commit_id"], commit.id.to_string());
    assert!(json["restored_state"].is_object());
}

#[tokio::test]
async fn test_restore_viewer_forbidden() {
    let ctx = TestContext::new().await;
    let (owner, _) = ctx.create_test_user("owner", "owner@example.com").await;
    let (viewer, viewer_token) = ctx.create_test_user("viewer", "viewer@example.com").await;
    let workspace_id = ctx.create_test_workspace(owner.id, "Test Workspace").await;

    // Add viewer
    ctx.add_workspace_member(workspace_id, owner.id, viewer.id, UserRole::Viewer)
        .await;

    // Create a commit as owner
    let commit = ctx
        .history
        .create_commit(
            workspace_id,
            owner.id,
            "Test".to_string(),
            None,
            1,
            json!({}),
            json!({}),
        )
        .await
        .unwrap();

    // Try to restore as viewer
    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/restore/{}", workspace_id, commit.id))
                .header(auth_header(&viewer_token).0, auth_header(&viewer_token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_create_snapshot_success() {
    let ctx = TestContext::new().await;
    let (user, token) = ctx.create_test_user("admin", "admin@example.com").await;
    let workspace_id = ctx.create_test_workspace(user.id, "Test Workspace").await;

    // Create a commit first
    let commit = ctx
        .history
        .create_commit(
            workspace_id,
            user.id,
            "Release state".to_string(),
            None,
            1,
            json!({}),
            json!({}),
        )
        .await
        .unwrap();

    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/snapshots", workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&token).0, auth_header(&token).1)
                .body(Body::from(
                    json!({
                        "name": "v1.0.0",
                        "description": "First release",
                        "commit_id": commit.id.to_string()
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["name"], "v1.0.0");
    assert_eq!(json["description"], "First release");
    assert_eq!(json["commit_id"], commit.id.to_string());
}

#[tokio::test]
async fn test_create_snapshot_invalid_name() {
    let ctx = TestContext::new().await;
    let (user, token) = ctx.create_test_user("admin", "admin@example.com").await;
    let workspace_id = ctx.create_test_workspace(user.id, "Test Workspace").await;

    let commit = ctx
        .history
        .create_commit(workspace_id, user.id, "Test".to_string(), None, 1, json!({}), json!({}))
        .await
        .unwrap();

    // Test with invalid characters
    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/snapshots", workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&token).0, auth_header(&token).1)
                .body(Body::from(
                    json!({
                        "name": "v1.0/invalid",
                        "commit_id": commit.id.to_string()
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_snapshot_name_too_long() {
    let ctx = TestContext::new().await;
    let (user, token) = ctx.create_test_user("admin", "admin@example.com").await;
    let workspace_id = ctx.create_test_workspace(user.id, "Test Workspace").await;

    let commit = ctx
        .history
        .create_commit(workspace_id, user.id, "Test".to_string(), None, 1, json!({}), json!({}))
        .await
        .unwrap();

    let long_name = "a".repeat(101);

    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/snapshots", workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&token).0, auth_header(&token).1)
                .body(Body::from(
                    json!({
                        "name": long_name,
                        "commit_id": commit.id.to_string()
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_snapshots() {
    let ctx = TestContext::new().await;
    let (user, token) = ctx.create_test_user("admin", "admin@example.com").await;
    let workspace_id = ctx.create_test_workspace(user.id, "Test Workspace").await;

    // Create commits and snapshots
    for i in 1..=3 {
        let commit = ctx
            .history
            .create_commit(
                workspace_id,
                user.id,
                format!("Release {}", i),
                None,
                i,
                json!({}),
                json!({}),
            )
            .await
            .unwrap();

        ctx.history
            .create_snapshot(
                workspace_id,
                format!("v{}.0.0", i),
                Some(format!("Release {}", i)),
                commit.id,
                user.id,
            )
            .await
            .unwrap();
    }

    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/workspaces/{}/snapshots", workspace_id))
                .header(auth_header(&token).0, auth_header(&token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn test_get_snapshot_by_name() {
    let ctx = TestContext::new().await;
    let (user, token) = ctx.create_test_user("admin", "admin@example.com").await;
    let workspace_id = ctx.create_test_workspace(user.id, "Test Workspace").await;

    let commit = ctx
        .history
        .create_commit(workspace_id, user.id, "Release".to_string(), None, 1, json!({}), json!({}))
        .await
        .unwrap();

    ctx.history
        .create_snapshot(
            workspace_id,
            "v1.0.0".to_string(),
            Some("First release".to_string()),
            commit.id,
            user.id,
        )
        .await
        .unwrap();

    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/workspaces/{}/snapshots/v1.0.0", workspace_id))
                .header(auth_header(&token).0, auth_header(&token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["name"], "v1.0.0");
    assert_eq!(json["description"], "First release");
}

#[tokio::test]
async fn test_commit_version_increment() {
    let ctx = TestContext::new().await;
    let (user, token) = ctx.create_test_user("editor", "editor@example.com").await;
    let workspace_id = ctx.create_test_workspace(user.id, "Test Workspace").await;

    // Create first commit via API
    let response1 = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/commits", workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&token).0, auth_header(&token).1)
                .body(Body::from(
                    json!({
                        "message": "First commit",
                        "changes": {}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body1 = axum::body::to_bytes(response1.into_body(), usize::MAX)
        .await
        .unwrap();
    let json1: serde_json::Value = serde_json::from_slice(&body1).unwrap();
    assert_eq!(json1["version"], 1);

    // Create second commit via API
    let response2 = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/commits", workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&token).0, auth_header(&token).1)
                .body(Body::from(
                    json!({
                        "message": "Second commit",
                        "changes": {}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body2 = axum::body::to_bytes(response2.into_body(), usize::MAX)
        .await
        .unwrap();
    let json2: serde_json::Value = serde_json::from_slice(&body2).unwrap();
    assert_eq!(json2["version"], 2);
    assert_eq!(json2["parent_id"], json1["id"]);
}
