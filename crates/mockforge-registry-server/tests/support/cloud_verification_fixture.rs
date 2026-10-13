//! Disposable cloud verification fixture shared by HTTP regressions.
use chrono::Utc;
use reqwest::Client;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

pub(crate) struct E2e {
    pub(crate) client: Client,
    pub(crate) base_url: String,
    pub(crate) access_token: String,
    pub(crate) org_id: String,
    pub(crate) workspace_id: String,
    pub(crate) pool: PgPool,
    pub(crate) deployment_id: Uuid,
}

impl E2e {
    pub(crate) fn auth(&self) -> String {
        format!("Bearer {}", self.access_token)
    }

    pub(crate) fn post(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .post(format!("{}{}", self.base_url, path))
            .header("Authorization", self.auth())
            .header("X-Organization-Id", &self.org_id)
    }

    pub(crate) fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .get(format!("{}{}", self.base_url, path))
            .header("Authorization", self.auth())
            .header("X-Organization-Id", &self.org_id)
    }
}

pub(crate) async fn register_and_setup(base_url: &str, pool: &PgPool) -> E2e {
    let client = Client::new();
    let ts = Utc::now().timestamp_micros();
    let username = format!("ver_{}", ts);
    let email = format!("ver_{}@e2e-test.local", ts);
    let password = format!("{}Aa1!", Uuid::new_v4());

    let res = client
        .post(format!("{}/api/v1/auth/register", base_url))
        .json(&json!({ "username": username, "email": email, "password": password }))
        .send()
        .await
        .expect("register failed");
    let status = res.status();
    let body: Value = res.json().await.expect("register not JSON");
    assert!(status.is_success(), "register returned {}", status);
    let access_token = body["access_token"]
        .as_str()
        .or_else(|| body["token"].as_str())
        .expect("no access token")
        .to_string();

    let org_slug = format!("ver-{}", ts);
    let res = client
        .post(format!("{}/api/v1/organizations", base_url))
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&json!({ "name": format!("Verification Test Org {}", ts), "slug": org_slug }))
        .send()
        .await
        .expect("create org failed");
    let status = res.status();
    let body: Value = res.json().await.expect("org not JSON");
    assert!(status.is_success(), "create org {}: {}", status, body);
    let org_id = body["id"].as_str().expect("no org id").to_string();

    // Lift the free-plan max_projects=1 cap so tests can create more than
    // one workspace under the same org (workspace_isolation needs two).
    // Mirrors the gate in handlers::cloud_workspaces::create_workspace,
    // which reads `org.limits_json.max_projects` and falls back to 1.
    // -1 means unlimited.
    let org_uuid_for_limits = Uuid::parse_str(&org_id).expect("org id not UUID");
    sqlx::query(
        r#"
        UPDATE organizations
        SET limits_json = jsonb_set(
            COALESCE(limits_json, '{}'::jsonb),
            '{max_projects}',
            '-1'::jsonb,
            true
        )
        WHERE id = $1
        "#,
    )
    .bind(org_uuid_for_limits)
    .execute(pool)
    .await
    .expect("relax org limits failed");

    // Workspace
    let res = client
        .post(format!("{}/api/v1/workspaces", base_url))
        .header("Authorization", format!("Bearer {}", access_token))
        .header("X-Organization-Id", &org_id)
        .json(&json!({ "name": "verification-e2e", "description": "e2e fixture" }))
        .send()
        .await
        .expect("create workspace failed");
    let status = res.status();
    let body: Value = res.json().await.expect("ws not JSON");
    assert!(status.is_success(), "create ws {}: {}", status, body);
    let workspace_id = body["id"].as_str().expect("no ws id").to_string();

    // Insert a hosted_mock row so `runtime_captures.deployment_id` FK
    // is satisfied. We don't actually deploy anything — `status='active'`
    // is just a label here, no Fly machine is created.
    let org_uuid = Uuid::parse_str(&org_id).expect("org id not UUID");
    let deployment_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO hosted_mocks (id, org_id, name, slug, config_json, status)
        VALUES ($1, $2, $3, $4, $5::jsonb, 'active')
        "#,
    )
    .bind(deployment_id)
    .bind(org_uuid)
    .bind("verification-fixture")
    .bind(format!("vfix-{}", ts))
    .bind("{}")
    .execute(pool)
    .await
    .expect("insert hosted_mock failed");

    E2e {
        client,
        base_url: base_url.to_string(),
        access_token,
        org_id,
        workspace_id,
        pool: pool.clone(),
        deployment_id,
    }
}
