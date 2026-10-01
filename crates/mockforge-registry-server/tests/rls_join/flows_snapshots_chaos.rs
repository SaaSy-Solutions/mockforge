//! RLS join-policy e2e: flows_snapshots_chaos. See ../rls_join_coverage_e2e.rs.
//!
//! Tables: flows, flow_versions, snapshots, chaos_campaigns,
//! chaos_campaign_reports, resilience_patterns, mock_environments (via
//! snapshot restore), capture_sessions, capture_session_members,
//! virtual_entities, cloud_mockai_rule_explanations (all scoped through
//! `workspaces.org_id`).
//!
//! Not covered here: `POST .../mockai/learn` (needs a live LLM provider),
//! `snapshot_schedules` (no HTTP surface), chaos reports with rows (written
//! only by the runner's internal callback), and restoring mock_environments
//! (the manifest only carries environments that already exist).

use super::*;

fn flow_version_numbers(body: &Value) -> Vec<i64> {
    arr("flow versions", body)
        .iter()
        .filter_map(|v| v["version_number"].as_i64())
        .collect()
}

/// flows + flow_versions + the workspace graph (`flows.rs`, `graph.rs`).
#[tokio::test]
#[ignore]
async fn rls_join_flows_and_versions() {
    let a = setup_team_org("rlsjfla").await;
    let b = setup_team_org("rlsjflb").await;
    let ws_a = create_workspace(&a, "rls-join flows A").await;
    let ws_b = create_workspace(&b, "rls-join flows B").await;
    let flows_a = format!("/api/v1/workspaces/{ws_a}/flows");

    // Create (flow + initial version in one Acquire'd transaction).
    let created = ok(
        "create flow",
        a.post(
            &flows_a,
            json!({ "kind": "scenario", "name": "Checkout", "config": { "steps": [1] } }),
        )
        .await,
    );
    let flow_id = str_field("create flow", &created, "id");
    let v1_id = str_field("create flow version", &created["version"], "id");
    assert_eq!(created["version"]["version_number"], 1, "initial version: {created}");

    // List (with and without the kind filter).
    assert_contains("list flows", &list_ids(&a, "list flows", &flows_a).await, &flow_id);
    assert_contains(
        "list flows ?kind=scenario",
        &list_ids(&a, "list flows kind", &format!("{flows_a}?kind=scenario")).await,
        &flow_id,
    );

    // Get: returns the flow plus its current version.
    let got = ok("get flow", a.get(&format!("/api/v1/flows/{flow_id}")).await);
    assert_eq!(str_field("get flow", &got, "name"), "Checkout");
    assert_eq!(str_field("get flow version", &got["version"], "id"), v1_id);

    // Update (rename).
    let renamed = ok(
        "rename flow",
        a.patch(&format!("/api/v1/flows/{flow_id}"), json!({ "name": "Checkout v2" }))
            .await,
    );
    assert_eq!(str_field("rename flow", &renamed, "name"), "Checkout v2");

    // Save a new version, list history, fetch an old version by id.
    let v2 = ok(
        "save flow version",
        a.post(
            &format!("/api/v1/flows/{flow_id}/versions"),
            json!({ "config": { "steps": [1, 2] } }),
        )
        .await,
    );
    assert_eq!(v2["version_number"], 2, "second version: {v2}");
    let versions = ok("list versions", a.get(&format!("/api/v1/flows/{flow_id}/versions")).await);
    assert_eq!(flow_version_numbers(&versions), vec![2, 1]);
    let old = ok("get flow version", a.get(&format!("/api/v1/flow-versions/{v1_id}")).await);
    assert_eq!(old["config"], json!({ "steps": [1] }));

    // Trigger a run: reads the current flow_version bound to the org.
    let run = ok(
        "trigger flow run",
        a.post(&format!("/api/v1/flows/{flow_id}/runs"), json!({})).await,
    );
    assert_eq!(str_field("trigger flow run", &run, "suite_id"), flow_id);

    // Graph lists the flow as a node.
    let graph = ok("graph", a.get(&format!("/api/v1/workspaces/{ws_a}/graph")).await);
    let node_ids: Vec<String> = arr("graph nodes", &graph["nodes"])
        .iter()
        .filter_map(|n| n["id"].as_str().map(str::to_string))
        .collect();
    assert_contains("graph nodes", &node_ids, &format!("flow:{flow_id}"));

    // ── Cross-tenant: org B must not see or mutate A's flow. ──
    assert_absent(
        "B's own flow list",
        &list_ids(&b, "B list flows", &format!("/api/v1/workspaces/{ws_b}/flows")).await,
        &flow_id,
    );
    client_err("B lists A's workspace flows", b.get(&flows_a).await);
    client_err(
        "B creates a flow in A's workspace",
        b.post(&flows_a, json!({ "kind": "chain", "name": "x", "config": {} })).await,
    );
    client_err("B gets A's flow", b.get(&format!("/api/v1/flows/{flow_id}")).await);
    client_err(
        "B renames A's flow",
        b.patch(&format!("/api/v1/flows/{flow_id}"), json!({ "name": "pwned" })).await,
    );
    client_err(
        "B saves a version on A's flow",
        b.post(&format!("/api/v1/flows/{flow_id}/versions"), json!({ "config": {} }))
            .await,
    );
    client_err(
        "B lists A's flow versions",
        b.get(&format!("/api/v1/flows/{flow_id}/versions")).await,
    );
    client_err(
        "B gets A's flow version",
        b.get(&format!("/api/v1/flow-versions/{v1_id}")).await,
    );
    client_err(
        "B triggers A's flow",
        b.post(&format!("/api/v1/flows/{flow_id}/runs"), json!({})).await,
    );
    client_err("B reads A's graph", b.get(&format!("/api/v1/workspaces/{ws_a}/graph")).await);
    client_err("B deletes A's flow", b.delete(&format!("/api/v1/flows/{flow_id}")).await);

    // A's data is unchanged.
    let got = ok("get flow after B", a.get(&format!("/api/v1/flows/{flow_id}")).await);
    assert_eq!(str_field("get flow after B", &got, "name"), "Checkout v2");
    let versions = ok(
        "list versions after B",
        a.get(&format!("/api/v1/flows/{flow_id}/versions")).await,
    );
    assert_eq!(flow_version_numbers(&versions), vec![2, 1]);

    // Delete, then it is gone.
    ok("delete flow", a.delete(&format!("/api/v1/flows/{flow_id}")).await);
    assert_absent("list flows after delete", &list_ids(&a, "list flows", &flows_a).await, &flow_id);
    client_err("get deleted flow", a.get(&format!("/api/v1/flows/{flow_id}")).await);
}

