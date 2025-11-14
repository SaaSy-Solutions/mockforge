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
    pub name: String,
    pub hostname: String,
    pub organization: FlyioOrganization,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FlyioOrganization {
    pub id: String,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FlyioMachine {
    pub id: String,
    pub name: String,
    pub state: String,
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
            anyhow::bail!("Failed to create Fly.io app: {} - {}", status, error_text);
        }

        let app: FlyioApp = response
            .json()
            .await
            .context("Failed to parse Fly.io app response")?;

        Ok(app)
    }

    /// Create a machine (instance) for the app
    pub async fn create_machine(
        &self,
        app_name: &str,
        config: FlyioMachineConfig,
        region: &str,
    ) -> Result<FlyioMachine> {
        let client = reqwest::Client::new();
        let url = format!("{}/v1/apps/{}/machines", self.base_url, app_name);

        let payload = serde_json::json!({
            "config": config,
            "region": region,
        });

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

        let machine: FlyioMachine = response
            .json()
            .await
            .context("Failed to parse Fly.io machine response")?;

        Ok(machine)
    }

    /// Get machine status
    pub async fn get_machine(&self, app_name: &str, machine_id: &str) -> Result<FlyioMachine> {
        let client = reqwest::Client::new();
        let url = format!("{}/v1/apps/{}/machines/{}", self.base_url, app_name, machine_id);

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await
            .context("Failed to get Fly.io machine")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to get Fly.io machine: {} - {}", status, error_text);
        }

        let machine: FlyioMachine = response
            .json()
            .await
            .context("Failed to parse Fly.io machine response")?;

        Ok(machine)
    }

    /// Delete a machine
    pub async fn delete_machine(&self, app_name: &str, machine_id: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!("{}/v1/apps/{}/machines/{}", self.base_url, app_name, machine_id);

        let response = client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await
            .context("Failed to delete Fly.io machine")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to delete Fly.io machine: {} - {}", status, error_text);
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

        let app: FlyioApp = response
            .json()
            .await
            .context("Failed to parse Fly.io app response")?;

        Ok(app)
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

        let machines: Vec<FlyioMachine> = response
            .json()
            .await
            .context("Failed to parse Fly.io machines response")?;

        Ok(machines)
    }
}
