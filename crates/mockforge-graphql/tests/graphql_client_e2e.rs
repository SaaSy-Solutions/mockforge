//! End-to-end regression: a real HTTP client (`reqwest`) posts a GraphQL
//! query to the mock server and validates the JSON response against the
//! schema.
//!
//! Existing tests in this crate exercise the Rust-level handler registry
//! and schema builders but never hit the `/graphql` HTTP endpoint with a
//! real request, so a regression in the router / executor / serializer
//! could ship silently. This locks in the on-the-wire contract.

use mockforge_graphql::create_graphql_router;
use serde_json::json;
use std::time::Duration;

async fn spawn_server() -> String {
    let (http_url, _ws_url) = spawn_server_pair().await;
    http_url
}

/// Spawn a GraphQL server and return both the HTTP `/graphql` URL
/// and the WebSocket `/graphql/ws` URL. Used by the subscriptions
/// test; the HTTP tests just discard the second element.
async fn spawn_server_pair() -> (String, String) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = create_graphql_router(None).await.expect("router builds cleanly");
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    // Give axum a tick to start accepting.
    tokio::time::sleep(Duration::from_millis(50)).await;
    (format!("http://{addr}/graphql"), format!("ws://{addr}/graphql/ws"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_real_client_users_query_returns_structured_list() {
    let url = spawn_server().await;

    let client = reqwest::Client::new();
    let body = json!({
        "query": "query($limit: Int!) { users(limit: $limit) { id name email } }",
        "variables": { "limit": 3 },
    });
    let response =
        tokio::time::timeout(Duration::from_secs(5), client.post(&url).json(&body).send())
            .await
            .expect("request should complete within 5s")
            .expect("reqwest transport ok");

    assert!(response.status().is_success(), "HTTP {}", response.status());
    let value: serde_json::Value = response.json().await.expect("response must be JSON");

    // No errors field (or if present, empty array).
    if let Some(errs) = value.get("errors") {
        assert!(
            errs.as_array().map(|a| a.is_empty()).unwrap_or(false),
            "expected no GraphQL errors, got: {errs}"
        );
    }

    let users = value
        .pointer("/data/users")
        .and_then(|v| v.as_array())
        .expect("response.data.users must be an array");
    assert_eq!(users.len(), 3, "limit=3 was requested; got {} users", users.len());

    for (i, u) in users.iter().enumerate() {
        let id = u.pointer("/id").and_then(|v| v.as_str()).unwrap();
        let name = u.pointer("/name").and_then(|v| v.as_str()).unwrap();
        let email = u.pointer("/email").and_then(|v| v.as_str()).unwrap();
        assert_eq!(id, format!("user-{i}"));
        assert_eq!(name, format!("User {i}"));
        assert!(email.ends_with("@example.com"), "unexpected email shape: {email}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_real_client_user_by_id_returns_single_record() {
    let url = spawn_server().await;
    let client = reqwest::Client::new();
    let body = json!({
        "query": r#"{ user(id: "42") { id name email } }"#,
    });
    let response = client.post(&url).json(&body).send().await.expect("reqwest transport ok");
    assert!(response.status().is_success());
    let value: serde_json::Value = response.json().await.unwrap();

    let user = value.pointer("/data/user").expect("data.user present");
    assert_eq!(user.pointer("/id").and_then(|v| v.as_str()), Some("42"));
    assert!(user.pointer("/name").and_then(|v| v.as_str()).is_some());
}

/// Standard introspection query (abridged) exercising `__schema`,
/// `types`, `queryType`, and field shapes — the shape Apollo / GraphiQL /
/// Relay compiler send during schema discovery. Covers the over-the-wire
/// path specifically; existing handler-level coverage lives in
/// `tests/introspection_test.rs` which bypasses axum routing.
const INTROSPECTION_QUERY: &str = r#"
    query IntrospectionQuery {
      __schema {
        queryType { name }
        mutationType { name }
        subscriptionType { name }
        types {
          name
          kind
          fields(includeDeprecated: true) { name }
        }
      }
    }
"#;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_real_client_introspection_query_returns_schema() {
    let url = spawn_server().await;
    let client = reqwest::Client::new();
    let body = json!({ "query": INTROSPECTION_QUERY });
    let response =
        tokio::time::timeout(Duration::from_secs(5), client.post(&url).json(&body).send())
            .await
            .expect("request should complete within 5s")
            .expect("reqwest transport ok");
    assert!(response.status().is_success(), "HTTP {}", response.status());

    let value: serde_json::Value = response.json().await.expect("response must be JSON");
    if let Some(errs) = value.get("errors") {
        assert!(
            errs.as_array().map(|a| a.is_empty()).unwrap_or(false),
            "expected no GraphQL errors, got: {errs}"
        );
    }

    let schema = value.pointer("/data/__schema").expect("response.data.__schema must be present");
    assert_eq!(
        schema.pointer("/queryType/name").and_then(|v| v.as_str()),
        Some("QueryRoot"),
        "queryType name should be QueryRoot"
    );

    // Default schema now exposes both a mutation root (`createUser`)
    // and a subscription root (`tick`) — introspection must report
    // both.
    assert_eq!(
        schema.pointer("/mutationType/name").and_then(|v| v.as_str()),
        Some("MutationRoot"),
        "default schema has a mutation root named MutationRoot"
    );
    assert_eq!(
        schema.pointer("/subscriptionType/name").and_then(|v| v.as_str()),
        Some("SubscriptionRoot"),
        "default schema has a subscription root named SubscriptionRoot"
    );

    let types = schema
        .pointer("/types")
        .and_then(|v| v.as_array())
        .expect("__schema.types must be an array");
    let names: std::collections::HashSet<&str> = types
        .iter()
        .filter_map(|t| t.pointer("/name").and_then(|v| v.as_str()))
        .collect();
    for required in ["QueryRoot", "User", "Post"] {
        assert!(
            names.contains(required),
            "introspection types missing `{required}`; returned: {names:?}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_real_client_create_user_mutation_returns_user() {
    let url = spawn_server().await;
    let client = reqwest::Client::new();
    let body = json!({
        "query": r#"
            mutation CreateUser($name: String!, $email: String!) {
              createUser(name: $name, email: $email) { id name email }
            }
        "#,
        "variables": { "name": "Alice", "email": "alice@example.com" },
    });
    let response =
        tokio::time::timeout(Duration::from_secs(5), client.post(&url).json(&body).send())
            .await
            .expect("request should complete within 5s")
            .expect("reqwest transport ok");
    assert!(response.status().is_success(), "HTTP {}", response.status());

    let value: serde_json::Value = response.json().await.expect("response must be JSON");
    if let Some(errs) = value.get("errors") {
        assert!(
            errs.as_array().map(|a| a.is_empty()).unwrap_or(false),
            "expected no GraphQL errors, got: {errs}"
        );
    }

    let user = value.pointer("/data/createUser").expect("data.createUser must be present");
    assert_eq!(user.pointer("/name").and_then(|v| v.as_str()), Some("Alice"));
    assert_eq!(user.pointer("/email").and_then(|v| v.as_str()), Some("alice@example.com"));
    let id = user.pointer("/id").and_then(|v| v.as_str()).expect("id must be a string");
    assert!(id.starts_with("user-"), "expected derived id `user-<hex>`, got {id}");
    assert_eq!(id.len(), "user-".len() + 12, "expected 12-hex-char suffix, got {id}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_syntactically_invalid_query_surfaces_error_field() {
    let url = spawn_server().await;
    let client = reqwest::Client::new();
    let body = json!({ "query": "{ this is not graphql" });
    let response = client.post(&url).json(&body).send().await.expect("reqwest transport ok");
    // GraphQL returns 200 with an errors array for parse errors (per spec).
    let value: serde_json::Value = response.json().await.unwrap();
    let errors = value.pointer("/errors").and_then(|v| v.as_array());
    assert!(
        errors.is_some_and(|e| !e.is_empty()),
        "expected non-empty errors array for invalid query, got: {value}"
    );
}

/// Real-client subscription via WebSocket. Uses the
/// `graphql-transport-ws` subprotocol (the current spec; Apollo
/// defaults to this, and async-graphql-axum's GraphQLSubscription
/// service handles it). The default schema exposes a `tick`
/// subscription that yields `count` monotonically increasing ints
/// at 100ms intervals; we ask for 3 and assert we see exactly those
/// three values in order.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_real_client_subscription_ticks_over_websocket() {
    use futures::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::handshake::client::generate_key;
    use tokio_tungstenite::tungstenite::http::Request;
    use tokio_tungstenite::tungstenite::Message;

    let (_http_url, ws_url) = spawn_server_pair().await;

    // Build the upgrade request by hand so we can advertise the
    // `graphql-transport-ws` subprotocol, which the server requires.
    let parsed = url::Url::parse(&ws_url).expect("valid ws url");
    let host = parsed.host_str().expect("host");
    let port = parsed.port().expect("port");
    let req = Request::builder()
        .uri(&ws_url)
        .header("Host", format!("{host}:{port}"))
        .header("Upgrade", "websocket")
        .header("Connection", "Upgrade")
        .header("Sec-WebSocket-Key", generate_key())
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Protocol", "graphql-transport-ws")
        .body(())
        .expect("request builds");

    let (mut ws, _resp) =
        tokio::time::timeout(Duration::from_secs(5), tokio_tungstenite::connect_async(req))
            .await
            .expect("ws connect should complete within 5s")
            .expect("ws handshake ok");

    // graphql-transport-ws handshake: client sends `connection_init`,
    // server replies `connection_ack`.
    ws.send(Message::Text(r#"{"type":"connection_init"}"#.into()))
        .await
        .expect("send connection_init");
    let ack = tokio::time::timeout(Duration::from_secs(5), ws.next())
        .await
        .expect("ack arrives within 5s")
        .expect("ack is present")
        .expect("no transport error");
    let ack_text = ack.to_text().expect("ack is text");
    assert!(ack_text.contains("connection_ack"), "expected connection_ack, got: {ack_text}");

    // Open the subscription. `id` is arbitrary; the server echoes it
    // back on every `next` frame so we can demultiplex. We ask for 3
    // ticks so the stream terminates quickly.
    ws.send(Message::Text(
        r#"{"type":"subscribe","id":"1","payload":{"query":"subscription { tick(count: 3) }"}}"#
            .into(),
    ))
    .await
    .expect("send subscribe");

    // Drain `next` frames until we've collected 3 ticks or the stream
    // ends. graphql-transport-ws emits:
    //   {"type":"next","id":"1","payload":{"data":{"tick":N}}}
    // followed by a final {"type":"complete","id":"1"}.
    let mut ticks: Vec<i64> = Vec::new();
    let mut saw_complete = false;
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(5), ws.next())
            .await
            .expect("frame arrives within 5s")
            .expect("frame present")
            .expect("no transport error");
        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break,
            _ => continue, // ping/pong/binary — ignore.
        };
        let v: serde_json::Value = serde_json::from_str(&text).expect("valid json frame");
        match v.pointer("/type").and_then(|t| t.as_str()) {
            Some("next") => {
                let n = v
                    .pointer("/payload/data/tick")
                    .and_then(|x| x.as_i64())
                    .expect("tick value present in frame");
                ticks.push(n);
            }
            Some("complete") => {
                saw_complete = true;
                break;
            }
            Some("error") => panic!("subscription error frame: {v}"),
            _ => {}
        }
    }

    assert!(saw_complete, "expected a terminal `complete` frame; got {} ticks", ticks.len());
    assert_eq!(ticks, vec![1, 2, 3], "expected 3 monotonically increasing ticks");
}
