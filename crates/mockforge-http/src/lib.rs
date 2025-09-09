use axum::{routing::{get, any}, Router, extract::{Query, State}, response::IntoResponse, http::{StatusCode, Uri}, Json};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, fs, collections::HashMap, sync::Arc};
use tracing::*;
use openapiv3 as oa;
use base64::{Engine as _, engine::general_purpose};

#[derive(Clone, Default)]
struct Cfg {
    spec: Option<oa::OpenAPI>,
    overrides: Option<String>,
    record_dir: Option<String>,
    replay_dir: Option<String>,
    proxy_base: Option<String>,
    latency_enabled: bool,
    failures_enabled: bool,
}

#[derive(Clone, Default)]
struct AppState { cfg: Arc<Cfg> }

pub async fn start(port: u16, spec: Option<String>) {
    let mut cfg = Cfg::default();
    if let Some(p) = spec {
        match fs::read_to_string(&p) {
            Ok(text) => {
                let spec: oa::OpenAPI = if p.ends_with(".yaml") || p.ends_with(".yml") {
                    serde_yaml::from_str(&text).unwrap()
                } else { serde_json::from_str(&text).unwrap() };
                info!("Loaded OpenAPI: {} with {} paths", p, spec.paths.paths.len());
                cfg.spec = Some(spec);
            }
            Err(e) => warn!("Failed reading spec {}: {}", p, e),
        }
    }
    cfg.overrides = std::env::var("MOCKFORGE_HTTP_OVERRIDES").ok();
    cfg.record_dir = std::env::var("MOCKFORGE_HTTP_RECORD_DIR").ok();
    cfg.replay_dir = std::env::var("MOCKFORGE_HTTP_REPLAY_DIR").ok();
    cfg.proxy_base = std::env::var("MOCKFORGE_HTTP_PROXY_BASE").ok();
    cfg.latency_enabled = std::env::var("MOCKFORGE_LATENCY_ENABLED").ok().map(|v| v!="false").unwrap_or(true);
    cfg.failures_enabled = std::env::var("MOCKFORGE_FAILURES_ENABLED").ok().map(|v| v!="false").unwrap_or(false);

    let state = AppState { cfg: Arc::new(cfg) };
    let app = Router::new()
        .route("/ping", get(|| async { "pong" }))
        .route("/__admin/api/state", get(admin_state).post(admin_state_post))
        .fallback(any(handler))
        .with_state(state.clone());

    let addr = SocketAddr::from(([0,0,0,0], port));
    info!("HTTP listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service()).await.unwrap();
}

#[derive(Serialize, Deserialize, Default)]
struct AdminState {
    proxy: Option<String>,
    overrides: Option<String>,
    httpRecord: Option<String>,
    httpReplay: Option<String>,
    latencyEnabled: Option<bool>,
    failuresEnabled: Option<bool>,
}

async fn admin_state(State(st): State<AppState>) -> impl IntoResponse {
    let c = &*st.cfg;
    Json(serde_json::json!({
        "proxy": c.proxy_base,
        "overrides": c.overrides,
        "httpRecord": c.record_dir,
        "httpReplay": c.replay_dir,
        "latencyEnabled": c.latency_enabled,
        "failuresEnabled": c.failures_enabled
    }))
}

async fn admin_state_post(State(_st): State<AppState>, Json(body): Json<AdminState>) -> impl IntoResponse {
    if let Some(v) = body.proxy { std::env::set_var("MOCKFORGE_HTTP_PROXY_BASE", v); }
    if let Some(v) = body.overrides { std::env::set_var("MOCKFORGE_HTTP_OVERRIDES", v); }
    if let Some(v) = body.httpRecord { std::env::set_var("MOCKFORGE_HTTP_RECORD_DIR", v); }
    if let Some(v) = body.httpReplay { std::env::set_var("MOCKFORGE_HTTP_REPLAY_DIR", v); }
    if let Some(v) = body.latencyEnabled { std::env::set_var("MOCKFORGE_LATENCY_ENABLED", if v { "true"} else {"false"}); }
    if let Some(v) = body.failuresEnabled { std::env::set_var("MOCKFORGE_FAILURES_ENABLED", if v { "true"} else {"false"}); }
    (StatusCode::OK, "{}")
}

#[derive(Deserialize)]
struct AnyQ(HashMap<String,String>);

async fn handler(State(st): State<AppState>, uri: Uri, query: Result<Query<AnyQ>, axum::extract::rejection::QueryRejection>) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/').to_string();
    let q = query.unwrap_or(Query(AnyQ(HashMap::new()))).0;
    if let Some(dir) = st.cfg.replay_dir.as_ref() {
        if let Some(resp) = try_replay(dir, uri.path(), &q.0).ok().flatten() {
            return (StatusCode::OK, axum::Json(resp));
        }
    }
    if let Some(base) = st.cfg.proxy_base.as_ref() {
        if let Ok(resp) = proxy_get(base, uri.path(), &q.0).await {
            return (StatusCode::OK, resp);
        }
    }
    let body = serde_json::json!({
        "mock": true,
        "path": format!("/{}", path),
        "query": q.0,
        "ts": chrono::Utc::now().to_rfc3339(),
        "id": uuid::Uuid::new_v4().to_string()
    });
    if let Some(dir) = st.cfg.record_dir.as_ref() {
        let _ = record(dir, uri.path(), &q.0, &body);
    }
    (StatusCode::OK, axum::Json(body))
}

fn key_from(path: &str, q: &HashMap<String,String>) -> String {
    let mut qp: Vec<_> = q.iter().collect();
    qp.sort_by_key(|(k,_)| *k);
    let qs: Vec<String> = qp.into_iter().map(|(k,v)| format!("{}={}", k, v)).collect();
    format!("{}?{}", path, qs.join("&"))
}

fn record(dir: &str, path: &str, q: &HashMap<String,String>, body: &serde_json::Value) -> std::io::Result<()> {
    std::fs::create_dir_all(dir).ok();
    let name = general_purpose::URL_SAFE_NO_PAD.encode(key_from(path,q));
    let p = format!("{}/{}.json", dir, name);
    std::fs::write(p, serde_json::to_vec_pretty(body)?)?;
    Ok(())
}
fn try_replay(dir: &str, path: &str, q: &HashMap<String,String>) -> std::io::Result<Option<serde_json::Value>> {
    let name = general_purpose::URL_SAFE_NO_PAD.encode(key_from(path,q));
    let p = format!("{}/{}.json", dir, name);
    match std::fs::read_to_string(p) {
        Ok(t) => Ok(serde_json::from_str(&t).ok()),
        Err(_) => Ok(None)
    }
}

async fn proxy_get(base: &str, path: &str, q: &HashMap<String,String>) -> Result<axum::Json<serde_json::Value>, ()> {
    let qs = if q.is_empty() { "".to_string() } else {
        let mut items: Vec<_> = q.iter().collect();
        items.sort_by_key(|(k,_)| *k);
        let s = items.into_iter().map(|(k,v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v))).collect::<Vec<_>>().join("&");
        format!("?{}", s)
    };
    let url = format!("{}{}{}", base.trim_end_matches('/'), path, qs);
    match reqwest::get(&url).await {
        Ok(rsp) => match rsp.json::<serde_json::Value>().await {
            Ok(v) => Ok(axum::Json(v)),
            Err(_) => Err(())
        }
        Err(_) => Err(())
    }
}
