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

    /// Send a GET and unwrap the `ApiResponse<T>` envelope.
    async fn get_api<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let resp: ApiResponse<T> = self
            .get(path)
            .send()
            .await
            .with_context(|| format!("GET {path}"))?
            .json()
            .await
            .with_context(|| format!("deserialise response from {path}"))?;

        if resp.success {
            resp.data.context("API returned success but no data")
        } else {
            anyhow::bail!("API error: {}", resp.error.unwrap_or_else(|| "unknown".into()))
        }
    }

    /// Send a GET and return raw JSON (for endpoints that don't use `ApiResponse`).
    async fn get_raw<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.get(path)
            .send()
            .await
            .with_context(|| format!("GET {path}"))?
            .json()
            .await
            .with_context(|| format!("deserialise response from {path}"))
    }

    // ── Tier 1 endpoints ─────────────────────────────────────────────

    pub async fn get_dashboard(&self) -> Result<DashboardData> {
        self.get_api("/__mockforge/dashboard").await
    }

    pub async fn get_routes(&self) -> Result<Vec<RouteInfo>> {
        self.get_api("/__mockforge/routes").await
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
        self.get_api("/__mockforge/server-info").await
    }

    pub async fn get_plugins(&self) -> Result<Vec<PluginInfo>> {
        self.get_api("/__mockforge/plugins").await
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
        let resp: ApiResponse<String> = self
            .post("/__mockforge/chaos/toggle", &serde_json::json!({ "enabled": enabled }))
            .send()
            .await
            .context("POST chaos/toggle")?
            .json()
            .await
            .context("deserialise chaos toggle response")?;
        if resp.success {
            Ok(resp.data.unwrap_or_default())
        } else {
            anyhow::bail!("chaos toggle failed: {}", resp.error.unwrap_or_else(|| "unknown".into()))
        }
    }

    pub async fn get_chaos_scenarios(&self) -> Result<serde_json::Value> {
        self.get_api("/__mockforge/chaos/scenarios/predefined").await
    }

    pub async fn start_chaos_scenario(&self, name: &str) -> Result<String> {
        let resp: ApiResponse<String> = self
            .post(&format!("/__mockforge/chaos/scenarios/{name}"), &serde_json::json!({}))
            .send()
            .await
            .context("POST chaos/scenarios start")?
            .json()
            .await
            .context("deserialise chaos scenario response")?;
        if resp.success {
            Ok(resp.data.unwrap_or_default())
        } else {
            anyhow::bail!(
                "start scenario failed: {}",
                resp.error.unwrap_or_else(|| "unknown".into())
            )
        }
    }

    pub async fn stop_chaos_scenario(&self, name: &str) -> Result<String> {
        let url = format!("{}/__mockforge/chaos/scenarios/{name}", self.base_url);
        let resp: ApiResponse<String> = self
            .client
            .delete(&url)
            .send()
            .await
            .context("DELETE chaos/scenarios stop")?
            .json()
            .await
            .context("deserialise chaos scenario stop response")?;
        if resp.success {
            Ok(resp.data.unwrap_or_default())
        } else {
            anyhow::bail!(
                "stop scenario failed: {}",
                resp.error.unwrap_or_else(|| "unknown".into())
            )
        }
    }

    pub async fn get_time_travel_status(&self) -> Result<TimeTravelStatus> {
        self.get_api("/__mockforge/time-travel/status").await
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
        self.get_api("/__mockforge/contract-diff/captures").await
    }

    // ── Behavioral cloning / VBR ───────────────────────────────────

    pub async fn get_vbr_status(&self) -> Result<serde_json::Value> {
        self.get_api("/__mockforge/vbr/status").await
    }

    // ── Config mutations ─────────────────────────────────────────────

    pub async fn update_latency(&self, config: &LatencyConfig) -> Result<String> {
        let resp: ApiResponse<String> = self
            .post("/__mockforge/config/latency", config)
            .send()
            .await
            .context("POST config/latency")?
            .json()
            .await
            .context("deserialise latency response")?;
        if resp.success {
            Ok(resp.data.unwrap_or_default())
        } else {
            anyhow::bail!(
                "update latency failed: {}",
                resp.error.unwrap_or_else(|| "unknown".into())
            )
        }
    }

    pub async fn update_faults(&self, config: &FaultConfig) -> Result<String> {
        let resp: ApiResponse<String> = self
            .post("/__mockforge/config/faults", config)
            .send()
            .await
            .context("POST config/faults")?
            .json()
            .await
            .context("deserialise faults response")?;
        if resp.success {
            Ok(resp.data.unwrap_or_default())
        } else {
            anyhow::bail!(
                "update faults failed: {}",
                resp.error.unwrap_or_else(|| "unknown".into())
            )
        }
    }

    pub async fn update_proxy(&self, config: &ProxyConfig) -> Result<String> {
        let resp: ApiResponse<String> = self
            .post("/__mockforge/config/proxy", config)
            .send()
            .await
            .context("POST config/proxy")?
            .json()
            .await
            .context("deserialise proxy response")?;
        if resp.success {
            Ok(resp.data.unwrap_or_default())
        } else {
            anyhow::bail!("update proxy failed: {}", resp.error.unwrap_or_else(|| "unknown".into()))
        }
    }

    // ── Verification ─────────────────────────────────────────────────

    pub async fn verify(&self, query: &serde_json::Value) -> Result<VerificationResult> {
        let resp: ApiResponse<VerificationResult> = self
            .post("/__mockforge/verification/verify", query)
            .send()
            .await
            .context("POST verification/verify")?
            .json()
            .await
            .context("deserialise verification response")?;
        if resp.success {
            resp.data.context("verification returned no data")
        } else {
            anyhow::bail!("verification failed: {}", resp.error.unwrap_or_else(|| "unknown".into()))
        }
    }

    // ── Time travel mutations ────────────────────────────────────────

    pub async fn enable_time_travel(&self) -> Result<String> {
        let resp: ApiResponse<String> = self
            .post("/__mockforge/time-travel/enable", &serde_json::json!({}))
            .send()
            .await
            .context("POST time-travel/enable")?
            .json()
            .await
            .context("deserialise time-travel response")?;
        if resp.success {
            Ok(resp.data.unwrap_or_default())
        } else {
            anyhow::bail!(
                "enable time-travel failed: {}",
                resp.error.unwrap_or_else(|| "unknown".into())
            )
        }
    }

    pub async fn disable_time_travel(&self) -> Result<String> {
        let resp: ApiResponse<String> = self
            .post("/__mockforge/time-travel/disable", &serde_json::json!({}))
            .send()
            .await
            .context("POST time-travel/disable")?
            .json()
            .await
            .context("deserialise time-travel response")?;
        if resp.success {
            Ok(resp.data.unwrap_or_default())
        } else {
            anyhow::bail!(
                "disable time-travel failed: {}",
                resp.error.unwrap_or_else(|| "unknown".into())
            )
        }
    }

    // ── Chain execution ──────────────────────────────────────────────

    pub async fn execute_chain(&self, id: &str) -> Result<serde_json::Value> {
        let path = format!("/__mockforge/chains/{id}/execute");
        let resp: ApiResponse<serde_json::Value> = self
            .post(&path, &serde_json::json!({}))
            .send()
            .await
            .with_context(|| format!("POST chains/{id}/execute"))?
            .json()
            .await
            .context("deserialise chain execution response")?;
        if resp.success {
            resp.data.context("chain execution returned no data")
        } else {
            anyhow::bail!(
                "chain execution failed: {}",
                resp.error.unwrap_or_else(|| "unknown".into())
            )
        }
    }

    // ── Import ───────────────────────────────────────────────────────

    pub async fn get_import_history(&self) -> Result<serde_json::Value> {
        self.get_api("/__mockforge/import/history").await
    }

    pub async fn clear_import_history(&self) -> Result<String> {
        let resp: ApiResponse<String> = self
            .post("/__mockforge/import/history/clear", &serde_json::json!({}))
            .send()
            .await
            .context("POST import/history/clear")?
            .json()
            .await
            .context("deserialise import clear response")?;
        if resp.success {
            Ok(resp.data.unwrap_or_default())
        } else {
            anyhow::bail!(
                "clear import history failed: {}",
                resp.error.unwrap_or_else(|| "unknown".into())
            )
        }
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
        let resp: ApiResponse<String> = self
            .post(path, &serde_json::json!({}))
            .send()
            .await
            .with_context(|| format!("POST {path}"))?
            .json()
            .await
            .context("deserialise recorder toggle response")?;
        if resp.success {
            Ok(resp.data.unwrap_or_default())
        } else {
            anyhow::bail!(
                "recorder toggle failed: {}",
                resp.error.unwrap_or_else(|| "unknown".into())
            )
        }
    }

    // ── Workspace activation ──────────────────────────────────────────

    pub async fn activate_workspace(&self, workspace_id: &str) -> Result<String> {
        let path = format!("/__mockforge/workspaces/{workspace_id}/activate");
        let resp: ApiResponse<String> = self
            .post(&path, &serde_json::json!({}))
            .send()
            .await
            .with_context(|| format!("POST {path}"))?
            .json()
            .await
            .context("deserialise workspace activation response")?;
        if resp.success {
            Ok(resp.data.unwrap_or_default())
        } else {
            anyhow::bail!(
                "workspace activation failed: {}",
                resp.error.unwrap_or_else(|| "unknown".into())
            )
        }
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
