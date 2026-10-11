use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    /// Optional Slack notification configuration.
    pub slack: Option<SlackConfig>,
    /// Optional Microsoft Teams notification configuration.
    pub teams: Option<TeamsConfig>,
    /// Optional Jira issue creation configuration.
    pub jira: Option<JiraConfig>,
    /// Optional `PagerDuty` incident configuration.
    pub pagerduty: Option<PagerDutyConfig>,
    /// Optional Grafana annotation and dashboard configuration.
    pub grafana: Option<GrafanaConfig>,
}

/// Slack integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    /// Webhook endpoint receiving notification payloads.
    pub webhook_url: String,
    /// Slack channel included in the notification payload.
    pub channel: String,
    /// Optional Slack display name; defaults to `MockForge`.
    pub username: Option<String>,
    /// Optional Slack icon emoji; defaults to the robot face.
    pub icon_emoji: Option<String>,
    /// User identifiers or names appended as mentions in the notification.
    pub mention_users: Vec<String>,
}

/// Microsoft Teams integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsConfig {
    /// Webhook endpoint receiving notification payloads.
    pub webhook_url: String,
    /// User identifiers or names appended as mentions in the notification.
    pub mention_users: Vec<String>,
}

/// Jira integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraConfig {
    /// Base URL of the service API.
    pub url: String,
    /// Jira login name used with HTTP basic authentication.
    pub username: String,
    /// Jira API token used with HTTP basic authentication.
    pub api_token: String,
    /// Jira project receiving newly created issues.
    pub project_key: String,
    /// Jira issue type name for new issues.
    pub issue_type: String,
    /// Optional Jira priority override; otherwise derived from notification severity.
    pub priority: Option<String>,
    /// Optional Jira assignee login name.
    pub assignee: Option<String>,
}

/// `PagerDuty` integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagerDutyConfig {
    /// `PagerDuty` Events API integration routing key.
    pub routing_key: String,
    /// Optional `PagerDuty` severity override; otherwise derived from notification severity.
    pub severity: Option<String>,
    /// Prefix for incident deduplication keys; defaults to mockforge.
    pub dedup_key_prefix: Option<String>,
}

/// Grafana integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrafanaConfig {
    /// Base URL of the service API.
    pub url: String,
    /// Grafana API credential used for bearer authentication.
    pub api_key: String,
    /// Optional dashboard identifier attached to annotations.
    pub dashboard_uid: Option<String>,
    /// Optional Grafana folder identifier for newly created dashboards.
    pub folder_uid: Option<String>,
}

/// Notification severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationSeverity {
    /// Informational event.
    Info,
    /// Potential degradation requiring attention.
    Warning,
    /// Failure requiring investigation.
    Error,
    /// Severe failure requiring urgent attention.
    Critical,
}

/// Notification message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// Human-readable notification summary.
    pub title: String,
    /// Notification details displayed in the destination service.
    pub message: String,
    /// Severity used to choose formatting and incident routing.
    pub severity: NotificationSeverity,
    /// UTC time associated with the notification.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Additional attributes included in service-specific notification payloads.
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Slack notifier
pub struct SlackNotifier {
    config: SlackConfig,
    client: reqwest::Client,
}

