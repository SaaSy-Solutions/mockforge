use super::*;
use crate::ai_studio::{
    nl_mock_generator::MockGenerator,
    persona_generator::{PersonaGenerationRequest, PersonaGenerator},
};
use axum::{extract::State, routing::post, Json, Router};
use serde_json::json;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("mockforge-replay-{}", uuid::Uuid::new_v4())))
    }
    fn config(&self, format: &str, track_metadata: bool) -> DeterministicModeConfig {
        DeterministicModeConfig {
            freeze_directory: self.0.to_string_lossy().into_owned(),
            freeze_format: format.into(),
            track_metadata,
            ..Default::default()
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Provider {
    endpoint: String,
    requests: Arc<Mutex<Vec<Value>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Provider {
    async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/generate", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let app = Router::new().route("/generate", post(reply)).with_state(requests.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self {
            endpoint,
            requests,
            task,
        }
    }
    fn config(&self) -> IntelligentBehaviorConfig {
        let mut config = IntelligentBehaviorConfig::default();
        config.behavior_model.llm_provider = "openai-compatible".into();
        config.behavior_model.api_endpoint = Some(self.endpoint.clone());
        config.behavior_model.model = "fixture-model-v1".into();
        config.behavior_model.seed = Some(42);
        config
    }
}
impl Drop for Provider {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn reply(
    State(requests): State<Arc<Mutex<Vec<Value>>>>,
    Json(request): Json<Value>,
) -> Json<Value> {
    let mut requests = requests.lock().unwrap();
    requests.push(request.clone());
    let number = requests.len();
    let content = if request["messages"][0]["content"]
        .as_str()
        .unwrap()
        .contains("expert API designer")
    {
        json!({"api_type":"test","title":format!("Snapshot {number}"),"description":"fixture","endpoints":[{"path":"/users","method":"GET","description":"users"}],"models":[]})
    } else {
        json!({"id":format!("user:{number}"),"name":format!("Snapshot {number}"),"domain":"test","traits":{"tier":"gold"},"backstory":"fixed fixture"})
    };
    Json(json!({"choices":[{"message":{"content":content.to_string()}}]}))
}

#[tokio::test]
async fn mock_replays_after_provider_shutdown_and_preserves_file_bytes() {
    let fixture = Fixture::new();
    let provider = Provider::start().await;
    let behavior = provider.config();
    let config = fixture.config("yaml", true);
    let first = MockGenerator::with_config(behavior.clone())
        .generate(
            "Create a user API",
            Some("workspace-a"),
            Some(AiMode::GenerateOnceFreeze),
            Some(&config),
        )
        .await
        .unwrap();
    assert_eq!(provider.requests.lock().unwrap().len(), 1);
    assert_eq!(provider.requests.lock().unwrap()[0]["seed"], 42);
    let artifact = first.frozen_artifact.unwrap();
    let bytes = tokio::fs::read(&artifact.path).await.unwrap();
    provider.task.abort();
    let mut behavior = behavior;
    behavior.behavior_model.api_key = Some("rotated-test-credential".into());
    let second = MockGenerator::with_config(behavior)
        .generate(
            "Create a user API",
            Some("workspace-a"),
            Some(AiMode::GenerateOnceFreeze),
            Some(&config),
        )
        .await
        .unwrap();
    assert_eq!(first.spec, second.spec);
    assert_eq!(second.frozen_artifact.unwrap().path, artifact.path);
    assert_eq!(bytes, tokio::fs::read(&artifact.path).await.unwrap());
    assert_eq!(provider.requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn persona_replays_without_optional_provenance_and_rejects_corruption() {
    let fixture = Fixture::new();
    let provider = Provider::start().await;
    let behavior = provider.config();
    let config = fixture.config("json", false);
    let request = PersonaGenerationRequest {
        description: "PROMPT_NOT_RETAINED_852".into(),
        base_persona_id: None,
        workspace_id: Some("workspace-a".into()),
    };
    let first = PersonaGenerator::with_config(behavior.clone())
        .generate(&request, Some(AiMode::GenerateOnceFreeze), Some(&config))
        .await
        .unwrap();
    let path = first.frozen_artifact.unwrap().path;
    let bytes = tokio::fs::read(&path).await.unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("PROMPT_NOT_RETAINED_852"));
    provider.task.abort();
    let generator = PersonaGenerator::with_config(behavior);
    let replay = generator
        .generate(&request, Some(AiMode::GenerateOnceFreeze), Some(&config))
        .await
        .unwrap();
    assert_eq!(first.persona, replay.persona);
    let mut modified: Value = serde_json::from_slice(&bytes).unwrap();
    modified["name"] = json!("tampered");
    tokio::fs::write(&path, serde_json::to_vec(&modified).unwrap()).await.unwrap();
    assert!(generator
        .generate(&request, Some(AiMode::GenerateOnceFreeze), Some(&config))
        .await
        .is_err());
    assert_eq!(provider.requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn changed_workspace_seed_and_model_do_not_reuse_another_snapshot() {
    let fixture = Fixture::new();
    let provider = Provider::start().await;
    let behavior = provider.config();
    let config = fixture.config("json", true);
    let request = PersonaGenerationRequest {
        description: "A user".into(),
        base_persona_id: None,
        workspace_id: Some("a".into()),
    };
    let mode = Some(AiMode::GenerateOnceFreeze);
    let first = PersonaGenerator::with_config(behavior.clone())
        .generate(&request, mode, Some(&config))
        .await
        .unwrap();
    let mut other_request = request.clone();
    other_request.workspace_id = Some("b".into());
    let other = PersonaGenerator::with_config(behavior.clone())
        .generate(&other_request, mode, Some(&config))
        .await
        .unwrap();
    assert_ne!(first.persona, other.persona);
    let mut other_behavior = behavior.clone();
    other_behavior.behavior_model.seed = Some(43);
    let seeded = PersonaGenerator::with_config(other_behavior)
        .generate(&request, mode, Some(&config))
        .await
        .unwrap();
    assert_ne!(first.persona, seeded.persona);
    let mut other_behavior = behavior;
    other_behavior.behavior_model.model = "fixture-model-v2".into();
    let model = PersonaGenerator::with_config(other_behavior)
        .generate(&request, mode, Some(&config))
        .await
        .unwrap();
    assert_ne!(first.persona, model.persona);
    let requests = provider.requests.lock().unwrap();
    assert_eq!(requests.len(), 4);
    assert_eq!(requests[2]["seed"], 43);
}

#[tokio::test]
async fn concurrent_publishers_return_one_complete_immutable_snapshot() {
    let fixture = Fixture::new();
    let config = fixture.config("json", false);
    let snapshot = FrozenGeneration::new(
        json!({"description":"same"}),
        &IntelligentBehaviorConfig::default(),
        Some(AiMode::GenerateOnceFreeze),
        Some(&config),
    )
    .unwrap();
    let metadata = FreezeMetadata {
        llm_provider: None,
        llm_model: None,
        llm_version: None,
        prompt_hash: None,
        output_hash: None,
        original_prompt: None,
    };
    let (one, two) = tokio::join!(
        snapshot.freeze("mock", json!({"value":1}), metadata.clone()),
        snapshot.freeze("mock", json!({"value":2}), metadata)
    );
    let one = one.unwrap().unwrap();
    let two = two.unwrap().unwrap();
    assert_eq!(one.path, two.path);
    assert_eq!(one.content, two.content);
    assert_eq!(snapshot.load("mock").await.unwrap().unwrap().content, one.content);
}
