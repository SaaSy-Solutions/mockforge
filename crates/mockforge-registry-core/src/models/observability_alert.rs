//! Persistent alert rules carried in a saved query's `filters.alert`.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertMetric {
    SampleCount,
    ErrorCount,
    ErrorRate,
    LatencyP95Ms,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservabilityAlert {
    pub metric: AlertMetric,
    pub threshold: f64,
    pub severity: String,
    pub window_minutes: i32,
    #[serde(default = "default_min_samples")]
    pub min_samples: i64,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub deployment_id: Option<Uuid>,
    #[serde(default)]
    pub service_name: Option<String>,
    #[serde(default)]
    pub path_contains: Option<String>,
    #[serde(default)]
    pub protocol: Option<String>,
}

fn default_min_samples() -> i64 {
    1
}
fn default_enabled() -> bool {
    true
}

/// Apply a validation invariant without repeating error-construction branches.
fn require(valid: bool, message: &str) -> Result<(), String> {
    valid.then_some(()).ok_or_else(|| message.to_owned())
}

impl ObservabilityAlert {
    pub fn from_filters(
        filters: &serde_json::Value,
        kind: &str,
        workspace_id: Option<Uuid>,
    ) -> Result<Option<Self>, String> {
        let Some(value) = filters.get("alert") else {
            return Ok(None);
        };
        // Null explicitly removes the rule while retaining the saved query.
        if value.is_null() {
            return Ok(None);
        }
        let rule: Self = serde_json::from_value(value.clone())
            .map_err(|e| format!("Invalid observability alert: {e}"))?;
        require(
            matches!(kind, "logs" | "traces" | "metrics"),
            "Alert kind must be logs, traces, or metrics",
        )?;
        rule.validate_bounds()?;
        rule.validate_scope(kind, workspace_id)?;
        rule.validate_text_filters()?;
        Ok(Some(rule))
    }

    /// Validate numerical bounds and routing vocabulary independently of the
    /// telemetry source. Keep the first failing field visible to API callers.
    fn validate_bounds(&self) -> Result<(), String> {
        require(
            self.threshold.is_finite() && self.threshold > 0.0,
            "Alert threshold must be finite and positive",
        )?;
        require(
            !matches!(self.metric, AlertMetric::ErrorRate) || self.threshold <= 1.0,
            "Error-rate threshold must be a fraction greater than 0 and at most 1",
        )?;
        require(
            (1..=1440).contains(&self.window_minutes) && self.min_samples >= 1,
            "Alert window must be 1..1440 minutes and min_samples must be positive",
        )?;
        require(
            crate::incident_bus::map_alert_severity(&self.severity).is_some(),
            "Alert severity must be critical, high, warning, medium, info, or low",
        )
    }

    /// Reject scopes the selected telemetry schema cannot attribute. A
    /// missing optional filter remains unrestricted within the tenant.
    fn validate_scope(&self, kind: &str, workspace_id: Option<Uuid>) -> Result<(), String> {
        require(
            workspace_id.is_none() || kind == "metrics",
            "Workspace alert scope requires metrics/captures; logs and traces have no workspace attribution",
        )?;
        if let Some(protocol) = &self.protocol {
            require(
                kind == "metrics",
                "protocol filters require metrics and a supported recorder protocol",
            )?;
            require(
                [
                    "http",
                    "grpc",
                    "graphql",
                    "websocket",
                    "kafka",
                    "mqtt",
                    "amqp",
                ]
                .contains(&protocol.as_str()),
                "protocol filters require metrics and a supported recorder protocol",
            )?;
        }
        require(
            self.service_name.is_none() || kind == "traces",
            "service_name is supported only for trace alerts",
        )?;
        require(
            self.path_contains.is_none() || kind != "traces",
            "path_contains is supported for logs and metrics alerts",
        )
    }

    fn validate_text_filters(&self) -> Result<(), String> {
        for text in [&self.service_name, &self.path_contains].into_iter().flatten() {
            require(
                !text.trim().is_empty() && text.len() <= 512,
                "Alert text filters must contain 1..512 bytes",
            )?;
        }
        Ok(())
    }

    /// No data is not recovery: preserve the incident until a sampled window
    /// proves the signal is below its threshold.
    pub fn breached(&self, samples: i64, errors: i64, latency: Option<f64>) -> Option<bool> {
        if samples < self.min_samples {
            return None;
        }
        let value = match self.metric {
            AlertMetric::SampleCount => samples as f64,
            AlertMetric::ErrorCount => errors as f64,
            AlertMetric::ErrorRate => errors as f64 / samples as f64,
            AlertMetric::LatencyP95Ms => latency?,
        };
        Some(value >= self.threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rule(metric: &str, threshold: f64) -> ObservabilityAlert {
        ObservabilityAlert::from_filters(
            &serde_json::json!({"alert":{
                "metric":metric,"threshold":threshold,"severity":"warning","window_minutes":5
            }}),
            "metrics",
            None,
        )
        .unwrap()
        .unwrap()
    }
    #[test]
    fn thresholds_and_missing_data() {
        assert_eq!(rule("error_count", 2.0).breached(3, 2, None), Some(true));
        assert_eq!(rule("error_count", 2.0).breached(3, 1, None), Some(false));
        assert_eq!(rule("error_rate", 0.5).breached(2, 1, None), Some(true));
        assert_eq!(rule("latency_p95_ms", 100.0).breached(2, 0, Some(100.0)), Some(true));
        assert_eq!(rule("latency_p95_ms", 100.0).breached(2, 0, None), None);
        assert_eq!(rule("sample_count", 1.0).breached(0, 0, None), None);
    }
    #[test]
    fn invalid_rule_and_scope_are_rejected() {
        let valid = serde_json::json!({"alert":{"metric":"error_count","threshold":1,"severity":"critical","window_minutes":5}});
        for (field, value) in [
            ("threshold", serde_json::json!(0)),
            ("window_minutes", serde_json::json!(1441)),
            ("severity", serde_json::json!("bogus")),
            ("min_samples", serde_json::json!(0)),
            ("unexpected", serde_json::json!(true)),
        ] {
            let mut filters = valid.clone();
            filters["alert"][field] = value;
            assert!(
                ObservabilityAlert::from_filters(&filters, "metrics", None).is_err(),
                "{field}"
            );
        }
        assert!(ObservabilityAlert::from_filters(&valid, "traces", Some(Uuid::nil())).is_err());
        let mut filters = valid;
        filters["alert"]["service_name"] = serde_json::json!("api");
        assert!(ObservabilityAlert::from_filters(&filters, "logs", None).is_err());
    }
    #[test]
    fn legacy_and_removed_rules_are_inert() {
        for filters in [
            serde_json::json!({"kind":"request_count"}),
            serde_json::json!({"alert":null}),
        ] {
            assert!(ObservabilityAlert::from_filters(&filters, "metrics", None).unwrap().is_none());
        }
    }
}
