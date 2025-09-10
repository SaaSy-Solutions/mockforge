use axum::{http::Request, body::Body};
use tower::ServiceExt; // for `oneshot`

#[tokio::test]
async fn serves_root_and_assets_and_health() {
    // admin router at root
    let app = mockforge_ui::create_admin_router(None, None, None, true);

    // /
    let res = app.clone().oneshot(Request::builder().uri("/").body(Body::empty()).unwrap()).await.unwrap();
    assert!(res.status().is_success());

    // /admin.css
    let res = app.clone().oneshot(Request::builder().uri("/admin.css").body(Body::empty()).unwrap()).await.unwrap();
    assert!(res.status().is_success());

    // /admin.js
    let res = app.clone().oneshot(Request::builder().uri("/admin.js").body(Body::empty()).unwrap()).await.unwrap();
    assert!(res.status().is_success());

    // /__mockforge/health
    let res = app.clone().oneshot(Request::builder().uri("/__mockforge/health").body(Body::empty()).unwrap()).await.unwrap();
    assert!(res.status().is_success());
}

#[tokio::test]
async fn works_under_mount_prefix() {
    // router nested under /admin
    let sub = mockforge_ui::create_admin_router(None, None, None, true);
    let app = axum::Router::new().nest("/admin", sub);

    // /admin (nested root)
    let res = app.clone().oneshot(Request::builder().uri("/admin").body(Body::empty()).unwrap()).await.unwrap();
    assert!(res.status().is_success());

    // /admin/admin.css
    let res = app.clone().oneshot(Request::builder().uri("/admin/admin.css").body(Body::empty()).unwrap()).await.unwrap();
    assert!(res.status().is_success());

    // /admin/admin.js
    let res = app.clone().oneshot(Request::builder().uri("/admin/admin.js").body(Body::empty()).unwrap()).await.unwrap();
    assert!(res.status().is_success());

    // /admin/__mockforge/health
    let res = app.clone().oneshot(Request::builder().uri("/admin/__mockforge/health").body(Body::empty()).unwrap()).await.unwrap();
    assert!(res.status().is_success());
}
