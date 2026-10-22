//! Exercise credential transport against the real registry and database.
//! Credentials remain in memory and never appear in assertion diagnostics.

use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

struct Fixture {
    client: Client,
    base: String,
    session: String,
    api_token: String,
}

impl Fixture {
    async fn status(
        &self,
        path: &str,
        header: Option<&str>,
        cookie: Option<&str>,
        query: Option<&str>,
    ) -> StatusCode {
        let mut request = self.client.get(format!("{}{path}", self.base));
        if let Some(token) = header {
            request = request.bearer_auth(token);
        }
        if let Some(token) = cookie {
            request = request.header("Cookie", format!("mockforge_session={token}"));
        }
        if let Some(token) = query {
            request = request.query(&[("token", token)]);
        }
        // Do not format reqwest::Error: it can contain a query credential.
        request
            .send()
            .await
            .unwrap_or_else(|_| panic!("fixture request failed"))
            .status()
    }
}

async fn fixture() -> Fixture {
    let base = std::env::var("REGISTRY_URL").expect("owned local REGISTRY_URL required");
    let url = reqwest::Url::parse(&base).expect("invalid fixture URL");
    assert!(
        matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1" | "[::1]")),
        "credential fixture must use a local registry"
    );
    // Deliberately no cookie jar: query-only denials cannot inherit a session.
    let client = Client::builder().timeout(Duration::from_secs(20)).build().unwrap();
    let suffix = Uuid::new_v4().simple().to_string();
    let response = client
        .post(format!("{base}/api/v1/auth/register"))
        .json(&json!({
            "username": format!("query_{suffix}"),
            "email": format!("query_{suffix}@e2e-test.local"),
            "password": format!("Owned-fixture-{suffix}!9")
        }))
        .send()
        .await
        .expect("fixture registration failed");
    assert!(response.status().is_success(), "registration status {}", response.status());
    let body: Value = response.json().await.expect("registration JSON missing");
    let session = body["access_token"]
        .as_str()
        .or_else(|| body["token"].as_str())
        .expect("session credential missing")
        .to_owned();
    let response = client
        .post(format!("{base}/api/v1/organizations"))
        .bearer_auth(&session)
        .json(&json!({"name": "Owned query fixture", "slug": format!("query-{suffix}")}))
        .send()
        .await
        .expect("fixture organization failed");
    assert!(response.status().is_success(), "organization status {}", response.status());
    let body: Value = response.json().await.expect("organization JSON missing");
    let org = body["id"].as_str().expect("organization id missing");
    let response = client
        .post(format!("{base}/api/v1/tokens"))
        .bearer_auth(&session)
        .header("X-Organization-Id", org)
        .json(&json!({"name": "Owned query fixture", "scopes": ["read:packages"]}))
        .send()
        .await
        .expect("fixture token creation failed");
    assert!(response.status().is_success(), "token creation status {}", response.status());
    let body: Value = response.json().await.expect("token JSON missing");
    let api_token = body["token"].as_str().expect("API token missing").to_owned();
    Fixture {
        client,
        base,
        session,
        api_token,
    }
}

#[tokio::test]
#[ignore = "requires an owned local registry and Postgres; required in both registry E2E gates"]
async fn real_registry_limits_query_credentials_to_session_sse() {
    let f = fixture().await;
    let normal = "/api/v1/auth/me";
    assert_eq!(f.status(normal, None, None, None).await, StatusCode::UNAUTHORIZED);
    assert_eq!(f.status(normal, Some(&f.session), None, None).await, StatusCode::OK);
    assert_eq!(f.status(normal, Some(&f.api_token), None, None).await, StatusCode::OK);
    assert_eq!(f.status(normal, None, Some(&f.session), None).await, StatusCode::OK);
    for token in [&f.session, &f.api_token] {
        assert_eq!(f.status(normal, None, None, Some(token)).await, StatusCode::UNAUTHORIZED);
    }
    let id = Uuid::new_v4();
    let paths = [
        format!("/api/v1/hosted-mocks/{id}/runtime-logs/stream"),
        format!("/api/v1/test-runs/{id}/stream"),
        format!("/api/v1/workspaces/{id}/test-generation/jobs/{id}/stream"),
    ];
    for path in paths {
        // Missing owned fixture resources produce 400 after authentication;
        // never read a successful SSE body, which intentionally stays open.
        for (header, cookie, query) in [
            (Some(f.session.as_str()), None, None),
            (None, Some(f.session.as_str()), None),
            (None, None, Some(f.session.as_str())),
            (Some(f.session.as_str()), None, Some("invalid-query")),
            (None, Some(f.session.as_str()), Some("invalid-query")),
        ] {
            assert_eq!(f.status(&path, header, cookie, query).await, StatusCode::BAD_REQUEST);
        }
        for (header, cookie, query) in [
            (None, None, None),
            (None, None, Some(f.api_token.as_str())),
            (Some("invalid-header"), Some(f.session.as_str()), Some(f.session.as_str())),
            (None, Some("invalid-cookie"), Some(f.session.as_str())),
        ] {
            assert_eq!(f.status(&path, header, cookie, query).await, StatusCode::UNAUTHORIZED);
        }
    }
}
