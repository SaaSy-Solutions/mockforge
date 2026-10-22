//! Fly.io API integration for deploying mock services

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Fly.io API client for managing deployments
pub struct FlyioClient {
    api_token: String,
    base_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FlyioApp {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub hostname: Option<String>,
    #[serde(default)]
    pub organization: Option<FlyioOrganization>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FlyioOrganization {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FlyioMachine {
    pub id: String,
    pub name: String,
    pub state: String,
    /// Current Machine version, required for stopped-state confirmation.
    #[serde(default)]
    pub instance_id: Option<String>,
    pub region: String,
    pub image_ref: Option<FlyioImageRef>,
    pub config: FlyioMachineConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FlyioImageRef {
    pub registry: String,
    pub repository: String,
    pub tag: String,
    pub digest: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FlyioMachineConfig {
    pub image: String,
    pub env: HashMap<String, String>,
    pub services: Vec<FlyioService>,
    pub checks: Option<HashMap<String, FlyioCheck>>,
    /// VM resource sizing. `None` means accept Fly's API default
    /// (currently `shared-cpu-1x:256MB`); existing deployments and
    /// non-plugin-enabled machines stay on that default. Cloud
    /// plugins requires explicit sizing — see
    /// `docs/plugins/security/cloud-runtime-sidecar-spike.md` for
    /// the measured-on-Fly numbers and tier table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guest: Option<FlyioGuest>,
}

/// VM sizing block sent to Fly Machines API as `config.guest`.
/// Mirrors the API's expected JSON shape:
/// <https://fly.io/docs/machines/api/machines-resource/#machine-config-object-properties>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlyioGuest {
    /// `"shared"` or `"performance"`.
    pub cpu_kind: String,
    pub cpus: u32,
    pub memory_mb: u32,
}

impl FlyioGuest {
    /// `shared-cpu-1x:256MB` — Fly's default for the registry today.
    /// Used when no plugins are attached so existing hosted-mocks
    /// keep their current footprint.
    pub fn shared_256() -> Self {
        Self {
            cpu_kind: "shared".into(),
            cpus: 1,
            memory_mb: 256,
        }
    }

    /// `shared-cpu-1x:512MB` — Pro tier with cloud plugins. Real-Fly
    /// measurement on the spike showed mockforge + sidecar idle at
    /// ~166 MB / 256 MB (65%), with only 71 MB headroom; bumping to
    /// 512 MB gives 316 MB headroom — comfortable for ≤5 plugins
    /// at typical sizes.
    pub fn shared_512() -> Self {
        Self {
            cpu_kind: "shared".into(),
            cpus: 1,
            memory_mb: 512,
        }
    }

    /// `shared-cpu-1x:1024MB` — Team tier with cloud plugins
    /// (≤25 plugins per mock).
    pub fn shared_1024() -> Self {
        Self {
            cpu_kind: "shared".into(),
            cpus: 1,
            memory_mb: 1024,
        }
    }

    /// `shared-cpu-2x:2048MB` — Enterprise tier with metered
    /// pass-through. Two cores so heavy plugin workloads don't
    /// starve mockforge's request path.
    pub fn shared_2x_2048() -> Self {
        Self {
            cpu_kind: "shared".into(),
            cpus: 2,
            memory_mb: 2048,
        }
    }

    /// Pick the right Fly machine size for a hosted-mock deployment
    /// based on org plan + whether cloud plugins are attached.
    ///
    /// Without plugins, every tier stays on the existing 256 MB
    /// default — no behavior change for legacy deployments.
    ///
    /// With plugins, the floor bumps according to the
    /// real-microVM measurements in
    /// `docs/plugins/security/cloud-runtime-sidecar-spike.md`:
    /// idle on 256 MB sits at ~166 MB / 65% utilization with only
    /// ~71 MB headroom. That's not enough for the plugin runtime to
    /// grow into without OOM-killing mockforge. 512 MB gives 316 MB
    /// headroom which fits ≤5 plugins comfortably; Team's larger
    /// plugin count needs 1024 MB.
    pub fn for_hosted_mock(plan: &str, plugins_enabled: bool) -> Self {
        if !plugins_enabled {
            return Self::shared_256();
        }
        match plan.to_lowercase().as_str() {
            "free" => Self::shared_256(), // Plugins not available on Free.
            "pro" => Self::shared_512(),
            "team" => Self::shared_1024(),
            // Unknown / future plans (e.g. "enterprise") get the
            // largest currently-available shape. The handler should
            // refuse plugin attachment for unknown plans before
            // ever reaching this; defaulting big is fail-safe.
            _ => Self::shared_2x_2048(),
        }
    }
}

/// Registry authentication for pulling private Docker images
#[derive(Debug, Serialize, Deserialize)]
pub struct FlyioRegistryAuth {
    pub server: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FlyioService {
    pub protocol: String,
    pub internal_port: u16,
    pub ports: Vec<FlyioPort>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FlyioPort {
    pub port: u16,
    pub handlers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FlyioCheck {
    #[serde(rename = "type")]
    pub check_type: String,
    pub port: u16,
    pub grace_period: String,
    pub interval: String,
    pub method: String,
    pub timeout: String,
    pub tls_skip_verify: bool,
    pub path: Option<String>,
}

impl FlyioClient {
    pub fn new(api_token: String) -> Self {
        Self {
            api_token,
            base_url: "https://api.machines.dev".to_string(),
        }
    }

    #[cfg(test)]
    pub(super) fn for_test_endpoint(base_url: String) -> Self {
        Self {
            api_token: uuid::Uuid::new_v4().to_string(),
            base_url,
        }
    }

    /// Get the API token (used for Fly.io registry auth)
    pub fn api_token(&self) -> &str {
        &self.api_token
    }

    /// Create a new Fly.io app
    pub async fn create_app(&self, app_name: &str, org_slug: &str) -> Result<FlyioApp> {
        let client = reqwest::Client::new();
        let url = format!("{}/v1/apps", self.base_url);

        let payload = serde_json::json!({
            "app_name": app_name,
            "org_slug": org_slug,
        });

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("Failed to create Fly.io app")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());

            // Handle "Name has already been taken" (422) — the app exists, fetch it instead
            if status.as_u16() == 422 && error_text.contains("already been taken") {
                tracing::info!("Fly.io app '{}' already exists, fetching existing app", app_name);
                return self.get_app(app_name).await;
            }

            anyhow::bail!("Failed to create Fly.io app: {} - {}", status, error_text);
        }

        let app: FlyioApp = response.json().await.context("Failed to parse Fly.io app response")?;

        Ok(app)
    }

    /// Create a machine (instance) for the app
    pub async fn create_machine(
        &self,
        app_name: &str,
        config: FlyioMachineConfig,
        region: &str,
        registry_auth: Option<FlyioRegistryAuth>,
    ) -> Result<FlyioMachine> {
        let client = reqwest::Client::new();
        let url = format!("{}/v1/apps/{}/machines", self.base_url, app_name);

        let mut payload = serde_json::json!({
            "config": config,
            "region": region,
        });
        if let Some(auth) = registry_auth {
            payload["config"]["image_registry_auth"] =
                serde_json::to_value(auth).context("Failed to serialize registry auth")?;
        }

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("Failed to create Fly.io machine")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to create Fly.io machine: {} - {}", status, error_text);
        }

        let machine: FlyioMachine =
            response.json().await.context("Failed to parse Fly.io machine response")?;

        Ok(machine)
    }

    /// Update a machine's configuration (image, env vars, etc.)
    pub async fn update_machine(
        &self,
        app_name: &str,
        machine_id: &str,
        config: FlyioMachineConfig,
        registry_auth: Option<FlyioRegistryAuth>,
    ) -> Result<FlyioMachine> {
        let client = reqwest::Client::new();
        let url = format!("{}/v1/apps/{}/machines/{}", self.base_url, app_name, machine_id);

        let mut payload = serde_json::json!({
            "config": config,
        });
        if let Some(auth) = registry_auth {
            payload["config"]["image_registry_auth"] =
                serde_json::to_value(auth).context("Failed to serialize registry auth")?;
        }

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("Failed to update Fly.io machine")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to update Fly.io machine: {} - {}", status, error_text);
        }

        let machine: FlyioMachine =
            response.json().await.context("Failed to parse Fly.io machine response")?;

        Ok(machine)
    }

    /// Get machine status
    pub async fn get_machine(&self, app_name: &str, machine_id: &str) -> Result<FlyioMachine> {
        let client = reqwest::Client::new();
        let url = format!("{}/v1/apps/{}/machines/{}", self.base_url, app_name, machine_id);

        let response = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(15))
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await
            .context("Failed to get Fly.io machine")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to get Fly.io machine: {} - {}", status, error_text);
        }

        let machine: FlyioMachine =
            response.json().await.context("Failed to parse Fly.io machine response")?;

        Ok(machine)
    }

    /// Stop a running machine (graceful shutdown, keeps the machine around
    /// so it can be restarted later without recreating its config).
    pub async fn stop_machine(&self, app_name: &str, machine_id: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!("{}/v1/apps/{}/machines/{}/stop", self.base_url, app_name, machine_id);

        let response = client
            .post(&url)
            .timeout(std::time::Duration::from_secs(15))
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({}))
            .send()
            .await
            .context("Failed to stop Fly.io machine")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to stop Fly.io machine: {} - {}", status, error_text);
        }

