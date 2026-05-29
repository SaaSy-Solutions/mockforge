//! End-to-end test for the workspace-content surface added in the cloud port.
//!
//! Covers every endpoint registered under `/api/v1/workspaces/{id}/*` in
//! migration `20250101000038_workspace_content.sql`:
//!
//!   environments CRUD + activate + reorder + variables
//!   folders CRUD (partial) + requests CRUD (partial)
//!   workspace detail (folders + requests counts)
//!   import preview + import execute + autocomplete
//!   workspace activate + reorder
//!
//! Each step asserts both the HTTP status and the response-shape fields the
//! cloud UI relies on (see `crates/mockforge-ui/ui/src/types/index.ts`).
//!
//! Requires:
//!   - PostgreSQL + MinIO running (docker-compose up db minio minio-init)
//!   - Registry server running (see signup_flow_e2e.rs header for the env vars)
//!
//! Run with:
//!   REGISTRY_URL=http://localhost:8080 \
//!   cargo test --test workspace_content_e2e -- --ignored --nocapture

use reqwest::{Client, StatusCode};
use serde_json::{json, Value};

/// Thin wrapper around reqwest::Client that tracks auth + org headers.
struct E2e {
    client: Client,
    base_url: String,
    access_token: String,
    org_id: String,
}

impl E2e {
    fn auth(&self) -> String {
        format!("Bearer {}", self.access_token)
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .get(format!("{}{}", self.base_url, path))
            .header("Authorization", self.auth())
            .header("X-Organization-Id", &self.org_id)
    }

    fn post(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .post(format!("{}{}", self.base_url, path))
            .header("Authorization", self.auth())
            .header("X-Organization-Id", &self.org_id)
    }

    fn put(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .put(format!("{}{}", self.base_url, path))
            .header("Authorization", self.auth())
            .header("X-Organization-Id", &self.org_id)
    }

    fn delete(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .delete(format!("{}{}", self.base_url, path))
            .header("Authorization", self.auth())
            .header("X-Organization-Id", &self.org_id)
    }
}

async fn register_and_setup(base_url: &str) -> E2e {
    let client = Client::new();
    let ts = chrono::Utc::now().timestamp_micros();
    let username = format!("wsct_{}", ts);
    let email = format!("wsct_{}@e2e-test.local", ts);
    let password = "SecureP@ssw0rd!2024";

    // Register
    let res = client
        .post(format!("{}/api/v1/auth/register", base_url))
        .json(&json!({ "username": username, "email": email, "password": password }))
        .send()
        .await
        .expect("register failed");
    let status = res.status();
    let body: Value = res.json().await.expect("register not JSON");
    assert!(status.is_success(), "register {}: {}", status, body);
    let access_token = body["access_token"]
        .as_str()
        .or_else(|| body["token"].as_str())
        .expect("no access token")
        .to_string();

    // Create org (using a slug unique per run)
    let org_slug = format!("wsct-{}", ts);
    let res = client
        .post(format!("{}/api/v1/organizations", base_url))
        .header("Authorization", format!("Bearer {}", access_token))
        // These tests create multiple workspaces in one org to exercise
        // cross-workspace isolation; the Free plan caps an org at 1 workspace
        // (max_projects), so provision a Team org which is uncapped.
        .json(&json!({ "name": format!("WS-Content Test Org {}", ts), "slug": org_slug, "plan": "team" }))
        .send()
        .await
        .expect("create org failed");
    let status = res.status();
    let body: Value = res.json().await.expect("org not JSON");
    assert!(status.is_success(), "create org {}: {}", status, body);
    let org_id = body["id"].as_str().expect("no org id").to_string();

    E2e {
        client,
        base_url: base_url.to_string(),
        access_token,
        org_id,
    }
}

async fn create_workspace(e: &E2e, name: &str) -> String {
    let res = e
        .post("/api/v1/workspaces")
        .json(&json!({ "name": name, "description": "e2e fixture" }))
        .send()
        .await
        .expect("create ws failed");
    let status = res.status();
    let body: Value = res.json().await.expect("ws not JSON");
    assert!(status.is_success(), "create workspace {}: {}", status, body);
    body["id"].as_str().expect("no ws id").to_string()
}

