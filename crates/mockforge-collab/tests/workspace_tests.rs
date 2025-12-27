//! Integration tests for workspace management endpoints

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{auth_header, TestContext};
use mockforge_collab::models::UserRole;
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn test_create_workspace() {
    let ctx = TestContext::new().await;
    let (_user, token) = ctx.create_test_user("owner", "owner@example.com").await;

    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/workspaces")
                .header("content-type", "application/json")
                .header(auth_header(&token).0, auth_header(&token).1)
                .body(Body::from(
                    json!({
                        "name": "My Workspace",
                        "description": "Test workspace"
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

    assert_eq!(json["name"], "My Workspace");
    assert_eq!(json["description"], "Test workspace");
    assert!(json["id"].is_string());
}

#[tokio::test]
async fn test_create_workspace_unauthorized() {
    let ctx = TestContext::new().await;

    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/workspaces")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "My Workspace"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_list_workspaces() {
    let ctx = TestContext::new().await;
    let (user, token) = ctx.create_test_user("owner", "owner@example.com").await;

    // Create multiple workspaces
    ctx.create_test_workspace(user.id, "Workspace 1").await;
    ctx.create_test_workspace(user.id, "Workspace 2").await;

    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/workspaces")
                .header(auth_header(&token).0, auth_header(&token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_get_workspace() {
    let ctx = TestContext::new().await;
    let (user, token) = ctx.create_test_user("owner", "owner@example.com").await;
    let workspace_id = ctx.create_test_workspace(user.id, "Test Workspace").await;

    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/workspaces/{}", workspace_id))
                .header(auth_header(&token).0, auth_header(&token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["name"], "Test Workspace");
}

#[tokio::test]
async fn test_update_workspace() {
    let ctx = TestContext::new().await;
    let (user, token) = ctx.create_test_user("owner", "owner@example.com").await;
    let workspace_id = ctx.create_test_workspace(user.id, "Original Name").await;

    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/workspaces/{}", workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&token).0, auth_header(&token).1)
                .body(Body::from(
                    json!({
                        "name": "Updated Name",
                        "description": "Updated description"
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

    assert_eq!(json["name"], "Updated Name");
    assert_eq!(json["description"], "Updated description");
}

#[tokio::test]
async fn test_delete_workspace() {
    let ctx = TestContext::new().await;
    let (user, token) = ctx.create_test_user("owner", "owner@example.com").await;
    let workspace_id = ctx.create_test_workspace(user.id, "To Delete").await;

    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/workspaces/{}", workspace_id))
                .header(auth_header(&token).0, auth_header(&token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_add_member_to_workspace() {
    let ctx = TestContext::new().await;
    let (owner, owner_token) = ctx.create_test_user("owner", "owner@example.com").await;
    let (member, _) = ctx.create_test_user("member", "member@example.com").await;
    let workspace_id = ctx.create_test_workspace(owner.id, "Shared Workspace").await;

    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/workspaces/{}/members", workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&owner_token).0, auth_header(&owner_token).1)
                .body(Body::from(
                    json!({
                        "user_id": member.id.to_string(),
                        "role": "editor"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_members() {
    let ctx = TestContext::new().await;
    let (owner, owner_token) = ctx.create_test_user("owner", "owner@example.com").await;
    let (member, _) = ctx.create_test_user("member", "member@example.com").await;
    let workspace_id = ctx.create_test_workspace(owner.id, "Team Workspace").await;

    // Add member
    ctx.add_workspace_member(workspace_id, owner.id, member.id, UserRole::Editor)
        .await;

    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/workspaces/{}/members", workspace_id))
                .header(auth_header(&owner_token).0, auth_header(&owner_token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json.is_array());
    // Owner + member = 2
    assert_eq!(json.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_viewer_cannot_update_workspace() {
    let ctx = TestContext::new().await;
    let (owner, _) = ctx.create_test_user("owner", "owner@example.com").await;
    let (viewer, viewer_token) = ctx.create_test_user("viewer", "viewer@example.com").await;
    let workspace_id = ctx.create_test_workspace(owner.id, "Test Workspace").await;

    // Add viewer
    ctx.add_workspace_member(workspace_id, owner.id, viewer.id, UserRole::Viewer)
        .await;

    // Try to update as viewer
    let response = ctx
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/workspaces/{}", workspace_id))
                .header("content-type", "application/json")
                .header(auth_header(&viewer_token).0, auth_header(&viewer_token).1)
                .body(Body::from(
                    json!({
                        "name": "Hacked Name"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