impl SlackNotifier {
    /// Create the integration with the supplied configuration and a new HTTP client.
    pub fn new(config: SlackConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Post the notification to the configured webhook.
    pub async fn send(&self, notification: &Notification) -> Result<()> {
        let color = match notification.severity {
            NotificationSeverity::Info => "#36a64f",
            NotificationSeverity::Warning => "#ff9900",
            NotificationSeverity::Error => "#ff0000",
            NotificationSeverity::Critical => "#8b0000",
        };

        let mentions = if !self.config.mention_users.is_empty() {
            format!(
                "\n{}",
                self.config
                    .mention_users
                    .iter()
                    .map(|u| format!("<@{}>", u))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        } else {
            String::new()
        };

        let payload = serde_json::json!({
            "channel": self.config.channel,
            "username": self.config.username.as_deref().unwrap_or("MockForge"),
            "icon_emoji": self.config.icon_emoji.as_deref().unwrap_or(":robot_face:"),
            "attachments": [{
                "color": color,
                "title": notification.title,
                "text": format!("{}{}", notification.message, mentions),
                "timestamp": notification.timestamp.timestamp(),
                "fields": notification.metadata.iter().map(|(k, v)| {
                    serde_json::json!({
                        "title": k,
                        "value": v.to_string(),
                        "short": true
                    })
                }).collect::<Vec<_>>()
            }]
        });

        self.client
            .post(&self.config.webhook_url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send Slack notification")?;

        Ok(())
    }
}

/// Microsoft Teams notifier
pub struct TeamsNotifier {
    config: TeamsConfig,
    client: reqwest::Client,
}

impl TeamsNotifier {
    /// Create the integration with the supplied configuration and a new HTTP client.
    pub fn new(config: TeamsConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Post the notification to the configured webhook.
    pub async fn send(&self, notification: &Notification) -> Result<()> {
        let theme_color = match notification.severity {
            NotificationSeverity::Info => "0078D4",
            NotificationSeverity::Warning => "FFA500",
            NotificationSeverity::Error => "FF0000",
            NotificationSeverity::Critical => "8B0000",
        };

        let mentions = if !self.config.mention_users.is_empty() {
            format!(
                "\n\n{}",
                self.config
                    .mention_users
                    .iter()
                    .map(|u| format!("<at>{}</at>", u))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        } else {
            String::new()
        };

        let facts: Vec<_> = notification
            .metadata
            .iter()
            .map(|(k, v)| {
                serde_json::json!({
                    "name": k,
                    "value": v.to_string()
                })
            })
            .collect();

        let payload = serde_json::json!({
            "@type": "MessageCard",
            "@context": "https://schema.org/extensions",
            "themeColor": theme_color,
            "summary": notification.title,
            "sections": [{
                "activityTitle": notification.title,
                "activitySubtitle": format!("Severity: {:?}", notification.severity),
                "text": format!("{}{}", notification.message, mentions),
                "facts": facts
            }]
        });

        self.client
            .post(&self.config.webhook_url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send Teams notification")?;

        Ok(())
    }
}

/// Jira ticket creator
pub struct JiraIntegration {
    config: JiraConfig,
    client: reqwest::Client,
}

impl JiraIntegration {
    /// Create the integration with the supplied configuration and a new HTTP client.
    pub fn new(config: JiraConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Create a Jira issue for the notification and return its issue key.
    pub async fn create_ticket(&self, notification: &Notification) -> Result<String> {
        let description = format!(
            "{}\n\n*Metadata:*\n{}",
            notification.message,
            notification
                .metadata
                .iter()
                .map(|(k, v)| format!("* {}: {}", k, v))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let priority = self.config.priority.as_deref().or({
            Some(match notification.severity {
                NotificationSeverity::Critical => "Highest",
                NotificationSeverity::Error => "High",
                NotificationSeverity::Warning => "Medium",
                NotificationSeverity::Info => "Low",
            })
        });

        let mut fields = serde_json::json!({
            "project": {
                "key": self.config.project_key
            },
            "summary": notification.title,
            "description": description,
            "issuetype": {
                "name": self.config.issue_type
            }
        });

        if let Some(priority) = priority {
            fields["priority"] = serde_json::json!({ "name": priority });
        }

        if let Some(assignee) = &self.config.assignee {
            fields["assignee"] = serde_json::json!({ "name": assignee });
        }

        let payload = serde_json::json!({ "fields": fields });

        let url = format!("{}/rest/api/2/issue", self.config.url);

        let response = self
            .client
            .post(&url)
            .basic_auth(&self.config.username, Some(&self.config.api_token))
            .json(&payload)
            .send()
            .await
            .context("Failed to create Jira ticket")?;

        let result: serde_json::Value = response.json().await?;
        let ticket_key = result["key"]
            .as_str()
            .context("Failed to get ticket key from response")?
            .to_string();

        Ok(ticket_key)
    }

    /// Append a comment to the specified Jira issue.
    pub async fn update_ticket(&self, ticket_key: &str, comment: &str) -> Result<()> {
        let payload = serde_json::json!({
            "body": comment
        });

        let url = format!("{}/rest/api/2/issue/{}/comment", self.config.url, ticket_key);

        self.client
            .post(&url)
            .basic_auth(&self.config.username, Some(&self.config.api_token))
            .json(&payload)
            .send()
            .await
            .context("Failed to add comment to Jira ticket")?;

        Ok(())
    }
}

/// `PagerDuty` integration
pub struct PagerDutyIntegration {
    config: PagerDutyConfig,
    client: reqwest::Client,
}

impl PagerDutyIntegration {
    /// Create the integration with the supplied configuration and a new HTTP client.
    pub fn new(config: PagerDutyConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Send a `PagerDuty` trigger event and return its deduplication key.
    pub async fn trigger_incident(&self, notification: &Notification) -> Result<String> {
        let severity = self.config.severity.as_deref().or({
            Some(match notification.severity {
                NotificationSeverity::Critical => "critical",
                NotificationSeverity::Error => "error",
                NotificationSeverity::Warning => "warning",
                NotificationSeverity::Info => "info",
            })
        });

        let dedup_key = format!(
            "{}-{}",
            self.config.dedup_key_prefix.as_deref().unwrap_or("mockforge"),
            notification.timestamp.timestamp()
        );

        let payload = serde_json::json!({
            "routing_key": self.config.routing_key,
            "event_action": "trigger",
            "dedup_key": dedup_key,
            "payload": {
                "summary": notification.title,
                "severity": severity,
                "source": "MockForge",
                "timestamp": notification.timestamp.to_rfc3339(),
                "custom_details": notification.metadata
            }
        });

        let response = self
            .client
            .post("https://events.pagerduty.com/v2/enqueue")
            .json(&payload)
            .send()
            .await
            .context("Failed to trigger PagerDuty incident")?;

        let _result: serde_json::Value = response.json().await?;
        Ok(dedup_key)
    }

    /// Send a `PagerDuty` resolve event for the supplied deduplication key.
    pub async fn resolve_incident(&self, dedup_key: &str) -> Result<()> {
        let payload = serde_json::json!({
            "routing_key": self.config.routing_key,
            "event_action": "resolve",
            "dedup_key": dedup_key
        });

        self.client
            .post("https://events.pagerduty.com/v2/enqueue")
            .json(&payload)
            .send()
            .await
            .context("Failed to resolve PagerDuty incident")?;

        Ok(())
    }
}

/// Grafana integration
pub struct GrafanaIntegration {
    config: GrafanaConfig,
    client: reqwest::Client,
}

impl GrafanaIntegration {
    /// Create the integration with the supplied configuration and a new HTTP client.
    pub fn new(config: GrafanaConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Post a Grafana annotation with the notification time, details, and severity.
    pub async fn create_annotation(&self, notification: &Notification) -> Result<()> {
        let tags = vec![
            format!("severity:{:?}", notification.severity).to_lowercase(),
            "mockforge".to_string(),
        ];

        let payload = serde_json::json!({
            "text": notification.message,
            "tags": tags,
            "time": notification.timestamp.timestamp_millis(),
            "dashboardUID": self.config.dashboard_uid
        });

        let url = format!("{}/api/annotations", self.config.url);

        self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&payload)
            .send()
            .await
            .context("Failed to create Grafana annotation")?;

        Ok(())
    }

    /// Create a Grafana dashboard and return its identifier.
    pub async fn create_dashboard(&self, dashboard_json: serde_json::Value) -> Result<String> {
        let payload = serde_json::json!({
            "dashboard": dashboard_json,
            "folderUid": self.config.folder_uid,
            "overwrite": false
        });

        let url = format!("{}/api/dashboards/db", self.config.url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&payload)
            .send()
            .await
            .context("Failed to create Grafana dashboard")?;

        let result: serde_json::Value = response.json().await?;
        let uid = result["uid"].as_str().context("Failed to get dashboard UID")?.to_string();

        Ok(uid)
    }
}

/// Integration manager
pub struct IntegrationManager {
    slack: Option<SlackNotifier>,
    teams: Option<TeamsNotifier>,
    jira: Option<JiraIntegration>,
    pagerduty: Option<PagerDutyIntegration>,
    grafana: Option<GrafanaIntegration>,
}

impl IntegrationManager {
    /// Construct all integrations enabled by the supplied configuration.
    pub fn new(config: IntegrationConfig) -> Self {
        Self {
            slack: config.slack.map(SlackNotifier::new),
            teams: config.teams.map(TeamsNotifier::new),
            jira: config.jira.map(JiraIntegration::new),
            pagerduty: config.pagerduty.map(PagerDutyIntegration::new),
            grafana: config.grafana.map(GrafanaIntegration::new),
        }
    }

    /// Send notification to all configured channels
    pub async fn notify(&self, notification: &Notification) -> Result<NotificationResults> {
        let mut results = NotificationResults::default();

        // Send to Slack
        if let Some(slack) = &self.slack {
            match slack.send(notification).await {
                Ok(_) => results.slack_sent = true,
                Err(e) => results.errors.push(format!("Slack: {}", e)),
            }
        }

        // Send to Teams
        if let Some(teams) = &self.teams {
            match teams.send(notification).await {
                Ok(_) => results.teams_sent = true,
                Err(e) => results.errors.push(format!("Teams: {}", e)),
            }
        }

        // Create Jira ticket for errors and critical
        if let Some(jira) = &self.jira {
            if matches!(
                notification.severity,
                NotificationSeverity::Error | NotificationSeverity::Critical
            ) {
                match jira.create_ticket(notification).await {
                    Ok(key) => {
                        results.jira_ticket = Some(key);
                    }
                    Err(e) => results.errors.push(format!("Jira: {}", e)),
                }
            }
        }

        // Trigger PagerDuty incident for critical
        if let Some(pd) = &self.pagerduty {
            if notification.severity == NotificationSeverity::Critical {
                match pd.trigger_incident(notification).await {
                    Ok(key) => {
                        results.pagerduty_incident = Some(key);
                    }
                    Err(e) => results.errors.push(format!("PagerDuty: {}", e)),
                }
            }
        }

        // Create Grafana annotation
        if let Some(grafana) = &self.grafana {
            match grafana.create_annotation(notification).await {
                Ok(_) => results.grafana_annotated = true,
                Err(e) => results.errors.push(format!("Grafana: {}", e)),
            }
        }

        Ok(results)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
/// Outcomes and errors collected while notifying configured integrations.
pub struct NotificationResults {
    /// Whether the Slack request completed without a reported transport error.
    pub slack_sent: bool,
    /// Whether the Teams request completed without a reported transport error.
    pub teams_sent: bool,
    /// Issue key returned by Jira, when issue creation succeeded.
    pub jira_ticket: Option<String>,
    /// Incident deduplication key returned by the `PagerDuty` integration, when available.
    pub pagerduty_incident: Option<String>,
    /// Whether the Grafana annotation request completed without a reported transport error.
    pub grafana_annotated: bool,
    /// Integration errors collected while notifying the configured channels.
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_creation() {
        let notification = Notification {
            title: "Test Alert".to_string(),
            message: "This is a test".to_string(),
            severity: NotificationSeverity::Warning,
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        assert_eq!(notification.severity, NotificationSeverity::Warning);
    }
}
