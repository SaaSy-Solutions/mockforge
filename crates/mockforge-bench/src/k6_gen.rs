//! k6 script generation for load testing real endpoints

use crate::error::{BenchError, Result};
use crate::request_gen::RequestTemplate;
use crate::scenarios::LoadScenario;
use handlebars::Handlebars;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Configuration for k6 script generation
pub struct K6Config {
    pub target_url: String,
    pub scenario: LoadScenario,
    pub duration_secs: u64,
    pub max_vus: u32,
    pub threshold_percentile: String,
    pub threshold_ms: u64,
    pub max_error_rate: f64,
    pub auth_header: Option<String>,
    pub custom_headers: HashMap<String, String>,
}

/// Generate k6 load test script
pub struct K6ScriptGenerator {
    config: K6Config,
    templates: Vec<RequestTemplate>,
}

impl K6ScriptGenerator {
    /// Create a new k6 script generator
    pub fn new(config: K6Config, templates: Vec<RequestTemplate>) -> Self {
        Self { config, templates }
    }

    /// Generate the k6 script
    pub fn generate(&self) -> Result<String> {
        let handlebars = Handlebars::new();

        let template = include_str!("templates/k6_script.hbs");

        let data = self.build_template_data()?;

        handlebars
            .render_template(template, &data)
            .map_err(|e| BenchError::ScriptGenerationFailed(e.to_string()))
    }

    /// Build the template data for rendering
    fn build_template_data(&self) -> Result<Value> {
        let stages = self
            .config
            .scenario
            .generate_stages(self.config.duration_secs, self.config.max_vus);

        let operations = self
            .templates
            .iter()
            .enumerate()
            .map(|(idx, template)| {
                json!({
                    "index": idx,
                    "name": template.operation.display_name(),
                    "method": template.operation.method.to_uppercase(),
                    "path": template.generate_path(),
                    "headers": self.build_headers(template),
                    "body": template.body.as_ref().map(|b| b.to_string()),
                    "has_body": template.body.is_some(),
                })
            })
            .collect::<Vec<_>>();

        Ok(json!({
            "base_url": self.config.target_url,
            "stages": stages.iter().map(|s| json!({
                "duration": s.duration,
                "target": s.target,
            })).collect::<Vec<_>>(),
            "operations": operations,
            "threshold_percentile": self.config.threshold_percentile,
            "threshold_ms": self.config.threshold_ms,
            "max_error_rate": self.config.max_error_rate,
            "scenario_name": format!("{:?}", self.config.scenario).to_lowercase(),
        }))
    }

    /// Build headers for a request template
    fn build_headers(&self, template: &RequestTemplate) -> Value {
        let mut headers = template.get_headers();

        // Add auth header if provided
        if let Some(auth) = &self.config.auth_header {
            headers.insert("Authorization".to_string(), auth.clone());
        }

        // Add custom headers
        for (key, value) in &self.config.custom_headers {
            headers.insert(key.clone(), value.clone());
        }

        json!(headers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec_parser::ApiOperation;
    use openapiv3::Operation;

    #[test]
    fn test_k6_config_creation() {
        let config = K6Config {
            target_url: "https://api.example.com".to_string(),
            scenario: LoadScenario::RampUp,
            duration_secs: 60,
            max_vus: 10,
            threshold_percentile: "p95".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: None,
            custom_headers: HashMap::new(),
        };

        assert_eq!(config.duration_secs, 60);
        assert_eq!(config.max_vus, 10);
    }

    #[test]
    fn test_script_generator_creation() {
        let config = K6Config {
            target_url: "https://api.example.com".to_string(),
            scenario: LoadScenario::Constant,
            duration_secs: 30,
            max_vus: 5,
            threshold_percentile: "p95".to_string(),
            threshold_ms: 500,
            max_error_rate: 0.05,
            auth_header: None,
            custom_headers: HashMap::new(),
        };

        let templates = vec![];
        let generator = K6ScriptGenerator::new(config, templates);

        assert_eq!(generator.templates.len(), 0);
    }
}
