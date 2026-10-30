//! Real registry authentication and scope containment, including persisted token lifecycle.
//! Run with the owned Postgres/registry fixtures in registry-e2e or rls-e2e-gate.
//! Credentials stay in memory; failure messages report statuses or error codes only.

use chrono::{Duration as ChronoDuration, Utc};
use reqwest::{Client, Method, StatusCode};
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

const ALL_SCOPES: [&str; 6] = [
    "read:packages",
    "publish:packages",
    "deploy:mocks",
    "admin:org",
    "read:usage",
    "manage:billing",
];

struct Fixture {
    client: Client,
    base: String,
    session: String,
    org: String,
}

impl Fixture {
    async fn request(
        &self,
        credential: &str,
        method: Method,
        path: &str,
        organization: Option<&str>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut request = self
            .client
            .request(method, format!("{}{path}", self.base))
            .bearer_auth(credential);
        if let Some(org) = organization {
            request = request.header("X-Organization-Id", org);
        }
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await.expect("fixture HTTP request failed");
        let status = response.status();
        let value = response.json().await.unwrap_or(Value::Null);
        (status, value)
    }

    async fn token_ids(&self) -> Vec<String> {
        let (status, body) = self
            .request(&self.session, Method::GET, "/api/v1/tokens", Some(&self.org), None)
            .await;
        assert_eq!(status, StatusCode::OK, "session token inventory failed");
        let mut ids: Vec<_> = body
            .as_array()
            .expect("token list must be an array")
            .iter()
            .map(|item| item["id"].as_str().expect("token id missing").to_owned())
            .collect();
        ids.sort();
        ids
    }

    async fn mint(&self, scopes: &[&str], expired: bool) -> (String, String) {
        let mut body = json!({"name": "owned scope fixture", "scopes": scopes});
        if expired {
            body["expires_at"] = json!((Utc::now() - ChronoDuration::hours(1)).to_rfc3339());
        }
        let (status, body) = self
            .request(&self.session, Method::POST, "/api/v1/tokens", Some(&self.org), Some(body))
            .await;
        assert!(status.is_success(), "session token creation returned {status}");
        (
            body["token"].as_str().expect("token missing").to_owned(),
            body["token_id"].as_str().expect("token id missing").to_owned(),
        )
    }

    async fn new_org(&self) -> String {
        let suffix = Uuid::new_v4().simple().to_string();
        let (status, body) = self
            .request(
                &self.session,
                Method::POST,
                "/api/v1/organizations",
                None,
                Some(json!({"name": "Owned scope fixture", "slug": format!("scope-{suffix}")})),
            )
            .await;
        assert!(status.is_success(), "fixture organization creation returned {status}");
        body["id"].as_str().expect("organization id missing").to_owned()
    }
}

async fn fixture() -> Fixture {
    let base =
        std::env::var("REGISTRY_URL").expect("REGISTRY_URL must name an owned local registry");
    let url = reqwest::Url::parse(&base).expect("invalid fixture registry URL");
    assert!(
        matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1" | "[::1]")),
        "scope fixtures must run on a local registry"
    );
    let client = Client::builder().timeout(Duration::from_secs(20)).build().unwrap();
    let suffix = Uuid::new_v4().simple().to_string();
    let response = client
        .post(format!("{base}/api/v1/auth/register"))
        .json(&json!({
            "username": format!("scope_{suffix}"),
            "email": format!("scope_{suffix}@e2e-test.local"),
            "password": format!("Owned-fixture-{suffix}!9")
        }))
        .send()
        .await
        .expect("fixture registration failed");
    assert!(
        response.status().is_success(),
        "fixture registration returned {}",
        response.status()
    );
    let body: Value = response.json().await.expect("registration JSON missing");
    let session = body["access_token"]
        .as_str()
        .or_else(|| body["token"].as_str())
        .expect("session credential missing")
        .to_owned();
    let mut fixture = Fixture {
        client,
        base,
        session,
        org: String::new(),
    };
    fixture.org = fixture.new_org().await;
    fixture
}

async fn assert_denied(f: &Fixture, token: &str, method: Method, path: &str, code: &str) {
    let request_body = if method == Method::POST && path == "/api/v1/tokens" {
        json!({"name": "must not be created by a token", "scopes": ALL_SCOPES})
    } else {
        json!({})
    };
    let (status, body) = f.request(token, method, path, None, Some(request_body)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "scope boundary returned {status}");
    assert_eq!(body["error_code"].as_str(), Some(code));
}

