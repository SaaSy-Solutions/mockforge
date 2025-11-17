//! Time-triggered data mutation rules
//!
//! This module provides a system for automatically mutating VBR entity data
//! based on time triggers. It supports both aging-style rules (expiration-based)
//! and arbitrary field mutations (time-triggered changes).
//!
//! ## Usage
//!
//! ```rust,no_run
//! use mockforge_vbr::mutation_rules::{MutationRule, MutationRuleManager, MutationTrigger, MutationOperation};
//! use mockforge_core::time_travel_now;
//! use std::sync::Arc;
//!
//! let manager = MutationRuleManager::new();
//!
//! // Create a rule that increments a counter every hour
//! let rule = MutationRule {
//!     id: "hourly-counter".to_string(),
//!     entity_name: "User".to_string(),
//!     trigger: MutationTrigger::Interval {
//!         duration_seconds: 3600,
//!     },
//!     operation: MutationOperation::Increment {
//!         field: "login_count".to_string(),
//!         amount: 1.0,
//!     },
//!     enabled: true,
//! };
//!
//! manager.add_rule(rule).await;
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
        threshold: serde_json::Value,
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
        value: serde_json::Value,
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
                    next = next + Duration::days(1);
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
            Err(crate::Error::generic(format!("Mutation rule '{}' not found", rule_id)))
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
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(num + amount)
                                    .unwrap_or_else(|| serde_json::Number::from(0)),
                            )
                        } else if let Some(num) = current.as_i64() {
                            serde_json::Value::Number(serde_json::Number::from(
                                num + *amount as i64,
                            ))
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
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(num - amount)
                                    .unwrap_or_else(|| serde_json::Number::from(0)),
                            )
                        } else if let Some(num) = current.as_i64() {
                            serde_json::Value::Number(serde_json::Number::from(
                                num - *amount as i64,
                            ))
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
                MutationOperation::Transform {
                    field,
                    expression: _,
                } => {
                    // TODO: Implement transformation expressions
                    warn!("Transform operation not yet implemented for field '{}'", field);
                }
                MutationOperation::UpdateStatus { status } => {
                    let update_query =
                        format!("UPDATE {} SET status = ? WHERE {} = ?", table_name, pk_field);
                    database
                        .execute(
                            &update_query,
                            &[serde_json::Value::String(status.clone()), pk_value.clone()],
                        )
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
}
