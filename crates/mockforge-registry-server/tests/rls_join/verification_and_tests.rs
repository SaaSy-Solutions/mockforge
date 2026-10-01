//! RLS join-policy e2e: verification_and_tests. See ../rls_join_coverage_e2e.rs.
//!
//! Tables: monitored_services, contract_diff_runs, contract_diff_findings,
//! fitness_functions, verification_suites, test_suites, test_schedules,
//! test_runs (via trigger), scenario_promotions, scenario_environment_versions,
//! incident_events.
//!
//! Not covered here: rows in contract_diff_runs / contract_diff_findings /
//! fitness_evaluations / test_run_events / test_run_artifacts are only written
//! by the runner through internal-token endpoints, so the user-facing read
//! paths are exercised for an empty result (own service) and for 4xx on an
//! unknown or cross-tenant id.

use super::*;

use base64::Engine;
use sha2::Digest;

/// Monitored services + diff-run read paths + manual diff trigger.
#[tokio::test]
#[ignore]
async fn monitored_services_round_trip_and_cross_tenant() {
    let owner = setup_team_org("rlsjms").await;
    let other = setup_team_org("rlsjmsx").await;
    let ws = create_workspace(&owner, &format!("ms-{}", unique())).await;
    let other_ws = create_workspace(&other, &format!("ms-x-{}", unique())).await;
    let list_path = format!("/api/v1/workspaces/{ws}/monitored-services");

    let svc = ok(
        "create monitored service",
        owner
            .post(
                &list_path,
                json!({
                    "name": "orders-api",
                    "base_url": "https://example.com",
                    "traffic_source": "probe",
                }),
            )
            .await,
    );
    let svc_id = str_field("create monitored service", &svc, "id");
    assert_contains(
        "list monitored services",
        &list_ids(&owner, "list monitored services", &list_path).await,
        &svc_id,
    );

    // Diff runs for the service: empty (runner-written), but must be a 2xx
    // list, not a fail-closed 4xx/5xx.
    let runs = ok(
        "list diff runs",
        owner.get(&format!("/api/v1/monitored-services/{svc_id}/diffs")).await,
    );
    arr("list diff runs", &runs);

    // Manual trigger enqueues a test_runs row bound to the owner's org.
    let run = ok(
        "trigger diff run",
        owner
            .post(&format!("/api/v1/monitored-services/{svc_id}/diff"), json!({}))
            .await,
    );
    str_field("trigger diff run", &run, "id");

    // Unknown diff run ids read as 4xx (no 5xx from an unbound join).
    let missing = uuid::Uuid::new_v4();
    client_err(
        "get unknown diff run",
        owner.get(&format!("/api/v1/contract-diff-runs/{missing}")).await,
    );
    client_err(
        "unknown diff run findings",
        owner.get(&format!("/api/v1/contract-diff-runs/{missing}/findings")).await,
    );

    // Cross-tenant: other org cannot list, read diffs, trigger, or delete.
    client_err("x-tenant list services", other.get(&list_path).await);
    client_err(
        "x-tenant create service in owner ws",
        other
            .post(
                &list_path,
                json!({ "name": "evil", "base_url": "https://example.com", "traffic_source": "probe" }),
            )
            .await,
    );
    client_err(
        "x-tenant list diffs",
        other.get(&format!("/api/v1/monitored-services/{svc_id}/diffs")).await,
    );
    client_err(
        "x-tenant trigger diff",
        other
            .post(&format!("/api/v1/monitored-services/{svc_id}/diff"), json!({}))
            .await,
    );
    client_err(
        "x-tenant delete service",
        other.delete(&format!("/api/v1/monitored-services/{svc_id}")).await,
    );
    assert_absent(
        "x-tenant own list",
        &list_ids(
            &other,
            "x-tenant own list",
            &format!("/api/v1/workspaces/{other_ws}/monitored-services"),
        )
        .await,
        &svc_id,
    );
    assert_contains(
        "owner list after x-tenant attempts",
        &list_ids(&owner, "owner list after x-tenant", &list_path).await,
        &svc_id,
    );

    ok(
        "delete monitored service",
        owner.delete(&format!("/api/v1/monitored-services/{svc_id}")).await,
    );
    assert_absent(
        "list after delete",
        &list_ids(&owner, "list after delete", &list_path).await,
        &svc_id,
    );
    client_err(
        "delete again",
        owner.delete(&format!("/api/v1/monitored-services/{svc_id}")).await,
    );
}