#[tokio::test]
#[ignore = "requires an owned local registry and Postgres; required registry/RLS E2E gates run it"]
async fn real_tokens_enforce_scopes_sessions_orgs_revocation_and_expiry() {
    let f = fixture().await;
    let (reader, _) = f.mint(&["read:packages"], false).await;
    let (publisher, _) = f.mint(&["publish:packages"], false).await;
    let deployment = Uuid::new_v4();
    let suite = Uuid::new_v4();
    let operations = [
        (Method::POST, "/api/v1/hosted-mocks".to_owned()),
        (Method::GET, "/api/v1/hosted-mocks".to_owned()),
        (Method::PATCH, format!("/api/v1/hosted-mocks/{deployment}/status")),
        (Method::DELETE, format!("/api/v1/hosted-mocks/{deployment}")),
        (Method::POST, format!("/api/v1/hosted-mocks/{deployment}/redeploy")),
        (Method::POST, format!("/api/v1/test-suites/{suite}/runs")),
        (Method::POST, "/api/v1/billing/checkout".to_owned()),
        (Method::GET, "/api/v1/usage".to_owned()),
        (Method::POST, format!("/api/v1/organizations/{}/members", f.org)),
    ];
    for token in [&reader, &publisher] {
        for (method, path) in &operations {
            assert_denied(&f, token, method.clone(), path, "INSUFFICIENT_SCOPE").await;
        }
    }

    let (all, all_id) = f.mint(&ALL_SCOPES, false).await;
    let token_ids_before_denials = f.token_ids().await;
    // Even every scope cannot grant the authority to mint, list, rotate or revoke tokens.
    for token in [&reader, &all] {
        for (method, path) in [
            (Method::POST, "/api/v1/tokens".to_owned()),
            (Method::GET, "/api/v1/tokens".to_owned()),
            (Method::POST, format!("/api/v1/tokens/{all_id}/rotate")),
            (Method::DELETE, format!("/api/v1/tokens/{all_id}")),
        ] {
            assert_denied(&f, token, method, &path, "TOKEN_NOT_ALLOWED").await;
        }
    }

    assert_eq!(
        f.token_ids().await,
        token_ids_before_denials,
        "denied token management must not create, rotate or revoke persisted credentials"
    );

    let (deployer, deployer_id) = f.mint(&["deploy:mocks"], false).await;
    let (status, _) = f.request(&deployer, Method::GET, "/api/v1/hosted-mocks", None, None).await;
    assert_eq!(status, StatusCode::OK, "correct deployment scope must reach the real handler");
    let (status, _) = f.request(&reader, Method::GET, "/api/v1/auth/me", None, None).await;
    assert_eq!(status, StatusCode::OK, "restricted tokens retain identity access");
    let (status, _) =
        f.request(&f.session, Method::GET, "/api/v1/tokens", Some(&f.org), None).await;
    assert_eq!(status, StatusCode::OK, "interactive session token management must still work");

    // The same user owns both organizations: a role check alone cannot explain this denial.
    let other = f.new_org().await;
    for credential in [&reader, &all] {
        let (status, listed) =
            f.request(credential, Method::GET, "/api/v1/organizations", None, None).await;
        assert_eq!(status, StatusCode::OK, "PAT organization inventory failed");
        let listed = listed.as_array().expect("organization list must be an array");
        assert_eq!(listed.len(), 1, "PAT must not list a sibling organization");
        assert_eq!(listed[0]["id"].as_str(), Some(f.org.as_str()));
    }
    let (status, listed) =
        f.request(&f.session, Method::GET, "/api/v1/organizations", None, None).await;
    assert_eq!(status, StatusCode::OK, "session organization inventory failed");
    let listed = listed.as_array().expect("organization list must be an array");
    for expected in [&f.org, &other] {
        assert!(
            listed.iter().any(|org| org["id"].as_str() == Some(expected.as_str())),
            "interactive session must retain its organization inventory"
        );
    }
    let (status, body) = f
        .request(&deployer, Method::GET, "/api/v1/hosted-mocks", Some(&other), None)
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error_code"].as_str(), Some("TOKEN_ORG_MISMATCH"));
    assert_denied(
        &f,
        &all,
        Method::GET,
        &format!("/api/v1/organizations/{other}"),
        "TOKEN_ORG_MISMATCH",
    )
    .await;

    let (status, _) = f
        .request(
            &f.session,
            Method::DELETE,
            &format!("/api/v1/tokens/{deployer_id}"),
            Some(&f.org),
            None,
        )
        .await;
    assert!(status.is_success(), "session revocation returned {status}");
    let (status, _) = f.request(&deployer, Method::GET, "/api/v1/hosted-mocks", None, None).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "revoked persisted token must fail authentication"
    );
    let (expired, _) = f.mint(&["deploy:mocks"], true).await;
    let (status, _) = f.request(&expired, Method::GET, "/api/v1/hosted-mocks", None, None).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "expired persisted token must fail authentication"
    );
}
