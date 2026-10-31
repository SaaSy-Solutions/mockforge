use super::*;
use serde_json::json;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("mockforge-freeze-{}", uuid::Uuid::new_v4())))
    }
    fn freezer(&self) -> ArtifactFreezer {
        ArtifactFreezer::with_base_dir(&self.0)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn request(format: &str) -> FreezeRequest {
    FreezeRequest {
        artifact_type: "mock".into(),
        content: json!({"name":"fixed fixture","value":42}),
        format: format.into(),
        path: None,
        metadata: Some(FreezeMetadata {
            llm_provider: None,
            llm_model: None,
            llm_version: None,
            prompt_hash: Some("prompt-identity".into()),
            output_hash: None,
            original_prompt: None,
        }),
    }
}

#[tokio::test]
async fn first_use_of_missing_directory_is_a_cache_miss() {
    let fixture = Fixture::new();
    assert!(fixture
        .freezer()
        .load_frozen("mock", Some("new-prompt"))
        .await
        .unwrap()
        .is_none());
    assert!(!fixture.0.exists(), "a cache read must not create directories");
}

#[tokio::test]
async fn fresh_artifact_hash_verifies_with_embedded_metadata() {
    let fixture = Fixture::new();
    let freezer = fixture.freezer();
    let frozen = freezer.freeze(&request("json")).await.unwrap();
    assert!(freezer.verify_frozen_artifact(&frozen).await.unwrap());
}

#[tokio::test]
async fn prompt_identity_round_trips_in_json_and_yaml() {
    for format in ["json", "yaml"] {
        let fixture = Fixture::new();
        let freezer = fixture.freezer();
        let frozen = freezer.freeze(&request(format)).await.unwrap();
        let loaded = freezer.load_frozen("mock", Some("prompt-identity")).await.unwrap().unwrap();
        assert_eq!(loaded.content, frozen.content);
        assert_eq!(loaded.output_hash, frozen.output_hash);
        assert!(freezer.load_frozen("mock", Some("different-prompt")).await.unwrap().is_none());
        assert!(freezer.load_frozen("persona", Some("prompt-identity")).await.unwrap().is_none());
    }
}

#[tokio::test]
async fn loading_rejects_modified_payload() {
    let fixture = Fixture::new();
    let freezer = fixture.freezer();
    let frozen = freezer.freeze(&request("json")).await.unwrap();
    let mut modified = frozen.content;
    modified["value"] = json!(99);
    fs::write(&frozen.path, serde_json::to_vec(&modified).unwrap()).await.unwrap();
    assert!(freezer.load_frozen("mock", None).await.is_err());
}

#[tokio::test]
async fn identified_snapshot_cannot_disable_integrity_by_removing_hash() {
    let fixture = Fixture::new();
    let freezer = fixture.freezer();
    let frozen = freezer.freeze(&request("json")).await.unwrap();
    let mut modified = frozen.content;
    modified["_frozen_metadata"].as_object_mut().unwrap().remove("output_hash");
    modified["value"] = json!(99);
    fs::write(&frozen.path, serde_json::to_vec(&modified).unwrap()).await.unwrap();
    assert!(freezer.load_frozen("mock", Some("prompt-identity")).await.is_err());
}

#[tokio::test]
async fn non_object_payloads_retain_identity_and_hash_without_wrapping() {
    for format in ["json", "yaml", "yml"] {
        for value in [
            json!([1, null, {"nested":true}]),
            json!("fixture"),
            json!(3.25),
            json!(false),
            Value::Null,
        ] {
            let fixture = Fixture::new();
            let freezer = fixture.freezer();
            let mut input = request(format);
            input.content = value.clone();
            let frozen = freezer.freeze(&input).await.unwrap();
            let raw = fs::read(&frozen.path).await.unwrap();
            let decoded: Value = if format == "json" {
                serde_json::from_slice(&raw).unwrap()
            } else {
                serde_yaml::from_slice(&raw).unwrap()
            };
            assert_eq!(decoded, value);
            let loaded = fixture
                .freezer()
                .load_frozen("mock", Some("prompt-identity"))
                .await
                .unwrap()
                .expect("saved identity must survive restart");
            assert_eq!(loaded.content, value);
            assert_eq!(loaded.output_hash, frozen.output_hash);
            assert!(freezer.verify_frozen_artifact(&loaded).await.unwrap());
            assert!(freezer.load_frozen("mock", Some("other-prompt")).await.unwrap().is_none());
        }
    }
}

#[tokio::test]
async fn non_object_payload_changes_are_rejected_without_identifier() {
    let fixture = Fixture::new();
    let freezer = fixture.freezer();
    let mut input = request("json");
    input.content = json!([1, 2]);
    let frozen = freezer.freeze(&input).await.unwrap();
    fs::write(&frozen.path, b"[9,9]").await.unwrap();
    assert!(freezer.load_frozen("mock", None).await.is_err());
}

#[tokio::test]
async fn missing_integrity_hash_is_not_verified() {
    let fixture = Fixture::new();
    let freezer = fixture.freezer();
    let mut frozen = freezer.freeze(&request("json")).await.unwrap();
    frozen.output_hash = None;
    assert!(!freezer.verify_frozen_artifact(&frozen).await.unwrap());
}

#[tokio::test]
async fn non_object_snapshot_publication_returns_one_complete_winner() {
    let fixture = Fixture::new();
    let freezer = fixture.freezer();
    let identifier = "a".repeat(64);
    let mut first = request("json");
    first.content = json!(["first"]);
    first.metadata.as_mut().unwrap().prompt_hash = Some(identifier.clone());
    let mut second = first.clone();
    second.content = Value::Null;
    let (a, b) = tokio::join!(
        freezer.freeze_once(&first, &identifier),
        freezer.freeze_once(&second, &identifier)
    );
    let a = a.unwrap();
    let b = b.unwrap();
    assert_eq!(a.content, b.content);
    assert_eq!(a.path, b.path);
    let raw: Value = serde_json::from_slice(&fs::read(&a.path).await.unwrap()).unwrap();
    assert_eq!(raw, a.content);
    let loaded = fixture
        .freezer()
        .load_snapshot("mock", &identifier, "json")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.content, a.content);
    let discovered =
        fixture.freezer().load_frozen("mock", Some(&identifier)).await.unwrap().unwrap();
    assert_eq!(discovered.content, a.content);
}

