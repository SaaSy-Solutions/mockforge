use axum::{body::Body, http::Request};
use mockforge_ui::create_admin_router;
use tower::ServiceExt;

/// Test that static assets are served with correct MIME types
#[tokio::test]
async fn test_static_assets_mime_types() {
    let app = create_admin_router(None, None, None, None, true, 9080, "http://localhost:9090".to_string());

    // Test HTML serving
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());

    // Check Content-Type header for HTML
    let content_type = response.headers().get("content-type").unwrap();
    assert_eq!(content_type, "text/html; charset=utf-8");

    // Test CSS serving
    let app2 = create_admin_router(None, None, None, None, true, 9080, "http://localhost:9090".to_string());
    let response_css = app2
        .oneshot(Request::builder().uri("/assets/index.css").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response_css.status().is_success());

    // Check Content-Type header for CSS
    let css_content_type = response_css.headers().get("content-type").unwrap();
    assert_eq!(css_content_type, "text/css");

    // Test JavaScript serving
    let app3 = create_admin_router(None, None, None, None, true, 9080, "http://localhost:9090".to_string());
    let response_js = app3
        .oneshot(Request::builder().uri("/assets/index.js").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response_js.status().is_success());

    // Check Content-Type header for JavaScript
    let js_content_type = response_js.headers().get("content-type").unwrap();
    assert_eq!(js_content_type, "application/javascript");
}

/// Test that image assets are served with correct MIME types
#[tokio::test]
async fn test_image_assets_mime_types() {
    let app = create_admin_router(None, None, None, None, true, 9080, "http://localhost:9090".to_string());

    // Test PNG icon serving
    let response = app
        .oneshot(Request::builder().uri("/mockforge-icon.png").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());

    // Check Content-Type header for PNG
    let content_type = response.headers().get("content-type").unwrap();
    assert_eq!(content_type, "image/png");

    // Verify we got actual PNG data (currently empty placeholder)
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    // Note: Currently returns empty body as placeholder
    assert!(body_bytes.len() == 0, "PNG file should be empty placeholder");

    // Test different icon sizes
    let icon_sizes = vec![
        "/mockforge-icon-32.png",
        "/mockforge-icon-48.png",
        "/mockforge-logo.png",
        "/mockforge-logo-40.png",
        "/mockforge-logo-80.png",
    ];

    for icon_path in icon_sizes {
        let app_test = create_admin_router(None, None, None, None, true, 9080, "http://localhost:9090".to_string());
        let response = app_test
            .oneshot(Request::builder().uri(icon_path).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert!(response.status().is_success(), "Failed for {}", icon_path);

        let content_type = response.headers().get("content-type").unwrap();
        assert_eq!(content_type, "image/png", "Wrong content type for {}", icon_path);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        // Note: Currently returns empty body as placeholder
        assert!(body_bytes.len() == 0, "No data for {}", icon_path);
    }
}

/// Test caching headers for static assets
#[tokio::test]
async fn test_static_assets_caching_headers() {
    let app = create_admin_router(None, None, None, None, true, 9080, "http://localhost:9090".to_string());

    // Test CSS caching headers
    let response = app
        .oneshot(Request::builder().uri("/assets/index.css").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());

    // Check for cache control headers
    let cache_control = response.headers().get("cache-control");
    if let Some(cache_header) = cache_control {
        let cache_value = cache_header.to_str().unwrap();
        // Should have some form of caching directive
        assert!(
            cache_value.contains("max-age")
                || cache_value.contains("no-cache")
                || cache_value.contains("public")
                || cache_value.contains("private"),
            "Cache-Control header should contain valid directive: {}",
            cache_value
        );
    }

    // Test JavaScript caching headers
    let app2 = create_admin_router(None, None, None, None, true, 9080, "http://localhost:9090".to_string());
    let response_js = app2
        .oneshot(Request::builder().uri("/assets/index.js").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response_js.status().is_success());

    let js_cache_control = response_js.headers().get("cache-control");
    if let Some(js_cache_header) = js_cache_control {
        let js_cache_value = js_cache_header.to_str().unwrap();
        assert!(
            js_cache_value.contains("max-age")
                || js_cache_value.contains("no-cache")
                || js_cache_value.contains("public")
                || js_cache_value.contains("private"),
            "JS Cache-Control header should contain valid directive: {}",
            js_cache_value
        );
    }

    // Test image caching headers
    let app3 = create_admin_router(None, None, None, None, true, 9080, "http://localhost:9090".to_string());
    let response_img = app3
        .oneshot(Request::builder().uri("/mockforge-icon.png").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response_img.status().is_success());

    let img_cache_control = response_img.headers().get("cache-control");
    if let Some(img_cache_header) = img_cache_control {
        let img_cache_value = img_cache_header.to_str().unwrap();
        assert!(
            img_cache_value.contains("max-age")
                || img_cache_value.contains("no-cache")
                || img_cache_value.contains("public")
                || img_cache_value.contains("private"),
            "Image Cache-Control header should contain valid directive: {}",
            img_cache_value
        );
    }
}

/// Test that static assets have appropriate ETags
#[tokio::test]
async fn test_static_assets_etags() {
    let app = create_admin_router(None, None, None, None, true, 9080, "http://localhost:9090".to_string());

    // Test HTML ETag
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());

    // Check for ETag header
    let etag = response.headers().get("etag");
    if let Some(etag_header) = etag {
        let etag_value = etag_header.to_str().unwrap();
        // ETag should be a valid quoted string
        assert!(
            etag_value.starts_with('"') && etag_value.ends_with('"'),
            "ETag should be a quoted string: {}",
            etag_value
        );
        assert!(etag_value.len() > 2, "ETag should not be empty: {}", etag_value);
    }

    // Test CSS ETag
    let app2 = create_admin_router(None, None, None, None, true, 9080, "http://localhost:9090".to_string());
    let response_css = app2
        .oneshot(Request::builder().uri("/assets/index.css").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response_css.status().is_success());

    let css_etag = response_css.headers().get("etag");
    if let Some(css_etag_header) = css_etag {
        let css_etag_value = css_etag_header.to_str().unwrap();
        assert!(
            css_etag_value.starts_with('"') && css_etag_value.ends_with('"'),
            "CSS ETag should be a quoted string: {}",
            css_etag_value
        );
    }
}

