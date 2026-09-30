//! E2E for `/api/v1/hosted-mocks/{id}/overrides` against a live registry.
//!
//! Needs `REGISTRY_URL` and `DATABASE_URL`, like the other registry E2E
//! suites. The deployment row is inserted directly with no Fly machine, so a
//! save reports `runtime: "unreachable"`.

use chrono::Utc;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

struct Tenant {
    client: Client,
    base_url: String,
    token: String,
    org_id: String,
}

impl Tenant {
    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.client
            .request(method, format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("X-Organization-Id", &self.org_id)
    }
}

async fn register(base_url: &str, tag: &str) -> Tenant {
    let client = Client::new();
    let ts = format!("{}{}", tag, Utc::now().timestamp_micros());
    let res = client
        .post(format!("{base_url}/api/v1/auth/register"))
        .json(&json!({
            "username": format!("ovr_{ts}"),
            "email": format!("ovr_{ts}@e2e-test.local"),
            "password": "SecureP@ssw0rd!2024",
        }))
        .send()
        .await
        .expect("register failed");
    let body: Value = res.json().await.expect("register not JSON");
    let token = body["access_token"]
        .as_str()
        .or_else(|| body["token"].as_str())
        .expect("no access token")
        .to_string();

    let res = client
        .post(format!("{base_url}/api/v1/organizations"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "name": format!("Overrides Org {ts}"), "slug": format!("ovr-{ts}") }))
        .send()
        .await
        .expect("create org failed");
    let body: Value = res.json().await.expect("org not JSON");
    let org_id = body["id"].as_str().expect("no org id").to_string();

    Tenant {
        client,
        base_url: base_url.to_string(),
        token,
        org_id,
    }
}

async fn insert_deployment(pool: &PgPool, org_id: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO hosted_mocks (id, org_id, name, slug, config_json, status, metadata_json) \
         VALUES ($1, $2, 'overrides-fixture', $3, '{}'::jsonb, 'active', $4)",
    )
    .bind(id)
    .bind(Uuid::parse_str(org_id).unwrap())
    .bind(format!("ovr-{}", id.simple()))
    .bind(json!({ "flyio_machine_id": "m-fixture" }))
    .execute(pool)
    .await
    .expect("insert hosted_mock failed");
    id
}

#[tokio::test]
#[ignore]
async fn overrides_round_trip_validate_and_stay_tenant_scoped() {
    let base_url = std::env::var("REGISTRY_URL").expect("REGISTRY_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
        .await
        .expect("DB connect failed");

    let owner = register(&base_url, "a").await;
    let deployment = insert_deployment(&pool, &owner.org_id).await;
    let path = format!("/api/v1/hosted-mocks/{deployment}/overrides");

    let res = owner.request(reqwest::Method::GET, &path).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.json::<Value>().await.unwrap()["rules"], json!([]));

    let rule = json!({
        "name": "vip tier",
        "targets": ["operation:getUser"],
        "when": "header[x-tier]=vip",
        "patch": [{"op": "replace", "path": "/tier", "value": "gold"}]
    });
    let res = owner
        .request(reqwest::Method::PUT, &path)
        .json(&json!({ "rules": [rule] }))
        .send()
        .await
        .unwrap();
    let status = res.status();
    let body: Value = res.json().await.unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["runtime"], "unreachable");
    assert_eq!(body["rules"][0]["name"], "vip tier");
    assert_eq!(body["rules"][0]["enabled"], true);

    let metadata: Value =
        sqlx::query_scalar("SELECT metadata_json FROM hosted_mocks WHERE id = $1")
            .bind(deployment)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(metadata["flyio_machine_id"], "m-fixture", "other metadata keys survive");
    assert_eq!(metadata["override_rules"][0]["targets"], json!(["operation:getUser"]));

    let res = owner
        .request(reqwest::Method::PUT, &path)
        .json(&json!({ "rules": [{"targets": ["path:(unclosed"], "patch": []}] }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    let res = owner.request(reqwest::Method::GET, &path).send().await.unwrap();
    assert_eq!(res.json::<Value>().await.unwrap()["rules"][0]["name"], "vip tier");

    let outsider = register(&base_url, "b").await;
    let res = outsider.request(reqwest::Method::GET, &path).send().await.unwrap();
    assert!(res.status().is_client_error(), "other org read: {}", res.status());
    let res = outsider
        .request(reqwest::Method::PUT, &path)
        .json(&json!({ "rules": [] }))
        .send()
        .await
        .unwrap();
    assert!(res.status().is_client_error(), "other org write: {}", res.status());
    let res = owner.request(reqwest::Method::GET, &path).send().await.unwrap();
    assert_eq!(res.json::<Value>().await.unwrap()["rules"].as_array().unwrap().len(), 1);
}
