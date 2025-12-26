//! Integration tests for fork, merge, backup, restore, and state sync

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{auth_header, TestContext};
use mockforge_collab::models::UserRole;
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_fork_workspace_success() {
    let ctx = TestContext::new().await;
    let (owner, owner_token) = ctx.create_test_user("owner", "owner@example.com").await;
    let (forker, forker_token) = ctx.create_test_user("forker", "forker@example.com").await;

    // Create source workspace
    let source_workspace_id = ctx.create_test_workspace(owner.id, "Source Workspace").await;

    // Add forker as viewer to source workspace
    ctx.add_workspace_member(source_workspace_id, owner.id, forker.id, UserRole::Viewer)
        .await;

    // Fork the workspace
    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/fork", source_workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&forker_token).0, auth_header(&forker_token).1)
                .body(Body::from(
                    json!({
                        "new_name": "Forked Workspace",
                        "fork_point_commit_id": null
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["name"], "Forked Workspace");
    assert!(json["id"].is_string());
    let forked_workspace_id = json["id"].as_str().unwrap();

    // Verify fork relationship
    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/workspaces/{}/forks", source_workspace_id))
                .header(auth_header(&owner_token).0, auth_header(&owner_token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let forks: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(forks.len(), 1);
    assert_eq!(forks[0]["forked_workspace_id"], forked_workspace_id);
}

#[tokio::test]
async fn test_fork_workspace_unauthorized() {
    let ctx = TestContext::new().await;
    let (owner, _owner_token) = ctx.create_test_user("owner", "owner@example.com").await;
    let (_unauthorized, unauthorized_token) =
        ctx.create_test_user("unauthorized", "unauthorized@example.com").await;

    let source_workspace_id = ctx.create_test_workspace(owner.id, "Source Workspace").await;

    // Try to fork without access
    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/fork", source_workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&unauthorized_token).0, auth_header(&unauthorized_token).1)
                .body(Body::from(
                    json!({
                        "new_name": "Forked Workspace"
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
async fn test_list_forks() {
    let ctx = TestContext::new().await;
    let (owner, owner_token) = ctx.create_test_user("owner", "owner@example.com").await;
    let (forker1, forker1_token) = ctx.create_test_user("forker1", "forker1@example.com").await;
    let (forker2, forker2_token) = ctx.create_test_user("forker2", "forker2@example.com").await;

    let source_workspace_id = ctx.create_test_workspace(owner.id, "Source Workspace").await;

    // Add both forkers as members
    ctx.add_workspace_member(source_workspace_id, owner.id, forker1.id, UserRole::Viewer)
        .await;
    ctx.add_workspace_member(source_workspace_id, owner.id, forker2.id, UserRole::Viewer)
        .await;

    // Create two forks
    let response1 = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/fork", source_workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&forker1_token).0, auth_header(&forker1_token).1)
                .body(Body::from(
                    json!({
                        "new_name": "Fork 1"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response1.status(), StatusCode::OK);

    let response2 = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/fork", source_workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&forker2_token).0, auth_header(&forker2_token).1)
                .body(Body::from(
                    json!({
                        "new_name": "Fork 2"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response2.status(), StatusCode::OK);

    // List forks
    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/workspaces/{}/forks", source_workspace_id))
                .header(auth_header(&owner_token).0, auth_header(&owner_token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let forks: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(forks.len(), 2);
}

#[tokio::test]
async fn test_merge_workspaces_success() {
    let ctx = TestContext::new().await;
    let (owner, owner_token) = ctx.create_test_user("owner", "owner@example.com").await;
    let (forker, forker_token) = ctx.create_test_user("forker", "forker@example.com").await;

    // Create source workspace with initial commit (required for merge)
    let source_workspace_id = ctx.create_test_workspace(owner.id, "Source Workspace").await;
    let source_commit_id = ctx.create_initial_commit(source_workspace_id, owner.id).await;

    // Add forker as member
    ctx.add_workspace_member(source_workspace_id, owner.id, forker.id, UserRole::Editor)
        .await;

    // Fork the workspace with fork point commit
    let fork_response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/fork", source_workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&forker_token).0, auth_header(&forker_token).1)
                .body(Body::from(
                    json!({
                        "new_name": "Forked Workspace",
                        "fork_point_commit_id": source_commit_id.to_string()
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(fork_response.status(), StatusCode::OK);
    let fork_body = axum::body::to_bytes(fork_response.into_body(), usize::MAX).await.unwrap();
    let fork_json: serde_json::Value = serde_json::from_slice(&fork_body).unwrap();
    let forked_workspace_id = fork_json["id"].as_str().unwrap();
    let forked_workspace_uuid = Uuid::parse_str(forked_workspace_id).unwrap();

    // Create a commit for the forked workspace (required for merge)
    ctx.create_initial_commit(forked_workspace_uuid, forker.id).await;

    // Merge forked workspace back into source
    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/merge", source_workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&owner_token).0, auth_header(&owner_token).1)
                .body(Body::from(
                    json!({
                        "source_workspace_id": forked_workspace_id
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(status, StatusCode::OK, "Merge error: {:?}", json);

    assert!(json["workspace"].is_object(), "Expected workspace in response: {:?}", json);
    assert!(json["conflicts"].is_array(), "Expected conflicts in response: {:?}", json);
}

#[tokio::test]
async fn test_merge_workspaces_with_conflicts() {
    let ctx = TestContext::new().await;
    let (owner, owner_token) = ctx.create_test_user("owner", "owner@example.com").await;
    let (forker, forker_token) = ctx.create_test_user("forker", "forker@example.com").await;

    // Create source workspace with initial commit
    let source_workspace_id = ctx.create_test_workspace(owner.id, "Source Workspace").await;
    let source_commit_id = ctx.create_initial_commit(source_workspace_id, owner.id).await;

    ctx.add_workspace_member(source_workspace_id, owner.id, forker.id, UserRole::Editor)
        .await;

    // Fork with fork point commit and make changes that will conflict
    let fork_response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/fork", source_workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&forker_token).0, auth_header(&forker_token).1)
                .body(Body::from(
                    json!({
                        "new_name": "Forked Workspace",
                        "fork_point_commit_id": source_commit_id.to_string()
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(fork_response.status(), StatusCode::OK);
    let fork_body = axum::body::to_bytes(fork_response.into_body(), usize::MAX).await.unwrap();
    let fork_json: serde_json::Value = serde_json::from_slice(&fork_body).unwrap();
    let forked_workspace_id = fork_json["id"].as_str().unwrap();
    let forked_workspace_uuid = Uuid::parse_str(forked_workspace_id).unwrap();

    // Create a commit for the forked workspace (required for merge)
    ctx.create_initial_commit(forked_workspace_uuid, forker.id).await;

    // Try to merge - may have conflicts depending on state
    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/merge", source_workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&owner_token).0, auth_header(&owner_token).1)
                .body(Body::from(
                    json!({
                        "source_workspace_id": forked_workspace_id
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should succeed even with conflicts (conflicts are returned in response)
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["conflicts"].is_array());
}

#[tokio::test]
async fn test_list_merges() {
    let ctx = TestContext::new().await;
    let (owner, owner_token) = ctx.create_test_user("owner", "owner@example.com").await;
    let (forker, forker_token) = ctx.create_test_user("forker", "forker@example.com").await;

    // Create source workspace with initial commit
    let source_workspace_id = ctx.create_test_workspace(owner.id, "Source Workspace").await;
    let source_commit_id = ctx.create_initial_commit(source_workspace_id, owner.id).await;

    ctx.add_workspace_member(source_workspace_id, owner.id, forker.id, UserRole::Editor)
        .await;

    // Fork with fork point commit and merge
    let fork_response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/fork", source_workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&forker_token).0, auth_header(&forker_token).1)
                .body(Body::from(
                    json!({
                        "new_name": "Forked Workspace",
                        "fork_point_commit_id": source_commit_id.to_string()
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let fork_body = axum::body::to_bytes(fork_response.into_body(), usize::MAX).await.unwrap();
    let fork_json: serde_json::Value = serde_json::from_slice(&fork_body).unwrap();
    let forked_workspace_id = fork_json["id"].as_str().unwrap();
    let forked_workspace_uuid = Uuid::parse_str(forked_workspace_id).unwrap();

    // Create a commit for the forked workspace (required for merge)
    ctx.create_initial_commit(forked_workspace_uuid, forker.id).await;

    // Perform merge
    let _merge_response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/merge", source_workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&owner_token).0, auth_header(&owner_token).1)
                .body(Body::from(
                    json!({
                        "source_workspace_id": forked_workspace_id
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // List merges
    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/workspaces/{}/merges", source_workspace_id))
                .header(auth_header(&owner_token).0, auth_header(&owner_token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let merges: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(!merges.is_empty());
}

#[tokio::test]
async fn test_create_backup() {
    let ctx = TestContext::new().await;
    let (owner, owner_token) = ctx.create_test_user("owner", "owner@example.com").await;

    let workspace_id = ctx.create_test_workspace(owner.id, "Test Workspace").await;

    // Create backup
    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/backup", workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&owner_token).0, auth_header(&owner_token).1)
                .body(Body::from(
                    json!({
                        "storage_backend": "local",
                        "format": "json"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(status, StatusCode::OK, "Backup error response: {:?}", json);

    assert!(json["id"].is_string());
    assert_eq!(json["workspace_id"], workspace_id.to_string());
    assert_eq!(json["storage_backend"], "local");
    assert_eq!(json["backup_format"], "json");
}

#[tokio::test]
async fn test_list_backups() {
    let ctx = TestContext::new().await;
    let (owner, owner_token) = ctx.create_test_user("owner", "owner@example.com").await;

    let workspace_id = ctx.create_test_workspace(owner.id, "Test Workspace").await;

    // Create multiple backups
    for i in 0..3 {
        let response = ctx
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/workspaces/{}/backup", workspace_id))
                    .header("content-type", "application/json")
                    .header(auth_header(&owner_token).0, auth_header(&owner_token).1)
                    .body(Body::from(
                        json!({
                            "storage_backend": "local",
                            "format": if i % 2 == 0 { "json" } else { "yaml" }
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // List backups
    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/workspaces/{}/backups", workspace_id))
                .header(auth_header(&owner_token).0, auth_header(&owner_token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(
        status,
        StatusCode::OK,
        "List backups error: {:?}",
        String::from_utf8_lossy(&body)
    );
    let backups: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(backups.len(), 3);
}

#[tokio::test]
async fn test_delete_backup() {
    let ctx = TestContext::new().await;
    let (owner, owner_token) = ctx.create_test_user("owner", "owner@example.com").await;

    let workspace_id = ctx.create_test_workspace(owner.id, "Test Workspace").await;

    // Create backup
    let create_response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/backup", workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&owner_token).0, auth_header(&owner_token).1)
                .body(Body::from(
                    json!({
                        "storage_backend": "local",
                        "format": "json"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&create_body).unwrap();
    let backup_id = create_json["id"].as_str().unwrap();

    // Delete backup
    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/workspaces/{}/backups/{}", workspace_id, backup_id))
                .header(auth_header(&owner_token).0, auth_header(&owner_token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    // DELETE returns 204 No Content on success
    assert!(
        status == StatusCode::OK || status == StatusCode::NO_CONTENT,
        "Delete backup error: {:?}, status: {:?}",
        String::from_utf8_lossy(&body),
        status
    );
}

#[tokio::test]
async fn test_restore_workspace() {
    let ctx = TestContext::new().await;
    let (owner, owner_token) = ctx.create_test_user("owner", "owner@example.com").await;

    let workspace_id = ctx.create_test_workspace(owner.id, "Test Workspace").await;

    // Create backup
    let create_response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/backup", workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&owner_token).0, auth_header(&owner_token).1)
                .body(Body::from(
                    json!({
                        "storage_backend": "local",
                        "format": "json"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&create_body).unwrap();
    let backup_id = create_json["id"].as_str().unwrap();

    // Restore from backup
    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/restore", workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&owner_token).0, auth_header(&owner_token).1)
                .body(Body::from(
                    json!({
                        "backup_id": backup_id
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["workspace_id"].is_string());
}

#[tokio::test]
async fn test_get_workspace_state() {
    let ctx = TestContext::new().await;
    let (owner, owner_token) = ctx.create_test_user("owner", "owner@example.com").await;

    let workspace_id = ctx.create_test_workspace(owner.id, "Test Workspace").await;

    // Get workspace state
    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/workspaces/{}/state", workspace_id))
                .header(auth_header(&owner_token).0, auth_header(&owner_token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["workspace_id"], workspace_id.to_string());
    assert!(json["version"].is_number());
    assert!(json["state"].is_object());
}

#[tokio::test]
async fn test_update_workspace_state() {
    let ctx = TestContext::new().await;
    let (owner, owner_token) = ctx.create_test_user("owner", "owner@example.com").await;

    let workspace_id = ctx.create_test_workspace(owner.id, "Test Workspace").await;

    // Update workspace state
    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/state", workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&owner_token).0, auth_header(&owner_token).1)
                .body(Body::from(
                    json!({
                        "state": {
                            "name": "Updated Workspace",
                            "folders": [],
                            "requests": []
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(status, StatusCode::OK, "Update state error: {:?}", json);
    assert_eq!(json["workspace_id"], workspace_id.to_string());
}

#[tokio::test]
async fn test_get_state_history() {
    let ctx = TestContext::new().await;
    let (owner, owner_token) = ctx.create_test_user("owner", "owner@example.com").await;

    let workspace_id = ctx.create_test_workspace(owner.id, "Test Workspace").await;

    // Make some state changes
    for i in 0..3 {
        let _response = ctx
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/workspaces/{}/state", workspace_id))
                    .header("content-type", "application/json")
                    .header(auth_header(&owner_token).0, auth_header(&owner_token).1)
                    .body(Body::from(
                        json!({
                            "state": {
                                "name": format!("Updated Workspace {}", i),
                                "folders": [],
                                "requests": []
                            }
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    // Get state history
    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/workspaces/{}/state/history", workspace_id))
                .header(auth_header(&owner_token).0, auth_header(&owner_token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["workspace_id"], workspace_id.to_string());
    let changes = json["changes"].as_array().expect("Expected changes to be an array");
    assert!(!changes.is_empty());
}
