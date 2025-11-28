//! AI-driven validation error generation
//!
//! This module generates realistic, context-aware validation error messages
//! using LLMs, learning from example error responses to create human-like
//! error messages.

use super::config::BehaviorModelConfig;
use super::llm_client::LlmClient;
use super::mutation_analyzer::{ValidationIssue, ValidationIssueType, ValidationSeverity};
use super::types::LlmGenerationRequest;
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Example error response for learning validation error formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationErrorExample {
    /// Field that caused the error (if applicable)
    pub field: Option<String>,
    /// Error type
    pub error_type: String,
    /// Error message
    pub message: String,
    /// Error response body
    pub response: Value,
    /// HTTP status code
    pub status_code: u16,
}

/// Request context for error generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Request body
    pub request_body: Option<Value>,
    /// Query parameters
    #[serde(default)]
    pub query_params: HashMap<String, String>,
    /// Headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

/// Validation error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationErrorResponse {
    /// HTTP status code
    pub status_code: u16,
    /// Error response body
    pub body: Value,
    /// Error format (field-level, object-level, custom)
    pub format: ErrorFormat,
}

/// Error response format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ErrorFormat {
    /// Field-level errors (each field has its own error)
    FieldLevel,
    /// Object-level error (single error message)
    ObjectLevel,
    /// Custom format
    Custom,
}

/// Field-level error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldError {
    /// Field name
    pub field: String,
    /// Error message
    pub message: String,
    /// Error code (optional)
    pub code: Option<String>,
    /// Rejected value (optional)
    pub rejected_value: Option<Value>,
}

/// Validation error generator
pub struct ValidationGenerator {
    /// LLM client for generating error messages
    llm_client: Option<LlmClient>,
    /// Configuration
    config: BehaviorModelConfig,
    /// Learned error examples
    error_examples: Vec<ValidationErrorExample>,
}

impl ValidationGenerator {
    /// Create a new validation generator
    pub fn new(config: BehaviorModelConfig) -> Self {
        let llm_client = if config.llm_provider != "disabled" {
            Some(LlmClient::new(config.clone()))
        } else {
            None
        };

        Self {
            llm_client,
            config,
            error_examples: Vec::new(),
        }
    }

    /// Learn from an error example
    pub fn learn_from_example(&mut self, example: ValidationErrorExample) {
        self.error_examples.push(example);
    }

    /// Generate validation error response
    ///
    /// Creates a realistic, context-aware validation error based on the
    /// validation issue and request context.
    pub async fn generate_validation_error(
        &self,
        issue: &ValidationIssue,
        context: &RequestContext,
    ) -> Result<ValidationErrorResponse> {
        // Determine error format based on issue
        let format = self.determine_error_format(issue);

        // Generate error message
        let error_message = self.format_error_message(issue, context).await?;

        // Build error response body
        let body = match format {
            ErrorFormat::FieldLevel => {
                self.build_field_level_error(issue, &error_message, context).await?
            }
            ErrorFormat::ObjectLevel => {
                self.build_object_level_error(issue, &error_message, context).await?
            }
            ErrorFormat::Custom => self.build_custom_error(issue, &error_message, context).await?,
        };

        Ok(ValidationErrorResponse {
            status_code: self.determine_status_code(issue),
            body,
            format,
        })
    }

    /// Generate field-level error
    pub async fn generate_field_error(
        &self,
        field: &str,
        issue: &ValidationIssue,
        context: &RequestContext,
    ) -> Result<FieldError> {
        let message = self.format_error_message(issue, context).await?;

        // Extract rejected value from request if available
        let rejected_value =
            context.request_body.as_ref().and_then(|body| body.get(field)).cloned();

        Ok(FieldError {
            field: field.to_string(),
            message,
            code: Some(self.generate_error_code(issue)),
            rejected_value,
        })
    }

    /// Format error message using LLM or templates
    async fn format_error_message(
        &self,
        issue: &ValidationIssue,
        context: &RequestContext,
    ) -> Result<String> {
        // First, try to find similar examples
        if let Some(similar_example) = self.find_similar_example(issue, &self.error_examples) {
            // Use similar example's message as template
            return Ok(similar_example.message.clone());
        }

        // If LLM is available, generate message
        if let Some(ref llm_client) = self.llm_client {
            return self.generate_message_with_llm(issue).await;
        }

        // Fallback to template-based message
        Ok(self.generate_template_message(issue))
    }

