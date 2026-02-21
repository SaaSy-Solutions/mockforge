//! Time-triggered data mutation rules
//!
//! This module provides a system for automatically mutating VBR entity data
//! based on time triggers. It supports both aging-style rules (expiration-based)
//! and arbitrary field mutations (time-triggered changes).
//!
//! ## Usage
//!
//! ```rust,ignore
//! use mockforge_vbr::mutation_rules::{MutationRule, MutationRuleManager, MutationTrigger, MutationOperation};
//!
//! let manager = MutationRuleManager::new();
//!
//! // Create a rule that increments a counter every hour
//! let rule = MutationRule::new(
//!     "hourly-counter".to_string(),
//!     "User".to_string(),
//!     MutationTrigger::Interval {
//!         duration_seconds: 3600,
//!     },
//!     MutationOperation::Increment {
//!         field: "login_count".to_string(),
//!         amount: 1.0,
//!     },
//! );
//!
//! // Add the rule (async operation)
//! // manager.add_rule(rule).await;
//! ```

use crate::{Error, Result};
use chrono::{DateTime, Duration, Utc};
use mockforge_core::time_travel_now;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Trigger condition for a mutation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MutationTrigger {
    /// Trigger after a duration has elapsed
    Interval {
        /// Duration in seconds
        duration_seconds: u64,
    },
    /// Trigger at a specific time (cron-like, but simpler)
    AtTime {
        /// Hour (0-23)
        hour: u8,
        /// Minute (0-59)
        minute: u8,
    },
    /// Trigger when a field value reaches a threshold
    FieldThreshold {
        /// Field to check
        field: String,
        /// Threshold value
        threshold: Value,
        /// Comparison operator
        operator: ComparisonOperator,
    },
}

/// Comparison operator for field threshold triggers
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ComparisonOperator {
    /// Greater than
    Gt,
    /// Less than
    Lt,
    /// Equal to
    Eq,
    /// Greater than or equal
    Gte,
    /// Less than or equal
    Lte,
}

/// Mutation operation to perform
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MutationOperation {
    /// Set a field to a specific value
    Set {
        /// Field name
        field: String,
        /// Value to set
        value: Value,
    },
    /// Increment a numeric field
    Increment {
        /// Field name
        field: String,
        /// Amount to increment by
        amount: f64,
    },
    /// Decrement a numeric field
    Decrement {
        /// Field name
        field: String,
        /// Amount to decrement by
        amount: f64,
    },
    /// Transform a field using a template or expression
    Transform {
        /// Field name
        field: String,
        /// Transformation expression (e.g., "{{field}} * 2")
        expression: String,
    },
    /// Update status field
    UpdateStatus {
        /// New status value
        status: String,
    },
}

/// A mutation rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationRule {
    /// Unique identifier for this rule
    pub id: String,
    /// Entity name to apply mutation to
    pub entity_name: String,
    /// Trigger condition
    pub trigger: MutationTrigger,
    /// Mutation operation
    pub operation: MutationOperation,
    /// Whether this rule is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
    /// Optional condition (JSONPath expression) that must be true
    #[serde(default)]
    pub condition: Option<String>,
    /// Last execution time
    #[serde(default)]
    pub last_execution: Option<DateTime<Utc>>,
    /// Next scheduled execution time
    #[serde(default)]
    pub next_execution: Option<DateTime<Utc>>,
    /// Number of times this rule has executed
    #[serde(default)]
    pub execution_count: usize,
}

fn default_true() -> bool {
    true
}

impl MutationRule {
    /// Create a new mutation rule
    pub fn new(
        id: String,
        entity_name: String,
        trigger: MutationTrigger,
        operation: MutationOperation,
    ) -> Self {
        Self {
            id,
            entity_name,
            trigger,
            operation,
            enabled: true,
            description: None,
            condition: None,
            last_execution: None,
            next_execution: None,
            execution_count: 0,
        }
    }

    /// Calculate the next execution time based on the trigger
    pub fn calculate_next_execution(&self, from: DateTime<Utc>) -> Option<DateTime<Utc>> {
        if !self.enabled {
            return None;
        }

        match &self.trigger {
            MutationTrigger::Interval { duration_seconds } => {
                Some(from + Duration::seconds(*duration_seconds as i64))
            }
            MutationTrigger::AtTime { hour, minute } => {
                // Calculate next occurrence of this time
                let mut next =
                    from.date_naive().and_hms_opt(*hour as u32, *minute as u32, 0)?.and_utc();

                // If the time has already passed today, move to tomorrow
                if next <= from {
                    next += Duration::days(1);
                }

                Some(next)
            }
            MutationTrigger::FieldThreshold { .. } => {
                // Field threshold triggers are evaluated on-demand, not scheduled
                None
            }
        }
    }
}

