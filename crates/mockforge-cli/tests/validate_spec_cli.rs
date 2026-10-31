//! `mockforge validate-spec` end to end: exit codes, and `--format json`
//! printing one document and nothing else on stdout.
#![allow(deprecated)]
use assert_cmd::prelude::*;
use std::process::Command;

fn run(spec: &std::path::Path) -> (i32, serde_json::Value) {
    let output = Command::cargo_bin("mockforge")
        .unwrap()
        .args(["validate-spec", "--format", "json"])
        .arg(spec)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let document = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout is not one JSON document ({e}): {stdout}"));
    (output.status.code().unwrap(), document)
}

#[test]
fn validate_spec_json_exits_by_verdict_and_prints_only_the_document() {
    let dir = tempfile::tempdir().unwrap();
    let valid = dir.path().join("api.yaml");
    std::fs::write(
        &valid,
        "openapi: 3.0.3\ninfo:\n  title: Users\n  version: \"1.0\"\npaths:\n  /users:\n    get:\n      responses:\n        \"200\":\n          description: ok\n",
    )
    .unwrap();
    let (code, document) = run(&valid);
    assert_eq!(code, 0, "{document}");
    assert_eq!(document["schema_version"], 1);
    assert_eq!(document["openapi"], "3.0");
    assert_eq!(document["valid"], true);

    let invalid = dir.path().join("broken.json");
    std::fs::write(&invalid, r#"{"openapi":"3.1.0","paths":{}}"#).unwrap();
    let (code, document) = run(&invalid);
    assert_eq!(code, 1, "{document}");
    assert_eq!(document["valid"], false);
    assert!(document["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["rule"] == "MISSING_INFO" && f["path"] == "/info"));

    let (code, document) = run(&dir.path().join("missing.yaml"));
    assert_eq!(code, 2, "{document}");
    assert_eq!(document["openapi"], serde_json::Value::Null);
    assert_eq!(document["findings"][0]["rule"], "UNREADABLE");

    // Nothing was written beside the specs.
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
}