        Ok(())
    }

    /// Start a stopped machine.
    pub async fn start_machine(&self, app_name: &str, machine_id: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!("{}/v1/apps/{}/machines/{}/start", self.base_url, app_name, machine_id);

        let response = client
            .post(&url)
            .timeout(std::time::Duration::from_secs(15))
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .send()
            .await
            .context("Failed to start Fly.io machine")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to start Fly.io machine: {} - {}", status, error_text);
        }

        Ok(())
    }

    /// Wait for a provider-confirmed state; stopped waits are scoped to the
    /// instance returned before requesting shutdown, as required by Fly.
    async fn wait_for_state(
        &self,
        app_name: &str,
        machine_id: &str,
        state: &str,
        instance_id: Option<&str>,
    ) -> Result<()> {
        let url = format!("{}/v1/apps/{}/machines/{}/wait", self.base_url, app_name, machine_id);
        let client = reqwest::Client::new();
        let mut request = client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .timeout(std::time::Duration::from_secs(15))
            .query(&[("state", state), ("timeout", "10")]);
        if let Some(instance_id) = instance_id {
            request = request.query(&[("instance_id", instance_id)]);
        }
        let response = request.send().await.context("Failed to confirm Fly.io machine state")?;
        if !response.status().is_success() {
            anyhow::bail!("Fly.io did not confirm machine state {}: {}", state, response.status());
        }
        let confirmation: serde_json::Value =
            response.json().await.context("Invalid Fly.io state confirmation response")?;
        if confirmation.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
            anyhow::bail!("Fly.io state confirmation did not acknowledge success");
        }
        Ok(())
    }

    /// Start a machine and confirm it reached the running state.
    pub async fn start_machine_confirmed(&self, app_name: &str, machine_id: &str) -> Result<()> {
        self.start_machine(app_name, machine_id).await?;
        self.wait_for_state(app_name, machine_id, "started", None).await
    }

    /// Stop a machine and confirm shutdown before a capacity slot is released.
    pub async fn stop_machine_confirmed(&self, app_name: &str, machine_id: &str) -> Result<()> {
        let machine = self.get_machine(app_name, machine_id).await?;
        if machine.state == "stopped" {
            return Ok(());
        }
        let instance_id = machine
            .instance_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Fly.io machine response lacks instance_id"))?;
        self.stop_machine(app_name, machine_id).await?;
        self.wait_for_state(app_name, machine_id, "stopped", Some(instance_id)).await
    }

    /// Delete a machine
    pub async fn delete_machine(&self, app_name: &str, machine_id: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!("{}/v1/apps/{}/machines/{}", self.base_url, app_name, machine_id);

        let response = client
            .delete(&url)
            .timeout(std::time::Duration::from_secs(15))
            .query(&[("force", "true")])
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await
            .context("Failed to delete Fly.io machine")?;

        let status = response.status();
        if !status.is_success() && status != reqwest::StatusCode::NOT_FOUND {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to delete Fly.io machine: {} - {}", status, error_text);
        }

        Ok(())
    }

    /// Delete a Fly.io app
    pub async fn delete_app(&self, app_name: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!("{}/v1/apps/{}", self.base_url, app_name);

        let response = client
            .delete(&url)
            .timeout(std::time::Duration::from_secs(15))
            .query(&[("force", "true")])
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await
            .context("Failed to delete Fly.io app")?;

        let status = response.status();
        if !status.is_success() && status != reqwest::StatusCode::NOT_FOUND {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to delete Fly.io app: {} - {}", status, error_text);
        }

        Ok(())
    }

    /// Delete all app resources and confirm absence. An accepted deletion is
    /// not proof that cleanup finished; keep the deployment retryable until
    /// the app-details endpoint returns 404.
    pub async fn delete_app_confirmed(&self, app_name: &str) -> Result<()> {
        self.delete_app(app_name).await?;
        let client = reqwest::Client::new();
        let url = format!("{}/v1/apps/{}", self.base_url, app_name);
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                anyhow::bail!("Fly.io app deletion is not yet confirmed; retry cleanup");
            }
            let response = client
                .get(&url)
                .timeout(remaining)
                .header("Authorization", format!("Bearer {}", self.api_token))
                .send()
                .await
                .context("Failed to confirm Fly.io app deletion")?;
            let status = response.status();
            if status == reqwest::StatusCode::NOT_FOUND {
                return Ok(());
            }
            if !status.is_success() {
                anyhow::bail!("Fly.io app absence is unconfirmed: {}", status);
            }
            tokio::time::sleep(std::time::Duration::from_millis(100).min(remaining)).await;
        }
    }

    /// Allocate a shared IPv4 and a dedicated IPv6 address for an app
    pub async fn allocate_ips(&self, app_name: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let graphql_url = "https://api.fly.io/graphql";

        // Allocate shared IPv4
        let ipv4_query = serde_json::json!({
            "query": "mutation($input: AllocateIPAddressInput!) { allocateIpAddress(input: $input) { ipAddress { id address type } } }",
            "variables": {
                "input": {
                    "appId": app_name,
                    "type": "shared_v4"
                }
            }
        });

        let response = client
            .post(graphql_url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .json(&ipv4_query)
            .send()
            .await
            .context("Failed to allocate shared IPv4")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to allocate shared IPv4: {}", error_text);
        }

        // Allocate IPv6
        let ipv6_query = serde_json::json!({
            "query": "mutation($input: AllocateIPAddressInput!) { allocateIpAddress(input: $input) { ipAddress { id address type } } }",
            "variables": {
                "input": {
                    "appId": app_name,
                    "type": "v6"
                }
            }
        });

        let response = client
            .post(graphql_url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .json(&ipv6_query)
            .send()
            .await
            .context("Failed to allocate IPv6")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to allocate IPv6: {}", error_text);
        }

        Ok(())
    }

    /// Get app info
    pub async fn get_app(&self, app_name: &str) -> Result<FlyioApp> {
        let client = reqwest::Client::new();
        let url = format!("{}/v1/apps/{}", self.base_url, app_name);

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await
            .context("Failed to get Fly.io app")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to get Fly.io app: {} - {}", status, error_text);
        }

        let app: FlyioApp = response.json().await.context("Failed to parse Fly.io app response")?;

        Ok(app)
    }

    /// Add a custom domain certificate to an app
    ///
    /// This tells Fly.io to provision a Let's Encrypt TLS certificate for the
    /// given hostname and route traffic for that hostname to this app via SNI.
    pub async fn add_certificate(&self, app_name: &str, hostname: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let graphql_url = "https://api.fly.io/graphql";

        let query = serde_json::json!({
            "query": "mutation($appId: ID!, $hostname: String!) { addCertificate(appId: $appId, hostname: $hostname) { certificate { id hostname } } }",
            "variables": {
                "appId": app_name,
                "hostname": hostname
            }
        });

        let response = client
            .post(graphql_url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .json(&query)
            .send()
            .await
            .context("Failed to add certificate")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to add certificate for {}: {}", hostname, error_text);
        }

        // Check for GraphQL-level errors
        let body: serde_json::Value =
            response.json().await.context("Failed to parse certificate response")?;
        if let Some(errors) = body.get("errors") {
            // "already exists" is fine — idempotent
            let err_str = errors.to_string();
            if !err_str.contains("already exists") {
                anyhow::bail!("Failed to add certificate for {}: {}", hostname, err_str);
            }
        }

        Ok(())
    }

    /// Remove a custom domain certificate from an app
    pub async fn delete_certificate(&self, app_name: &str, hostname: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let graphql_url = "https://api.fly.io/graphql";

        let query = serde_json::json!({
            "query": "mutation($appId: ID!, $hostname: String!) { deleteCertificate(appId: $appId, hostname: $hostname) { app { name } } }",
            "variables": {
                "appId": app_name,
                "hostname": hostname
            }
        });

        let response = client
            .post(graphql_url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .json(&query)
            .send()
            .await
            .context("Failed to delete certificate")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete certificate for {}: {}", hostname, error_text);
        }

        Ok(())
    }

    /// List machines for an app
    pub async fn list_machines(&self, app_name: &str) -> Result<Vec<FlyioMachine>> {
        let client = reqwest::Client::new();
        let url = format!("{}/v1/apps/{}/machines", self.base_url, app_name);

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await
            .context("Failed to list Fly.io machines")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to list Fly.io machines: {} - {}", status, error_text);
        }

        let machines: Vec<FlyioMachine> =
            response.json().await.context("Failed to parse Fly.io machines response")?;

        Ok(machines)
    }
}