/// Manager for mutation rules
pub struct MutationRuleManager {
    /// Registered mutation rules
    rules: Arc<RwLock<HashMap<String, MutationRule>>>,
}

impl MutationRuleManager {
    /// Create a new mutation rule manager
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a mutation rule
    pub async fn add_rule(&self, mut rule: MutationRule) -> Result<()> {
        // Calculate next execution time
        let now = time_travel_now();
        rule.next_execution = rule.calculate_next_execution(now);

        let rule_id = rule.id.clone();

        let mut rules = self.rules.write().await;
        rules.insert(rule_id.clone(), rule);

        info!("Added mutation rule '{}' for entity '{}'", rule_id, rules[&rule_id].entity_name);
        Ok(())
    }

    /// Remove a mutation rule
    pub async fn remove_rule(&self, rule_id: &str) -> bool {
        let mut rules = self.rules.write().await;
        let removed = rules.remove(rule_id).is_some();

        if removed {
            info!("Removed mutation rule '{}'", rule_id);
        }

        removed
    }

    /// Get a mutation rule by ID
    pub async fn get_rule(&self, rule_id: &str) -> Option<MutationRule> {
        let rules = self.rules.read().await;
        rules.get(rule_id).cloned()
    }

    /// List all mutation rules
    pub async fn list_rules(&self) -> Vec<MutationRule> {
        let rules = self.rules.read().await;
        rules.values().cloned().collect()
    }

    /// List rules for a specific entity
    pub async fn list_rules_for_entity(&self, entity_name: &str) -> Vec<MutationRule> {
        let rules = self.rules.read().await;
        rules.values().filter(|rule| rule.entity_name == entity_name).cloned().collect()
    }

    /// Enable or disable a mutation rule
    pub async fn set_rule_enabled(&self, rule_id: &str, enabled: bool) -> Result<()> {
        let mut rules = self.rules.write().await;

        if let Some(rule) = rules.get_mut(rule_id) {
            rule.enabled = enabled;

            // Recalculate next execution if enabling
            if enabled {
                let now = time_travel_now();
                rule.next_execution = rule.calculate_next_execution(now);
            } else {
                rule.next_execution = None;
            }

            info!("Mutation rule '{}' {}", rule_id, if enabled { "enabled" } else { "disabled" });
            Ok(())
        } else {
            Err(Error::generic(format!("Mutation rule '{}' not found", rule_id)))
        }
    }

    /// Check for rules that should execute now and execute them
    ///
    /// This should be called periodically or when time advances
    /// to check if any rules are due for execution.
    pub async fn check_and_execute(
        &self,
        database: &dyn crate::database::VirtualDatabase,
        registry: &crate::entities::EntityRegistry,
    ) -> Result<usize> {
        let now = time_travel_now();
        let mut executed = 0;

        // Get rules that need to execute
        let mut rules_to_execute = Vec::new();

        {
            let rules = self.rules.read().await;
            for rule in rules.values() {
                if !rule.enabled {
                    continue;
                }

                if let Some(next) = rule.next_execution {
                    if now >= next {
                        rules_to_execute.push(rule.id.clone());
                    }
                }
            }
        }

        // Execute rules
        for rule_id in rules_to_execute {
            if let Err(e) = self.execute_rule(&rule_id, database, registry).await {
                warn!("Error executing mutation rule '{}': {}", rule_id, e);
            } else {
                executed += 1;
            }
        }

        Ok(executed)
    }