/// snapshots (+ the flows / chaos_campaigns reads in the manifest build and
/// the chaos_campaigns writes in restore) via `snapshots.rs`.
#[tokio::test]
#[ignore]
async fn rls_join_snapshots() {
    let a = setup_team_org("rlsjsna").await;
    let b = setup_team_org("rlsjsnb").await;
    let ws_a = create_workspace(&a, "rls-join snapshots A").await;
    let ws_b = create_workspace(&b, "rls-join snapshots B").await;
    let snaps_a = format!("/api/v1/workspaces/{ws_a}/snapshots");
    let camps_a = format!("/api/v1/workspaces/{ws_a}/chaos-campaigns");

    // Seed a flow and a chaos campaign so the manifest has join-scoped rows.
    ok(
        "seed flow",
        a.post(
            &format!("/api/v1/workspaces/{ws_a}/flows"),
            json!({ "kind": "chain", "name": "Seed", "config": {} }),
        )
        .await,
    );
    let camp = ok(
        "seed campaign",
        a.post(
            &camps_a,
            json!({
                "name": "snap-camp",
                "target_kind": "external",
                "target_ref": "https://example.invalid",
                "config": {},
                "safety_config": {}
            }),
        )
        .await,
    );
    let camp_id = str_field("seed campaign", &camp, "id");

    // Capture.
    let snap = ok("capture snapshot", a.post(&snaps_a, json!({ "name": "before" })).await);
    let snap_id = str_field("capture snapshot", &snap, "id");
    assert_eq!(str_field("capture snapshot", &snap, "status"), "ready", "{snap}");
    let counts = &snap["manifest"]["counts"];
    assert_eq!(counts["flows"], 1, "manifest flows (bound read): {snap}");
    assert_eq!(counts["chaos_campaigns"], 1, "manifest chaos (bound read): {snap}");
    assert_eq!(snap["manifest"]["partial"], false, "manifest partial: {snap}");

    // List + get + diff.
    assert_contains("list snapshots", &list_ids(&a, "list snapshots", &snaps_a).await, &snap_id);
    let got = ok("get snapshot", a.get(&format!("/api/v1/snapshots/{snap_id}")).await);
    assert_eq!(str_field("get snapshot", &got, "name"), "before");
    ok("diff snapshot", a.get(&format!("/api/v1/snapshots/{snap_id}/diff")).await);

    // Restore after deleting the campaign recreates it (chaos_campaigns write).
    ok("delete campaign", a.delete(&format!("/api/v1/chaos-campaigns/{camp_id}")).await);
    let restored = ok(
        "restore snapshot",
        a.post(&format!("/api/v1/snapshots/{snap_id}/restore"), json!({})).await,
    );
    assert_eq!(restored["chaos_campaigns"]["created"], 1, "restore: {restored}");
    assert_eq!(restored["errors"], json!([]), "restore errors: {restored}");
    let camps = ok("list campaigns after restore", a.get(&camps_a).await);
    assert!(
        arr("campaigns", &camps).iter().any(|c| c["name"] == "snap-camp"),
        "restored campaign missing: {camps}"
    );

    // ── Cross-tenant ──
    assert_absent(
        "B's own snapshot list",
        &list_ids(&b, "B list snapshots", &format!("/api/v1/workspaces/{ws_b}/snapshots")).await,
        &snap_id,
    );
    client_err("B lists A's snapshots", b.get(&snaps_a).await);
    client_err("B captures in A's workspace", b.post(&snaps_a, json!({})).await);
    client_err("B gets A's snapshot", b.get(&format!("/api/v1/snapshots/{snap_id}")).await);
    client_err(
        "B diffs A's snapshot",
        b.get(&format!("/api/v1/snapshots/{snap_id}/diff")).await,
    );
    client_err(
        "B restores A's snapshot",
        b.post(&format!("/api/v1/snapshots/{snap_id}/restore"), json!({})).await,
    );
    client_err(
        "B deletes A's snapshot",
        b.delete(&format!("/api/v1/snapshots/{snap_id}")).await,
    );
    ok("A still sees snapshot", a.get(&format!("/api/v1/snapshots/{snap_id}")).await);

    // Delete, then it is gone.
    ok("delete snapshot", a.delete(&format!("/api/v1/snapshots/{snap_id}")).await);
    assert_absent(
        "list snapshots after delete",
        &list_ids(&a, "list snapshots", &snaps_a).await,
        &snap_id,
    );
    client_err("get deleted snapshot", a.get(&format!("/api/v1/snapshots/{snap_id}")).await);
}

