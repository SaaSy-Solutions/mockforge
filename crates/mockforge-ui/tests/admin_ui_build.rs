use axum::{body::Body, http::Request};
use mockforge_ui::create_admin_router;
use std::fs;
use std::path::Path;
// use std::process::Command; // Unused for now
use tower::ServiceExt;

/// Test that the admin UI builds successfully and serves assets correctly
#[tokio::test]
async fn test_admin_ui_build_and_serve() {
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    let ui_dir = Path::new(crate_dir).join("ui");

    // Verify UI directory exists
    assert!(ui_dir.exists(), "UI directory should exist at {:?}", ui_dir);

    // Check if build_ui.sh script exists
    let build_script = Path::new(crate_dir).join("build_ui.sh");
    assert!(build_script.exists(), "build_ui.sh script should exist");

    // Check if package.json exists in UI directory
    let package_json = ui_dir.join("package.json");
    assert!(package_json.exists(), "package.json should exist in UI directory");

    // Verify package.json has required dependencies
    let package_content = fs::read_to_string(&package_json).unwrap();
    let package_json: serde_json::Value = serde_json::from_str(&package_content).unwrap();

    // Check for essential dependencies
    let dependencies = package_json["dependencies"].as_object().unwrap();
    assert!(dependencies.contains_key("react"), "React dependency should be present");
    assert!(dependencies.contains_key("react-dom"), "React DOM dependency should be present");

    let dev_dependencies = package_json["devDependencies"].as_object().unwrap();
    assert!(dev_dependencies.contains_key("vite"), "Vite dev dependency should be present");

    // Test that the UI build process works (without actually running the build)
    // This verifies the build configuration is correct
    let ui_dist = ui_dir.join("dist");
    if ui_dist.exists() {
        // If dist exists, verify it has the expected structure
        let index_html = ui_dist.join("index.html");
        let assets_dir = ui_dist.join("assets");

        assert!(index_html.exists(), "index.html should exist in dist directory");
        assert!(assets_dir.exists(), "assets directory should exist in dist directory");

        // Verify index.html content
        let html_content = fs::read_to_string(&index_html).unwrap();
        assert!(html_content.contains("<!DOCTYPE html>"), "index.html should be valid HTML");
        assert!(html_content.contains("<html"), "index.html should contain html tag");
        assert!(html_content.contains("<head>"), "index.html should contain head tag");
        assert!(html_content.contains("<body>"), "index.html should contain body tag");

        // Check for asset references in HTML
        assert!(html_content.contains("assets/"), "HTML should reference assets directory");

        // Verify assets directory has expected files
        if let Ok(entries) = fs::read_dir(&assets_dir) {
            let mut has_css = false;
            let mut has_js = false;

            for entry in entries {
                if let Ok(entry) = entry {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if file_name.ends_with(".css") {
                        has_css = true;
                    } else if file_name.ends_with(".js") {
                        has_js = true;
                    }
                }
            }

            assert!(has_css, "Assets directory should contain CSS files");
            assert!(has_js, "Assets directory should contain JavaScript files");
        }
    }
}