/// Fitness functions + verification suites.
#[tokio::test]
#[ignore]
async fn fitness_functions_and_verification_suites_round_trip_and_cross_tenant() {
    let owner = setup_team_org("rlsjff").await;
    let other = setup_team_org("rlsjffx").await;
    let ws = create_workspace(&owner, &format!("ff-{}", unique())).await;
    let ff_list = format!("/api/v1/workspaces/{ws}/fitness-functions");
    let vs_list = format!("/api/v1/workspaces/{ws}/verification-suites");

    let ff = ok(
        "create fitness function",
        owner
            .post(
                &ff_list,
                json!({ "name": "p95", "kind": "latency_threshold", "config": { "p95_ms": 250 } }),
            )
            .await,
    );
    let ff_id = str_field("create fitness function", &ff, "id");
    assert_contains("list fitness functions", &list_ids(&owner, "list ff", &ff_list).await, &ff_id);

    let updated = ok(
        "update fitness function",
        owner
            .patch(
                &format!("/api/v1/fitness-functions/{ff_id}"),
                json!({ "name": "p99", "kind": "latency_threshold", "config": { "p99_ms": 500 } }),
            )
            .await,
    );
    assert_eq!(str_field("update ff", &updated, "name"), "p99");

    let vs = ok(
        "create verification suite",
        owner
            .post(&vs_list, json!({ "name": "release-gate", "fitness_function_ids": [ff_id] }))
            .await,
    );
    let vs_id = str_field("create verification suite", &vs, "id");
    assert_contains(
        "list verification suites",
        &list_ids(&owner, "list vs", &vs_list).await,
        &vs_id,
    );

    // Cross-tenant.
    client_err("x-tenant list ff", other.get(&ff_list).await);
    client_err("x-tenant list vs", other.get(&vs_list).await);
    client_err(
        "x-tenant update ff",
        other
            .patch(
                &format!("/api/v1/fitness-functions/{ff_id}"),
                json!({ "name": "pwned", "kind": "error_rate", "config": {} }),
            )
            .await,
    );
    client_err(
        "x-tenant delete ff",
        other.delete(&format!("/api/v1/fitness-functions/{ff_id}")).await,
    );
    client_err(
        "x-tenant delete vs",
        other.delete(&format!("/api/v1/verification-suites/{vs_id}")).await,
    );
    let ff_rows = ok("owner ff after x-tenant", owner.get(&ff_list).await);
    let row = arr("owner ff after x-tenant", &ff_rows)
        .iter()
        .find(|r| r["id"].as_str() == Some(ff_id.as_str()))
        .expect("fitness function still present");
    assert_eq!(row["name"], json!("p99"), "x-tenant PATCH must not mutate the row");
    assert_contains("owner vs after x-tenant", &list_ids(&owner, "vs", &vs_list).await, &vs_id);

    ok("delete vs", owner.delete(&format!("/api/v1/verification-suites/{vs_id}")).await);
    assert_absent("vs after delete", &list_ids(&owner, "vs", &vs_list).await, &vs_id);
    ok("delete ff", owner.delete(&format!("/api/v1/fitness-functions/{ff_id}")).await);
    assert_absent("ff after delete", &list_ids(&owner, "ff", &ff_list).await, &ff_id);
}