/// chaos_campaigns, chaos_campaign_reports, resilience_patterns (`chaos.rs`).
#[tokio::test]
#[ignore]
async fn rls_join_chaos_campaigns() {
    let a = setup_team_org("rlsjcha").await;
    let b = setup_team_org("rlsjchb").await;
    let ws_a = create_workspace(&a, "rls-join chaos A").await;
    let ws_b = create_workspace(&b, "rls-join chaos B").await;
    let camps_a = format!("/api/v1/workspaces/{ws_a}/chaos-campaigns");

    let camp = ok(
        "create campaign",
        a.post(
            &camps_a,
            json!({
                "name": "latency-storm",
                "description": "rls join",
                "target_kind": "external",
                "target_ref": "https://example.invalid",
                "config": { "faults": [] },
                "safety_config": { "max_duration_secs": 30 }
            }),
        )
        .await,
    );
    let camp_id = str_field("create campaign", &camp, "id");

    assert_contains("list campaigns", &list_ids(&a, "list campaigns", &camps_a).await, &camp_id);
    let got = ok("get campaign", a.get(&format!("/api/v1/chaos-campaigns/{camp_id}")).await);
    assert_eq!(str_field("get campaign", &got, "name"), "latency-storm");
    let reports = ok(
        "list reports",
        a.get(&format!("/api/v1/chaos-campaigns/{camp_id}/reports")).await,
    );
    arr("reports", &reports);
    let patterns = ok(
        "list resilience patterns",
        a.get(&format!("/api/v1/workspaces/{ws_a}/resilience-patterns")).await,
    );
    arr("patterns", &patterns);

    // ── Cross-tenant ──
    assert_absent(
        "B's own campaign list",
        &list_ids(&b, "B list campaigns", &format!("/api/v1/workspaces/{ws_b}/chaos-campaigns"))
            .await,
        &camp_id,
    );
    client_err("B lists A's campaigns", b.get(&camps_a).await);
    client_err(
        "B creates a campaign in A's workspace",
        b.post(
            &camps_a,
            json!({
                "name": "x", "target_kind": "external", "target_ref": "https://x.invalid",
                "config": {}, "safety_config": {}
            }),
        )
        .await,
    );
    client_err(
        "B gets A's campaign",
        b.get(&format!("/api/v1/chaos-campaigns/{camp_id}")).await,
    );
    client_err(
        "B lists A's reports",
        b.get(&format!("/api/v1/chaos-campaigns/{camp_id}/reports")).await,
    );
    client_err(
        "B lists A's resilience patterns",
        b.get(&format!("/api/v1/workspaces/{ws_a}/resilience-patterns")).await,
    );
    client_err(
        "B triggers A's campaign",
        b.post(&format!("/api/v1/chaos-campaigns/{camp_id}/runs"), json!({})).await,
    );
    client_err(
        "B deletes A's campaign",
        b.delete(&format!("/api/v1/chaos-campaigns/{camp_id}")).await,
    );
    ok(
        "A still sees campaign",
        a.get(&format!("/api/v1/chaos-campaigns/{camp_id}")).await,
    );

    ok("delete campaign", a.delete(&format!("/api/v1/chaos-campaigns/{camp_id}")).await);
    assert_absent(
        "list campaigns after delete",
        &list_ids(&a, "list campaigns", &camps_a).await,
        &camp_id,
    );
    client_err(
        "get deleted campaign",
        a.get(&format!("/api/v1/chaos-campaigns/{camp_id}")).await,
    );
}

