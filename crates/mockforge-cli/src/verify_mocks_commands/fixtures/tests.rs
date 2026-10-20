use super::*;
use serde_json::json;
use std::fs;

fn fixture(path: &str, body: &str) -> Value {
    json!({"fingerprint":{"method":"GET","path":path,"query":"","headers":{},"body_hash":null},
        "timestamp":"2026-10-20T00:00:00Z","status_code":200,"response_headers":{"content-type":"application/json"},
        "response_body":body,"metadata":{}})
}

fn registry() -> (OpenApiRouteRegistry, Value) {
    let spec = json!({"openapi":"3.0.3","info":{"title":"fixtures","version":"1"},"paths":{
        "/items":{"get":{"responses":{"200":{"description":"ok","content":{"application/json":{
            "schema":{"type":"object","required":["id"],"properties":{"id":{"type":"integer"}}}
        }}}}}}
    }});
    (
        OpenApiRouteRegistry::new(super::super::OpenApiSpec::from_json(spec.clone()).unwrap()),
        spec,
    )
}

#[test]
fn valid_fixture_matches_and_invalid_response_preserves_field_details() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.json");
    let (registry, spec) = registry();
    fs::write(&path, fixture("/items", r#"{"id":1}"#).to_string()).unwrap();
    let before = fs::read(&path).unwrap();
    let report = verify(&path, &registry, &spec).unwrap();
    assert_eq!(report.files_checked, 1);
    assert_eq!(report.routes[0].status, "verified");
    assert_eq!(fs::read(&path).unwrap(), before);
    fs::write(&path, fixture("/items", r#"{"id":"wrong"}"#).to_string()).unwrap();
    let report = verify(&path, &registry, &spec).unwrap();
    assert_eq!(report.routes[0].change, "changed");
    let fields = &report.routes[0].findings[0].response.as_ref().unwrap().fields;
    assert!(fields.iter().any(|f| f.field.contains("id")));
}

#[test]
fn reports_source_operations_without_fixtures_and_removed_fixture_routes() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("fixture.json"), fixture("/old", "{}").to_string()).unwrap();
    let (registry, spec) = registry();
    let report = verify(dir.path(), &registry, &spec).unwrap();
    assert!(report.routes.iter().any(|r| r.endpoint == "GET /items"
        && r.change == "added"
        && r.findings[0].kind == "missing-fixture"));
    assert!(report.routes.iter().any(|r| r.endpoint == "GET /old" && r.change == "removed"));
}

#[test]
fn root_selects_http_subtree_and_rejects_empty_or_malformed_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let (registry, spec) = registry();
    assert!(verify(dir.path(), &registry, &spec).is_err());
    fs::write(dir.path().join("not-http.json"), "not an HTTP fixture").unwrap();
    fs::create_dir(dir.path().join("http")).unwrap();
    fs::write(
        dir.path().join("http/sample.json"),
        fixture("/items", r#"{"id":1}"#).to_string(),
    )
    .unwrap();
    assert_eq!(verify(dir.path(), &registry, &spec).unwrap().files_checked, 1);
    fs::write(dir.path().join("http/broken.json"), "{}").unwrap();
    assert!(verify(dir.path(), &registry, &spec).is_err());
}

#[cfg(unix)]
#[test]
fn fixture_symlinks_fail_without_following_external_data() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("real.json");
    fs::write(&file, fixture("/items", r#"{"id":1}"#).to_string()).unwrap();
    let link = dir.path().join("link.json");
    std::os::unix::fs::symlink(&file, &link).unwrap();
    let (registry, spec) = registry();
    assert!(verify(&link, &registry, &spec).is_err());
}