/// Test suites, schedules, and run trigger/list.
#[tokio::test]
#[ignore]
async fn test_suites_schedules_runs_round_trip_and_cross_tenant() {
    let owner = setup_team_org("rlsjts").await;
    let other = setup_team_org("rlsjtsx").await;
    let ws = create_workspace(&owner, &format!("ts-{}", unique())).await;
    let suites_path = format!("/api/v1/workspaces/{ws}/test-suites");

    let suite = ok(
        "create suite",
        owner
            .post(&suites_path, json!({ "name": "smoke", "kind": "unit", "config": {} }))
            .await,
    );
    let suite_id = str_field("create suite", &suite, "id");
    let suite_path = format!("/api/v1/test-suites/{suite_id}");
    assert_contains("list suites", &list_ids(&owner, "list suites", &suites_path).await, &suite_id);
    assert_contains(
        "list suites ?kind",
        &list_ids(&owner, "list suites kind", &format!("{suites_path}?kind=unit")).await,
        &suite_id,
    );
    ok("get suite", owner.get(&suite_path).await);
    let patched = ok("patch suite", owner.patch(&suite_path, json!({ "name": "smoke-2" })).await);
    assert_eq!(str_field("patch suite", &patched, "name"), "smoke-2");

    // Schedules.
    let sched_list = format!("/api/v1/test-suites/{suite_id}/schedules");
    let sched = ok(
        "create schedule",
        owner
            .post(&sched_list, json!({ "cron": "0 0 * * * *", "timezone": "UTC" }))
            .await,
    );
    let sched_id = str_field("create schedule", &sched, "id");
    assert_contains(
        "list schedules",
        &list_ids(&owner, "list schedules", &sched_list).await,
        &sched_id,
    );
    let toggled = ok(
        "disable schedule",
        owner
            .patch(&format!("/api/v1/test-schedules/{sched_id}"), json!({ "enabled": false }))
            .await,
    );
    assert_eq!(toggled["enabled"], json!(false));

    // Runs.
    let runs_path = format!("/api/v1/test-suites/{suite_id}/runs");
    let run = ok("trigger run", owner.post(&runs_path, json!({})).await);
    let run_id = str_field("trigger run", &run, "id");
    assert_contains("list suite runs", &list_ids(&owner, "list runs", &runs_path).await, &run_id);

    // Cross-tenant: nothing visible or mutable.
    client_err("x-tenant list suites", other.get(&suites_path).await);
    client_err("x-tenant get suite", other.get(&suite_path).await);
    client_err(
        "x-tenant patch suite",
        other.patch(&suite_path, json!({ "name": "pwned" })).await,
    );
    client_err("x-tenant list schedules", other.get(&sched_list).await);
    client_err(
        "x-tenant create schedule",
        other.post(&sched_list, json!({ "cron": "0 0 * * * *" })).await,
    );
    client_err(
        "x-tenant toggle schedule",
        other
            .patch(&format!("/api/v1/test-schedules/{sched_id}"), json!({ "enabled": true }))
            .await,
    );
    client_err(
        "x-tenant delete schedule",
        other.delete(&format!("/api/v1/test-schedules/{sched_id}")).await,
    );
    client_err("x-tenant list runs", other.get(&runs_path).await);
    client_err("x-tenant trigger run", other.post(&runs_path, json!({})).await);
    client_err("x-tenant delete suite", other.delete(&suite_path).await);

    let still = ok("owner get suite after x-tenant", owner.get(&suite_path).await);
    assert_eq!(str_field("suite after x-tenant", &still, "name"), "smoke-2");
    let scheds = ok("owner schedules after x-tenant", owner.get(&sched_list).await);
    let row = arr("owner schedules", &scheds)
        .iter()
        .find(|r| r["id"].as_str() == Some(sched_id.as_str()))
        .expect("schedule still present");
    assert_eq!(row["enabled"], json!(false), "x-tenant PATCH must not re-enable the schedule");

    ok(
        "delete schedule",
        owner.delete(&format!("/api/v1/test-schedules/{sched_id}")).await,
    );
    assert_absent(
        "schedules after delete",
        &list_ids(&owner, "schedules", &sched_list).await,
        &sched_id,
    );
    ok("delete suite", owner.delete(&suite_path).await);
    assert_absent(
        "suites after delete",
        &list_ids(&owner, "suites", &suites_path).await,
        &suite_id,
    );
    client_err("get deleted suite", owner.get(&suite_path).await);
}

/// Publish a minimal marketplace scenario in `e`'s org and return its id.
async fn publish_scenario(e: &E2e) -> String {
    publish_named_scenario(e).await.0
}

