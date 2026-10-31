//! Management credentials must not be persisted by the HTTP recorder.

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{header::AUTHORIZATION, Request, StatusCode},
    middleware,
    routing::get,
    Router,
};
use mockforge_http::management_auth::{install, protect, ControlPlanePolicy};
use mockforge_recorder::{
    recording_middleware,
    scrubbing::{CaptureFilter, Scrubber},
    Recorder, RecorderDatabase,
};
use std::{net::SocketAddr, sync::Arc};
use tower::ServiceExt;

#[tokio::test]
async fn duplicate_authorization_headers_do_not_record_the_management_token() {
    let token = "mfm_synthetic_management_token_0123456789abcdef";
    let bearer = format!("Bearer {token}");
    let db = RecorderDatabase::new_in_memory().await.unwrap();
    let recorder = Arc::new(Recorder::with_scrubbing(
        db.clone(),
        Scrubber::default(),
        CaptureFilter::default(),
    ));
    let app = install(
        protect(Router::new().route("/__mockforge/recorded", get(|| async { "ok" })))
            .merge(Router::new().route("/mock", get(|| async { "ok" })))
            .layer(middleware::from_fn_with_state(
                recorder,
                |State(recorder): State<Arc<Recorder>>,
                 addr: ConnectInfo<SocketAddr>,
                 req,
                 next| { recording_middleware(recorder, addr, req, next) },
            )),
        ControlPlanePolicy::with_token(token).unwrap(),
    );

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