#[tokio::test]
async fn mixed_object_and_scalar_publishers_share_one_winner() {
    for _ in 0..12 {
        let fixture = Fixture::new();
        let freezer = fixture.freezer();
        let identifier = "b".repeat(64);
        let mut first = request("yaml");
        first.metadata.as_mut().unwrap().prompt_hash = Some(identifier.clone());
        let mut second = first.clone();
        second.content = Value::Null;
        let (a, b) = tokio::join!(
            freezer.freeze_once(&first, &identifier),
            freezer.freeze_once(&second, &identifier)
        );
        let a = a.unwrap();
        let b = b.unwrap();
        assert_eq!(a.content, b.content);
        assert_eq!(a.path, b.path);
    }
}

#[tokio::test]
async fn missing_or_unhashed_scalar_provenance_fails_closed() {
    for modification in ["missing", "hash", "type", "oversize"] {
        let fixture = Fixture::new();
        let freezer = fixture.freezer();
        let mut input = request("json");
        input.content = json!(false);
        let frozen = freezer.freeze(&input).await.unwrap();
        let sidecar = provenance::path(Path::new(&frozen.path));
        let mut meta: Value = serde_json::from_slice(&fs::read(&sidecar).await.unwrap()).unwrap();
        match modification {
            "missing" => fs::remove_file(&sidecar).await.unwrap(),
            "oversize" => fs::write(&sidecar, vec![b' '; 64 * 1024 + 1]).await.unwrap(),
            field => {
                let key = if field == "hash" {
                    "output_hash"
                } else {
                    "artifact_type"
                };
                meta.as_object_mut().unwrap().remove(key);
                fs::write(&sidecar, serde_json::to_vec(&meta).unwrap()).await.unwrap();
            }
        }
        assert!(freezer.load_frozen("mock", None).await.is_err(), "{modification}");
    }
}

#[tokio::test]
async fn direct_paths_and_latest_scalar_discovery_are_preserved() {
    let fixture = Fixture::new();
    let freezer = fixture.freezer();
    let mut input = request("json");
    input.content = json!("first");
    input.path = Some(fixture.0.join("mock_first.json").to_string_lossy().into_owned());
    let first = freezer.freeze(&input).await.unwrap();
    assert_eq!(Some(&first.path), input.path.as_ref());
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    input.content = json!("latest");
    input.path = None;
    let second = freezer.freeze(&input).await.unwrap();
    assert_eq!(freezer.load_frozen("mock", None).await.unwrap().unwrap().path, second.path);
}

#[cfg(unix)]
#[tokio::test]
async fn scalar_provenance_symlink_and_directory_are_rejected() {
    for kind in ["symlink", "directory"] {
        let fixture = Fixture::new();
        let freezer = fixture.freezer();
        let mut input = request("json");
        input.content = Value::Null;
        let frozen = freezer.freeze(&input).await.unwrap();
        let sidecar = provenance::path(Path::new(&frozen.path));
        let saved = fs::read(&sidecar).await.unwrap();
        fs::remove_file(&sidecar).await.unwrap();
        if kind == "symlink" {
            let other = fixture.0.join("target");
            fs::write(&other, saved).await.unwrap();
            std::os::unix::fs::symlink(other, &sidecar).unwrap();
        } else {
            fs::create_dir(&sidecar).await.unwrap();
        }
        assert!(freezer.load_frozen("mock", None).await.is_err());
    }
}
