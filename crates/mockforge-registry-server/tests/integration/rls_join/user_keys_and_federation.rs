//! RLS e2e: the per-user GUC tables (`user_public_keys`,
//! `cloud_plugin_beta_interest`, migration 088) and
//! `federation_scenario_activations` (join policy via `federations`).

use super::*;

fn ed25519_b64() -> String {
    use base64::Engine;
    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn key_ids(what: &str, body: &Value) -> Vec<String> {
    ids(what, &body["keys"])
}

/// Personal keys: create -> list -> rotate -> revoke, all bound to the caller
/// through `app.current_user_id`. Another user never sees or revokes them.
#[tokio::test]
#[ignore]
async fn user_public_keys_round_trip_and_isolation() {
    let a = setup_team_org("rlsuk").await;
    let b = setup_team_org("rlsukb").await;
    let keys = "/api/v1/users/me/public-keys";

    let body = ok(
        "A create key",
        a.post(keys, json!({ "publicKeyB64": ed25519_b64(), "label": "a-laptop" }))
            .await,
    );
    let k1 = str_field("A key", &body, "id");
    assert!(body["orgId"].is_null(), "personal key has no org tag: {body}");

    let listed = key_ids("A list", &ok("A list keys", a.get(keys).await));
    assert_contains("A sees own key", &listed, &k1);
    // Same user, no org header: the user GUC does not depend on the org.
    let listed = key_ids("A list bare", &ok("A list bare", a.without_org_header().get(keys).await));
    assert_contains("A sees own key without org header", &listed, &k1);

    let listed = key_ids("B list", &ok("B list keys", b.get(keys).await));
    assert_absent("B never sees A's key", &listed, &k1);
    client_err("B revoke A's key", b.delete(&format!("{keys}/{k1}")).await);
    client_err(
        "B rotate A's key",
        b.post(
            &format!("{keys}/{k1}/rotate"),
            json!({ "newPublicKeyB64": ed25519_b64(), "newLabel": "stolen" }),
        )
        .await,
    );

    let body = ok(
        "A rotate key",
        a.post(
            &format!("{keys}/{k1}/rotate"),
            json!({ "newPublicKeyB64": ed25519_b64(), "newLabel": "a-laptop-2" }),
        )
        .await,
    );
    let k2 = str_field("rotated key", &body, "id");
    let listed = key_ids("A list after rotate", &ok("A list", a.get(keys).await));
    assert_contains("rotated key listed", &listed, &k2);
    assert_absent("old key revoked", &listed, &k1);
    let body = ok("A list incl revoked", a.get(&format!("{keys}?includeRevoked=true")).await);
    assert_contains("revoked history", &key_ids("history", &body), &k1);

    ok("A revoke rotated key", a.delete(&format!("{keys}/{k2}")).await);
    let listed = key_ids("A list after revoke", &ok("A list", a.get(keys).await));
    assert_absent("revoked key gone", &listed, &k2);
}

/// Org-tagged keys: an org admin lists and revokes a member's org-tagged key
/// (bound through the `org_shared_keys` policy), a plain member cannot.
#[tokio::test]
#[ignore]
async fn org_tagged_public_keys_admin_list_and_revoke() {
    let a = setup_team_org("rlsok").await;
    let org = a.org_id.clone();
    let (c, _) = register_user("rlsokc").await;
    let c = c.as_org(&org);
    ok(
        "add C as admin",
        a.post(
            &format!("/api/v1/organizations/{org}/members"),
            json!({ "user_id": c.user_id, "role": "admin" }),
        )
        .await,
    );
    let keys = "/api/v1/users/me/public-keys";
    let body = ok(
        "C create org-tagged key",
        c.post(keys, json!({ "publicKeyB64": ed25519_b64(), "label": "c-ci", "orgId": org }))
            .await,
    );
    let kc = str_field("C key", &body, "id");
    assert_eq!(body["orgId"], org.as_str());

    let org_keys = format!("/api/v1/organizations/{org}/public-keys");
    let listed = key_ids("A org keys", &ok("A list org keys", a.get(&org_keys).await));
    assert_contains("owner sees member's org key", &listed, &kc);
    // A's own personal-key list does not include C's key.
    let listed = key_ids("A own keys", &ok("A own keys", a.get(keys).await));
    assert_absent("org key is not A's personal key", &listed, &kc);

    // An outsider org admin cannot list or revoke it.
    let d = setup_team_org("rlsokd").await;
    client_err("outsider list org keys", d.get(&org_keys).await);
    client_err("outsider revoke org key", d.delete(&format!("{keys}/{kc}")).await);

    // Owner revokes the member's org-tagged key (org-admin path).
    ok("A revoke C's org key", a.delete(&format!("{keys}/{kc}")).await);
    let listed = key_ids("C keys after revoke", &ok("C keys", c.get(keys).await));
    assert_absent("C's key revoked by org owner", &listed, &kc);
}

/// `cloud_plugin_beta_interest` is per user: upsert then read back, and a
/// second user starts with nothing.
#[tokio::test]
#[ignore]
async fn cloud_plugin_beta_interest_is_per_user() {
    let a = setup_team_org("rlsbi").await;
    let b = setup_team_org("rlsbib").await;
    let path = "/api/v1/cloud-plugins/beta-interest";

    let body = ok("B status before", b.get(&format!("{path}/me")).await);
    assert_eq!(body["signed_up"], false, "{body}");

    ok("A submit", a.post(path, json!({ "use_case": "first" })).await);
    ok("A resubmit (upsert)", a.post(path, json!({ "use_case": "second" })).await);
    let body = ok("A status", a.get(&format!("{path}/me")).await);
    assert_eq!(body["signed_up"], true, "{body}");
    assert_eq!(body["use_case"], "second", "{body}");
    let body = ok("A status bare", a.without_org_header().get(&format!("{path}/me")).await);
    assert_eq!(body["signed_up"], true, "user binding must not need an org: {body}");

    let body = ok("B status after A", b.get(&format!("{path}/me")).await);
    assert_eq!(body["signed_up"], false, "B must not see A's signup: {body}");
}

/// Federation scenario activation lifecycle (activate -> get -> report ->
/// deactivate) under the join policy, plus cross-tenant denial.
#[tokio::test]
#[ignore]
async fn federation_scenario_activation_round_trip() {
    let e = setup_team_org("rlsfa").await;
    let services = json!([
        { "name": "auth", "workspace_id": uuid::Uuid::new_v4().to_string(),
          "base_path": "/auth", "reality_level": "real", "dependencies": [] },
        { "name": "payments", "workspace_id": uuid::Uuid::new_v4().to_string(),
          "base_path": "/payments", "reality_level": "real", "dependencies": [] }
    ]);
    let body = ok(
        "create federation",
        e.post(
            "/api/v1/federation",
            json!({ "name": "fa", "description": "e2e", "services": services }),
        )
        .await,
    );
    let f = str_field("federation", &body, "id");
    let manifest = json!({
        "manifest_version": "1.0", "name": "payment-outage", "version": "0.1.0",
        "title": "Payment outage", "description": "e2e", "author": "e2e",
        "category": "other", "compatibility": {"min_version": "0.3.0"}, "files": [],
    });
    let activate = format!("/api/v1/federation/{f}/scenarios/activate");
    let active = format!("/api/v1/federation/{f}/scenarios/active");

    let body = ok(
        "activate",
        e.post(
            &activate,
            json!({ "scenario_name": "payment-outage", "manifest": manifest,
                    "service_overrides": { "payments": { "failure_rate": 0.5 } } }),
        )
        .await,
    );
    let act = str_field("activation", &body, "id");
    assert_eq!(body["status"], "active");
    client_err(
        "double activate",
        e.post(&activate, json!({ "scenario_name": "second", "manifest": manifest }))
            .await,
    );
    let body = ok("get active", e.get(&active).await);
    assert_eq!(body["id"], act.as_str(), "{body}");

    let body = ok(
        "report",
        e.post(
            &format!("{active}/report"),
            json!({ "service_name": "payments", "status": "applied" }),
        )
        .await,
    );
    assert!(body["per_service_state"].to_string().contains("applied"), "{body}");

    // Another tenant cannot read or deactivate it.
    let other = setup_team_org("rlsfb").await;
    client_err("other get active", other.get(&active).await);
    client_err("other deactivate", other.delete(&active).await);

    let body = ok("deactivate", e.delete(&active).await);
    assert_eq!(body["status"], "deactivated");
    ok("delete federation", e.delete(&format!("/api/v1/federation/{f}")).await);
}
