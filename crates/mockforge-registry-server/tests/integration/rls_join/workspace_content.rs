//! RLS join-policy e2e: workspace_content. See ../rls_join_coverage_e2e.rs.
//!
//! Tables: workspace_folders, workspace_requests, workspace_request_history,
//! workspace_environments, workspace_env_variables (all scoped through
//! `workspaces.org_id`).
//!
//! Every cross-tenant probe uses the attacker's OWN workspace in the path with
//! the victim's child id (folder / request / environment), which is the case
//! only RLS + the handler's `workspace_id` check stop, plus the plain "victim's
//! workspace in the path" case.

use super::*;

fn env_entry<'a>(body: &'a Value, env_id: &str) -> Option<&'a Value> {
    body["environments"]
        .as_array()
        .and_then(|a| a.iter().find(|e| e["id"].as_str() == Some(env_id)))
}

fn env_ids(body: &Value) -> Vec<String> {
    ids("environments", &body["environments"])
}

fn variable_keys(body: &Value) -> Vec<String> {
    body["variables"]
        .as_array()
        .unwrap_or_else(|| panic!("variables: expected an array in {body}"))
        .iter()
        .filter_map(|v| v["key"].as_str().map(str::to_string))
        .collect()
}

/// workspace_folders + workspace_requests through folders/requests CRUD and
/// the workspace detail / list summaries (`cloud_workspaces.rs`).
#[tokio::test]
#[ignore]
async fn rls_join_workspace_folders_and_requests() {
    let a = setup_team_org("rlsjwfa").await;
    let b = setup_team_org("rlsjwfb").await;
    let ws_a = create_workspace(&a, "rls-join folders A").await;
    let ws_b = create_workspace(&b, "rls-join folders B").await;

    // Create: a folder, a request inside it, and a top-level request.
    let folder = ok(
        "create folder",
        a.post(&format!("/api/v1/workspaces/{ws_a}/folders"), json!({ "name": "Users API" }))
            .await,
    );
    let folder_id = str_field("create folder", &folder, "id");

    let sub = ok(
        "create subfolder",
        a.post(
            &format!("/api/v1/workspaces/{ws_a}/folders"),
            json!({ "name": "Nested", "parent_id": folder_id }),
        )
        .await,
    );
    let sub_id = str_field("create subfolder", &sub, "id");

    let nested = ok(
        "create nested request",
        a.post(
            &format!("/api/v1/workspaces/{ws_a}/requests"),
            json!({ "name": "List users", "method": "get", "path": "/users", "folder_id": folder_id }),
        )
        .await,
    );
    let nested_id = str_field("create nested request", &nested, "id");

    let top = ok(
        "create top-level request",
        a.post(
            &format!("/api/v1/workspaces/{ws_a}/requests"),
            json!({ "name": "Health", "method": "GET", "path": "/health", "status_code": 204 }),
        )
        .await,
    );
    let top_id = str_field("create top-level request", &top, "id");

    // Workspace detail lists the folders + top-level requests with counts.
    let detail = ok("get workspace", a.get(&format!("/api/v1/workspaces/{ws_a}")).await);
    let w = &detail["workspace"];
    assert_contains("detail folders", &ids("detail folders", &w["folders"]), &folder_id);
    assert_contains("detail folders", &ids("detail folders", &w["folders"]), &sub_id);
    assert_contains("detail requests", &ids("detail requests", &w["requests"]), &top_id);
    assert_absent("detail requests", &ids("detail requests", &w["requests"]), &nested_id);
    assert_eq!(w["summary"]["folder_count"], 2, "folder_count in {detail}");
    assert_eq!(w["summary"]["request_count"], 2, "request_count in {detail}");
    let folder_summary = w["folders"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"].as_str() == Some(folder_id.as_str()))
        .unwrap();
    assert_eq!(folder_summary["subfolder_count"], 1, "{folder_summary}");
    assert_eq!(folder_summary["request_count"], 1, "{folder_summary}");

    // Workspace list summaries carry the same counts.
    let list = ok("list workspaces", a.get("/api/v1/workspaces").await);
    let entry = list
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"].as_str() == Some(ws_a.as_str()))
        .unwrap_or_else(|| panic!("ws_a missing from {list}"));
    assert_eq!(entry["folder_count"], 2, "{entry}");
    assert_eq!(entry["request_count"], 2, "{entry}");

    // Folder detail.
    let fd = ok(
        "get folder",
        a.get(&format!("/api/v1/workspaces/{ws_a}/folders/{folder_id}")).await,
    );
    assert_eq!(fd["folder"]["summary"]["id"], folder_id.as_str(), "{fd}");
    assert_eq!(fd["folder"]["summary"]["request_count"], 1, "{fd}");
    assert_contains(
        "folder requests",
        &ids("folder requests", &fd["folder"]["requests"]),
        &nested_id,
    );

    // ── Cross-tenant: B must not see or mutate A's rows ──────────────────
    assert_absent("B workspace list", &list_ids(&b, "B list", "/api/v1/workspaces").await, &ws_a);
    client_err("B get A workspace", b.get(&format!("/api/v1/workspaces/{ws_a}")).await);
    client_err(
        "B get A folder via A ws",
        b.get(&format!("/api/v1/workspaces/{ws_a}/folders/{folder_id}")).await,
    );
    client_err(
        "B get A folder via B ws",
        b.get(&format!("/api/v1/workspaces/{ws_b}/folders/{folder_id}")).await,
    );
    client_err(
        "B create folder under A parent",
        b.post(
            &format!("/api/v1/workspaces/{ws_b}/folders"),
            json!({ "name": "sneaky", "parent_id": folder_id }),
        )
        .await,
    );
    client_err(
        "B create request into A folder",
        b.post(
            &format!("/api/v1/workspaces/{ws_b}/requests"),
            json!({ "name": "sneaky", "method": "GET", "path": "/x", "folder_id": folder_id }),
        )
        .await,
    );
    client_err(
        "B create request in A ws",
        b.post(
            &format!("/api/v1/workspaces/{ws_a}/requests"),
            json!({ "name": "sneaky", "method": "GET", "path": "/x" }),
        )
        .await,
    );
    client_err(
        "B delete A request via B ws",
        b.delete(&format!("/api/v1/workspaces/{ws_b}/requests/{top_id}")).await,
    );
    client_err(
        "B delete A request via A ws",
        b.delete(&format!("/api/v1/workspaces/{ws_a}/requests/{top_id}")).await,
    );
    client_err(
        "B delete A folder via B ws",
        b.delete(&format!("/api/v1/workspaces/{ws_b}/folders/{sub_id}")).await,
    );
    // B's own workspace detail stays empty.
    let bd = ok("B get own workspace", b.get(&format!("/api/v1/workspaces/{ws_b}")).await);
    assert_eq!(bd["workspace"]["summary"]["folder_count"], 0, "{bd}");
    assert_eq!(bd["workspace"]["summary"]["request_count"], 0, "{bd}");

    // A's data is unchanged.
    let detail = ok("get workspace after B", a.get(&format!("/api/v1/workspaces/{ws_a}")).await);
    assert_eq!(detail["workspace"]["summary"]["folder_count"], 2, "{detail}");
    assert_eq!(detail["workspace"]["summary"]["request_count"], 2, "{detail}");

    // ── Delete ───────────────────────────────────────────────────────────
    ok(
        "delete nested request",
        a.delete(&format!("/api/v1/workspaces/{ws_a}/requests/{nested_id}")).await,
    );
    ok(
        "delete top request",
        a.delete(&format!("/api/v1/workspaces/{ws_a}/requests/{top_id}")).await,
    );
    ok(
        "delete subfolder",
        a.delete(&format!("/api/v1/workspaces/{ws_a}/folders/{sub_id}")).await,
    );
    ok(
        "delete folder",
        a.delete(&format!("/api/v1/workspaces/{ws_a}/folders/{folder_id}")).await,
    );

    let detail =
        ok("get workspace after delete", a.get(&format!("/api/v1/workspaces/{ws_a}")).await);
    let w = &detail["workspace"];
    assert_absent("detail folders", &ids("detail folders", &w["folders"]), &folder_id);
    assert_absent("detail requests", &ids("detail requests", &w["requests"]), &top_id);
    assert_eq!(w["summary"]["folder_count"], 0, "{detail}");
    assert_eq!(w["summary"]["request_count"], 0, "{detail}");
    client_err(
        "get deleted folder",
        a.get(&format!("/api/v1/workspaces/{ws_a}/folders/{folder_id}")).await,
    );
}

/// workspace_environments + workspace_env_variables through the environment
/// CRUD, activate/reorder, variables, autocomplete and encryption
/// security-check endpoints.
#[tokio::test]
#[ignore]
async fn rls_join_workspace_environments_and_variables() {
    let a = setup_team_org("rlsjwea").await;
    let b = setup_team_org("rlsjweb").await;
    let ws_a = create_workspace(&a, "rls-join envs A").await;
    let ws_b = create_workspace(&b, "rls-join envs B").await;
    let envs_a = format!("/api/v1/workspaces/{ws_a}/environments");

    // Create two environments.
    let dev = ok("create dev", a.post(&envs_a, json!({ "name": "dev" })).await);
    let dev_id = str_field("create dev", &dev, "id");
    let stg = ok(
        "create staging",
        a.post(
            &envs_a,
            json!({ "name": "staging", "color": { "hex": "#ff0000", "name": "red" } }),
        )
        .await,
    );
    let stg_id = str_field("create staging", &stg, "id");

    // Duplicate name keeps its 4xx mapping (unique violation inside the bound tx).
    client_err("duplicate env name", a.post(&envs_a, json!({ "name": "dev" })).await);

    let list = ok("list envs", a.get(&envs_a).await);
    assert_contains("list envs", &env_ids(&list), &dev_id);
    assert_contains("list envs", &env_ids(&list), &stg_id);

    // Update.
    ok(
        "update dev",
        a.put(&format!("{envs_a}/{dev_id}"), json!({ "description": "dev (updated)" }))
            .await,
    );
    let list = ok("list after update", a.get(&envs_a).await);
    assert_eq!(env_entry(&list, &dev_id).unwrap()["description"], "dev (updated)", "{list}");

    // Reorder + activate.
    ok(
        "reorder",
        a.put(&format!("{envs_a}/order"), json!({ "environment_ids": [stg_id, dev_id] }))
            .await,
    );
    ok("activate dev", a.post(&format!("{envs_a}/{dev_id}/activate"), json!({})).await);
    let list = ok("list after activate", a.get(&envs_a).await);
    assert_eq!(env_entry(&list, &dev_id).unwrap()["active"], true, "{list}");
    assert_eq!(env_entry(&list, &stg_id).unwrap()["active"], false, "{list}");
    assert_eq!(env_entry(&list, &stg_id).unwrap()["order"], 0, "{list}");

    // Variables.
    let vars_dev = format!("{envs_a}/{dev_id}/variables");
    ok(
        "set API_URL",
        a.post(&vars_dev, json!({ "key": "API_URL", "value": "https://dev.example.test" }))
            .await,
    );
    ok(
        "set AUTH_TOKEN plaintext",
        a.post(&vars_dev, json!({ "key": "AUTH_TOKEN", "value": "ghp_notreal" })).await,
    );
    let vars = ok("list vars", a.get(&vars_dev).await);
    assert_contains("vars", &variable_keys(&vars), "API_URL");
    assert_contains("vars", &variable_keys(&vars), "AUTH_TOKEN");
    let list = ok("list envs w/ vars", a.get(&envs_a).await);
    assert_eq!(env_entry(&list, &dev_id).unwrap()["variable_count"], 2, "{list}");

    // Autocomplete reads the active env's variables.
    let input = "hello {{API_";
    let ac = ok(
        "autocomplete",
        a.post(
            &format!("/api/v1/workspaces/{ws_a}/autocomplete"),
            json!({ "input": input, "cursor_position": input.len() }),
        )
        .await,
    );
    assert!(
        ac["suggestions"].as_array().unwrap().iter().any(|s| s["text"] == "API_URL"),
        "expected API_URL in {ac}"
    );

    // Security check sees the plaintext token-looking variable.
    let sc = ok(
        "security check",
        a.post(&format!("/api/v1/workspaces/{ws_a}/encryption/security-check"), json!({}))
            .await,
    );
    let warnings = sc["warnings"].as_array().unwrap();
    assert!(
        warnings.iter().any(|w| w.as_str().unwrap_or("").contains("AUTH_TOKEN")),
        "expected an AUTH_TOKEN warning in {sc}"
    );

    // ── Cross-tenant ─────────────────────────────────────────────────────
    let envs_b = format!("/api/v1/workspaces/{ws_b}/environments");
    client_err("B list A envs", b.get(&envs_a).await);
    client_err("B create env in A ws", b.post(&envs_a, json!({ "name": "evil" })).await);
    client_err(
        "B update A env via B ws",
        b.put(&format!("{envs_b}/{dev_id}"), json!({ "description": "pwned" })).await,
    );
    client_err("B delete A env via B ws", b.delete(&format!("{envs_b}/{stg_id}")).await);
    client_err(
        "B activate A env via B ws",
        b.post(&format!("{envs_b}/{stg_id}/activate"), json!({})).await,
    );
    client_err("B list A vars via B ws", b.get(&format!("{envs_b}/{dev_id}/variables")).await);
    client_err(
        "B set var on A env via B ws",
        b.post(
            &format!("{envs_b}/{dev_id}/variables"),
            json!({ "key": "API_URL", "value": "evil" }),
        )
        .await,
    );
    client_err(
        "B delete A var via B ws",
        b.delete(&format!("{envs_b}/{dev_id}/variables/API_URL")).await,
    );
    client_err("B list A vars via A ws", b.get(&vars_dev).await);
    client_err(
        "B security check on A ws",
        b.post(&format!("/api/v1/workspaces/{ws_a}/encryption/security-check"), json!({}))
            .await,
    );
    client_err(
        "B autocomplete on A ws",
        b.post(
            &format!("/api/v1/workspaces/{ws_a}/autocomplete"),
            json!({ "input": input, "cursor_position": input.len() }),
        )
        .await,
    );
    // B's own workspace sees no environments.
    let b_list = ok("B list own envs", b.get(&envs_b).await);
    assert!(env_ids(&b_list).is_empty(), "{b_list}");

    // A unchanged.
    let list = ok("A list after B", a.get(&envs_a).await);
    assert_eq!(env_entry(&list, &dev_id).unwrap()["description"], "dev (updated)", "{list}");
    assert_eq!(env_entry(&list, &dev_id).unwrap()["active"], true, "{list}");
    assert_contains("A envs after B", &env_ids(&list), &stg_id);
    let vars = ok("A vars after B", a.get(&vars_dev).await);
    let api_url = vars["variables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["key"] == "API_URL")
        .unwrap();
    assert_eq!(api_url["value"], "https://dev.example.test", "{vars}");

    // ── Delete ───────────────────────────────────────────────────────────
    ok("delete var", a.delete(&format!("{vars_dev}/AUTH_TOKEN")).await);
    client_err("delete missing var", a.delete(&format!("{vars_dev}/AUTH_TOKEN")).await);
    let vars = ok("vars after delete", a.get(&vars_dev).await);
    assert_absent("vars after delete", &variable_keys(&vars), "AUTH_TOKEN");

    ok("delete staging", a.delete(&format!("{envs_a}/{stg_id}")).await);
    let list = ok("list after delete", a.get(&envs_a).await);
    assert_absent("list after delete", &env_ids(&list), &stg_id);
    assert_contains("list after delete", &env_ids(&list), &dev_id);
}

/// workspace_requests via import, plus workspace_request_history via
/// execute/history.
#[tokio::test]
#[ignore]
async fn rls_join_workspace_import_execute_history() {
    let a = setup_team_org("rlsjwia").await;
    let b = setup_team_org("rlsjwib").await;
    let ws_a = create_workspace(&a, "rls-join import A").await;
    let ws_b = create_workspace(&b, "rls-join import B").await;

    // Import two Postman routes with one folder per method.
    let postman = json!({
        "info": { "name": "tiny", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json" },
        "item": [
            { "name": "Ping", "request": { "method": "GET", "url": { "raw": "{{base}}/ping", "host": ["{{base}}"], "path": ["ping"] } } },
            { "name": "Echo", "request": { "method": "POST", "url": { "raw": "{{base}}/echo", "host": ["{{base}}"], "path": ["echo"] } } }
        ]
    })
    .to_string();
    let imp = ok(
        "import",
        a.post(
            &format!("/api/v1/workspaces/{ws_a}/import"),
            json!({ "format": "postman", "data": postman, "create_folders": true }),
        )
        .await,
    );
    assert_eq!(imp["imported"], 2, "{imp}");
    let detail = ok("get workspace", a.get(&format!("/api/v1/workspaces/{ws_a}")).await);
    assert_eq!(detail["workspace"]["summary"]["folder_count"], 2, "{detail}");
    assert_eq!(detail["workspace"]["summary"]["request_count"], 2, "{detail}");

    // Import into an existing folder.
    let folder = ok(
        "create folder",
        a.post(&format!("/api/v1/workspaces/{ws_a}/folders"), json!({ "name": "Target" }))
            .await,
    );
    let folder_id = str_field("create folder", &folder, "id");
    let imp = ok(
        "import into folder",
        a.post(
            &format!("/api/v1/workspaces/{ws_a}/import"),
            json!({ "format": "postman", "data": postman, "folder_id": folder_id, "selected_routes": [0] }),
        )
        .await,
    );
    assert_eq!(imp["imported"], 1, "{imp}");
    let fd = ok(
        "get folder",
        a.get(&format!("/api/v1/workspaces/{ws_a}/folders/{folder_id}")).await,
    );
    assert_eq!(fd["folder"]["summary"]["request_count"], 1, "{fd}");

    // Execute a request with a variable from the active environment.
    let env = ok(
        "create env",
        a.post(&format!("/api/v1/workspaces/{ws_a}/environments"), json!({ "name": "dev" }))
            .await,
    );
    let env_id = str_field("create env", &env, "id");
    ok(
        "activate env",
        a.post(&format!("/api/v1/workspaces/{ws_a}/environments/{env_id}/activate"), json!({}))
            .await,
    );
    ok(
        "set greeting",
        a.post(
            &format!("/api/v1/workspaces/{ws_a}/environments/{env_id}/variables"),
            json!({ "key": "greeting", "value": "Hello" }),
        )
        .await,
    );
    let req = ok(
        "create request",
        a.post(
            &format!("/api/v1/workspaces/{ws_a}/requests"),
            json!({
                "name": "Greet",
                "method": "GET",
                "path": "/greet/{{name}}",
                "response_body": "{{greeting}}, {{name}}!"
            }),
        )
        .await,
    );
    let req_id = str_field("create request", &req, "id");
    let history_path = format!("/api/v1/workspaces/{ws_a}/requests/{req_id}/history");

    let h = ok("empty history", a.get(&history_path).await);
    assert_eq!(h["total"], 0, "{h}");

    let exec = ok(
        "execute",
        a.post(
            &format!("/api/v1/workspaces/{ws_a}/requests/{req_id}/execute"),
            json!({ "variables": { "name": "Alex" } }),
        )
        .await,
    );
    assert_eq!(exec["response_body"], "Hello, Alex!", "{exec}");
    assert_eq!(exec["request_path"], "/greet/Alex", "{exec}");

    let h = ok("history", a.get(&history_path).await);
    assert_eq!(h["total"], 1, "{h}");
    assert_eq!(h["history"][0]["response_body"], "Hello, Alex!", "{h}");

    // ── Cross-tenant ─────────────────────────────────────────────────────
    client_err(
        "B import into A ws",
        b.post(
            &format!("/api/v1/workspaces/{ws_a}/import"),
            json!({ "format": "postman", "data": postman }),
        )
        .await,
    );
    client_err(
        "B import into A folder via B ws",
        b.post(
            &format!("/api/v1/workspaces/{ws_b}/import"),
            json!({ "format": "postman", "data": postman, "folder_id": folder_id }),
        )
        .await,
    );
    client_err(
        "B execute A request via B ws",
        b.post(&format!("/api/v1/workspaces/{ws_b}/requests/{req_id}/execute"), json!({}))
            .await,
    );
    client_err(
        "B execute A request via A ws",
        b.post(&format!("/api/v1/workspaces/{ws_a}/requests/{req_id}/execute"), json!({}))
            .await,
    );
    client_err(
        "B history via B ws",
        b.get(&format!("/api/v1/workspaces/{ws_b}/requests/{req_id}/history")).await,
    );
    client_err("B history via A ws", b.get(&history_path).await);
    let bd = ok("B own workspace", b.get(&format!("/api/v1/workspaces/{ws_b}")).await);
    assert_eq!(bd["workspace"]["summary"]["request_count"], 0, "{bd}");
    assert_eq!(bd["workspace"]["summary"]["folder_count"], 0, "{bd}");

    // A unchanged: still exactly one history row, still 4 requests.
    let h = ok("history after B", a.get(&history_path).await);
    assert_eq!(h["total"], 1, "{h}");
    let detail = ok("A workspace after B", a.get(&format!("/api/v1/workspaces/{ws_a}")).await);
    assert_eq!(detail["workspace"]["summary"]["request_count"], 4, "{detail}");

    // Delete the executed request; its history is gone with it.
    ok(
        "delete request",
        a.delete(&format!("/api/v1/workspaces/{ws_a}/requests/{req_id}")).await,
    );
    client_err("history of deleted request", a.get(&history_path).await);
    let detail = ok("A workspace after delete", a.get(&format!("/api/v1/workspaces/{ws_a}")).await);
    assert_absent(
        "top-level requests",
        &ids("top-level requests", &detail["workspace"]["requests"]),
        &req_id,
    );
}
