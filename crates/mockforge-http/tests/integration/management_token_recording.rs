//! Management credentials must not be persisted by the HTTP recorder.

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{header::AUTHORIZATION, HeaderMap, Request, StatusCode},
    middleware,
    routing::get,
    Router,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use mockforge_http::management_auth::{install, protect, AdminCredential, ControlPlanePolicy};
use mockforge_recorder::{
    recording_middleware,
    scrubbing::{CaptureFilter, Scrubber},
    Recorder, RecorderDatabase,
};
use std::{net::SocketAddr, sync::Arc};
use tower::ServiceExt;

async fn recorded_app(policy: ControlPlanePolicy) -> (Router, RecorderDatabase) {
    let db = RecorderDatabase::new_in_memory().await.unwrap();
    let recorder = Arc::new(Recorder::with_scrubbing(
        db.clone(),
        Scrubber::default(),
        CaptureFilter::default(),
    ));
    let app = install(
        protect(
            Router::new()
                .route("/__mockforge/recorded", get(|| async { "ok" }))
                .route("/api/chaos/recorded", get(|| async { "ok" })),
        )
        .merge(Router::new().route(
            "/mock",
            get(|headers: HeaderMap| async move {
                headers
                    .get(AUTHORIZATION)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("")
                    .to_owned()
            }),
        ))
        .layer(middleware::from_fn_with_state(
            recorder,
            |State(recorder): State<Arc<Recorder>>, addr: ConnectInfo<SocketAddr>, req, next| {
                recording_middleware(recorder, addr, req, next)
            },
        )),
        policy,
    );
    (app, db)
}

#[tokio::test]
async fn duplicate_authorization_headers_do_not_record_the_management_token() {
    let token = "mfm_synthetic_management_token_0123456789abcdef";
    let bearer = format!("Bearer {token}");
    let (app, db) = recorded_app(ControlPlanePolicy::with_token(token).unwrap()).await;

    for path in ["/__mockforge/recorded", "/mock"] {
        for credentials in [
            ["Bearer mock-user-token", bearer.as_str()],
            [bearer.as_str(), "Bearer mock-user-token"],
        ] {
            db.clear_all().await.unwrap();
            let request = Request::builder()
                .uri(path)
                .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 40000))))
                .header(AUTHORIZATION, credentials[0])
                .header(AUTHORIZATION, credentials[1])
                .body(Body::empty())
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let recorded = db.list_recent(10).await.unwrap();
            assert_eq!(recorded.len(), 1, "the recorder must actually persist the request");
            assert_eq!(recorded[0].path, path);
            assert!(!recorded[0].headers.contains(token), "management token was recorded");
        }
    }
}

#[tokio::test]
async fn admin_basic_credentials_are_redacted_but_mock_basic_is_preserved() {
    let admin = format!("Basic {}", STANDARD.encode("operator:synthetic-admin-password"));
    let ordinary = format!("Basic {}", STANDARD.encode("mock-user:synthetic-mock-password"));
    let policy = ControlPlanePolicy::locked()
        .with_admin(AdminCredential::new("operator", "synthetic-admin-password"));
    let (app, db) = recorded_app(policy).await;
    for path in ["/__mockforge/recorded", "/api/chaos/recorded", "/mock"] {
        for credentials in [
            vec![admin.as_str()],
            vec![ordinary.as_str(), admin.as_str()],
            vec![admin.as_str(), ordinary.as_str()],
        ] {
            db.clear_all().await.unwrap();
            let mut request = Request::builder()
                .uri(path)
                .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 40000))));
            for credential in credentials {
                request = request.header(AUTHORIZATION, credential);
            }
            let response = app.clone().oneshot(request.body(Body::empty()).unwrap()).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let recorded = db.list_recent(10).await.unwrap();
            assert_eq!(recorded.len(), 1);
            assert!(!recorded[0].headers.contains(&admin), "admin Basic credentials were recorded");
        }
    }
    db.clear_all().await.unwrap();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/mock")
                .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 40000))))
                .header(AUTHORIZATION, &ordinary)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
    assert_eq!(
        body.as_ref(),
        ordinary.as_bytes(),
        "mock handler must receive its Basic credentials"
    );
    let recorded = db.list_recent(10).await.unwrap();
    assert_eq!(recorded.len(), 1);
    assert!(recorded[0].headers.contains(&ordinary));
}
