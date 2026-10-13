//! Exercise legacy state and auth DTO paths through a real TCP router.

use axum::Router;
use mockforge_http::{
    auth::risk_engine::{RiskAction, RiskAssessment, RiskEngine, RiskEngineConfig},
    handlers::risk_simulation::{risk_simulation_router, RiskSimulationState},
};
use reqwest::{Client, Method, StatusCode};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc, time::Duration};

struct Server {
    base: String,
    client: Client,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Server {
    async fn start(engine: Arc<RiskEngine>) -> Self {
        // Existing downstream struct literals remain valid and share the engine.
        let state = RiskSimulationState {
            risk_engine: engine,
        };
        let router = Router::new().nest("/api/v1/auth", risk_simulation_router(state));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        Self {
            base: format!("http://{address}/api/v1/auth"),
            client: Client::builder().timeout(Duration::from_secs(5)).build().unwrap(),
            task,
        }
    }

    async fn call(&self, method: Method, path: &str, body: Option<Value>) -> Value {
        let mut request = self.client.request(method, format!("{}{path}", self.base));
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        response.json().await.unwrap()
    }

    async fn assessment(&self, user: &str) -> Value {
        self.call(Method::GET, &format!("/risk/assessment/{user}"), None).await
    }
}

#[tokio::test]
async fn overrides_mfa_block_and_clear_share_the_existing_auth_engine() {
    let engine = Arc::new(RiskEngine::default());
    let server = Server::start(engine.clone()).await;
    assert_eq!(server.assessment("user-a").await["recommended_action"], "Allow");

    let score = server
        .call(
            Method::POST,
            "/risk/simulate",
            Some(json!({"user_id":"user-a","risk_score":0.6})),
        )
        .await;
    assert_eq!(score, json!({"success":true,"user_id":"user-a","risk_score":0.6}));
    let direct: RiskAssessment = engine.assess_risk("user-a", &HashMap::new()).await;
    assert_eq!(direct.recommended_action, RiskAction::DeviceChallenge);
    assert_eq!(server.assessment("user-a").await["risk_score"], direct.risk_score);
    assert_eq!(server.assessment("user-b").await["risk_score"], 0.0);

    let factors = json!({"unusual_location":0.5});
    let response = server
        .call(
            Method::POST,
            "/risk/factors",
            Some(json!({"user_id":"user-a","risk_factors":factors})),
        )
        .await;
    assert_eq!(response["risk_factors"], factors);
    // The existing engine gives a score override precedence over factor overrides.
    assert_eq!(server.assessment("user-a").await["risk_score"], 0.6);
    server.call(Method::DELETE, "/risk/clear/user-a", None).await;
    assert!(!engine.simulated_risks.read().await.contains_key("user-a"));
    assert!(!engine.simulated_factors.read().await.contains_key("user-a"));

    server
        .call(
            Method::POST,
            "/risk/factors",
            Some(json!({"user_id":"user-a","risk_factors":factors})),
        )
        .await;
    let assessment = server.assessment("user-a").await;
    assert_eq!(assessment["risk_score"], 0.2);
    assert_eq!(
        assessment["risk_factors"],
        json!([{
            "name":"unusual_location", "weight":0.4, "value":0.5, "contribution":0.2
        }])
    );
    assert_eq!(assessment["recommended_action"], "Allow");

    let mfa = server
        .call(
            Method::POST,
            "/risk/trigger-mfa",
            Some(json!({"user_id":"user-a","mfa_type":"totp"})),
        )
        .await;
    assert_eq!(mfa["mfa_type"], "totp");
    assert_eq!(
        engine.assess_risk("user-a", &HashMap::new()).await.recommended_action,
        RiskAction::RequireMfa
    );
    assert_eq!(server.assessment("user-a").await["risk_score"], 0.8);

    let blocked = server
        .call(
            Method::POST,
            "/risk/block",
            Some(json!({"user_id":"user-a","reason":"synthetic fixture"})),
        )
        .await;
    assert_eq!(blocked["reason"], "synthetic fixture");
    assert_eq!(server.assessment("user-a").await["recommended_action"], "Block");
    assert_eq!(engine.assess_risk("user-a", &HashMap::new()).await.risk_score, 0.95);

    server.call(Method::DELETE, "/risk/clear/user-a", None).await;
    assert_eq!(server.assessment("user-a").await["risk_score"], 0.0);
    server.call(Method::DELETE, "/risk/clear/unknown-user", None).await;
    let invalid = server
        .client
        .post(format!("{}/risk/simulate", server.base))
        .json(&json!({"user_id":"user-a"}))
        .send()
        .await
        .unwrap();
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn simulation_uses_the_supplied_threshold_configuration() {
    let config = RiskEngineConfig {
        mfa_threshold: 0.3,
        device_challenge_threshold: 0.1,
        blocked_login_threshold: 0.5,
        risk_rules: vec![],
        ..RiskEngineConfig::default()
    };
    let engine = Arc::new(RiskEngine::new(config));
    let server = Server::start(engine.clone()).await;
    server
        .call(
            Method::POST,
            "/risk/simulate",
            Some(json!({"user_id":"custom-user","risk_score":0.4})),
        )
        .await;
    assert_eq!(server.assessment("custom-user").await["recommended_action"], "RequireMfa");
    let direct = engine.assess_risk("custom-user", &HashMap::new()).await;
    assert_eq!(direct.recommended_action, RiskAction::RequireMfa);
}