#[tokio::test]
#[ignore]
async fn workspace_content_full_flow() {
    let base_url =
        std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let e = register_and_setup(&base_url).await;

    let ws_id = create_workspace(&e, "content-primary").await;
    println!("created workspace {}", ws_id);

    // ────────────────────────────────────────────────────────────────────
    // Environments
    // ────────────────────────────────────────────────────────────────────

    // Empty list
    let res = e
        .get(&format!("/api/v1/workspaces/{}/environments", ws_id))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["environments"].as_array().unwrap().len(), 0);
    assert_eq!(body["total"].as_i64().unwrap(), 0);

    // Create two envs
    let mut env_ids = Vec::new();
    for name in &["dev", "staging"] {
        let res = e
            .post(&format!("/api/v1/workspaces/{}/environments", ws_id))
            .json(&json!({
                "name": name,
                "description": format!("{} env", name),
                "color": { "hex": "#3B82F6", "name": "Blue" }
            }))
            .send()
            .await
            .unwrap();
        let status = res.status();
        let body: Value = res.json().await.unwrap();
        assert!(status.is_success(), "create env {}: {}", status, body);
        env_ids.push(body["id"].as_str().unwrap().to_string());
    }

    // Duplicate name rejects
    let res = e
        .post(&format!("/api/v1/workspaces/{}/environments", ws_id))
        .json(&json!({ "name": "dev" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // List shows both in insertion order
    let res = e
        .get(&format!("/api/v1/workspaces/{}/environments", ws_id))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let envs = body["environments"].as_array().unwrap();
    assert_eq!(envs.len(), 2);
    assert_eq!(envs[0]["name"], "dev");
    assert_eq!(envs[1]["name"], "staging");
    assert_eq!(envs[0]["active"], false);
    assert_eq!(envs[0]["variable_count"], 0);
    assert!(envs[0]["color"]["hex"].as_str().is_some());

    // Update
    let res = e
        .put(&format!("/api/v1/workspaces/{}/environments/{}", ws_id, env_ids[0]))
        .json(&json!({ "description": "dev (updated)" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Reorder: staging first
    let res = e
        .put(&format!("/api/v1/workspaces/{}/environments/order", ws_id))
        .json(&json!({ "environment_ids": [env_ids[1], env_ids[0]] }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = e
        .get(&format!("/api/v1/workspaces/{}/environments", ws_id))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let envs = body["environments"].as_array().unwrap();
    assert_eq!(envs[0]["name"], "staging");
    assert_eq!(envs[1]["name"], "dev");

    // Activate dev — only dev should end up active
    let res = e
        .post(&format!("/api/v1/workspaces/{}/environments/{}/activate", ws_id, env_ids[0]))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = e
        .get(&format!("/api/v1/workspaces/{}/environments", ws_id))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let envs = body["environments"].as_array().unwrap();
    let actives: Vec<_> = envs.iter().filter(|e| e["active"] == true).collect();
    assert_eq!(actives.len(), 1, "exactly one active env");
    assert_eq!(actives[0]["name"], "dev");

    // Activate staging — dev should become inactive
    let res = e
        .post(&format!("/api/v1/workspaces/{}/environments/{}/activate", ws_id, env_ids[1]))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = e
        .get(&format!("/api/v1/workspaces/{}/environments", ws_id))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let envs = body["environments"].as_array().unwrap();
    let actives: Vec<_> = envs.iter().filter(|e| e["active"] == true).collect();
    assert_eq!(actives.len(), 1);
    assert_eq!(actives[0]["name"], "staging");

    // ────────────────────────────────────────────────────────────────────
    // Variables
    // ────────────────────────────────────────────────────────────────────

    // Empty
    let res = e
        .get(&format!("/api/v1/workspaces/{}/environments/{}/variables", ws_id, env_ids[0]))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["variables"].as_array().unwrap().len(), 0);

    // Upsert
    for (k, v, secret) in &[
        ("API_URL", "https://example.test", false),
        ("TOKEN", "abc", true),
    ] {
        let res = e
            .post(&format!("/api/v1/workspaces/{}/environments/{}/variables", ws_id, env_ids[0]))
            .json(&json!({ "key": k, "value": v, "encrypted": secret }))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "set {}", k);
    }

    // Re-upsert same key — should succeed (idempotent) and not duplicate
    let res = e
        .post(&format!("/api/v1/workspaces/{}/environments/{}/variables", ws_id, env_ids[0]))
        .json(&json!({ "key": "API_URL", "value": "https://v2.example.test" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = e
        .get(&format!("/api/v1/workspaces/{}/environments/{}/variables", ws_id, env_ids[0]))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let vars = body["variables"].as_array().unwrap();
    assert_eq!(vars.len(), 2);
    let api_url = vars.iter().find(|v| v["key"] == "API_URL").unwrap();
    assert_eq!(api_url["value"], "https://v2.example.test");
    let token = vars.iter().find(|v| v["key"] == "TOKEN").unwrap();
    assert_eq!(token["encrypted"], true);

    // variable_count should now be 2 on env[0] via list_environments
    let res = e
        .get(&format!("/api/v1/workspaces/{}/environments", ws_id))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let envs = body["environments"].as_array().unwrap();
    let dev_env = envs.iter().find(|e| e["id"] == env_ids[0].as_str()).unwrap();
    assert_eq!(dev_env["variable_count"], 2);

    // Delete a variable
    let res = e
        .delete(&format!(
            "/api/v1/workspaces/{}/environments/{}/variables/TOKEN",
            ws_id, env_ids[0]
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Delete missing variable → 400
    let res = e
        .delete(&format!(
            "/api/v1/workspaces/{}/environments/{}/variables/NOT_THERE",
            ws_id, env_ids[0]
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // ────────────────────────────────────────────────────────────────────
    // Folders + requests
    // ────────────────────────────────────────────────────────────────────

    let res = e
        .post(&format!("/api/v1/workspaces/{}/folders", ws_id))
        .json(&json!({ "name": "Users API" }))
        .send()
        .await
        .unwrap();
    let status = res.status();
    let body: Value = res.json().await.unwrap();
    assert!(status.is_success(), "create folder {}: {}", status, body);
    let folder_id = body["id"].as_str().unwrap().to_string();

    // Folder name is required
    let res = e
        .post(&format!("/api/v1/workspaces/{}/folders", ws_id))
        .json(&json!({ "name": "  " }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Requests: one in folder, one at root
    let res = e
        .post(&format!("/api/v1/workspaces/{}/requests", ws_id))
        .json(&json!({
            "name": "List users",
            "method": "get",
            "path": "/api/users",
            "status_code": 200,
            "response_body": "[]",
            "folder_id": folder_id,
        }))
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());

    let res = e
        .post(&format!("/api/v1/workspaces/{}/requests", ws_id))
        .json(&json!({
            "name": "Health check",
            "method": "GET",
            "path": "/healthz",
        }))
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());

    // Name + path are required
    let res = e
        .post(&format!("/api/v1/workspaces/{}/requests", ws_id))
        .json(&json!({ "name": "bad", "method": "GET", "path": "" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Folder detail should show one request in it
    let res = e
        .get(&format!("/api/v1/workspaces/{}/folders/{}", ws_id, folder_id))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["folder"]["summary"]["name"], "Users API");
    assert_eq!(body["folder"]["summary"]["request_count"], 1);
    assert_eq!(body["folder"]["summary"]["subfolder_count"], 0);
    assert_eq!(body["folder"]["requests"].as_array().unwrap().len(), 1);
    assert_eq!(body["folder"]["requests"][0]["method"], "GET");

    // Workspace detail: 1 folder, 1 top-level request (folder request excluded from top-level)
    let res = e.get(&format!("/api/v1/workspaces/{}", ws_id)).send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    let ws = &body["workspace"];
    assert_eq!(ws["summary"]["folder_count"], 1);
    assert_eq!(ws["summary"]["request_count"], 2); // total in workspace
    assert_eq!(ws["folders"].as_array().unwrap().len(), 1);
    assert_eq!(ws["requests"].as_array().unwrap().len(), 1); // root-only
    assert_eq!(ws["requests"][0]["name"], "Health check");

    // Creating a request in a folder that doesn't belong to this workspace → 400
    let other_ws_id = create_workspace(&e, "content-other").await;
    let res = e
        .post(&format!("/api/v1/workspaces/{}/folders", other_ws_id))
        .json(&json!({ "name": "Alien" }))
        .send()
        .await
        .unwrap();
    let alien_folder_id = res.json::<Value>().await.unwrap()["id"].as_str().unwrap().to_string();

    let res = e
        .post(&format!("/api/v1/workspaces/{}/requests", ws_id))
        .json(&json!({
            "name": "misplaced",
            "method": "GET",
            "path": "/",
            "folder_id": alien_folder_id,
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // ────────────────────────────────────────────────────────────────────
    // Import preview + execute
    // ────────────────────────────────────────────────────────────────────

    // Minimal Postman collection v2.1
    let postman = json!({
        "info": { "name": "tiny", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json" },
        "item": [
            {
                "name": "Ping",
                "request": { "method": "GET", "url": { "raw": "{{base}}/ping", "host": ["{{base}}"], "path": ["ping"] } }
            },
            {
                "name": "Echo",
                "request": { "method": "POST", "url": { "raw": "{{base}}/echo", "host": ["{{base}}"], "path": ["echo"] } }
            }
        ]
    })
    .to_string();

    // Preview
    let res = e
        .post("/api/v1/import/preview")
        .json(&json!({ "format": "postman", "data": postman, "base_url": "https://example.test" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    let routes = body["routes"].as_array().unwrap();
    assert_eq!(routes.len(), 2);
    let methods: Vec<&str> = routes.iter().map(|r| r["method"].as_str().unwrap()).collect();
    assert!(methods.iter().any(|m| m.eq_ignore_ascii_case("GET")));
    assert!(methods.iter().any(|m| m.eq_ignore_ascii_case("POST")));

    // Unsupported format → 400
    let res = e
        .post("/api/v1/import/preview")
        .json(&json!({ "format": "bogus", "data": "{}" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Execute: select only the first route, no folder creation
    let baseline_count = e
        .get(&format!("/api/v1/workspaces/{}", ws_id))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap()["workspace"]["summary"]["request_count"]
        .as_i64()
        .unwrap();

    let res = e
        .post(&format!("/api/v1/workspaces/{}/import", ws_id))
        .json(&json!({
            "format": "postman",
            "data": postman,
            "selected_routes": [0],
            "create_folders": false
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["imported"], 1);

    let new_count = e
        .get(&format!("/api/v1/workspaces/{}", ws_id))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap()["workspace"]["summary"]["request_count"]
        .as_i64()
        .unwrap();
    assert_eq!(new_count, baseline_count + 1);

    // Execute: both routes with create_folders=true (should create GET and POST folders)
    let folders_before = e
        .get(&format!("/api/v1/workspaces/{}", ws_id))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap()["workspace"]["summary"]["folder_count"]
        .as_i64()
        .unwrap();

    let res = e
        .post(&format!("/api/v1/workspaces/{}/import", ws_id))
        .json(&json!({
            "format": "postman",
            "data": postman,
            "create_folders": true,
        }))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["imported"], 2);

    let folders_after = e
        .get(&format!("/api/v1/workspaces/{}", ws_id))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap()["workspace"]["summary"]["folder_count"]
        .as_i64()
        .unwrap();
    assert_eq!(folders_after, folders_before + 2, "GET + POST folders created");

    // ────────────────────────────────────────────────────────────────────
    // Autocomplete
    // ────────────────────────────────────────────────────────────────────

    // Put a variable on the active env so we have something to suggest.
    let res = e
        .post(&format!("/api/v1/workspaces/{}/environments/{}/variables", ws_id, env_ids[1]))
        .json(&json!({ "key": "STAGING_URL", "value": "https://staging.example.test" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Cursor sits right after "{{STAG" — prefix should match STAGING_URL
    let input = "hello {{STAG";
    let res = e
        .post(&format!("/api/v1/workspaces/{}/autocomplete", ws_id))
        .json(&json!({ "input": input, "cursor_position": input.len() }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    let suggestions = body["suggestions"].as_array().unwrap();
    assert!(
        suggestions.iter().any(|s| s["text"] == "STAGING_URL"),
        "expected STAGING_URL in {}",
        body
    );

    // Outside a template span, no suggestions
    let res = e
        .post(&format!("/api/v1/workspaces/{}/autocomplete", ws_id))
        .json(&json!({ "input": "plain text", "cursor_position": 5 }))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["suggestions"].as_array().unwrap().len(), 0);

    // ────────────────────────────────────────────────────────────────────
    // Workspace activate + reorder
    // ────────────────────────────────────────────────────────────────────

    // Activate other_ws_id; primary ws_id should become inactive.
    let res = e
        .post(&format!("/api/v1/workspaces/{}/activate", other_ws_id))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = e.get("/api/v1/workspaces").send().await.unwrap();
    let list: Value = res.json().await.unwrap();
    let arr = list.as_array().unwrap();
    let primary = arr.iter().find(|w| w["id"] == ws_id.as_str()).unwrap();
    let other = arr.iter().find(|w| w["id"] == other_ws_id.as_str()).unwrap();
    assert_eq!(primary["is_active"], false);
    assert_eq!(other["is_active"], true);

    // Reorder: primary first, other second.
    let res = e
        .put("/api/v1/workspaces/order")
        .json(&json!({ "workspace_ids": [ws_id, other_ws_id] }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = e.get("/api/v1/workspaces").send().await.unwrap();
    let list: Value = res.json().await.unwrap();
    let arr = list.as_array().unwrap();
    // Find positional indices — find_by_org orders by sort_order ASC.
    let primary_idx = arr.iter().position(|w| w["id"] == ws_id.as_str()).unwrap();
    let other_idx = arr.iter().position(|w| w["id"] == other_ws_id.as_str()).unwrap();
    assert!(primary_idx < other_idx, "primary should sort before other after reorder");

    // ────────────────────────────────────────────────────────────────────
    // Environment deletion + cleanup
    // ────────────────────────────────────────────────────────────────────

    // Delete an env; its variables should cascade away with it.
    let res = e
        .delete(&format!("/api/v1/workspaces/{}/environments/{}", ws_id, env_ids[0]))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = e
        .get(&format!("/api/v1/workspaces/{}/environments", ws_id))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["environments"].as_array().unwrap().len(), 1);

    // Cross-workspace isolation: can't see other_ws_id's folder from ws_id
    let res = e
        .get(&format!("/api/v1/workspaces/{}/folders/{}", ws_id, alien_folder_id))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Delete workspaces — should cascade content.
    let res = e.delete(&format!("/api/v1/workspaces/{}", ws_id)).send().await.unwrap();
    assert!(res.status().is_success());
    let res = e.delete(&format!("/api/v1/workspaces/{}", other_ws_id)).send().await.unwrap();
    assert!(res.status().is_success());

    println!("workspace_content_full_flow: OK");
}

// ────────────────────────────────────────────────────────────────────────
// Unauthorized access
// ────────────────────────────────────────────────────────────────────────
//
// Quickly verify that unauthed + cross-org requests are rejected. This
// catches the "I forgot `resolve_org_context`" class of regression.

#[tokio::test]
#[ignore]
async fn workspace_content_requires_auth_and_org_scope() {
    let base_url =
        std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let client = Client::new();

    // No Authorization header → 401
    let res = client
        .get(format!(
            "{}/api/v1/workspaces/00000000-0000-0000-0000-000000000000/environments",
            base_url
        ))
        .send()
        .await
        .expect("req failed");
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Two orgs belonging to different users: user A cannot touch user B's workspace.
    let a = register_and_setup(&base_url).await;
    let b = register_and_setup(&base_url).await;
    let a_ws = create_workspace(&a, "a-only").await;

    // B attempts to list A's workspace environments → 400 (not-found-ish) because
    // `workspace.org_id != b.org_id`.
    let res = b
        .get(&format!("/api/v1/workspaces/{}/environments", a_ws))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // B attempts to create a folder under A's workspace → 400.
    let res = b
        .post(&format!("/api/v1/workspaces/{}/folders", a_ws))
        .json(&json!({ "name": "sneaky" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Cleanup
    let _ = a.delete(&format!("/api/v1/workspaces/{}", a_ws)).send().await;
}

// ────────────────────────────────────────────────────────────────────────
// Encryption policy
// ────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn workspace_encryption_policy_flow() {
    let base_url =
        std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let e = register_and_setup(&base_url).await;
    let ws = create_workspace(&e, "enc-ws").await;

    // Default status: disabled, no key_id, no last_rotated.
    let res = e
        .get(&format!("/api/v1/workspaces/{}/encryption/status", ws))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["enabled"], false);
    assert_eq!(body["algorithm"], "aes-256-gcm");
    assert!(body["last_rotated"].is_null());

    // Enable → status flips, key_rotated_at stamped.
    let res = e
        .post(&format!("/api/v1/workspaces/{}/encryption/enable", ws))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = e
        .get(&format!("/api/v1/workspaces/{}/encryption/status", ws))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["enabled"], true);
    assert!(body["last_rotated"].as_str().is_some());
    assert!(body["key_id"].as_str().is_some());

    // Config GET echoes `enabled` + `algorithm`.
    let res = e
        .get(&format!("/api/v1/workspaces/{}/encryption/config", ws))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["enabled"], true);
    assert_eq!(body["algorithm"], "aes-256-gcm");

    // Config PUT stores sensitiveFields; `enabled`/`algorithm` in the payload are ignored
    // because the flag column is authoritative.
    let res = e
        .put(&format!("/api/v1/workspaces/{}/encryption/config", ws))
        .json(&json!({
            "enabled": false,
            "algorithm": "nope",
            "sensitiveFields": ["password", "apiKey"],
            "sensitiveHeaders": ["Authorization"],
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = e
        .get(&format!("/api/v1/workspaces/{}/encryption/config", ws))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["enabled"], true, "flag untouched by config PUT");
    assert_eq!(body["algorithm"], "aes-256-gcm", "algorithm untouched");
    assert_eq!(body["sensitiveFields"][0], "password");

    // Non-object config rejected
    let res = e
        .put(&format!("/api/v1/workspaces/{}/encryption/config", ws))
        .json(&json!(["not", "an", "object"]))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Security check flags suspicious variables.
    // Create an env with one safely-named, is_secret=false var, one sensitively-named one.
    let res = e
        .post(&format!("/api/v1/workspaces/{}/environments", ws))
        .json(&json!({ "name": "dev" }))
        .send()
        .await
        .unwrap();
    let env_id = res.json::<Value>().await.unwrap()["id"].as_str().unwrap().to_string();

    for (k, v, secret) in &[
        ("BASE_URL", "https://example.test", false),
        ("api_key", "sk_live_plaintext_leak", false),
    ] {
        e.post(&format!("/api/v1/workspaces/{}/environments/{}/variables", ws, env_id))
            .json(&json!({ "key": k, "value": v, "encrypted": secret }))
            .send()
            .await
            .unwrap();
    }

    let res = e
        .post(&format!("/api/v1/workspaces/{}/encryption/security-check", ws))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    let warnings = body["warnings"].as_array().unwrap();
    assert!(
        warnings.iter().any(|w| w.as_str().unwrap().contains("api_key")),
        "expected api_key warning, got {:?}",
        warnings
    );
    // The sk_ prefix should also trigger the value-pattern check.
    let value_pattern_warning =
        warnings.iter().any(|w| w.as_str().unwrap().contains("secret pattern"));
    assert!(value_pattern_warning, "expected secret-pattern warning: {:?}", warnings);

    // Every named check appears in the report.
    let check_names: Vec<&str> = body["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    for expected in &[
        "workspace_encryption_enabled",
        "byok_master_key_configured",
        "no_sensitive_named_plaintext_vars",
        "no_suspicious_valued_plaintext_vars",
    ] {
        assert!(check_names.contains(expected), "missing check {}", expected);
    }

    // Disable → reflected immediately; `last_rotated` is retained (we don't wipe key
    // metadata on toggle-off; it's just informational).
    let res = e
        .post(&format!("/api/v1/workspaces/{}/encryption/disable", ws))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = e
        .get(&format!("/api/v1/workspaces/{}/encryption/status", ws))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["enabled"], false);
    assert!(body["last_rotated"].as_str().is_some());

    let _ = e.delete(&format!("/api/v1/workspaces/{}", ws)).send().await;
    println!("workspace_encryption_policy_flow: OK");
}

// ────────────────────────────────────────────────────────────────────────
// Request execute + history
// ────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn workspace_request_execute_and_history() {
    let base_url =
        std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let e = register_and_setup(&base_url).await;
    let ws = create_workspace(&e, "exec-ws").await;

    // Env with a variable to use in template expansion.
    let env_id = e
        .post(&format!("/api/v1/workspaces/{}/environments", ws))
        .json(&json!({ "name": "dev" }))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    e.post(&format!("/api/v1/workspaces/{}/environments/{}/activate", ws, env_id))
        .send()
        .await
        .unwrap();

    for (k, v) in &[("name", "Ray"), ("host", "api.example.test")] {
        e.post(&format!("/api/v1/workspaces/{}/environments/{}/variables", ws, env_id))
            .json(&json!({ "key": k, "value": v }))
            .send()
            .await
            .unwrap();
    }

    // Request whose response_body and path both use `{{var}}` tokens.
    let res = e
        .post(&format!("/api/v1/workspaces/{}/requests", ws))
        .json(&json!({
            "name": "Greet",
            "method": "GET",
            "path": "/greet/{{name}}",
            "status_code": 201,
            "response_body": "Hello {{name}} from {{host}}",
            "response_headers": { "X-Host": "{{host}}" },
        }))
        .send()
        .await
        .unwrap();
    let req_id = res.json::<Value>().await.unwrap()["id"].as_str().unwrap().to_string();

    // Empty history initially
    let res = e
        .get(&format!("/api/v1/workspaces/{}/requests/{}/history", ws, req_id))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["total"].as_i64().unwrap(), 0);
    assert_eq!(body["history"].as_array().unwrap().len(), 0);

    // Execute with an override that wins over the env var.
    let res = e
        .post(&format!("/api/v1/workspaces/{}/requests/{}/execute", ws, req_id))
        .json(&json!({ "variables": { "name": "Alex" } }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["request_method"], "GET");
    assert_eq!(body["request_path"], "/greet/Alex", "override wins over env");
    assert_eq!(body["response_status_code"], 201);
    assert_eq!(
        body["response_body"].as_str().unwrap(),
        "Hello Alex from api.example.test",
        "env fills remaining tokens"
    );
    assert_eq!(body["response_headers"]["X-Host"], "api.example.test");
    assert!(body["response_size_bytes"].as_i64().unwrap() > 0);

    // Execute again with NO override; env's `name=Ray` should apply.
    let res = e
        .post(&format!("/api/v1/workspaces/{}/requests/{}/execute", ws, req_id))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["request_path"], "/greet/Ray");

    // History now has 2 entries, newest first.
    let res = e
        .get(&format!("/api/v1/workspaces/{}/requests/{}/history", ws, req_id))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["total"].as_i64().unwrap(), 2);
    let entries = body["history"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    // Newest is the second execute (env-based Ray), oldest is the override Alex.
    assert_eq!(entries[0]["request_path"], "/greet/Ray");
    assert_eq!(entries[1]["request_path"], "/greet/Alex");

    // Unknown tokens survive (so users see what's missing).
    let res = e
        .post(&format!("/api/v1/workspaces/{}/requests", ws))
        .json(&json!({
            "name": "Missing",
            "method": "GET",
            "path": "/a/{{nonexistent}}",
            "response_body": "body {{also_missing}}",
        }))
        .send()
        .await
        .unwrap();
    let rid2 = res.json::<Value>().await.unwrap()["id"].as_str().unwrap().to_string();

    let res = e
        .post(&format!("/api/v1/workspaces/{}/requests/{}/execute", ws, rid2))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["request_path"], "/a/{{nonexistent}}");
    assert_eq!(body["response_body"].as_str().unwrap(), "body {{also_missing}}");

    // Cross-workspace request → 400
    let other = create_workspace(&e, "exec-other").await;
    let res = e
        .post(&format!("/api/v1/workspaces/{}/requests/{}/execute", other, req_id))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Deleting the request cascades its history rows.
    e.delete(&format!("/api/v1/workspaces/{}", ws)).send().await.unwrap();
    e.delete(&format!("/api/v1/workspaces/{}", other)).send().await.unwrap();

    println!("workspace_request_execute_and_history: OK");
}