    /// Evaluate a transformation expression
    ///
    /// Supports expressions like:
    /// - "{{field}} * 2" - multiply field by 2
    /// - "{{field1}} + {{field2}}" - add two fields
    /// - "{{field}}.toUpperCase()" - string operations
    /// - "{{field}} + 10" - add constant
    /// - Simple template strings with variable substitution
    fn evaluate_transformation_expression(
        expression: &str,
        record: &HashMap<String, Value>,
    ) -> Result<Value> {
        use regex::Regex;

        // Convert record to Value for easier manipulation
        let record_value: Value =
            Value::Object(record.iter().map(|(k, v)| (k.clone(), v.clone())).collect());

        // Substitute variables in expression (e.g., "{{field}}" -> actual value)
        let re = Regex::new(r"\{\{([^}]+)\}\}")
            .map_err(|e| Error::generic(format!("Failed to compile regex: {}", e)))?;

        let substituted = re.replace_all(expression, |caps: &regex::Captures| {
            let var_name = caps.get(1).unwrap().as_str().trim();
            // Try to get value from record
            if let Some(value) = record.get(var_name) {
                // Convert to string representation for substitution
                if let Some(s) = value.as_str() {
                    s.to_string()
                } else if let Some(n) = value.as_f64() {
                    n.to_string()
                } else if let Some(b) = value.as_bool() {
                    b.to_string()
                } else {
                    value.to_string()
                }
            } else {
                // If not found, try JSONPath expression
                if var_name.starts_with('$') {
                    // Use JSONPath to extract value
                    if let Ok(selector) = jsonpath::Selector::new(var_name) {
                        let results: Vec<_> = selector.find(&record_value).collect();
                        if let Some(first) = results.first() {
                            if let Some(s) = first.as_str() {
                                return s.to_string();
                            } else if let Some(n) = first.as_f64() {
                                return n.to_string();
                            } else if let Some(b) = first.as_bool() {
                                return b.to_string();
                            }
                        }
                    }
                }
                format!("{{{{{}}}}}", var_name) // Keep original if not found
            }
        });

        // Try to evaluate as a mathematical expression
        let substituted_str = substituted.to_string();

        // Check for mathematical operations
        if substituted_str.contains('+')
            || substituted_str.contains('-')
            || substituted_str.contains('*')
            || substituted_str.contains('/')
        {
            // Try to parse and evaluate as math expression
            if let Ok(result) = Self::evaluate_math_expression(&substituted_str) {
                return Ok(serde_json::json!(result));
            }
        }

        // Check for string operations
        if substituted_str.contains(".toUpperCase()") {
            let base = substituted_str.replace(".toUpperCase()", "");
            return Ok(Value::String(base.to_uppercase()));
        }
        if substituted_str.contains(".toLowerCase()") {
            let base = substituted_str.replace(".toLowerCase()", "");
            return Ok(Value::String(base.to_lowercase()));
        }
        if substituted_str.contains(".trim()") {
            let base = substituted_str.replace(".trim()", "");
            return Ok(Value::String(base.trim().to_string()));
        }

        // If no operations detected, return as string
        Ok(Value::String(substituted_str))
    }

    /// Evaluate a simple mathematical expression
    ///
    /// Supports basic operations: +, -, *, /
    /// Example: "10 + 5 * 2" -> 20
    fn evaluate_math_expression(expr: &str) -> Result<f64> {
        // Simple expression evaluator (handles basic arithmetic)
        // For more complex expressions, consider using a proper expression parser

        // Remove whitespace
        let expr = expr.replace(' ', "");

        // Try to parse as a simple expression
        // This is a simplified evaluator - for production, use a proper math parser
        let mut result = 0.0;
        let mut current_num = String::new();
        let mut last_op = '+';

        for ch in expr.chars() {
            match ch {
                '+' | '-' | '*' | '/' => {
                    if !current_num.is_empty() {
                        let num: f64 = current_num.parse().map_err(|_| {
                            Error::generic(format!("Invalid number: {}", current_num))
                        })?;

                        match last_op {
                            '+' => result += num,
                            '-' => result -= num,
                            '*' => result *= num,
                            '/' => {
                                if num == 0.0 {
                                    return Err(Error::generic("Division by zero".to_string()));
                                }
                                result /= num;
                            }
                            _ => {}
                        }

                        current_num.clear();
                    }
                    last_op = ch;
                }
                '0'..='9' | '.' => {
                    current_num.push(ch);
                }
                _ => {
                    return Err(Error::generic(format!("Invalid character in expression: {}", ch)));
                }
            }
        }

        // Handle last number
        if !current_num.is_empty() {
            let num: f64 = current_num
                .parse()
                .map_err(|_| Error::generic(format!("Invalid number: {}", current_num)))?;

            match last_op {
                '+' => result += num,
                '-' => result -= num,
                '*' => result *= num,
                '/' => {
                    if num == 0.0 {
                        return Err(Error::generic("Division by zero".to_string()));
                    }
                    result /= num;
                }
                _ => result = num, // First number
            }
        }

        Ok(result)
    }

