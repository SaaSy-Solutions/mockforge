//! The explicit AI-authoring startup path must enforce admin bind credentials.

#[tokio::test]
async fn authoring_startup_refuses_non_loopback_without_credentials() {
    // This integration binary has one test and owns its process environment.
    for name in [
        "MOCKFORGE_MANAGEMENT_TOKEN",
        "MOCKFORGE_ADMIN_PASSWORD",
        "JWT_SECRET",
    ] {
        std::env::remove_var(name);
    }
    std::env::set_var("ENVIRONMENT", "production");
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        mockforge_ui::start_admin_server_notify_with_authoring(
            "192.0.2.1:0".parse().unwrap(),
            None,
            None,
            None,
            None,
            true,
            "http://127.0.0.1:9".to_owned(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(mockforge_ui::handlers::ai_studio::AiAuthoringDefaults::new(Default::default())),
        ),
    )
    .await
    .expect("unauthenticated startup must fail before serving");
    let error = result.expect_err("non-loopback admin requires credentials").to_string();
    assert!(error.contains("without credentials"), "unexpected startup failure: {error}");
}
