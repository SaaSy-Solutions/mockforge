//! Downstream compatibility between the extracted auth crate and HTTP exports.
use axum::{
    body::Body,
    extract::Extension,
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use mockforge_auth::{
    auth::{AuthConfig, BasicAuthConfig},
    AuthClaims, AuthState,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tower::ServiceExt;

// The `mockforge_http::auth::` paths are the point: they prove the legacy
// re-exports name the same types as the extracted crate.
#[allow(unused_qualifications)]
#[tokio::test]
async fn extracted_state_and_claims_work_through_the_legacy_http_middleware() {
    let direct = AuthState {
        config: AuthConfig {
            require_auth: true,
            api_key: None,
            basic_auth: Some(BasicAuthConfig {
                credentials: HashMap::from([(
                    "fixture-user".into(),
                    "fixture-only-password".into(),
                )]),
            }),
            ..Default::default()
        },
        spec: None,
        oauth2_client: None,
        introspection_cache: Arc::new(RwLock::new(HashMap::new())),
    };
    let legacy: mockforge_http::auth::AuthState = direct;
    let app = Router::new()
        .route(
            "/guarded",
            get(|Extension(claims): Extension<AuthClaims>, req: Request<Body>| async move {
                let legacy = req
                    .extensions()
                    .get::<Extension<mockforge_http::auth::AuthClaims>>()
                    .expect("legacy wrapped claims remain available");
                assert_eq!(legacy.0.username, claims.username);
                claims.username.unwrap_or_default()
            }),
        )
        .layer(axum::middleware::from_fn_with_state(
            legacy,
            mockforge_http::auth::auth_middleware,
        ));
    let denied = app
        .clone()
        .oneshot(Request::builder().uri("/guarded").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
    let credential = format!("Basic {}", STANDARD.encode("fixture-user:fixture-only-password"));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/guarded")
                .header("Authorization", credential)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        axum::body::to_bytes(response.into_body(), 64).await.unwrap().as_ref(),
        b"fixture-user"
    );
}

#[test]
fn oauth_and_risk_public_state_types_keep_their_identity() {
    use std::any::TypeId;
    assert_eq!(
        TypeId::of::<mockforge_auth::oauth2_server::OAuth2ServerState>(),
        TypeId::of::<mockforge_http::handlers::oauth2_server::OAuth2ServerState>()
    );
    assert_eq!(
        TypeId::of::<mockforge_auth::auth::risk_engine::RiskEngine>(),
        TypeId::of::<mockforge_http::auth::risk_engine::RiskEngine>()
    );
    assert_eq!(
        TypeId::of::<mockforge_auth::auth::token_lifecycle::TokenLifecycleManager>(),
        TypeId::of::<mockforge_http::auth::token_lifecycle::TokenLifecycleManager>()
    );
}
