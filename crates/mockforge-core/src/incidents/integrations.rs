//! External system integrations for incidents
//!
//! This module provides integrations with external systems like Jira, Linear, etc.

use crate::incidents::types::{DriftIncident, ExternalTicket};
use serde::{Deserialize, Serialize};

/// Configuration for external integrations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ExternalIntegrationConfig {
    /// Jira configuration
    pub jira: Option<JiraConfig>,
    /// Linear configuration
    pub linear: Option<LinearConfig>,
    /// Generic webhook configuration
    pub webhooks: Vec<WebhookConfig>,
}

/// Jira integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraConfig {
    /// Jira base URL
    pub url: String,
    /// Username or email
    pub username: String,
    /// API token
    pub api_token: String,
    /// Project key
    pub project_key: String,
    /// Issue type
    pub issue_type: String,
    /// Priority mapping (incident severity -> Jira priority)
    pub priority_mapping: std::collections::HashMap<String, String>,
}

/// Linear integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearConfig {
    /// Linear API key
    pub api_key: String,
    /// Team ID
    pub team_id: String,
    /// Priority mapping
    pub priority_mapping: std::collections::HashMap<String, String>,
}

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Webhook URL
    pub url: String,
    /// HTTP method (default: POST)
    pub method: Option<String>,
    /// Headers to include
    pub headers: std::collections::HashMap<String, String>,
    /// HMAC secret for signature verification (optional)
    pub hmac_secret: Option<String>,
    /// Events to subscribe to (empty means all events)
    pub events: Vec<String>,
    /// Whether webhook is enabled
    pub enabled: bool,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            method: Some("POST".to_string()),
            headers: std::collections::HashMap::new(),
            hmac_secret: None,
            events: Vec::new(),
            enabled: true,
        }
    }
}

/// Trait for external integrations
#[async_trait::async_trait]
pub trait ExternalIntegration: Send + Sync {
    /// Create a ticket from an incident
    async fn create_ticket(&self, incident: &DriftIncident) -> Result<ExternalTicket, String>;

    /// Update ticket status
    async fn update_ticket_status(&self, ticket_id: &str, status: &str) -> Result<(), String>;
}

/// Jira integration implementation
pub struct JiraIntegration {
    config: JiraConfig,
    client: reqwest::Client,
}

