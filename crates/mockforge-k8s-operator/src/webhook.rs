//! Admission webhook for validating and mutating ChaosOrchestration resources

use crate::crd::{ChaosOrchestration, ChaosOrchestrationSpec};
use cron::Schedule;
// Note: k8s-openapi doesn't include admission API types, so we define them manually
// These match the Kubernetes admission webhook API v1
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdmissionReview {
    pub request: Option<AdmissionRequest>,
    pub response: Option<AdmissionResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdmissionRequest {
    pub uid: String,
    pub operation: String,
    pub object: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdmissionResponse {
    pub uid: String,
    pub allowed: bool,
    pub status: Option<k8s_openapi::apimachinery::pkg::apis::meta::v1::Status>,
}

/// Webhook handler
pub struct WebhookHandler;

impl WebhookHandler {
    /// Create a new webhook handler
    pub fn new() -> Self {
        Self
    }

    /// Handle admission review request
    pub async fn handle_admission_review(
        &self,
        review: AdmissionReview,
    ) -> Result<AdmissionReview, String> {
        let request =
            review.request.ok_or_else(|| "Missing request in AdmissionReview".to_string())?;

        let response = match request.operation.as_str() {
            "CREATE" | "UPDATE" => self.validate_orchestration(&request).await,
            "DELETE" => self.validate_delete(&request).await,
            _ => AdmissionResponse {
                uid: request.uid.clone(),
                allowed: true,
                ..Default::default()
            },
        };

        Ok(AdmissionReview {
            request: Some(request),
            response: Some(response),
        })
    }

    /// Validate orchestration on create/update
    async fn validate_orchestration(&self, request: &AdmissionRequest) -> AdmissionResponse {
        let object = match &request.object {
            Some(obj) => obj,
            None => {
                return AdmissionResponse {
                    uid: request.uid.clone(),
                    allowed: false,
                    status: Some(k8s_openapi::apimachinery::pkg::apis::meta::v1::Status {
                        message: Some("Missing object in request".to_string()),
                        ..Default::default()
                    }),
                    ..Default::default()
                };
            }
        };

        // Parse the ChaosOrchestration
        let orchestration: ChaosOrchestration = match serde_json::from_value(object.clone()) {
            Ok(orch) => orch,
            Err(e) => {
                return AdmissionResponse {
                    uid: request.uid.clone(),
                    allowed: false,
                    status: Some(k8s_openapi::apimachinery::pkg::apis::meta::v1::Status {
                        message: Some(format!("Failed to parse ChaosOrchestration: {}", e)),
                        ..Default::default()
                    }),
                    ..Default::default()
                };
            }
        };

        // Validate the spec
        if let Err(e) = self.validate_spec(&orchestration.spec) {
            return AdmissionResponse {
                uid: request.uid.clone(),
                allowed: false,
                status: Some(k8s_openapi::apimachinery::pkg::apis::meta::v1::Status {
                    message: Some(format!("Validation failed: {}", e)),
                    ..Default::default()
                }),
                ..Default::default()
            };
        }

        info!("Validated ChaosOrchestration: {}", orchestration.spec.name);

        AdmissionResponse {
            uid: request.uid.clone(),
            allowed: true,
            ..Default::default()
        }
    }

    /// Validate delete operation
    async fn validate_delete(&self, request: &AdmissionRequest) -> AdmissionResponse {
        // Allow deletion, but could add checks for running orchestrations
        AdmissionResponse {
            uid: request.uid.clone(),
            allowed: true,
            ..Default::default()
        }
    }

    /// Validate orchestration spec
    fn validate_spec(&self, spec: &ChaosOrchestrationSpec) -> Result<(), String> {
        // Validate name
        if spec.name.is_empty() {
            return Err("Orchestration name cannot be empty".to_string());
        }

        // Validate steps
        if spec.steps.is_empty() {
            return Err("Orchestration must have at least one step".to_string());
        }

        // Validate each step
        for (idx, step) in spec.steps.iter().enumerate() {
            if step.name.is_empty() {
                return Err(format!("Step {} must have a name", idx));
            }

            if step.scenario.is_empty() {
                return Err(format!("Step {} must specify a scenario", idx));
            }
        }

        // Validate schedule if present
        if let Some(schedule) = &spec.schedule {
            if !self.is_valid_cron(schedule) {
                return Err(format!("Invalid cron schedule: {}", schedule));
            }
        }

        // Validate target services
        for service in &spec.target_services {
            if service.name.is_empty() {
                return Err("Target service name cannot be empty".to_string());
            }
        }

        Ok(())
    }

    /// Check if cron expression is valid
    ///
    /// Validates the cron expression using the `cron` crate which supports
    /// standard cron syntax with seconds (6 or 7 fields).
    fn is_valid_cron(&self, schedule: &str) -> bool {
        if schedule.is_empty() {
            return false;
        }

        // Try to parse as a cron expression
        // The cron crate expects 6 or 7 fields: sec min hour day-of-month month day-of-week [year]
        // Kubernetes uses 5-field cron (min hour day month weekday), so we prepend "0" for seconds
        let cron_expr = if schedule.split_whitespace().count() == 5 {
            format!("0 {}", schedule)
        } else {
            schedule.to_string()
        };

        Schedule::from_str(&cron_expr).is_ok()
    }

    /// Mutate orchestration (set defaults)
    pub fn mutate_orchestration(&self, spec: &mut ChaosOrchestrationSpec) {
        // Set default values if not specified
        for step in &mut spec.steps {
            if step.duration_seconds.is_none() {
                step.duration_seconds = Some(60); // Default 60 seconds
            }
        }
    }
}

impl Default for WebhookHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_name() {
        let handler = WebhookHandler::new();
        let spec = ChaosOrchestrationSpec {
            name: "".to_string(),
            description: None,
            schedule: None,
            steps: vec![],
            variables: HashMap::new(),
            hooks: vec![],
            assertions: vec![],
            enable_reporting: true,
            target_services: vec![],
        };

        assert!(handler.validate_spec(&spec).is_err());
    }

    #[test]
    fn test_validate_no_steps() {
        let handler = WebhookHandler::new();
        let spec = ChaosOrchestrationSpec {
            name: "test".to_string(),
            description: None,
            schedule: None,
            steps: vec![],
            variables: HashMap::new(),
            hooks: vec![],
            assertions: vec![],
            enable_reporting: true,
            target_services: vec![],
        };

        assert!(handler.validate_spec(&spec).is_err());
    }

    #[test]
    fn test_valid_cron_expressions() {
        let handler = WebhookHandler::new();

        // Standard 5-field K8s cron expressions
        assert!(handler.is_valid_cron("0 * * * *")); // Every hour
        assert!(handler.is_valid_cron("*/15 * * * *")); // Every 15 minutes
        assert!(handler.is_valid_cron("0 0 * * *")); // Daily at midnight
        assert!(handler.is_valid_cron("0 0 * * 0")); // Weekly on Sunday
        assert!(handler.is_valid_cron("0 0 1 * *")); // Monthly on the 1st

        // 6-field cron expressions (with seconds)
        assert!(handler.is_valid_cron("0 0 * * * *")); // Every hour at :00
        assert!(handler.is_valid_cron("30 */5 * * * *")); // Every 5 minutes at :30 sec
    }

    #[test]
    fn test_invalid_cron_expressions() {
        let handler = WebhookHandler::new();

        assert!(!handler.is_valid_cron("")); // Empty
        assert!(!handler.is_valid_cron("invalid")); // Invalid text
        assert!(!handler.is_valid_cron("60 * * * *")); // Invalid minute (>59)
        assert!(!handler.is_valid_cron("* 25 * * *")); // Invalid hour (>23)
        assert!(!handler.is_valid_cron("* * 32 * *")); // Invalid day (>31)
    }
}
