//! Synthetic end-to-end authentication event and diagnostic privacy regressions.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware,
    routing::get,
    Router,
};
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use jsonwebtoken::{encode, EncodingKey, Header};
use mockforge_core::{
    config::{ApiKeyConfig, AuthConfig, BasicAuthConfig, JwtConfig, OAuth2Config},
    security::{
        emitter::init_global_siem_emitter,
        siem::{SiemConfig, SiemDestination},
    },
};
use mockforge_http::auth::{auth_middleware, state::CachedIntrospection, AuthResult, AuthState};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    io::{self, Write},
    sync::{Arc, Mutex},
};
use tokio::sync::RwLock;
use tower::ServiceExt;

#[derive(Clone, Default)]
struct Logs(Arc<Mutex<Vec<u8>>>);

impl Write for Logs {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Exercise the actual middleware, authenticator, emitter and JSONL transport.
/// One test owns the process-global emitter and tracing subscriber throughout.
#[tokio::test]
async fn submitted_credentials_never_reach_events_or_logs() {
    let logs = Logs::default();
    let writer = logs.clone();
    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_ansi(false)
            .with_writer(move || writer.clone())
            .finish(),
    )
    .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let events_path = temp.path().join("synthetic-events.jsonl");
    init_global_siem_emitter(SiemConfig {
        enabled: true,
        protocol: None,
        filters: None,
        destinations: vec![SiemDestination::File {
            path: events_path.to_str().unwrap().to_owned(),
            format: "jsonl".into(),
            rotation: None,
        }],
    })
    .await
    .unwrap();

    let fake_password = "synthetic-rejected-password-71";
    let fake_username = "synthetic-unverified-user-83";
    let credentials = format!("{fake_username}:{fake_password}");
    let encoded_basic = STANDARD.encode(&credentials);
    let no_colon = STANDARD.encode("synthetic-no-colon-credentials-43");
    let invalid_utf8 = STANDARD.encode([0xff, 0xfe, 0xfd]);
    let fake_token = "synthetic-rejected-bearer-29";
    let fake_key = "synthetic-rejected-api-key-91";
    let fake_claim = "synthetic-parser-secret-67";
    let signing_key = "synthetic-local-signing-key-long-enough-for-hs256";
    let token = |claims: Value| {
        encode(&Header::default(), &claims, &EncodingKey::from_secret(signing_key.as_bytes()))
            .unwrap()
    };
    let wrong_signature = encode(
        &Header::default(),
        &json!({"sub":"verified-user", "exp":4102444800u64}),
        &EncodingKey::from_secret(b"synthetic-wrong-signing-key"),
    )
    .unwrap();
    let invalid_claim = token(json!(fake_claim));
    let expired = token(json!({"exp":1}));
    let bad_header =
        format!("{}.e30.c2ln", URL_SAFE_NO_PAD.encode(json!({"alg":fake_claim}).to_string()));
    let valid = token(json!({"sub":"verified-user", "exp":4102444800u64}));
    let state = AuthState {
        config: AuthConfig {
            require_auth: true,
            jwt: Some(JwtConfig {
                secret: Some(signing_key.into()),
                rsa_public_key: None,
                ecdsa_public_key: None,
                issuer: None,
                audience: None,
                algorithms: vec!["HS256".into()],
            }),
            basic_auth: Some(BasicAuthConfig {
                credentials: HashMap::from([(
                    "verified-basic-user".into(),
                    "synthetic-valid-password".into(),
                )]),
            }),
            api_key: Some(ApiKeyConfig {
                header_name: "X-API-Key".into(),
                query_name: Some("api_key".into()),
                keys: vec!["synthetic-valid-api-key".into()],
            }),
            oauth2: None,
        },
        spec: None,
        oauth2_client: None,
        introspection_cache: Arc::new(RwLock::new(HashMap::new())),
    };
    let app = |state| {
        Router::new()
            .route("/private", get(|| async { "allowed" }))
            .layer(middleware::from_fn_with_state(state, auth_middleware))
    };