    /// Evaluate a JSONPath condition against a record
    ///
    /// The condition can be:
    /// - A simple JSONPath expression that checks for existence (e.g., "$.status")
    /// - A JSONPath expression with comparison (e.g., "$.status == 'active'")
    /// - A boolean JSONPath expression (e.g., "$.enabled")
    ///
    /// Returns true if the condition is met, false otherwise.
    fn evaluate_condition(condition: &str, record: &Value) -> Result<bool> {
        // Simple JSONPath evaluation
        // For basic existence checks (e.g., "$.field"), check if path exists and is truthy
        // For comparison expressions (e.g., "$.field == 'value'"), parse and evaluate

        // Try to parse as JSONPath selector
        if let Ok(selector) = jsonpath::Selector::new(condition) {
            // If condition is just a path (no comparison), check if it exists and is truthy
            let results: Vec<_> = selector.find(record).collect();
            if !results.is_empty() {
                // Check if any result is truthy
                for result in results {
                    match result {
                        Value::Bool(b) => {
                            if *b {
                                return Ok(true);
                            }
                        }
                        Value::Null => continue,
                        Value::String(s) => {
                            if !s.is_empty() {
                                return Ok(true);
                            }
                        }
                        Value::Number(n) => {
                            if n.as_f64().map(|f| f != 0.0).unwrap_or(false) {
                                return Ok(true);
                            }
                        }
                        _ => return Ok(true), // Other types (objects, arrays) are truthy
                    }
                }
            }
            return Ok(false);
        }

        // If JSONPath parsing fails, try to parse as a comparison expression
        // Simple pattern: "$.field == 'value'" or "$.field > 10"
        if condition.contains("==") {
            let parts: Vec<&str> = condition.split("==").map(|s| s.trim()).collect();
            if parts.len() == 2 {
                let path = parts[0].trim();
                let expected = parts[1].trim().trim_matches('\'').trim_matches('"');

                if let Ok(selector) = jsonpath::Selector::new(path) {
                    let results: Vec<_> = selector.find(record).collect();
                    for result in results {
                        match result {
                            Value::String(s) if s == expected => return Ok(true),
                            Value::Number(n) => {
                                if let Ok(expected_num) = expected.parse::<f64>() {
                                    if n.as_f64().map(|f| f == expected_num).unwrap_or(false) {
                                        return Ok(true);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        } else if condition.contains(">") {
            let parts: Vec<&str> = condition.split(">").map(|s| s.trim()).collect();
            if parts.len() == 2 {
                let path = parts[0].trim();
                if let Ok(expected_num) = parts[1].trim().parse::<f64>() {
                    if let Ok(selector) = jsonpath::Selector::new(path) {
                        let results: Vec<_> = selector.find(record).collect();
                        for result in results {
                            if let Value::Number(n) = result {
                                if n.as_f64().map(|f| f > expected_num).unwrap_or(false) {
                                    return Ok(true);
                                }
                            }
                        }
                    }
                }
            }
        } else if condition.contains("<") {
            let parts: Vec<&str> = condition.split("<").map(|s| s.trim()).collect();
            if parts.len() == 2 {
                let path = parts[0].trim();
                if let Ok(expected_num) = parts[1].trim().parse::<f64>() {
                    if let Ok(selector) = jsonpath::Selector::new(path) {
                        let results: Vec<_> = selector.find(record).collect();
                        for result in results {
                            if let Value::Number(n) = result {
                                if n.as_f64().map(|f| f < expected_num).unwrap_or(false) {
                                    return Ok(true);
                                }
                            }
                        }
                    }
                }
            }
        }

        // If we can't parse the condition, log a warning and return false
        warn!("Could not evaluate condition '{}', treating as false", condition);
        Ok(false)
    }

    /// Execute a specific mutation rule
    async fn execute_rule(
        &self,
        rule_id: &str,
        database: &dyn crate::database::VirtualDatabase,
        registry: &crate::entities::EntityRegistry,
    ) -> Result<()> {
        let now = time_travel_now();

        // Get rule
        let rule = {
            let rules = self.rules.read().await;
            rules
                .get(rule_id)
                .ok_or_else(|| Error::generic(format!("Mutation rule '{}' not found", rule_id)))?
                .clone()
        };

        // Get entity info
        let entity = registry
            .get(&rule.entity_name)
            .ok_or_else(|| Error::generic(format!("Entity '{}' not found", rule.entity_name)))?;

        let table_name = entity.table_name();

        // Query all records for this entity
        let query = format!("SELECT * FROM {}", table_name);
        let records = database.query(&query, &[]).await?;

        // Apply mutation to each record
        let pk_field = entity.schema.primary_key.first().map(|s| s.as_str()).unwrap_or("id");

        for record in records {
            // Check condition if specified
            if let Some(ref condition) = rule.condition {
                // Convert HashMap to Value for JSONPath evaluation
                let record_value =
                    Value::Object(record.iter().map(|(k, v)| (k.clone(), v.clone())).collect());

                // Evaluate JSONPath condition
                // Condition should be a JSONPath expression that evaluates to a truthy value
                // Examples: "$.status == 'active'", "$.age > 18", "$.enabled"
                if !MutationRuleManager::evaluate_condition(condition, &record_value)? {
                    debug!("Condition '{}' not met for record, skipping", condition);
                    continue;
                }
            }

            // Get primary key value
            let pk_value = record
                .get(pk_field)
                .ok_or_else(|| Error::generic(format!("Primary key '{}' not found", pk_field)))?;

            // Apply mutation operation
            match &rule.operation {
                MutationOperation::Set { field, value } => {
                    let update_query =
                        format!("UPDATE {} SET {} = ? WHERE {} = ?", table_name, field, pk_field);
                    database.execute(&update_query, &[value.clone(), pk_value.clone()]).await?;
                }
                MutationOperation::Increment { field, amount } => {
                    // Get current value
                    if let Some(current) = record.get(field) {
                        let new_value = if let Some(num) = current.as_f64() {
                            Value::Number(
                                serde_json::Number::from_f64(num + amount)
                                    .unwrap_or_else(|| serde_json::Number::from(0)),
                            )
                        } else if let Some(num) = current.as_i64() {
                            Value::Number(serde_json::Number::from(num + *amount as i64))
                        } else {
                            continue; // Skip non-numeric fields
                        };

                        let update_query = format!(
                            "UPDATE {} SET {} = ? WHERE {} = ?",
                            table_name, field, pk_field
                        );
                        database.execute(&update_query, &[new_value, pk_value.clone()]).await?;
                    }
                }
                MutationOperation::Decrement { field, amount } => {
                    // Get current value
                    if let Some(current) = record.get(field) {
                        let new_value = if let Some(num) = current.as_f64() {
                            Value::Number(
                                serde_json::Number::from_f64(num - amount)
                                    .unwrap_or_else(|| serde_json::Number::from(0)),
                            )
                        } else if let Some(num) = current.as_i64() {
                            Value::Number(serde_json::Number::from(num - *amount as i64))
                        } else {
                            continue; // Skip non-numeric fields
                        };

                        let update_query = format!(
                            "UPDATE {} SET {} = ? WHERE {} = ?",
                            table_name, field, pk_field
                        );
                        database.execute(&update_query, &[new_value, pk_value.clone()]).await?;
                    }
                }
                MutationOperation::Transform { field, expression } => {
                    // Evaluate transformation expression
                    let transformed_value =
                        Self::evaluate_transformation_expression(expression, &record)?;

                    let update_query =
                        format!("UPDATE {} SET {} = ? WHERE {} = ?", table_name, field, pk_field);
                    database.execute(&update_query, &[transformed_value, pk_value.clone()]).await?;
                }
                MutationOperation::UpdateStatus { status } => {
                    let update_query =
                        format!("UPDATE {} SET status = ? WHERE {} = ?", table_name, pk_field);
                    database
                        .execute(&update_query, &[Value::String(status.clone()), pk_value.clone()])
                        .await?;
                }
            }
        }

        // Update rule state
        {
            let mut rules = self.rules.write().await;
            if let Some(rule) = rules.get_mut(rule_id) {
                rule.last_execution = Some(now);
                rule.execution_count += 1;

                // Calculate next execution
                rule.next_execution = rule.calculate_next_execution(now);
            }
        }

        info!("Executed mutation rule '{}' on entity '{}'", rule_id, rule.entity_name);
        Ok(())
    }
}

impl Default for MutationRuleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // MutationTrigger tests
    #[test]
    fn test_mutation_trigger_interval_serialize() {
        let trigger = MutationTrigger::Interval {
            duration_seconds: 3600,
        };
        let json = serde_json::to_string(&trigger).unwrap();
        assert!(json.contains("interval"));
        assert!(json.contains("3600"));
    }

    #[test]
    fn test_mutation_trigger_at_time_serialize() {
        let trigger = MutationTrigger::AtTime {
            hour: 9,
            minute: 30,
        };
        let json = serde_json::to_string(&trigger).unwrap();
        assert!(json.contains("attime"));
        assert!(json.contains("\"hour\":9"));
    }

    #[test]
    fn test_mutation_trigger_field_threshold_serialize() {
        let trigger = MutationTrigger::FieldThreshold {
            field: "age".to_string(),
            threshold: serde_json::json!(100),
            operator: ComparisonOperator::Gt,
        };
        let json = serde_json::to_string(&trigger).unwrap();
        assert!(json.contains("fieldthreshold"));
        assert!(json.contains("age"));
    }

    #[test]
    fn test_mutation_trigger_clone() {
        let trigger = MutationTrigger::Interval {
            duration_seconds: 60,
        };
        let cloned = trigger.clone();
        match cloned {
            MutationTrigger::Interval { duration_seconds } => {
                assert_eq!(duration_seconds, 60);
            }
            _ => panic!("Expected Interval variant"),
        }
    }

    #[test]
    fn test_mutation_trigger_debug() {
        let trigger = MutationTrigger::Interval {
            duration_seconds: 120,
        };
        let debug = format!("{:?}", trigger);
        assert!(debug.contains("Interval"));
    }

    // MutationOperation tests
    #[test]
    fn test_mutation_operation_set() {
        let op = MutationOperation::Set {
            field: "status".to_string(),
            value: serde_json::json!("active"),
        };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("set"));
        assert!(json.contains("status"));
    }

    #[test]
    fn test_mutation_operation_increment() {
        let op = MutationOperation::Increment {
            field: "count".to_string(),
            amount: 5.0,
        };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("increment"));
        assert!(json.contains("count"));
    }

    #[test]
    fn test_mutation_operation_decrement() {
        let op = MutationOperation::Decrement {
            field: "balance".to_string(),
            amount: 10.5,
        };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("decrement"));
        assert!(json.contains("balance"));
    }

    #[test]
    fn test_mutation_operation_transform() {
        let op = MutationOperation::Transform {
            field: "value".to_string(),
            expression: "{{value}} * 2".to_string(),
        };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("transform"));
    }

    #[test]
    fn test_mutation_operation_update_status() {
        let op = MutationOperation::UpdateStatus {
            status: "completed".to_string(),
        };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("updatestatus"));
        assert!(json.contains("completed"));
    }

    #[test]
    fn test_mutation_operation_clone() {
        let op = MutationOperation::Set {
            field: "test".to_string(),
            value: serde_json::json!(42),
        };
        let cloned = op.clone();
        match cloned {
            MutationOperation::Set { field, value } => {
                assert_eq!(field, "test");
                assert_eq!(value, serde_json::json!(42));
            }
            _ => panic!("Expected Set variant"),
        }
    }

    // MutationRule tests
    #[test]
    fn test_mutation_rule_creation() {
        let rule = MutationRule::new(
            "test-1".to_string(),
            "User".to_string(),
            MutationTrigger::Interval {
                duration_seconds: 3600,
            },
            MutationOperation::Increment {
                field: "count".to_string(),
                amount: 1.0,
            },
        );

        assert_eq!(rule.id, "test-1");
        assert_eq!(rule.entity_name, "User");
        assert!(rule.enabled);
        assert!(rule.description.is_none());
        assert!(rule.condition.is_none());
        assert!(rule.last_execution.is_none());
        assert_eq!(rule.execution_count, 0);
    }

    #[test]
    fn test_mutation_rule_defaults() {
        let rule = MutationRule::new(
            "rule-1".to_string(),
            "Order".to_string(),
            MutationTrigger::Interval {
                duration_seconds: 60,
            },
            MutationOperation::Set {
                field: "status".to_string(),
                value: serde_json::json!("processed"),
            },
        );

        // Default values
        assert!(rule.enabled);
        assert!(rule.description.is_none());
        assert!(rule.condition.is_none());
        assert!(rule.last_execution.is_none());
        assert_eq!(rule.execution_count, 0);
    }

    #[test]
    fn test_mutation_rule_clone() {
        let rule = MutationRule::new(
            "clone-test".to_string(),
            "Entity".to_string(),
            MutationTrigger::Interval {
                duration_seconds: 100,
            },
            MutationOperation::Increment {
                field: "counter".to_string(),
                amount: 1.0,
            },
        );

        let cloned = rule.clone();
        assert_eq!(rule.id, cloned.id);
        assert_eq!(rule.entity_name, cloned.entity_name);
        assert_eq!(rule.enabled, cloned.enabled);
    }

    #[test]
    fn test_mutation_rule_debug() {
        let rule = MutationRule::new(
            "debug-rule".to_string(),
            "Test".to_string(),
            MutationTrigger::Interval {
                duration_seconds: 10,
            },
            MutationOperation::Set {
                field: "f".to_string(),
                value: serde_json::json!("v"),
            },
        );

        let debug = format!("{:?}", rule);
        assert!(debug.contains("MutationRule"));
        assert!(debug.contains("debug-rule"));
    }

    #[test]
    fn test_mutation_trigger_interval() {
        let rule = MutationRule::new(
            "test-1".to_string(),
            "User".to_string(),
            MutationTrigger::Interval {
                duration_seconds: 3600,
            },
            MutationOperation::Set {
                field: "status".to_string(),
                value: serde_json::json!("active"),
            },
        );

        let now = Utc::now();
        let next = rule.calculate_next_execution(now).unwrap();
        let duration = next - now;

        // Should be approximately 1 hour
        assert!(duration.num_seconds() >= 3599 && duration.num_seconds() <= 3601);
    }

    #[test]
    fn test_mutation_rule_calculate_next_execution_disabled() {
        let mut rule = MutationRule::new(
            "disabled-rule".to_string(),
            "Entity".to_string(),
            MutationTrigger::Interval {
                duration_seconds: 60,
            },
            MutationOperation::Set {
                field: "f".to_string(),
                value: serde_json::json!("v"),
            },
        );
        rule.enabled = false;

        let now = Utc::now();
        assert!(rule.calculate_next_execution(now).is_none());
    }

    #[test]
    fn test_mutation_rule_calculate_next_execution_field_threshold() {
        let rule = MutationRule::new(
            "threshold-rule".to_string(),
            "Entity".to_string(),
            MutationTrigger::FieldThreshold {
                field: "value".to_string(),
                threshold: serde_json::json!(100),
                operator: ComparisonOperator::Gt,
            },
            MutationOperation::Set {
                field: "f".to_string(),
                value: serde_json::json!("v"),
            },
        );

        // Field threshold triggers don't have scheduled execution
        let now = Utc::now();
        assert!(rule.calculate_next_execution(now).is_none());
    }

    // MutationRuleManager tests
    #[tokio::test]
    async fn test_mutation_rule_manager() {
        let manager = MutationRuleManager::new();

        let rule = MutationRule::new(
            "test-1".to_string(),
            "User".to_string(),
            MutationTrigger::Interval {
                duration_seconds: 3600,
            },
            MutationOperation::Increment {
                field: "count".to_string(),
                amount: 1.0,
            },
        );

        manager.add_rule(rule).await.unwrap();

        let rules = manager.list_rules().await;
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, "test-1");
    }

