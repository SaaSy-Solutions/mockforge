//! These controls also run with `--no-default-features`: configuration must
//! remain portable between builds, and basic request handling must still work.

use axum::http::{HeaderMap, Method, Uri};
use mockforge_core::{
    config::ServerConfig, PriorityHttpHandler, RecordReplayHandler, SimpleMockGenerator,
};

#[test]
fn optional_configuration_survives_json_and_yaml_roundtrips() {
    let mut config = ServerConfig::default();
    config.multi_tenant.workspace_prefix = "/portable".into();
    config.multi_tenant.max_workspaces = Some(17);
    config.reality_continuum.default_ratio = 0.375;
    config.drift_budget.incident_retention_days = 47;
    config.core.time_travel.scale_factor = 2.5;
    config.behavioral_economics.global_sensitivity = 0.75;

    let json = serde_json::to_string(&config).unwrap();
    let yaml = serde_yaml::to_string(&config).unwrap();
    for restored in [
        serde_json::from_str::<ServerConfig>(&json).unwrap(),
        serde_yaml::from_str::<ServerConfig>(&yaml).unwrap(),
    ] {
        assert_eq!(restored.multi_tenant.workspace_prefix, "/portable");
        assert_eq!(restored.multi_tenant.max_workspaces, Some(17));
        assert_eq!(restored.reality_continuum.default_ratio, 0.375);
        assert_eq!(restored.drift_budget.incident_retention_days, 47);
        assert_eq!(restored.core.time_travel.scale_factor, 2.5);
        assert_eq!(restored.behavioral_economics.global_sensitivity, 0.75);
    }
}

#[tokio::test]
async fn basic_mock_request_preserves_status_and_body() {
    let fixtures = tempfile::tempdir().unwrap();
    let record_replay = RecordReplayHandler::new(fixtures.path().into(), false, false, false);
    let handler = PriorityHttpHandler::new(
        record_replay,
        None,
        None,
        Some(Box::new(SimpleMockGenerator::new(201, r#"{"created":true}"#.into()))),
    );
    let response = handler
        .process_request(&Method::GET, &Uri::from_static("/basic"), &HeaderMap::new(), None)
        .await
        .unwrap();
    assert_eq!(response.status_code, 201);
    assert_eq!(response.body, br#"{"created":true}"#);
    assert_eq!(response.source.source_type, "mock");
}
