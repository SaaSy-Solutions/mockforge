//! HTTP client for the MockForge admin API.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::de::DeserializeOwned;

use super::models::*;

/// HTTP client wrapping `reqwest` with base URL and optional auth.
#[derive(Clone)]
pub struct MockForgeClient {
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl MockForgeClient {
    /// Create a new client pointing at the given admin server.
    pub fn new(base_url: String, token: Option<String>) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .context("failed to create HTTP client")?;

        let base_url = base_url.trim_end_matches('/').to_string();

        Ok(Self {
            client,
            base_url,
            token,
        })
    }

    /// Base URL for SSE stream connections.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Build a GET request with auth header if configured.
    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{path}", self.base_url);
        let mut req = self.client.get(&url);
        if let Some(ref token) = self.token {
            req = req.bearer_auth(token);
        }
        req
    }

    /// Build a POST request with auth header and JSON body.
    fn post<T: serde::Serialize>(&self, path: &str, body: &T) -> reqwest::RequestBuilder {
        let url = format!("{}{path}", self.base_url);
        let mut req = self.client.post(&url).json(body);
        if let Some(ref token) = self.token {
            req = req.bearer_auth(token);
        }
        req
    }

    /// Send a GET, check for JSON content type, and unwrap the `ApiResponse<T>` envelope.
    async fn get_api<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let resp = self.get(path).send().await.with_context(|| format!("GET {path}"))?;

        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("HTTP {status} from {path}");
        }

        // Guard against HTML responses from the SPA fallback (endpoint doesn't exist).
        let ct = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if ct.contains("text/html") {
            anyhow::bail!("endpoint {path} not available (got HTML)");
        }

        let body = resp.text().await.with_context(|| format!("read body from {path}"))?;

        let envelope: ApiResponse<T> = serde_json::from_str(&body)
            .with_context(|| format!("deserialise response from {path}"))?;

        if envelope.success {
            envelope.data.context("API returned success but no data")
        } else {
            anyhow::bail!("API error: {}", envelope.error.unwrap_or_else(|| "unknown".into()))
        }
    }

    /// Send a GET, check for JSON content type, and return raw JSON.
    async fn get_raw<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let resp = self.get(path).send().await.with_context(|| format!("GET {path}"))?;

        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("HTTP {status} from {path}");
        }

        let ct = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if ct.contains("text/html") {
            anyhow::bail!("endpoint {path} not available (got HTML)");
        }

        let body = resp.text().await.with_context(|| format!("read body from {path}"))?;

        serde_json::from_str(&body).with_context(|| format!("deserialise response from {path}"))
    }

    /// POST helper that expects an `ApiResponse<String>` result.
    async fn post_api(&self, path: &str, body: &serde_json::Value) -> Result<String> {
        let resp = self.post(path, body).send().await.with_context(|| format!("POST {path}"))?;

        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("HTTP {status} from {path}");
        }

        let ct = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if ct.contains("text/html") {
            anyhow::bail!("endpoint {path} not available");
        }

        let body_text = resp.text().await.context("read POST response body")?;
        let envelope: ApiResponse<String> = serde_json::from_str(&body_text)
            .with_context(|| format!("deserialise response from {path}"))?;

        if envelope.success {
            Ok(envelope.data.unwrap_or_default())
        } else {
            anyhow::bail!("API error: {}", envelope.error.unwrap_or_else(|| "unknown".into()))
        }
    }

    // ── Tier 1 endpoints ─────────────────────────────────────────────

    pub async fn get_dashboard(&self) -> Result<DashboardData> {
        self.get_api("/__mockforge/dashboard").await
    }

    pub async fn get_routes(&self) -> Result<Vec<RouteInfo>> {
        // Server may return ApiResponse<Vec<RouteInfo>> or {"routes": [...]}
        let resp = self
            .get("/__mockforge/routes")
            .send()
            .await
            .context("GET /__mockforge/routes")?;

        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("HTTP {status} from /__mockforge/routes");
        }

        let ct = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if ct.contains("text/html") {
            anyhow::bail!("endpoint /__mockforge/routes not available");
        }

        let body = resp.text().await.context("read routes response")?;

        // Try ApiResponse envelope first
        if let Ok(envelope) = serde_json::from_str::<ApiResponse<Vec<RouteInfo>>>(&body) {
            if envelope.success {
                return envelope.data.context("routes: no data");
            }
        }

        // Try {"routes": [...]} wrapper
        if let Ok(wrapper) = serde_json::from_str::<RoutesWrapper>(&body) {
            return Ok(wrapper.routes);
        }

        // Try raw array
        serde_json::from_str::<Vec<RouteInfo>>(&body).context("deserialise routes response")
    }

    pub async fn get_logs(&self, limit: Option<u32>) -> Result<Vec<RequestLog>> {
        let path = match limit {
            Some(n) => format!("/__mockforge/logs?limit={n}"),
            None => "/__mockforge/logs".into(),
        };
        self.get_api(&path).await
    }

    pub async fn get_metrics(&self) -> Result<MetricsData> {
        self.get_api("/__mockforge/metrics").await
    }

    pub async fn get_config(&self) -> Result<ConfigState> {
        self.get_api("/__mockforge/config").await
    }

    pub async fn get_health(&self) -> Result<HealthCheck> {
        self.get_raw("/__mockforge/health").await
    }

    pub async fn get_server_info(&self) -> Result<ServerInfo> {
        // Server may return ApiResponse<ServerInfo> or raw ServerInfo
        let resp = self
            .get("/__mockforge/server-info")
            .send()
            .await
            .context("GET /__mockforge/server-info")?;

        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("HTTP {status} from /__mockforge/server-info");
        }

        let ct = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if ct.contains("text/html") {
            anyhow::bail!("endpoint /__mockforge/server-info not available");
        }

        let body = resp.text().await.context("read server-info response")?;

        // Try ApiResponse envelope first
        if let Ok(envelope) = serde_json::from_str::<ApiResponse<ServerInfo>>(&body) {
            if envelope.success {
                return envelope.data.context("server-info: no data");
            }
        }

        // Try raw ServerInfo
        serde_json::from_str::<ServerInfo>(&body).context("deserialise server-info response")
    }

    pub async fn get_plugins(&self) -> Result<Vec<PluginInfo>> {
        // Server returns ApiResponse<{"plugins": [...], "total": N}>
        let resp = self
            .get("/__mockforge/plugins")
            .send()
            .await
            .context("GET /__mockforge/plugins")?;

        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("HTTP {status} from /__mockforge/plugins");
        }

        let ct = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if ct.contains("text/html") {
            anyhow::bail!("endpoint /__mockforge/plugins not available");
        }

        let body = resp.text().await.context("read plugins response")?;

        // Try ApiResponse<Vec<PluginInfo>> first
        if let Ok(envelope) = serde_json::from_str::<ApiResponse<Vec<PluginInfo>>>(&body) {
            if envelope.success {
                return envelope.data.context("plugins: no data");
            }
        }

        // Try ApiResponse<PluginsWrapper>
        if let Ok(envelope) = serde_json::from_str::<ApiResponse<PluginsWrapper>>(&body) {
            if envelope.success {
                return Ok(envelope.data.map(|w| w.plugins).unwrap_or_default());
            }
        }

        Ok(Vec::new())
    }

    pub async fn get_fixtures(&self) -> Result<Vec<FixtureInfo>> {
        self.get_api("/__mockforge/fixtures").await
    }

    pub async fn get_smoke_tests(&self) -> Result<Vec<SmokeTestResult>> {
        self.get_api("/__mockforge/smoke").await
    }

    pub async fn run_smoke_tests(&self) -> Result<Vec<SmokeTestResult>> {
        self.get_api("/__mockforge/smoke/run").await
    }

    pub async fn get_workspaces(&self) -> Result<Vec<WorkspaceInfo>> {
        self.get_api("/__mockforge/workspaces").await
    }

    // ── Tier 2 endpoints ─────────────────────────────────────────────

    pub async fn get_chaos_status(&self) -> Result<serde_json::Value> {
        self.get_api("/__mockforge/chaos").await
    }

    pub async fn toggle_chaos(&self, enabled: bool) -> Result<String> {
        self.post_api("/__mockforge/chaos/toggle", &serde_json::json!({ "enabled": enabled }))
            .await
    }

    pub async fn get_chaos_scenarios(&self) -> Result<serde_json::Value> {
        self.get_api("/__mockforge/chaos/scenarios/predefined").await
    }

    pub async fn start_chaos_scenario(&self, name: &str) -> Result<String> {
        self.post_api(&format!("/__mockforge/chaos/scenarios/{name}"), &serde_json::json!({}))
            .await
    }

    pub async fn stop_chaos_scenario(&self, name: &str) -> Result<String> {
        let url = format!("{}/__mockforge/chaos/scenarios/{name}", self.base_url);
        let resp = self.client.delete(&url).send().await.context("DELETE chaos/scenarios stop")?;

        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("HTTP {status} from chaos stop");
        }

        let body = resp.text().await.context("read chaos stop response")?;
        let envelope: ApiResponse<String> =
            serde_json::from_str(&body).context("deserialise chaos stop response")?;
        if envelope.success {
            Ok(envelope.data.unwrap_or_default())
        } else {
            anyhow::bail!(
                "stop scenario failed: {}",
                envelope.error.unwrap_or_else(|| "unknown".into())
            )
        }
    }

    pub async fn get_time_travel_status(&self) -> Result<TimeTravelStatus> {
        // Server returns raw TimeTravelStatus, not ApiResponse-wrapped
        self.get_raw("/__mockforge/time-travel/status").await
    }

    pub async fn get_chains(&self) -> Result<Vec<ChainInfo>> {
        self.get_api("/__mockforge/chains").await
    }

    pub async fn get_audit_logs(&self) -> Result<Vec<AuditEntry>> {
        self.get_api("/__mockforge/audit/logs").await
    }

    pub async fn get_analytics_summary(&self) -> Result<AnalyticsSummary> {
        self.get_api("/__mockforge/analytics/summary").await
    }

    // ── Tier 3 endpoints ─────────────────────────────────────────────

    pub async fn get_federation_peers(&self) -> Result<Vec<FederationPeer>> {
        self.get_api("/__mockforge/federation/peers").await
    }

    pub async fn get_contract_diff_captures(&self) -> Result<Vec<ContractDiffCapture>> {
        // Server returns {"captures": [...]} not ApiResponse-wrapped
        let resp = self
            .get("/__mockforge/contract-diff/captures")
            .send()
            .await
            .context("GET /__mockforge/contract-diff/captures")?;

        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("HTTP {status} from contract-diff/captures");
        }

        let ct = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if ct.contains("text/html") {
            anyhow::bail!("endpoint contract-diff/captures not available");
        }

        let body = resp.text().await.context("read contract-diff response")?;

        // Try {"captures": [...]} wrapper first (actual server format)
        if let Ok(wrapper) = serde_json::from_str::<ContractDiffWrapper>(&body) {
            return Ok(wrapper.captures);
        }

        // Try ApiResponse envelope
        if let Ok(envelope) = serde_json::from_str::<ApiResponse<Vec<ContractDiffCapture>>>(&body) {
            if envelope.success {
                return envelope.data.context("contract-diff: no data");
            }
        }

        // Try raw array
        serde_json::from_str::<Vec<ContractDiffCapture>>(&body)
            .context("deserialise contract-diff response")
    }

    // ── Behavioral cloning / VBR ───────────────────────────────────

    pub async fn get_vbr_status(&self) -> Result<serde_json::Value> {
        self.get_api("/__mockforge/vbr/status").await
    }

    // ── Config mutations ─────────────────────────────────────────────

    pub async fn update_latency(&self, config: &LatencyConfig) -> Result<String> {
        self.post_api("/__mockforge/config/latency", &serde_json::to_value(config)?)
            .await
    }

    pub async fn update_faults(&self, config: &FaultConfig) -> Result<String> {
        self.post_api("/__mockforge/config/faults", &serde_json::to_value(config)?)
            .await
    }

    pub async fn update_proxy(&self, config: &ProxyConfig) -> Result<String> {
        self.post_api("/__mockforge/config/proxy", &serde_json::to_value(config)?).await
    }

    // ── Verification ─────────────────────────────────────────────────

    pub async fn verify(&self, query: &serde_json::Value) -> Result<VerificationResult> {
        let resp = self
            .post("/__mockforge/verification/verify", query)
            .send()
            .await
            .context("POST verification/verify")?;

        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("HTTP {status} from verification/verify");
        }

        let body = resp.text().await.context("read verification response")?;
        let envelope: ApiResponse<VerificationResult> =
            serde_json::from_str(&body).context("deserialise verification response")?;

        if envelope.success {
            envelope.data.context("verification returned no data")
        } else {
            anyhow::bail!(
                "verification failed: {}",
                envelope.error.unwrap_or_else(|| "unknown".into())
            )
        }
    }

    // ── Time travel mutations ────────────────────────────────────────

    pub async fn enable_time_travel(&self) -> Result<String> {
        self.post_api("/__mockforge/time-travel/enable", &serde_json::json!({})).await
    }

    pub async fn disable_time_travel(&self) -> Result<String> {
        self.post_api("/__mockforge/time-travel/disable", &serde_json::json!({})).await
    }

    // ── Chain execution ──────────────────────────────────────────────

    pub async fn execute_chain(&self, id: &str) -> Result<serde_json::Value> {
        let path = format!("/__mockforge/chains/{id}/execute");
        let resp = self
            .post(&path, &serde_json::json!({}))
            .send()
            .await
            .with_context(|| format!("POST {path}"))?;

        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("HTTP {status} from {path}");
        }

        let body = resp.text().await.context("read chain execution response")?;
        let envelope: ApiResponse<serde_json::Value> =
            serde_json::from_str(&body).context("deserialise chain execution response")?;

        if envelope.success {
            envelope.data.context("chain execution returned no data")
        } else {
            anyhow::bail!(
                "chain execution failed: {}",
                envelope.error.unwrap_or_else(|| "unknown".into())
            )
        }
    }

    // ── Import ───────────────────────────────────────────────────────

    pub async fn get_import_history(&self) -> Result<serde_json::Value> {
        self.get_api("/__mockforge/import/history").await
    }

    pub async fn clear_import_history(&self) -> Result<String> {
        self.post_api("/__mockforge/import/history/clear", &serde_json::json!({})).await
    }

    // ── Recorder ─────────────────────────────────────────────────────

    pub async fn get_recorder_status(&self) -> Result<serde_json::Value> {
        self.get_api("/__mockforge/recorder/status").await
    }

    pub async fn toggle_recorder(&self, enable: bool) -> Result<String> {
        let path = if enable {
            "/__mockforge/recorder/start"
        } else {
            "/__mockforge/recorder/stop"
        };
        self.post_api(path, &serde_json::json!({})).await
    }

    // ── Workspace activation ──────────────────────────────────────────

    pub async fn activate_workspace(&self, workspace_id: &str) -> Result<String> {
        self.post_api(
            &format!("/__mockforge/workspaces/{workspace_id}/activate"),
            &serde_json::json!({}),
        )
        .await
    }

    // ── World State ──────────────────────────────────────────────────

    pub async fn get_world_state(&self) -> Result<serde_json::Value> {
        self.get_api("/__mockforge/world-state").await
    }

    // ── Connectivity check ───────────────────────────────────────────

    /// Quick ping to verify the admin server is reachable.
    pub async fn ping(&self) -> bool {
        self.get("/__mockforge/health")
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_strips_trailing_slash() {
        let client = MockForgeClient::new("http://localhost:9080/".into(), None).unwrap();
        assert_eq!(client.base_url(), "http://localhost:9080");
    }

    #[test]
    fn client_preserves_clean_url() {
        let client = MockForgeClient::new("http://localhost:9080".into(), None).unwrap();
        assert_eq!(client.base_url(), "http://localhost:9080");
    }
}