#[cfg(test)]
mod guest_tests {
    use super::*;

    #[test]
    fn for_hosted_mock_no_plugins_uses_legacy_256() {
        // Existing hosted-mocks without cloud plugins must keep
        // their current 256 MB footprint — no surprise pricing
        // bumps for legacy deployments.
        for plan in ["free", "pro", "team", "enterprise", "weird-future-plan"] {
            let g = FlyioGuest::for_hosted_mock(plan, false);
            assert_eq!(g.memory_mb, 256, "plan {} without plugins must stay at 256MB", plan);
            assert_eq!(g.cpu_kind, "shared");
            assert_eq!(g.cpus, 1);
        }
    }

    #[test]
    fn for_hosted_mock_pro_with_plugins_bumps_to_512() {
        let g = FlyioGuest::for_hosted_mock("pro", true);
        assert_eq!(g.memory_mb, 512);
        assert_eq!(g.cpus, 1);
    }

    #[test]
    fn for_hosted_mock_team_with_plugins_bumps_to_1024() {
        let g = FlyioGuest::for_hosted_mock("team", true);
        assert_eq!(g.memory_mb, 1024);
        assert_eq!(g.cpus, 1);
    }

    #[test]
    fn for_hosted_mock_free_with_plugins_stays_256() {
        // Plugins aren't available on Free — handler rejects before
        // we get here — but if we do, don't accidentally upsize
        // a Free org's machine.
        let g = FlyioGuest::for_hosted_mock("free", true);
        assert_eq!(g.memory_mb, 256);
    }

