//! Exercise the actual Axum generation handlers with persisted workspace policy.
use super::*;
use axum::{routing::post, Router};
use mockforge_core::{
    ai_studio::ConversationStore, intelligent_behavior::MockAI, Workspace, WorkspacePersistence,
};
use serde_json::json;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::RwLock;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("mockforge-admin-policy-{}", uuid::Uuid::new_v4())))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Server {
    url: String,
    task: tokio::task::JoinHandle<()>,
}
impl Server {
    async fn start(app: Router) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self { url, task }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn provider_reply(
    State(requests): State<Arc<Mutex<Vec<Value>>>>,
    Json(request): Json<Value>,
) -> Json<Value> {
    let mock = request["messages"][0]["content"]
        .as_str()
        .unwrap()
        .contains("expert API designer");
    let mut requests = requests.lock().unwrap();
    requests.push(request);
    let n = requests.len();
    let body = if mock {
        json!({"api_type":"test","title":format!("Fixture {n}"),"description":"fixture","endpoints":[{"path":"/users","method":"GET","description":"users"}],"models":[]})
    } else {
        json!({"id":format!("user:{n}"),"name":"Fixture","domain":"test","traits":{"tier":"gold"},"backstory":"fixed"})
    };
    Json(json!({"choices":[{"message":{"content":body.to_string()}}]}))
}

fn state(fixture: &Fixture, endpoint: String) -> AdminState {
    let mut behavior = IntelligentBehaviorConfig::default();
    behavior.behavior_model.llm_provider = "openai-compatible".into();
    behavior.behavior_model.api_endpoint = Some(endpoint);
    behavior.behavior_model.model = "workspace-test-model".into();
    behavior.behavior_model.seed = Some(7);
    let mut state = AdminState::new(
        None,
        None,
        None,
        None,
        true,
        0,
        None,
        None,
        Some(Arc::new(RwLock::new(MockAI::new(behavior)))),
        None,
        None,
        None,
        None,
        None,
    );
    state.workspace_persistence = Arc::new(WorkspacePersistence::new(fixture.0.join("workspaces")));
    state.ai_conversations = Arc::new(ConversationStore::new());
    state
}

async fn workspace(state: &AdminState, fixture: &Fixture, format: &str) -> Workspace {
    let mut workspace = Workspace::new("Policy fixture".into());
    let mut value = serde_json::to_value(&workspace.config).unwrap();
    value["ai_mode"] = json!("generate_once_freeze");
    value["ai_generation"] = json!({"seed":42,"deterministic_mode":{"freeze_directory":fixture.0.join("snapshots"),"freeze_format":format,"track_metadata":false}});
    workspace.config = serde_json::from_value(value).unwrap();
    state.workspace_persistence.save_workspace(&workspace).await.unwrap();
    workspace
}

fn router(state: AdminState) -> Router {
    Router::new()
        .route("/chat", post(chat))
        .route("/mock", post(generate_mock))
        .route("/persona", post(generate_persona))
        .route("/debug", post(debug_test))
        .route("/debug-context", post(debug_analyze_with_context))
        .with_state(state)
}

fn payload<'a>(response: &'a Value, path: &str, key: &str) -> &'a Value {
    if path == "/chat" {
        &response["data"]["data"][key]
    } else {
        &response["data"][key]
    }
}

#[tokio::test]
async fn persisted_policy_replays_through_all_generation_handlers() {
    for (path, message, key) in [
        ("/chat", "Create a user API", "spec"),
        ("/chat", "Generate a user persona", "persona"),
        ("/mock", "Create a user API", "spec"),
        ("/persona", "Generate a user persona", "persona"),
    ] {
        for format in ["json", "yaml"] {
            let fixture = Fixture::new();
            let requests = Arc::new(Mutex::new(Vec::new()));
            let provider = Server::start(
                Router::new()
                    .route("/generate", post(provider_reply))
                    .with_state(requests.clone()),
            )
            .await;
            let state = state(&fixture, format!("{}/generate", provider.url));
            let workspace = workspace(&state, &fixture, format).await;
            let api = Server::start(router(state.clone())).await;
            let client = reqwest::Client::new();
            let body = if path == "/chat" {
                json!({"message":message,"workspace_id":workspace.id})
            } else {
                json!({"description":message,"workspace_id":workspace.id})
            };
            let first = client.post(format!("{}{path}", api.url)).json(&body).send().await.unwrap();
            assert_eq!(first.status(), StatusCode::OK);
            let first: Value = first.json().await.unwrap();
            assert_eq!(first["success"], true, "{first}");
            assert!(payload(&first, path, key).is_object(), "{first}");
            {
                let requests = requests.lock().unwrap();
                assert_eq!(requests.len(), 1);
                assert_eq!(requests[0]["seed"], 42);
                assert_eq!(requests[0]["model"], "workspace-test-model");
            }
            provider.task.abort();
            drop(api);
            let restarted = Server::start(router(state)).await;
            let second: Value = client
                .post(format!("{}{path}", restarted.url))
                .json(&body)
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            assert_eq!(second["success"], true, "{second}");
            assert_eq!(payload(&first, path, key), payload(&second, path, key));
            if path == "/chat" {
                assert_eq!(second["data"]["tokens_used"], 0);
                assert_eq!(second["data"]["cost_usd"], 0.0);
            }
            assert_eq!(requests.lock().unwrap().len(), 1);
            let entries: Vec<_> = std::fs::read_dir(fixture.0.join("snapshots")).unwrap().collect();
            assert_eq!(entries.len(), 1);
            let bundle = entries[0].as_ref().unwrap().path();
            let payload = bundle.join(format!("payload.{format}"));
            assert!(payload.is_file());
            let text = std::fs::read_to_string(payload).unwrap();
            assert!(!text.contains("original_prompt"));
        }
    }
}