    #[test]
    fn test_mutation_rule_manager_new() {
        let _manager = MutationRuleManager::new();
        // Manager should be created without error
        assert!(true);
    }

    #[test]
    fn test_mutation_rule_manager_default() {
        let _manager = MutationRuleManager::default();
        // Default should work like new
        assert!(true);
    }

    #[tokio::test]
    async fn test_mutation_rule_manager_add_and_get_rule() {
        let manager = MutationRuleManager::new();

        let rule = MutationRule::new(
            "get-test".to_string(),
            "Order".to_string(),
            MutationTrigger::Interval {
                duration_seconds: 60,
            },
            MutationOperation::Set {
                field: "status".to_string(),
                value: serde_json::json!("done"),
            },
        );

        manager.add_rule(rule).await.unwrap();

        let retrieved = manager.get_rule("get-test").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "get-test");
    }

    #[tokio::test]
    async fn test_mutation_rule_manager_get_nonexistent() {
        let manager = MutationRuleManager::new();
        let retrieved = manager.get_rule("nonexistent").await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_mutation_rule_manager_remove_rule() {
        let manager = MutationRuleManager::new();

        let rule = MutationRule::new(
            "remove-test".to_string(),
            "Entity".to_string(),
            MutationTrigger::Interval {
                duration_seconds: 60,
            },
            MutationOperation::Set {
                field: "f".to_string(),
                value: serde_json::json!("v"),
            },
        );

        manager.add_rule(rule).await.unwrap();
        assert!(manager.get_rule("remove-test").await.is_some());

        let removed = manager.remove_rule("remove-test").await;
        assert!(removed);
        assert!(manager.get_rule("remove-test").await.is_none());
    }

    #[tokio::test]
    async fn test_mutation_rule_manager_remove_nonexistent() {
        let manager = MutationRuleManager::new();
        let removed = manager.remove_rule("nonexistent").await;
        assert!(!removed);
    }

    #[tokio::test]
    async fn test_mutation_rule_manager_list_rules() {
        let manager = MutationRuleManager::new();

        // Add multiple rules
        for i in 1..=3 {
            let rule = MutationRule::new(
                format!("rule-{}", i),
                "Entity".to_string(),
                MutationTrigger::Interval {
                    duration_seconds: 60,
                },
                MutationOperation::Set {
                    field: "f".to_string(),
                    value: serde_json::json!("v"),
                },
            );
            manager.add_rule(rule).await.unwrap();
        }

        let rules = manager.list_rules().await;
        assert_eq!(rules.len(), 3);
    }

    #[tokio::test]
    async fn test_mutation_rule_manager_list_rules_for_entity() {
        let manager = MutationRuleManager::new();

        // Add rules for different entities
        let rule1 = MutationRule::new(
            "user-rule".to_string(),
            "User".to_string(),
            MutationTrigger::Interval {
                duration_seconds: 60,
            },
            MutationOperation::Set {
                field: "f".to_string(),
                value: serde_json::json!("v"),
            },
        );

        let rule2 = MutationRule::new(
            "order-rule".to_string(),
            "Order".to_string(),
            MutationTrigger::Interval {
                duration_seconds: 60,
            },
            MutationOperation::Set {
                field: "f".to_string(),
                value: serde_json::json!("v"),
            },
        );

        let rule3 = MutationRule::new(
            "user-rule-2".to_string(),
            "User".to_string(),
            MutationTrigger::Interval {
                duration_seconds: 120,
            },
            MutationOperation::Increment {
                field: "count".to_string(),
                amount: 1.0,
            },
        );

        manager.add_rule(rule1).await.unwrap();
        manager.add_rule(rule2).await.unwrap();
        manager.add_rule(rule3).await.unwrap();

        let user_rules = manager.list_rules_for_entity("User").await;
        assert_eq!(user_rules.len(), 2);

        let order_rules = manager.list_rules_for_entity("Order").await;
        assert_eq!(order_rules.len(), 1);

        let product_rules = manager.list_rules_for_entity("Product").await;
        assert!(product_rules.is_empty());
    }

    #[tokio::test]
    async fn test_mutation_rule_manager_set_rule_enabled() {
        let manager = MutationRuleManager::new();

        let rule = MutationRule::new(
            "enable-test".to_string(),
            "Entity".to_string(),
            MutationTrigger::Interval {
                duration_seconds: 60,
            },
            MutationOperation::Set {
                field: "f".to_string(),
                value: serde_json::json!("v"),
            },
        );

        manager.add_rule(rule).await.unwrap();

        // Disable the rule
        manager.set_rule_enabled("enable-test", false).await.unwrap();
        let disabled_rule = manager.get_rule("enable-test").await.unwrap();
        assert!(!disabled_rule.enabled);
        assert!(disabled_rule.next_execution.is_none());

        // Re-enable the rule
        manager.set_rule_enabled("enable-test", true).await.unwrap();
        let enabled_rule = manager.get_rule("enable-test").await.unwrap();
        assert!(enabled_rule.enabled);
        assert!(enabled_rule.next_execution.is_some());
    }

    #[tokio::test]
    async fn test_mutation_rule_manager_set_rule_enabled_nonexistent() {
        let manager = MutationRuleManager::new();
        let result = manager.set_rule_enabled("nonexistent", true).await;
        assert!(result.is_err());
    }

    // ComparisonOperator tests
    #[test]
    fn test_comparison_operator_variants() {
        let operators = vec![
            ComparisonOperator::Gt,
            ComparisonOperator::Lt,
            ComparisonOperator::Eq,
            ComparisonOperator::Gte,
            ComparisonOperator::Lte,
        ];

        for op in operators {
            let json = serde_json::to_string(&op).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_comparison_operator_clone() {
        let op = ComparisonOperator::Gt;
        let cloned = op.clone();
        assert!(matches!(cloned, ComparisonOperator::Gt));
    }

    #[test]
    fn test_comparison_operator_debug() {
        let op = ComparisonOperator::Lt;
        let debug = format!("{:?}", op);
        assert!(debug.contains("Lt"));
    }

    #[test]
    fn test_comparison_operator_eq() {
        let op1 = ComparisonOperator::Gt;
        let op2 = ComparisonOperator::Gt;
        let op3 = ComparisonOperator::Lt;
        assert_eq!(op1, op2);
        assert_ne!(op1, op3);
    }

    #[test]
    fn test_comparison_operator_serialize() {
        assert_eq!(serde_json::to_string(&ComparisonOperator::Gt).unwrap(), "\"gt\"");
        assert_eq!(serde_json::to_string(&ComparisonOperator::Lt).unwrap(), "\"lt\"");
        assert_eq!(serde_json::to_string(&ComparisonOperator::Eq).unwrap(), "\"eq\"");
        assert_eq!(serde_json::to_string(&ComparisonOperator::Gte).unwrap(), "\"gte\"");
        assert_eq!(serde_json::to_string(&ComparisonOperator::Lte).unwrap(), "\"lte\"");
    }
}