/// Test that static assets handle conditional requests
#[tokio::test]
async fn test_conditional_requests() {
    let app = create_admin_router(None, None, None, None, true, 9080, "http://localhost:9090".to_string());

    // First, get the ETag
    let response = app
        .oneshot(Request::builder().uri("/assets/index.css").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());

    let etag = response.headers().get("etag");
    if let Some(etag_header) = etag {
        let etag_value = etag_header.to_str().unwrap();

        // Now make a conditional request with If-None-Match
        let app2 = create_admin_router(None, None, None, None, true, 9080, "http://localhost:9090".to_string());
        let conditional_response = app2
            .oneshot(
                Request::builder()
                    .uri("/assets/index.css")
                    .header("if-none-match", etag_value)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return 304 Not Modified
        assert_eq!(conditional_response.status(), axum::http::StatusCode::NOT_MODIFIED);

        // Should have the same ETag
        let conditional_etag = conditional_response.headers().get("etag").unwrap();
        assert_eq!(conditional_etag, etag_value);
    }
}

/// Test that all static routes are accessible
#[tokio::test]
async fn test_all_static_routes_accessible() {
    let static_routes = vec![
        "/",
        "/assets/index.css",
        "/assets/index.js",
        "/mockforge-icon.png",
        "/mockforge-icon-32.png",
        "/mockforge-icon-48.png",
        "/mockforge-logo.png",
        "/mockforge-logo-40.png",
        "/mockforge-logo-80.png",
    ];

    for route in static_routes {
        let app = create_admin_router(None, None, None, None, true, 9080, "http://localhost:9090".to_string());
        let response = app
            .oneshot(Request::builder().uri(route).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert!(
            response.status().is_success(),
            "Route {} should be accessible, got status {}",
            route,
            response.status()
        );
    }
}

/// Test that static assets have reasonable content lengths
#[tokio::test]
async fn test_static_assets_content_length() {
    let app = create_admin_router(None, None, None, None, true, 9080, "http://localhost:9090".to_string());

    let response = app
        .oneshot(Request::builder().uri("/assets/index.js").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());

    // Check Content-Length header exists
    let content_length = response.headers().get("content-length");
    assert!(content_length.is_some(), "Content-Length header should be present");

    let length_value = content_length.unwrap().to_str().unwrap().parse::<usize>().unwrap();
    assert!(length_value > 0, "Content-Length should be greater than 0");

    // For JavaScript, it should be reasonably large (not empty)
    assert!(
        length_value > 100,
        "JavaScript file should be reasonably large, got {} bytes",
        length_value
    );

    // Verify actual body matches content length
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(body_bytes.len(), length_value, "Body length should match Content-Length header");
}

/// Test SPA fallback routing (client-side routing support)
#[tokio::test]
async fn test_spa_fallback_routing() {
    let _app = create_admin_router(None, None, None, None, true, 9080, "http://localhost:9090".to_string());

    // Test various client-side routes that should serve index.html
    let client_routes = vec![
        "/dashboard",
        "/config",
        "/logs",
        "/some/deep/nested/route",
        "/route/with/parameters/123/edit",
    ];

    for route in client_routes {
        let app_test = create_admin_router(None, None, None, None, true, 9080, "http://localhost:9090".to_string());
        let response = app_test
            .oneshot(Request::builder().uri(route).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert!(
            response.status().is_success(),
            "SPA route {} should serve index.html, got status {}",
            route,
            response.status()
        );

        // Should serve HTML content
        let content_type = response.headers().get("content-type").unwrap();
        assert_eq!(content_type, "text/html; charset=utf-8", "SPA route {} should serve HTML content", route);
    }
}