/// capture_sessions + capture_session_members (`captures.rs`).
#[tokio::test]
#[ignore]
async fn rls_join_capture_sessions() {
    let a = setup_team_org("rlsjcsa").await;
    let b = setup_team_org("rlsjcsb").await;
    let ws_a = create_workspace(&a, "rls-join captures A").await;
    let ws_b = create_workspace(&b, "rls-join captures B").await;
    let sessions_a = format!("/api/v1/workspaces/{ws_a}/capture-sessions");

    let session = ok(
        "create session",
        a.post(&sessions_a, json!({ "name": "checkout traffic", "description": "rls" }))
            .await,
    );
    let session_id = str_field("create session", &session, "id");
    assert_contains(
        "list sessions",
        &list_ids(&a, "list sessions", &sessions_a).await,
        &session_id,
    );

    let capture_id = uuid::Uuid::new_v4().to_string();
    let members = format!("/api/v1/capture-sessions/{session_id}/members");
    let added = ok(
        "add member",
        a.patch(&members, json!({ "op": "add", "capture_id": capture_id })).await,
    );
    assert_eq!(added["changed"], true, "add member: {added}");
    let session_count = |body: &Value| {
        arr("sessions", body)
            .iter()
            .find(|s| s["id"].as_str() == Some(session_id.as_str()))
            .map(|s| s["capture_count"].clone())
    };
    let listed = ok("list sessions after add", a.get(&sessions_a).await);
    assert_eq!(session_count(&listed), Some(json!(1)), "capture_count: {listed}");

    // ── Cross-tenant ──
    assert_absent(
        "B's own session list",
        &list_ids(&b, "B list sessions", &format!("/api/v1/workspaces/{ws_b}/capture-sessions"))
            .await,
        &session_id,
    );
    client_err("B lists A's sessions", b.get(&sessions_a).await);
    client_err(
        "B creates a session in A's workspace",
        b.post(&sessions_a, json!({ "name": "x" })).await,
    );
    client_err(
        "B adds a member to A's session",
        b.patch(&members, json!({ "op": "add", "capture_id": uuid::Uuid::new_v4().to_string() }))
            .await,
    );
    client_err(
        "B removes A's member",
        b.patch(&members, json!({ "op": "remove", "capture_id": capture_id })).await,
    );
    client_err(
        "B deletes A's session",
        b.delete(&format!("/api/v1/capture-sessions/{session_id}")).await,
    );
    let listed = ok("list sessions after B", a.get(&sessions_a).await);
    assert_eq!(session_count(&listed), Some(json!(1)), "capture_count after B: {listed}");

    // Remove the member, then delete the session.
    let removed = ok(
        "remove member",
        a.patch(&members, json!({ "op": "remove", "capture_id": capture_id })).await,
    );
    assert_eq!(removed["changed"], true, "remove member: {removed}");
    let listed = ok("list sessions after remove", a.get(&sessions_a).await);
    assert_eq!(session_count(&listed), Some(json!(0)), "capture_count after remove: {listed}");

    ok(
        "delete session",
        a.delete(&format!("/api/v1/capture-sessions/{session_id}")).await,
    );
    assert_absent(
        "list sessions after delete",
        &list_ids(&a, "list sessions", &sessions_a).await,
        &session_id,
    );
}