impl JiraIntegration {
    /// Create a new Jira integration
    pub fn new(config: JiraConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl ExternalIntegration for JiraIntegration {
    async fn create_ticket(&self, incident: &DriftIncident) -> Result<ExternalTicket, String> {
        // Map incident severity to Jira priority
        let priority = self
            .config
            .priority_mapping
            .get(&format!("{:?}", incident.severity))
            .cloned()
            .unwrap_or_else(|| "Medium".to_string());

        // Create Jira issue
        let issue_data = serde_json::json!({
            "fields": {
                "project": {"key": self.config.project_key},
                "summary": format!("Contract Drift: {} {} {}", incident.method, incident.endpoint, format!("{:?}", incident.incident_type)),
                "description": format!(
                    "Contract drift detected on endpoint {} {}\n\nType: {:?}\nSeverity: {:?}\n\nDetails:\n{}",
                    incident.method,
                    incident.endpoint,
                    incident.incident_type,
                    incident.severity,
                    serde_json::to_string_pretty(&incident.details).unwrap_or_default()
                ),
                "issuetype": {"name": self.config.issue_type},
                "priority": {"name": priority},
            }
        });

        let url = format!("{}/rest/api/3/issue", self.config.url);
        let response = self
            .client
            .post(&url)
            .basic_auth(&self.config.username, Some(&self.config.api_token))
            .header("Content-Type", "application/json")
            .json(&issue_data)
            .send()
            .await
            .map_err(|e| format!("Failed to create Jira ticket: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Jira API error: {}", error_text));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Jira response: {}", e))?;

        let ticket_id = result["key"]
            .as_str()
            .ok_or_else(|| "Missing ticket key in Jira response".to_string())?
            .to_string();

        let ticket_url = format!("{}/browse/{}", self.config.url, ticket_id);

        // Convert result to HashMap
        let metadata = if let serde_json::Value::Object(map) = result {
            map.into_iter().map(|(k, v)| (k, v)).collect()
        } else {
            std::collections::HashMap::new()
        };

        Ok(ExternalTicket {
            ticket_id,
            ticket_url: Some(ticket_url),
            system_type: "jira".to_string(),
            metadata,
        })
    }

    async fn update_ticket_status(&self, ticket_id: &str, status: &str) -> Result<(), String> {
        // Jira status updates require transitions
        // This is a simplified implementation
        let url = format!("{}/rest/api/3/issue/{}/transitions", self.config.url, ticket_id);
        let transition_data = serde_json::json!({
            "transition": {"name": status}
        });

        let response = self
            .client
            .post(&url)
            .basic_auth(&self.config.username, Some(&self.config.api_token))
            .header("Content-Type", "application/json")
            .json(&transition_data)
            .send()
            .await
            .map_err(|e| format!("Failed to update Jira ticket: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Jira API error: {}", error_text));
        }

        Ok(())
    }
}

/// Generic webhook integration
pub struct WebhookIntegration {
    config: WebhookConfig,
    client: reqwest::Client,
}

impl WebhookIntegration {
    /// Create a new webhook integration
    pub fn new(config: WebhookConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Send incident to webhook
    pub async fn send_incident(&self, incident: &DriftIncident) -> Result<(), String> {
        let payload = serde_json::json!({
            "event": "drift_incident",
            "incident": incident,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let mut request = self
            .client
            .request(
                reqwest::Method::from_bytes(
                    self.config.method.as_deref().unwrap_or("POST").as_bytes(),
                )
                .unwrap_or(reqwest::Method::POST),
                &self.config.url,
            )
            .json(&payload);

        // Add headers
        for (key, value) in &self.config.headers {
            request = request.header(key, value);
        }

        // Add HMAC signature if configured
        if let Some(ref secret) = self.config.hmac_secret {
            use hmac::{Hmac, Mac};
            use sha2::Sha256;
            type HmacSha256 = Hmac<Sha256>;

            let payload_str = serde_json::to_string(&payload)
                .map_err(|e| format!("Failed to serialize payload: {}", e))?;

            let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
                .map_err(|e| format!("Failed to create HMAC: {}", e))?;
            mac.update(payload_str.as_bytes());
            let signature = hex::encode(mac.finalize().into_bytes());

            request = request.header("X-Webhook-Signature", format!("sha256={}", signature));
        }

        let response =
            request.send().await.map_err(|e| format!("Failed to send webhook: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Webhook error: {}", error_text));
        }

        Ok(())
    }
}

/// Send a webhook notification
pub async fn send_webhook(
    config: &WebhookConfig,
    payload: &serde_json::Value,
) -> Result<(), String> {
    let client = reqwest::Client::new();

    let mut request = client
        .request(
            reqwest::Method::from_bytes(config.method.as_deref().unwrap_or("POST").as_bytes())
                .unwrap_or(reqwest::Method::POST),
            &config.url,
        )
        .json(payload);

    // Add headers
    for (key, value) in &config.headers {
        request = request.header(key, value);
    }

    // Add HMAC signature if configured
    if let Some(ref secret) = config.hmac_secret {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let payload_str = serde_json::to_string(payload)
            .map_err(|e| format!("Failed to serialize payload: {}", e))?;

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| format!("Failed to create HMAC: {}", e))?;
        mac.update(payload_str.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        request = request.header("X-Webhook-Signature", format!("sha256={}", signature));
    }

    let response = request.send().await.map_err(|e| format!("Failed to send webhook: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Webhook error: {}", error_text));
    }

    Ok(())
}