    #[test]
    fn for_hosted_mock_unknown_plan_with_plugins_fails_safe_high() {
        // Unknown plans (e.g. future "enterprise") get the largest
        // shape rather than under-provisioning.
        let g = FlyioGuest::for_hosted_mock("enterprise", true);
        assert_eq!(g.memory_mb, 2048);
        assert_eq!(g.cpus, 2);
    }

    #[test]
    fn for_hosted_mock_plan_string_is_case_insensitive() {
        let g = FlyioGuest::for_hosted_mock("PRO", true);
        assert_eq!(g.memory_mb, 512);
        let g = FlyioGuest::for_hosted_mock("Team", true);
        assert_eq!(g.memory_mb, 1024);
    }

    #[test]
    fn machine_config_serialization_omits_guest_when_none() {
        // Existing call sites that pass `guest: None` (or use the
        // legacy struct without the field via Default) must produce
        // JSON without a `guest` key — so Fly's API default kicks in.
        let cfg = FlyioMachineConfig {
            image: "img".into(),
            env: HashMap::new(),
            services: vec![],
            checks: None,
            guest: None,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(!json.contains("guest"), "guest=None must be omitted, got {}", json);
    }

    #[test]
    fn machine_config_serializes_guest_when_set() {
        let cfg = FlyioMachineConfig {
            image: "img".into(),
            env: HashMap::new(),
            services: vec![],
            checks: None,
            guest: Some(FlyioGuest::shared_512()),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(json.contains("\"guest\""));
        assert!(json.contains("\"memory_mb\":512"));
        assert!(json.contains("\"cpu_kind\":\"shared\""));
    }
}

#[cfg(test)]
mod lifecycle_wire_tests {
    use super::*;
    use axum::{
        extract::Request,
        http::{Method, StatusCode},
        response::IntoResponse,
        routing::any,
        Json, Router,
    };
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn app_cleanup_requires_confirmed_absence_and_preserves_provider_failures() {
        use std::collections::VecDeque;
        // Accepted cleanup must observe disappearance; a present app or an
        // inaccessible existence check must never count as completed cleanup.
        for (statuses, succeeds, expected_calls) in [
            (vec![StatusCode::ACCEPTED, StatusCode::OK, StatusCode::NOT_FOUND], true, 3),
            (vec![StatusCode::NOT_FOUND, StatusCode::NOT_FOUND], true, 2),
            (vec![StatusCode::ACCEPTED, StatusCode::SERVICE_UNAVAILABLE], false, 2),
            (vec![StatusCode::INTERNAL_SERVER_ERROR], false, 1),
        ] {
            let remaining = Arc::new(Mutex::new(VecDeque::from(statuses)));
            let calls = Arc::new(Mutex::new(Vec::new()));
            let captured = calls.clone();
            let router = Router::new().fallback(any(move |request: Request| {
                let remaining = remaining.clone();
                let captured = captured.clone();
                async move {
                    captured
                        .lock()
                        .unwrap()
                        .push((request.method().clone(), request.uri().to_string()));
                    remaining.lock().unwrap().pop_front().expect("unexpected provider call")
                }
            }));
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let client = FlyioClient {
                api_token: uuid::Uuid::new_v4().to_string(),
                base_url: format!("http://{}", listener.local_addr().unwrap()),
            };
            let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
            let result = client.delete_app_confirmed("app-fixture").await;
            task.abort();
            assert_eq!(result.is_ok(), succeeds);
            let calls = calls.lock().unwrap();
            assert_eq!(calls.len(), expected_calls);
            assert_eq!(calls[0].0, Method::DELETE);
            assert!(calls[0].1.contains("force=true"));
            for call in calls.iter().skip(1) {
                assert_eq!(call.0, Method::GET);
                assert_eq!(call.1, "/v1/apps/app-fixture");
            }
        }
    }

    struct Fixture {
        client: FlyioClient,
        requests: Arc<Mutex<Vec<(Method, String)>>>,
        task: tokio::task::JoinHandle<()>,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    async fn fixture(
        machine_state: &str,
        instance: Option<&str>,
        wait_status: StatusCode,
    ) -> Fixture {
        fixture_with_response(machine_state, instance, wait_status, serde_json::json!({"ok":true}))
            .await
    }

    async fn fixture_with_response(
        machine_state: &str,
        instance: Option<&str>,
        wait_status: StatusCode,
        wait_body: serde_json::Value,
    ) -> Fixture {
        let machine = serde_json::json!({
            "id":"machine-fixture", "name":"fixture", "state":machine_state,
            "instance_id":instance, "region":"iad", "image_ref":null,
            "config":{"image":"fixture", "env":{}, "services":[], "checks":null}
        });
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = requests.clone();
        let router = Router::new().fallback(any(move |request: Request| {
            let captured = captured.clone();
            let machine = machine.clone();
            let wait_body = wait_body.clone();
            async move {
                let method = request.method().clone();
                let uri = request.uri().to_string();
                let path = request.uri().path().to_string();
                captured.lock().unwrap().push((method.clone(), uri));
                if path.ends_with("/wait") {
                    return (wait_status, Json(wait_body)).into_response();
                }
                if method == Method::GET {
                    return Json(machine).into_response();
                }
                if method == Method::DELETE {
                    return StatusCode::NOT_FOUND.into_response();
                }
                StatusCode::OK.into_response()
            }
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        // This value has no authority outside the local fixture and is never
        // captured: the fixture records only methods and non-secret URIs.
        let client = FlyioClient {
            api_token: uuid::Uuid::new_v4().to_string(),
            base_url,
        };
        Fixture {
            client,
            requests,
            task,
        }
    }

    #[tokio::test]
    async fn accepted_start_with_failed_confirmation_returns_error() {
        let f = fixture("starting", Some("instance-fixture"), StatusCode::GATEWAY_TIMEOUT).await;
        assert!(f
            .client
            .start_machine_confirmed("app-fixture", "machine-fixture")
            .await
            .is_err());
        let calls = f.requests.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, Method::POST);
        assert!(calls[1].1.contains("state=started"));
    }

    #[tokio::test]
    async fn confirmed_start_waits_for_started() {
        let f = fixture("starting", Some("instance-fixture"), StatusCode::OK).await;
        f.client
            .start_machine_confirmed("app-fixture", "machine-fixture")
            .await
            .unwrap();
        let calls = f.requests.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert!(calls[1].1.contains("state=started"));
    }

    #[tokio::test]
    async fn successful_http_status_without_acknowledgment_is_not_confirmation() {
        for body in [
            serde_json::json!({"ok":false}),
            serde_json::json!({"ok":"true"}),
            serde_json::json!({}),
        ] {
            let f =
                fixture_with_response("starting", Some("instance-fixture"), StatusCode::OK, body)
                    .await;
            assert!(f
                .client
                .start_machine_confirmed("app-fixture", "machine-fixture")
                .await
                .is_err());
        }
    }

    #[tokio::test]
    async fn stop_wait_uses_provider_instance_id() {
        let f = fixture("started", Some("instance-fixture"), StatusCode::OK).await;
        f.client.stop_machine_confirmed("app-fixture", "machine-fixture").await.unwrap();
        let calls = f.requests.lock().unwrap();
        assert_eq!(calls.len(), 3);
        assert!(calls[1].1.ends_with("/stop"));
        assert!(calls[2].1.contains("state=stopped"));
        assert!(calls[2].1.contains("instance_id=instance-fixture"));
    }

    #[tokio::test]
    async fn already_stopped_does_not_request_another_stop() {
        let f = fixture("stopped", None, StatusCode::GATEWAY_TIMEOUT).await;
        f.client.stop_machine_confirmed("app-fixture", "machine-fixture").await.unwrap();
        assert_eq!(f.requests.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn missing_instance_id_does_not_claim_confirmation() {
        let f = fixture("started", None, StatusCode::OK).await;
        assert!(f.client.stop_machine_confirmed("app-fixture", "machine-fixture").await.is_err());
        assert_eq!(f.requests.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn failed_stop_confirmation_returns_error() {
        let f = fixture("started", Some("instance-fixture"), StatusCode::GATEWAY_TIMEOUT).await;
        assert!(f.client.stop_machine_confirmed("app-fixture", "machine-fixture").await.is_err());
    }

    #[tokio::test]
    async fn deletion_is_forced_and_already_missing_resources_are_idempotent() {
        let f = fixture("started", Some("instance-fixture"), StatusCode::OK).await;
        f.client.delete_machine("app-fixture", "machine-fixture").await.unwrap();
        f.client.delete_app("app-fixture").await.unwrap();
        let calls = f.requests.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, Method::DELETE);
        assert!(calls[0].1.contains("force=true"));
    }
}