    // ===== Private helper methods =====

    /// Determine error format based on issue
    fn determine_error_format(&self, issue: &ValidationIssue) -> ErrorFormat {
        // If field is specified, use field-level format
        if issue.field.is_some() {
            return ErrorFormat::FieldLevel;
        }

        // Otherwise, use object-level
        ErrorFormat::ObjectLevel
    }

    /// Build field-level error response
    async fn build_field_level_error(
        &self,
        issue: &ValidationIssue,
        message: &str,
        _context: &RequestContext,
    ) -> Result<Value> {
        let field = issue.field.as_deref().unwrap_or("unknown");

        // Standard field-level error format
        Ok(serde_json::json!({
            "error": {
                "type": "validation_error",
                "message": "Validation failed",
                "fields": {
                    field: {
                        "message": message,
                        "code": self.generate_error_code(issue),
                        "type": format!("{:?}", issue.issue_type).to_lowercase()
                    }
                }
            }
        }))
    }

    /// Build object-level error response
    async fn build_object_level_error(
        &self,
        issue: &ValidationIssue,
        message: &str,
        _context: &RequestContext,
    ) -> Result<Value> {
        Ok(serde_json::json!({
            "error": {
                "type": "validation_error",
                "message": message,
                "code": self.generate_error_code(issue)
            }
        }))
    }

    /// Build custom error response
    async fn build_custom_error(
        &self,
        issue: &ValidationIssue,
        message: &str,
        context: &RequestContext,
    ) -> Result<Value> {
        // Use LLM to generate custom format if available
        if let Some(ref llm_client) = self.llm_client {
            return self.generate_custom_format_with_llm(issue, message, context).await;
        }

        // Fallback to object-level
        self.build_object_level_error(issue, message, context).await
    }

    /// Determine HTTP status code from issue
    fn determine_status_code(&self, issue: &ValidationIssue) -> u16 {
        match issue.severity {
            ValidationSeverity::Critical | ValidationSeverity::Error => 400,
            ValidationSeverity::Warning => 422,
            ValidationSeverity::Info => 200, // Info doesn't block
        }
    }

    /// Generate error code from issue type
    fn generate_error_code(&self, issue: &ValidationIssue) -> String {
        match issue.issue_type {
            ValidationIssueType::Required => "REQUIRED_FIELD".to_string(),
            ValidationIssueType::Format => "INVALID_FORMAT".to_string(),
            ValidationIssueType::MinLength => "MIN_LENGTH".to_string(),
            ValidationIssueType::MaxLength => "MAX_LENGTH".to_string(),
            ValidationIssueType::Pattern => "INVALID_PATTERN".to_string(),
            ValidationIssueType::Range => "OUT_OF_RANGE".to_string(),
            ValidationIssueType::Type => "INVALID_TYPE".to_string(),
            ValidationIssueType::Custom => "VALIDATION_ERROR".to_string(),
        }
    }