/// virtual_entities (`consistency.rs`). There is no update/delete endpoint;
/// re-applying the preset is the idempotent update path.
#[tokio::test]
#[ignore]
async fn rls_join_virtual_entities() {
    let a = setup_team_org("rlsjvea").await;
    let b = setup_team_org("rlsjveb").await;
    let ws_a = create_workspace(&a, "rls-join entities A").await;
    let ws_b = create_workspace(&b, "rls-join entities B").await;
    let apply_a = format!("/api/v1/workspaces/{ws_a}/consistency/lifecycle-presets/apply");
    let entities_a = format!("/api/v1/workspaces/{ws_a}/consistency/entities");

    let ent = ok(
        "apply preset",
        a.post(&apply_a, json!({ "preset": "subscription", "persona_id": "persona-1" }))
            .await,
    );
    let ent_id = str_field("apply preset", &ent, "id");
    assert_eq!(str_field("apply preset", &ent, "current_state"), "new");

    assert_contains("list entities", &list_ids(&a, "list entities", &entities_a).await, &ent_id);
    assert_contains(
        "list entities filtered",
        &list_ids(&a, "list entities", &format!("{entities_a}?entity_type=subscription")).await,
        &ent_id,
    );
    let got = ok("get entity", a.get(&format!("/api/v1/consistency/entities/{ent_id}")).await);
    assert_eq!(str_field("get entity", &got, "persona_id"), "persona-1");

    // Re-apply: ON CONFLICT DO UPDATE keeps the same row.
    let again = ok(
        "re-apply preset",
        a.post(&apply_a, json!({ "preset": "subscription", "persona_id": "persona-1" }))
            .await,
    );
    assert_eq!(str_field("re-apply preset", &again, "id"), ent_id);

    // ── Cross-tenant ──
    assert_absent(
        "B's own entity list",
        &list_ids(
            &b,
            "B list entities",
            &format!("/api/v1/workspaces/{ws_b}/consistency/entities"),
        )
        .await,
        &ent_id,
    );
    client_err("B lists A's entities", b.get(&entities_a).await);
    client_err(
        "B gets A's entity",
        b.get(&format!("/api/v1/consistency/entities/{ent_id}")).await,
    );
    client_err(
        "B applies a preset in A's workspace",
        b.post(&apply_a, json!({ "preset": "loan", "persona_id": "persona-1" })).await,
    );
    let got = ok(
        "get entity after B",
        a.get(&format!("/api/v1/consistency/entities/{ent_id}")).await,
    );
    assert_eq!(str_field("get entity after B", &got, "entity_type"), "subscription");
    assert_eq!(list_ids(&a, "list entities after B", &entities_a).await, vec![ent_id]);
}

/// cloud_mockai_rule_explanations read paths (`mockai.rs`). Rows are only
/// written by `/mockai/learn`, which needs a live LLM provider, so this checks
/// the bound reads return 2xx (not a fail-closed 5xx) and that B is refused.
#[tokio::test]
#[ignore]
async fn rls_join_mockai_rule_explanations() {
    let a = setup_team_org("rlsjmaa").await;
    let b = setup_team_org("rlsjmab").await;
    let ws_a = create_workspace(&a, "rls-join mockai A").await;
    let list_a = format!("/api/v1/workspaces/{ws_a}/mockai/rule-explanations");

    let listed = ok("list rule explanations", a.get(&list_a).await);
    assert_eq!(listed["total"], 0, "fresh workspace: {listed}");
    arr("explanations", &listed["explanations"]);
    client_err("get missing rule", a.get(&format!("{list_a}/no-such-rule")).await);

    client_err("B lists A's rule explanations", b.get(&list_a).await);
    client_err("B gets A's rule", b.get(&format!("{list_a}/no-such-rule")).await);
    client_err(
        "B learns into A's workspace",
        b.post(
            &format!("/api/v1/workspaces/{ws_a}/mockai/learn"),
            json!({ "examples": [{ "request": {}, "response": {} }] }),
        )
        .await,
    );
}
