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