    /// Find similar error example
    fn find_similar_example<'a>(
        &self,
        issue: &ValidationIssue,
        examples: &'a [ValidationErrorExample],
    ) -> Option<&'a ValidationErrorExample> {
        examples.iter().find(|ex| {
            // Match by field if available
            if let Some(ref field) = issue.field {
                if let Some(ref ex_field) = ex.field {
                    if field == ex_field {
                        return true;
                    }
                }
            }

            // Match by error type
            ex.error_type == format!("{:?}", issue.issue_type)
        })
    }

    /// Generate error message using LLM
    async fn generate_message_with_llm(&self, issue: &ValidationIssue) -> Result<String> {
        let llm_client = self
            .llm_client
            .as_ref()
            .ok_or_else(|| crate::Error::generic("LLM client not available"))?;

        let issue_type_str = format!("{:?}", issue.issue_type);
        let field_str =
            issue.field.as_ref().map(|f| format!(" for field '{}'", f)).unwrap_or_default();

        let system_prompt = "You are an API error message generator. Generate clear, helpful validation error messages.";
        let user_prompt = format!(
            "Generate a validation error message{} for a {} error. \
             The error should be clear, helpful, and professional. \
             Return only the error message text, no additional formatting.",
            field_str, issue_type_str
        );

        let request = LlmGenerationRequest {
            system_prompt: system_prompt.to_string(),
            user_prompt,
            temperature: 0.3, // Lower temperature for consistent error messages
            max_tokens: 100,
            schema: None,
        };

        let response = llm_client.generate(&request).await?;

        // Extract message from response
        if let Some(text) = response.as_str() {
            Ok(text.trim().to_string())
        } else if let Some(message) = response.get("message").and_then(|m| m.as_str()) {
            Ok(message.to_string())
        } else {
            Ok(self.generate_template_message(issue))
        }
    }

    /// Generate template-based error message
    fn generate_template_message(&self, issue: &ValidationIssue) -> String {
        let field_str = issue.field.as_ref().map(|f| format!("Field '{}' ", f)).unwrap_or_default();

        match issue.issue_type {
            ValidationIssueType::Required => {
                format!("{}is required", field_str)
            }
            ValidationIssueType::Format => {
                format!("{}has an invalid format", field_str)
            }
            ValidationIssueType::MinLength => {
                format!("{}is too short", field_str)
            }
            ValidationIssueType::MaxLength => {
                format!("{}is too long", field_str)
            }
            ValidationIssueType::Pattern => {
                format!("{}does not match the required pattern", field_str)
            }
            ValidationIssueType::Range => {
                format!("{}is out of range", field_str)
            }
            ValidationIssueType::Type => {
                format!("{}has an invalid type", field_str)
            }
            ValidationIssueType::Custom => issue.error_message.clone(),
        }
    }

    /// Generate custom error format using LLM
    async fn generate_custom_format_with_llm(
        &self,
        issue: &ValidationIssue,
        message: &str,
        context: &RequestContext,
    ) -> Result<Value> {
        let llm_client = self
            .llm_client
            .as_ref()
            .ok_or_else(|| crate::Error::generic("LLM client not available"))?;

        let system_prompt = "You are an API error response generator. Generate realistic error responses in JSON format.";
        let user_prompt = format!(
            "Generate a validation error response for:\n\
             Method: {}\n\
             Path: {}\n\
             Error: {}\n\
             Message: {}\n\n\
             Return a JSON object with error details. Use a realistic API error format.",
            context.method,
            context.path,
            format!("{:?}", issue.issue_type),
            message
        );

        let request = LlmGenerationRequest {
            system_prompt: system_prompt.to_string(),
            user_prompt,
            temperature: 0.5,
            max_tokens: 300,
            schema: None,
        };

        let response = llm_client.generate(&request).await?;

        // Try to parse as JSON, fallback to wrapping in error object
        if let Some(obj) = response.as_object() {
            Ok(Value::Object(obj.clone()))
        } else {
            Ok(serde_json::json!({
                "error": {
                    "message": message,
                    "type": format!("{:?}", issue.issue_type)
                }
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_generate_template_message() {
        let config = BehaviorModelConfig::default();
        let generator = ValidationGenerator::new(config);

        let issue = ValidationIssue {
            field: Some("email".to_string()),
            issue_type: ValidationIssueType::Required,
            severity: ValidationSeverity::Error,
            context: json!({}),
            error_message: "".to_string(),
        };

        let message = generator.generate_template_message(&issue);
        assert!(message.contains("email"));
        assert!(message.contains("required"));
    }

    #[tokio::test]
    async fn test_determine_error_format() {
        let config = BehaviorModelConfig::default();
        let generator = ValidationGenerator::new(config);

        let field_issue = ValidationIssue {
            field: Some("email".to_string()),
            issue_type: ValidationIssueType::Required,
            severity: ValidationSeverity::Error,
            context: json!({}),
            error_message: "".to_string(),
        };

        assert_eq!(generator.determine_error_format(&field_issue), ErrorFormat::FieldLevel);

        let object_issue = ValidationIssue {
            field: None,
            issue_type: ValidationIssueType::Required,
            severity: ValidationSeverity::Error,
            context: json!({}),
            error_message: "".to_string(),
        };

        assert_eq!(generator.determine_error_format(&object_issue), ErrorFormat::ObjectLevel);
    }

    #[tokio::test]
    async fn test_generate_error_code() {
        let config = BehaviorModelConfig::default();
        let generator = ValidationGenerator::new(config);

        let issue = ValidationIssue {
            field: Some("email".to_string()),
            issue_type: ValidationIssueType::Format,
            severity: ValidationSeverity::Error,
            context: json!({}),
            error_message: "".to_string(),
        };

        let code = generator.generate_error_code(&issue);
        assert_eq!(code, "INVALID_FORMAT");
    }
}