    let cases = vec![
        ("authorization", format!("Bearer {fake_token}"), "/private".to_owned()),
        ("authorization", format!("Bearer {wrong_signature}"), "/private".to_owned()),
        ("authorization", format!("Bearer {invalid_claim}"), "/private".to_owned()),
        ("authorization", format!("Bearer {expired}"), "/private".to_owned()),
        ("authorization", format!("Bearer {bad_header}"), "/private".to_owned()),
        ("authorization", format!("Basic {encoded_basic}"), "/private".to_owned()),
        ("authorization", format!("Basic {fake_password}!"), "/private".to_owned()),
        ("authorization", format!("Basic {no_colon}"), "/private".to_owned()),
        ("authorization", format!("Basic {invalid_utf8}"), "/private".to_owned()),
        ("X-API-Key", fake_key.into(), "/private".to_owned()),
        ("user-agent", "synthetic-client".into(), format!("/private?api_key={fake_key}")),
        ("authorization", "Bearer ".into(), "/private".to_owned()),
        ("authorization", "Basic ".into(), "/private".to_owned()),
        ("authorization", format!("Unknown {fake_token}"), "/private".to_owned()),
    ];
    let mut expected_events = 0;
    for (header, value, uri) in cases {
        let response = app(state.clone())
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header("user-agent", "synthetic-client")
                    .header("x-real-ip", "192.0.2.10")
                    .header(header, value)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(response.headers().contains_key("www-authenticate"));
        expected_events += 1;
    }
    // Inactive/expired OAuth tokens use the real cached introspection path; no network.
    for result in [
        AuthResult::TokenInvalid("Token is not active".into()),
        AuthResult::TokenExpired,
    ] {
        let mut oauth_state = state.clone();
        oauth_state.config.jwt = None;
        oauth_state.config.oauth2 = Some(OAuth2Config {
            client_id: "synthetic-client".into(),
            client_secret: "synthetic-client-secret".into(),
            introspection_url: "http://127.0.0.1:1/unused".into(),
            auth_url: None,
            token_url: None,
            token_type_hint: None,
        });
        oauth_state.introspection_cache.write().await.insert(
            fake_token.into(),
            CachedIntrospection {
                result,
                expires_at: 4102444800,
            },
        );
        let response = app(oauth_state)
            .oneshot(
                Request::builder()
                    .uri("/private")
                    .header("authorization", format!("Bearer {fake_token}"))
                    .header("user-agent", "synthetic-client")
                    .header("x-real-ip", "192.0.2.10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        expected_events += 1;
    }
    let failure_events = std::fs::read_to_string(&events_path).unwrap();
    let events: Vec<Value> =
        failure_events.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
    assert_eq!(events.len(), expected_events);
    for event in &events {
        assert!(event["actor"]["username"].is_null());
        assert!(event["actor"]["user_id"].is_null());
        assert_eq!(event["actor"]["ip_address"], "192.0.2.10");
        assert_eq!(event["actor"]["user_agent"], "synthetic-client");
        assert_eq!(event["target"]["resource_id"], "/private");
        assert_eq!(event["target"]["method"], "GET");
        assert_eq!(event["outcome"]["success"], false);
        assert!(event["outcome"]["reason"].as_str().unwrap().len() > 5);
    }
    assert!(failure_events.contains("InvalidSignature"));
    assert!(failure_events.contains("InvalidJson"));
    assert!(failure_events.contains("Invalid credentials"));

    // Successful authentication still reaches the handler and retains verified identity.
    for (header, value) in [
        ("authorization", format!("Bearer {valid}")),
        (
            "authorization",
            format!("Basic {}", STANDARD.encode("verified-basic-user:synthetic-valid-password")),
        ),
        ("X-API-Key", "synthetic-valid-api-key".into()),
    ] {
        let response = app(state.clone())
            .oneshot(
                Request::builder()
                    .uri("/private")
                    .header(header, value)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    let all_events = std::fs::read_to_string(&events_path).unwrap();
    assert!(all_events.contains("verified-user"));
    let diagnostics = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
    for secret in [
        fake_password,
        fake_username,
        &credentials,
        &encoded_basic,
        &no_colon,
        &invalid_utf8,
        fake_token,
        fake_key,
        fake_claim,
        &wrong_signature,
        &invalid_claim,
        &expired,
        &bad_header,
        &valid,
        "synthetic-valid-password",
        "synthetic-valid-api-key",
    ] {
        for representation in [
            secret.to_owned(),
            STANDARD.encode(secret),
            URL_SAFE_NO_PAD.encode(secret),
        ] {
            assert!(
                !all_events.contains(&representation),
                "credential appeared in serialized events"
            );
            assert!(
                !diagnostics.contains(&representation),
                "credential appeared in diagnostic logs"
            );
        }
    }
}