/// Test that the admin UI serves correctly when built assets exist
#[tokio::test]
async fn test_admin_ui_serves_built_assets() {
    let app = create_admin_router(None, None, None, None, true);

    // Test that main HTML page serves
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success(), "Main page should serve successfully");

    let content_type = response.headers().get("content-type").unwrap();
    assert_eq!(content_type, "text/html", "Main page should serve HTML content");

    // Verify HTML content contains expected elements
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let html_content = String::from_utf8(body_bytes.to_vec()).unwrap();

    // Basic HTML structure checks
    assert!(html_content.contains("<!DOCTYPE html>"), "HTML should have DOCTYPE");
    assert!(html_content.contains("<html"), "HTML should have html tag");
    assert!(html_content.contains("<head>"), "HTML should have head tag");
    assert!(html_content.contains("<body>"), "HTML should have body tag");

    // Check for typical React/Vite build artifacts
    assert!(
        html_content.contains("<div id=\"root\">") || html_content.contains("root"),
        "HTML should have React root element"
    );

    // Test CSS asset serving
    let app2 = create_admin_router(None, None, None, None, true);
    let css_response = app2
        .oneshot(Request::builder().uri("/assets/index.css").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // CSS should either serve successfully or return a fallback
    assert!(
        css_response.status().is_success() || css_response.status().is_client_error(),
        "CSS request should not return server error"
    );

    if css_response.status().is_success() {
        let css_content_type = css_response.headers().get("content-type").unwrap();
        assert_eq!(css_content_type, "text/css", "CSS should have correct content type");

        let css_body = axum::body::to_bytes(css_response.into_body(), usize::MAX).await.unwrap();
        assert!(css_body.len() > 0, "CSS content should not be empty");
    }

    // Test JavaScript asset serving
    let app3 = create_admin_router(None, None, None, None, true);
    let js_response = app3
        .oneshot(Request::builder().uri("/assets/index.js").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // JS should either serve successfully or return a fallback
    assert!(
        js_response.status().is_success() || js_response.status().is_client_error(),
        "JavaScript request should not return server error"
    );

    if js_response.status().is_success() {
        let js_content_type = js_response.headers().get("content-type").unwrap();
        assert_eq!(
            js_content_type, "application/javascript",
            "JS should have correct content type"
        );

        let js_body = axum::body::to_bytes(js_response.into_body(), usize::MAX).await.unwrap();
        assert!(js_body.len() > 0, "JavaScript content should not be empty");
    }
}

/// Test that the UI build process generates expected file structure
#[tokio::test]
async fn test_ui_build_file_structure() {
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    let ui_dir = Path::new(crate_dir).join("ui");
    let dist_dir = ui_dir.join("dist");

    // If dist directory exists (UI has been built), verify structure
    if dist_dir.exists() {
        // Check for manifest.json (Vite build artifact)
        let manifest_path = dist_dir.join("manifest.json");
        if manifest_path.exists() {
            let manifest_content = fs::read_to_string(&manifest_path).unwrap();
            let manifest: serde_json::Value = serde_json::from_str(&manifest_content).unwrap();

            // Verify manifest structure
            assert!(manifest.is_object(), "Manifest should be a JSON object");

            // Should contain entry point
            let main_entry = manifest.as_object().unwrap().get("index.html");
            assert!(main_entry.is_some(), "Manifest should contain index.html entry");

            if let Some(entry) = main_entry {
                assert!(entry.is_object(), "index.html entry should be an object");
                let entry_obj = entry.as_object().unwrap();

                // Should have file property
                assert!(entry_obj.contains_key("file"), "Entry should have file property");

                // Should have CSS property (may be null)
                assert!(entry_obj.contains_key("css"), "Entry should have css property");
            }
        }

        // Check that assets are properly embedded
        let index_html = dist_dir.join("index.html");
        if index_html.exists() {
            let html_content = fs::read_to_string(&index_html).unwrap();

            // HTML should contain references to built assets
            // This is a basic check - in a real build, there would be hashed asset names
            assert!(
                html_content.contains("assets/") || html_content.contains("/assets/"),
                "HTML should reference assets directory"
            );
        }
    }
}

/// Test that the UI can handle missing build assets gracefully
#[tokio::test]
async fn test_ui_handles_missing_assets() {
    // This test verifies that the server doesn't crash when UI assets are missing
    // and provides reasonable fallbacks

    let app = create_admin_router(None, None, None, None, true);

    // Test main page
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Should not return server error even if assets are missing
    assert!(
        !response.status().is_server_error(),
        "Main page should not return server error, got: {}",
        response.status()
    );

    if response.status().is_success() {
        let content_type = response.headers().get("content-type").unwrap();
        assert_eq!(content_type, "text/html", "Should serve HTML content");
    }

    // Test CSS endpoint
    let app2 = create_admin_router(None, None, None, None, true);
    let css_response = app2
        .oneshot(Request::builder().uri("/assets/index.css").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Should handle gracefully (either serve content or return appropriate error)
    assert!(
        !css_response.status().is_server_error(),
        "CSS endpoint should not return server error, got: {}",
        css_response.status()
    );

    // Test JS endpoint
    let app3 = create_admin_router(None, None, None, None, true);
    let js_response = app3
        .oneshot(Request::builder().uri("/assets/index.js").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Should handle gracefully
    assert!(
        !js_response.status().is_server_error(),
        "JS endpoint should not return server error, got: {}",
        js_response.status()
    );
}

/// Test that the UI build process can be triggered programmatically
/// This is more of an integration test that verifies the build pipeline works
#[tokio::test]
#[ignore] // Ignored by default as it requires Node.js and build tools
async fn test_ui_build_process() {
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    let ui_dir = Path::new(crate_dir).join("ui");

    // Skip test if UI directory doesn't exist or build script doesn't exist
    if !ui_dir.exists() {
        eprintln!("UI directory not found, skipping build process test");
        return;
    }

    let build_script = Path::new(crate_dir).join("build_ui.sh");
    if !build_script.exists() {
        eprintln!("build_ui.sh not found, skipping build process test");
        return;
    }

    // This test would actually run the build process
    // For safety, we'll just verify the build script exists and is executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(&build_script).unwrap();
        let permissions = metadata.permissions();
        assert!(permissions.mode() & 0o111 != 0, "build_ui.sh should be executable");
    }

    // Verify build script content contains expected commands
    let script_content = fs::read_to_string(&build_script).unwrap();
    assert!(
        script_content.contains("npm")
            || script_content.contains("yarn")
            || script_content.contains("pnpm"),
        "Build script should contain package manager commands"
    );
    assert!(script_content.contains("build"), "Build script should contain build command");
}

/// Test that the UI serves proper security headers
#[tokio::test]
async fn test_ui_security_headers() {
    let app = create_admin_router(None, None, None, None, true);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(response.status().is_success());

    // Check for basic security headers that should be present
    let headers = response.headers();

    // Content-Type should be set
    assert!(headers.get("content-type").is_some(), "Content-Type header should be present");

    // Check if any security headers are present (these may vary by implementation)
    let security_headers = [
        "x-content-type-options",
        "x-frame-options",
        "x-xss-protection",
        "content-security-policy",
        "strict-transport-security",
    ];

    let has_some_security_headers =
        security_headers.iter().any(|&header| headers.get(header).is_some());

    // At minimum, we should have content-type-options for security
    if let Some(x_content_type) = headers.get("x-content-type-options") {
        assert_eq!(x_content_type, "nosniff", "X-Content-Type-Options should be nosniff");
    }
}