#[tokio::test]
async fn invalid_or_missing_workspace_never_reaches_provider() {
    let fixture = Fixture::new();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let provider = Server::start(
        Router::new()
            .route("/generate", post(provider_reply))
            .with_state(requests.clone()),
    )
    .await;
    let state = state(&fixture, format!("{}/generate", provider.url));
    state.workspace_persistence.ensure_workspace_dir().await.unwrap();
    tokio::fs::write(state.workspace_persistence.workspace_file_path("corrupt"), b"not: [valid")
        .await
        .unwrap();
    let api = Server::start(router(state)).await;
    let client = reqwest::Client::new();
    for path in ["/chat", "/mock", "/persona"] {
        for (id, status) in [
            ("not-present", StatusCode::NOT_FOUND),
            ("../other", StatusCode::BAD_REQUEST),
            ("corrupt", StatusCode::INTERNAL_SERVER_ERROR),
        ] {
            let body = if path == "/chat" {
                json!({"message":"Create a user API","workspace_id":id})
            } else {
                json!({"description":"Create a user API","workspace_id":id})
            };
            assert_eq!(
                client
                    .post(format!("{}{path}", api.url))
                    .json(&body)
                    .send()
                    .await
                    .unwrap()
                    .status(),
                status
            );
        }
    }
    assert!(requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn workspace_seed_and_mode_override_server_defaults() {
    let fixture = Fixture::new();
    let state = state(&fixture, "http://127.0.0.1:1/generate".into());
    state.update_ai_mode_config(Some("generate_once_freeze".into())).await.unwrap();
    let defaults = policy::resolve(&state, None, None).await.unwrap();
    assert_eq!(defaults.behavior.behavior_model.seed, Some(7));
    assert!(defaults.deterministic.is_none());
    let mut workspace = workspace(&state, &fixture, "json").await;
    workspace.config.ai_mode = Some(mockforge_core::ai_studio::config::AiMode::Live);
    state.workspace_persistence.save_workspace(&workspace).await.unwrap();
    let effective = policy::resolve(&state, Some(&workspace.id), None).await.unwrap();
    assert_eq!(effective.mode, workspace.config.ai_mode);
    assert_eq!(effective.behavior.behavior_model.seed, Some(42));
    assert!(effective.deterministic.is_some());
}

#[tokio::test]
async fn chat_inherits_conversation_workspace_and_rejects_conflicts() {
    let fixture = Fixture::new();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let provider = Server::start(
        Router::new()
            .route("/generate", post(provider_reply))
            .with_state(requests.clone()),
    )
    .await;
    let state = state(&fixture, format!("{}/generate", provider.url));
    let workspace = workspace(&state, &fixture, "json").await;
    let conversation = state
        .ai_conversations
        .create_conversation(Some(workspace.id.clone()))
        .await
        .unwrap();
    let api = Server::start(router(state)).await;
    let client = reqwest::Client::new();
    let body = json!({"message":"Create a user API","conversation_id":conversation});
    let first: Value = client
        .post(format!("{}/chat", api.url))
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(first["success"], true, "{first}");
    assert!(payload(&first, "/chat", "spec").is_object());
    provider.task.abort();
    let second: Value = client
        .post(format!("{}/chat", api.url))
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(second["success"], true, "{second}");
    assert_eq!(payload(&first, "/chat", "spec"), payload(&second, "/chat", "spec"));
    let mut conflict = body;
    conflict["workspace_id"] = json!(uuid::Uuid::new_v4().to_string());
    assert_eq!(
        client
            .post(format!("{}/chat", api.url))
            .json(&conflict)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn workspace_api_and_generation_share_live_registry_policy() {
    use crate::handlers::workspaces::{self, WorkspaceState};
    let fixture = Fixture::new();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let provider = Server::start(
        Router::new()
            .route("/generate", post(provider_reply))
            .with_state(requests.clone()),
    )
    .await;
    let mut state = state(&fixture, format!("{}/generate", provider.url));
    let registry = Arc::new(RwLock::new(mockforge_core::MultiTenantWorkspaceRegistry::new(
        mockforge_core::multi_tenant::MultiTenantConfig {
            enabled: true,
            ..Default::default()
        },
    )));
    state.workspace_registry = Some(registry.clone());
    let app = router(state.clone()).merge(
        Router::new()
            .route("/workspaces", post(workspaces::create_workspace))
            .route("/workspaces/{id}", axum::routing::put(workspaces::update_workspace))
            .with_state(WorkspaceState::new(registry.clone())),
    );
    let api = Server::start(app).await;
    let client = reqwest::Client::new();
    assert_eq!(
        client
            .post(format!("{}/workspaces", api.url))
            .json(&json!({"id":"browser-workspace","name":"Browser workspace"}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    let update = json!({"ai_mode":"generate_once_freeze","ai_generation":{"seed":42,"deterministic_mode":{"freeze_directory":fixture.0.join("snapshots"),"freeze_format":"json"}}});
    assert_eq!(
        client
            .put(format!("{}/workspaces/browser-workspace", api.url))
            .json(&update)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert!(!state.workspace_persistence.workspace_file_path("browser-workspace").exists());
    let body = json!({"description":"Create a user API","workspace_id":"browser-workspace"});
    let first: Value = client
        .post(format!("{}/mock", api.url))
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(first["success"], true, "{first}");
    assert!(first["data"]["spec"].is_object());
    provider.task.abort();
    let second: Value = client
        .post(format!("{}/mock", api.url))
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(second["success"], true, "{second}");
    assert_eq!(first["data"]["spec"], second["data"]["spec"]);
    assert_eq!(requests.lock().unwrap().len(), 1);
    assert_eq!(requests.lock().unwrap()[0]["seed"], 42);
    assert_eq!(
        client
            .put(format!("{}/workspaces/browser-workspace", api.url))
            .json(&json!({"ai_mode":null,"ai_generation":null,"enabled":false}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    {
        let registry = registry.read().await;
        let current = registry.get_workspace("browser-workspace").unwrap();
        assert!(current.workspace.config.ai_mode.is_none());
        assert!(current.workspace.config.ai_generation.is_none());
    }
    assert_eq!(
        client
            .post(format!("{}/mock", api.url))
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn authoring_defaults_work_with_runtime_mockai_disabled() {
    for (path, message, key) in [
        ("/chat", "Create a user API", "spec"),
        ("/chat", "Generate a user persona", "persona"),
        ("/mock", "Create a user API", "spec"),
        ("/persona", "Generate a user persona", "persona"),
    ] {
        let fixture = Fixture::new();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let provider = Server::start(
            Router::new()
                .route("/generate", post(provider_reply))
                .with_state(requests.clone()),
        )
        .await;
        let mut state = state(&fixture, format!("{}/generate", provider.url));
        let defaults = AiAuthoringDefaults::new(
            state.mockai.as_ref().unwrap().read().await.get_config().clone(),
        );
        state.mockai = None;
        assert!(state.mockai.is_none());
        let workspace = workspace(&state, &fixture, "json").await;
        let api =
            Server::start(router(state.clone()).layer(axum::Extension(defaults.clone()))).await;
        let client = reqwest::Client::new();
        let body = if path == "/chat" {
            json!({"message":message,"workspace_id":workspace.id})
        } else {
            json!({"description":message,"workspace_id":workspace.id})
        };
        let first: Value = client
            .post(format!("{}{path}", api.url))
            .json(&body)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(first["success"], true, "{first}");
        assert!(payload(&first, path, key).is_object());
        {
            let requests = requests.lock().unwrap();
            assert_eq!(requests.len(), 1);
            assert_eq!(requests[0]["model"], "workspace-test-model");
            assert_eq!(requests[0]["seed"], 42);
        }
        provider.task.abort();
        drop(api);
        let restarted = Server::start(router(state.clone()).layer(axum::Extension(defaults))).await;
        let second: Value = client
            .post(format!("{}{path}", restarted.url))
            .json(&body)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(second["success"], true, "{second}");
        assert_eq!(payload(&first, path, key), payload(&second, path, key));
        assert_eq!(requests.lock().unwrap().len(), 1);
        assert!(state.mockai.is_none());
    }
}

#[tokio::test]
async fn live_mockai_config_overrides_startup_authoring_defaults() {
    let fixture = Fixture::new();
    let mut state = state(&fixture, "http://127.0.0.1:1/generate".into());
    let mut behavior = IntelligentBehaviorConfig::default();
    behavior.behavior_model.model = "startup-model".into();
    behavior.behavior_model.seed = Some(99);
    let defaults = AiAuthoringDefaults::new(behavior);
    let live = policy::resolve(&state, None, Some(&defaults)).await.unwrap();
    assert_eq!(live.behavior.behavior_model.model, "workspace-test-model");
    assert_eq!(live.behavior.behavior_model.seed, Some(7));
    state.mockai = None;
    let authoring = policy::resolve(&state, None, Some(&defaults)).await.unwrap();
    assert_eq!(authoring.behavior.behavior_model.model, "startup-model");
    assert_eq!(authoring.behavior.behavior_model.seed, Some(99));
    assert!(!authoring.behavior.enabled);
    assert!(state.mockai.is_none());
}

async fn debug_provider_reply(
    State(requests): State<Arc<Mutex<Vec<Value>>>>,
    Json(request): Json<Value>,
) -> Json<Value> {
    requests.lock().unwrap().push(request);
    let body = json!({
        "summary":"fixture root cause", "explanation":"fixture explanation",
        "stack_trace":[], "contributing_factors":[], "suggested_fixes":[],
        "confidence":1.0, "suggestions":[]
    });
    Json(json!({"choices":[{"message":{"content":body.to_string()}}]}))
}

fn debug_body(path: &str, workspace_id: Option<&str>) -> Value {
    let key = if path == "/chat" {
        "message"
    } else {
        "test_logs"
    };
    let mut body = json!({"workspace_id":workspace_id});
    body[key] = json!("debug GET /users 500 test failure");
    body
}

#[tokio::test]
async fn debug_handlers_keep_provider_and_workspace_seed_with_runtime_on_or_off() {
    for runtime_enabled in [true, false] {
        let fixture = Fixture::new();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let provider = Server::start(
            Router::new()
                .route("/generate", post(debug_provider_reply))
                .with_state(requests.clone()),
        )
        .await;
        let mut state = state(&fixture, format!("{}/generate", provider.url));
        let defaults = AiAuthoringDefaults::new(
            state.mockai.as_ref().unwrap().read().await.get_config().clone(),
        );
        if !runtime_enabled {
            state.mockai = None;
        }
        let workspace = workspace(&state, &fixture, "json").await;
        let api = Server::start(router(state).layer(axum::Extension(defaults))).await;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        for path in ["/debug", "/debug-context", "/chat"] {
            for (workspace_id, seed) in [(Some(workspace.id.as_str()), 42), (None, 7)] {
                let before = requests.lock().unwrap().len();
                let response = client
                    .post(format!("{}{path}", api.url))
                    .json(&debug_body(path, workspace_id))
                    .send()
                    .await
                    .unwrap();
                assert_eq!(response.status(), StatusCode::OK);
                let body: Value = response.json().await.unwrap();
                assert_eq!(body["success"], true, "{body}");
                let data = if path == "/chat" {
                    &body["data"]["data"]
                } else {
                    &body["data"]
                };
                assert_eq!(data["root_cause"], "fixture root cause", "{body}");
                let requests = requests.lock().unwrap();
                assert_eq!(requests.len() - before, 2, "{path}");
                for request in &requests[before..] {
                    assert_eq!(request["model"], "workspace-test-model");
                    assert_eq!(request["seed"], seed);
                }
            }
        }
    }
}

#[tokio::test]
async fn direct_debug_handlers_validate_workspace_before_provider_calls() {
    let fixture = Fixture::new();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let provider = Server::start(
        Router::new()
            .route("/generate", post(debug_provider_reply))
            .with_state(requests.clone()),
    )
    .await;
    let state = state(&fixture, format!("{}/generate", provider.url));
    let api = Server::start(router(state)).await;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    for path in ["/debug", "/debug-context"] {
        for (id, expected) in [
            ("../missing", StatusCode::BAD_REQUEST),
            ("missing-workspace", StatusCode::NOT_FOUND),
        ] {
            let response = client
                .post(format!("{}{path}", api.url))
                .json(&debug_body(path, Some(id)))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), expected);
        }
    }
    assert!(requests.lock().unwrap().is_empty());
}