/// Like [`publish_scenario`], also returning the scenario's name.
async fn publish_named_scenario(e: &E2e) -> (String, String) {
    // Minimal gzip stream, same fixture as marketplace_e2e.rs.
    let package: Vec<u8> = vec![
        0x1F, 0x8B, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x03, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    let checksum = hex::encode(sha2::Sha256::digest(&package));
    let name = format!("rls-promo-{}", unique());
    ok(
        "publish scenario",
        e.post(
            "/api/v1/marketplace/scenarios/publish",
            json!({
                "manifest": json!({ "name": name, "version": "1.0.0", "description": "rls e2e" }).to_string(),
                "checksum": checksum,
                "size": package.len() as u64,
                "package": base64::engine::general_purpose::STANDARD.encode(&package),
            }),
        )
        .await,
    );
    let list = ok("list org scenarios", e.get("/api/v1/scenarios").await);
    let id = arr("list org scenarios", &list)
        .iter()
        .find(|s| s["name"].as_str() == Some(name.as_str()))
        .and_then(|s| s["id"].as_str())
        .unwrap_or_else(|| panic!("published scenario {name} not in {list}"))
        .to_string();
    (id, name)
}

fn promotion_ids(what: &str, body: &Value) -> Vec<String> {
    ids(what, &body["promotions"])
}

/// Scenario promotions (+ scenario_environment_versions via auto-complete and
/// approve).
#[tokio::test]
#[ignore]
async fn scenario_promotions_round_trip_and_cross_tenant() {
    let owner = setup_team_org("rlsjsp").await;
    let other = setup_team_org("rlsjspx").await;
    let ws = create_workspace(&owner, &format!("sp-{}", unique())).await;
    let scenario_id = publish_scenario(&owner).await;
    let list_path = format!("/api/v1/workspaces/{ws}/promotions");
    let promote_path = format!("/api/v1/workspaces/{ws}/environments/dev/promote-scenario");
    let promote = |from: &str, to: &str| {
        json!({
            "scenario_id": scenario_id,
            "scenario_version": "1.0.0",
            "from_environment": from,
            "to_environment": to,
        })
    };

    // dev -> test auto-completes (writes scenario_environment_versions).
    let a = ok("promote dev->test", owner.post(&promote_path, promote("dev", "test")).await);
    let a_id = str_field("promote dev->test", &a, "promotion_id");
    assert_eq!(a["requires_approval"], json!(false), "{a}");
    // test -> prod requires approval.
    let b = ok(
        "promote test->prod (b)",
        owner.post(&promote_path, promote("test", "prod")).await,
    );
    let b_id = str_field("promote b", &b, "promotion_id");
    assert_eq!(b["requires_approval"], json!(true), "{b}");
    let c = ok(
        "promote test->prod (c)",
        owner.post(&promote_path, promote("test", "prod")).await,
    );
    let c_id = str_field("promote c", &c, "promotion_id");

    let listed =
        promotion_ids("list promotions", &ok("list promotions", owner.get(&list_path).await));
    for id in [&a_id, &b_id, &c_id] {
        assert_contains("list promotions", &listed, id);
    }

    // Cross-tenant: other org cannot see, approve, reject, or promote into
    // the owner's workspace.
    let (status, body) = other.get(&list_path).await;
    if status.is_success() {
        let xs = promotion_ids("x-tenant list", &body);
        for id in [&a_id, &b_id, &c_id] {
            assert_absent("x-tenant list promotions", &xs, id);
        }
    } else {
        client_err("x-tenant list promotions", (status, body));
    }
    client_err(
        "x-tenant approve",
        other
            .post(&format!("{list_path}/{b_id}/approve"), json!({ "comments": "pwned" }))
            .await,
    );
    client_err(
        "x-tenant reject",
        other
            .post(&format!("{list_path}/{c_id}/reject"), json!({ "reason": "pwned" }))
            .await,
    );
    client_err(
        "x-tenant promote into owner ws",
        other.post(&promote_path, promote("dev", "test")).await,
    );

    // Owner approves b, rejects c.
    let approved = ok(
        "approve b",
        owner
            .post(&format!("{list_path}/{b_id}/approve"), json!({ "comments": "lgtm" }))
            .await,
    );
    assert_eq!(approved["status"], json!("completed"), "{approved}");
    let rejected = ok(
        "reject c",
        owner
            .post(&format!("{list_path}/{c_id}/reject"), json!({ "reason": "not yet" }))
            .await,
    );
    assert_eq!(rejected["status"], json!("rejected"), "{rejected}");

    let completed = promotion_ids(
        "list completed",
        &ok("list completed", owner.get(&format!("{list_path}?status=completed")).await),
    );
    assert_contains("completed", &completed, &a_id);
    assert_contains("completed", &completed, &b_id);
    assert_absent("completed", &completed, &c_id);
}

/// Incident events (incident_events joins to incidents.org_id).
#[tokio::test]
#[ignore]
async fn incident_events_round_trip_and_cross_tenant() {
    let owner = setup_team_org("rlsjie").await;
    let other = setup_team_org("rlsjiex").await;
    let org_id = owner.org_id.clone();

    let incident = ok(
        "raise incident",
        owner
            .post(
                &format!("/api/v1/organizations/{org_id}/incidents"),
                json!({
                    "source": "rls-e2e",
                    "dedupe_key": format!("rls-{}", unique()),
                    "severity": "low",
                    "title": "rls join e2e",
                }),
            )
            .await,
    );
    let incident_id = str_field("raise incident", &incident, "id");
    let events_path = format!("/api/v1/incidents/{incident_id}/events");

    let events = ok("list events", owner.get(&events_path).await);
    let created = arr("list events", &events).len();
    assert!(created >= 1, "expected a 'created' event, got {events}");

    ok(
        "acknowledge",
        owner
            .post(&format!("/api/v1/incidents/{incident_id}/acknowledge"), json!({}))
            .await,
    );
    let events = ok("list events after ack", owner.get(&events_path).await);
    assert!(arr("events after ack", &events).len() > created, "ack event missing: {events}");

    client_err("x-tenant list events", other.get(&events_path).await);
    client_err(
        "x-tenant acknowledge",
        other
            .post(&format!("/api/v1/incidents/{incident_id}/acknowledge"), json!({}))
            .await,
    );
}

/// Promoting ANOTHER org's scenario: allowed when it is published in the
/// marketplace (has a non-yanked version), 404 once its only version is
/// yanked. The promotion is authorized by the caller's workspace, not by
/// scenario ownership.
#[tokio::test]
#[ignore]
async fn promote_other_orgs_published_scenario() {
    let a = setup_team_org("rlsjxp").await;
    let b = setup_team_org("rlsjxpb").await;
    let ws = create_workspace(&a, &format!("xp-{}", unique())).await;
    let promote_path = format!("/api/v1/workspaces/{ws}/environments/dev/promote-scenario");
    let body = |id: &str| {
        json!({
            "scenario_id": id,
            "scenario_version": "1.0.0",
            "from_environment": "dev",
            "to_environment": "test",
        })
    };

    let (published, _) = publish_named_scenario(&b).await;
    let res = ok(
        "A promotes B's published scenario",
        a.post(&promote_path, body(&published)).await,
    );
    let pid = str_field("cross-org promotion", &res, "promotion_id");
    let listed = promotion_ids(
        "A promotions",
        &ok("A list promotions", a.get(&format!("/api/v1/workspaces/{ws}/promotions")).await),
    );
    assert_contains("A sees its promotion of B's scenario", &listed, &pid);

    let (unpublished, name) = publish_named_scenario(&b).await;
    ok(
        "B yanks its only version",
        b.delete(&format!("/api/v1/marketplace/scenarios/{name}/versions/1.0.0/yank"))
            .await,
    );
    expect_status(
        "A promotes B's unpublished scenario",
        StatusCode::NOT_FOUND,
        a.post(&promote_path, body(&unpublished)).await,
    );
    // A random id is the same 404.
    expect_status(
        "A promotes a missing scenario",
        StatusCode::NOT_FOUND,
        a.post(&promote_path, body(&uuid::Uuid::new_v4().to_string())).await,
    );
}
