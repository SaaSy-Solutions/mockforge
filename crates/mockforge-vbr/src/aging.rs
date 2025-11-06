//! Time-based data evolution
//!
//! This module handles data aging rules, automatic cleanup of expired data,
//! and time-based field updates.

use crate::Result;

/// Data aging rule
#[derive(Debug, Clone)]
pub struct AgingRule {
    /// Entity name
    pub entity_name: String,
    /// Field to check for expiration
    pub expiration_field: String,
    /// Expiration duration in seconds
    pub expiration_duration: u64,
    /// Action to take when expired
    pub action: AgingAction,
}

/// Action to take when data expires
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgingAction {
    /// Delete the record
    Delete,
    /// Mark as expired (set a flag)
    MarkExpired,
    /// Archive (move to archive table)
    Archive,
}

/// Data aging manager
pub struct AgingManager {
    /// Aging rules
    rules: Vec<AgingRule>,
}

impl AgingManager {
    /// Create a new aging manager
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add an aging rule
    pub fn add_rule(&mut self, rule: AgingRule) {
        self.rules.push(rule);
    }

    /// Clean up expired data
    ///
    /// Checks all aging rules and applies the configured action to expired records.
    pub async fn cleanup_expired(
        &self,
        database: &dyn crate::database::VirtualDatabase,
        registry: &crate::entities::EntityRegistry,
    ) -> Result<usize> {
        let mut total_cleaned = 0;

        for rule in &self.rules {
            // Get entity info
            let entity = match registry.get(&rule.entity_name) {
                Some(e) => e,
                None => continue, // Entity not found, skip this rule
            };

            let table_name = entity.table_name();
            let now = chrono::Utc::now();

            // Query all records for this entity
            let query = format!("SELECT * FROM {}", table_name);
            let records = database.query(&query, &[]).await?;

            for record in records {
                // Check expiration field
                if let Some(expiration_value) = record.get(&rule.expiration_field) {
                    // Parse timestamp
                    let expiration_time = match expiration_value {
                        serde_json::Value::String(s) => {
                            // Try parsing as ISO8601 timestamp
                            match chrono::DateTime::parse_from_rfc3339(s) {
                                Ok(dt) => dt.with_timezone(&chrono::Utc),
                                Err(_) => continue, // Invalid timestamp, skip
                            }
                        }
                        serde_json::Value::Number(n) => {
                            // Unix timestamp
                            if let Some(ts) = n.as_i64() {
                                chrono::DateTime::from_timestamp(ts, 0)
                                    .unwrap_or_else(|| chrono::Utc::now())
                            } else {
                                continue; // Invalid timestamp
                            }
                        }
                        _ => continue, // Not a timestamp field
                    };

                    // Check if expired
                    let age = now.signed_duration_since(expiration_time);
                    if age.num_seconds() > rule.expiration_duration as i64 {
                        // Apply action
                        match rule.action {
                            AgingAction::Delete => {
                                // Get primary key value
                                let pk_field = entity
                                    .schema
                                    .primary_key
                                    .first()
                                    .map(|s| s.as_str())
                                    .unwrap_or("id");
                                if let Some(pk_value) = record.get(pk_field) {
                                    let delete_query = format!(
                                        "DELETE FROM {} WHERE {} = ?",
                                        table_name, pk_field
                                    );
                                    database.execute(&delete_query, &[pk_value.clone()]).await?;
                                    total_cleaned += 1;
                                }
                            }
                            AgingAction::MarkExpired => {
                                // Update status field
                                let pk_field = entity
                                    .schema
                                    .primary_key
                                    .first()
                                    .map(|s| s.as_str())
                                    .unwrap_or("id");
                                if let Some(pk_value) = record.get(pk_field) {
                                    let update_query = format!(
                                        "UPDATE {} SET status = ? WHERE {} = ?",
                                        table_name, pk_field
                                    );
                                    database
                                        .execute(
                                            &update_query,
                                            &[
                                                serde_json::Value::String("expired".to_string()),
                                                pk_value.clone(),
                                            ],
                                        )
                                        .await?;
                                    total_cleaned += 1;
                                }
                            }
                            AgingAction::Archive => {
                                // For now, just mark as archived (full archive would require archive table)
                                let pk_field = entity
                                    .schema
                                    .primary_key
                                    .first()
                                    .map(|s| s.as_str())
                                    .unwrap_or("id");
                                if let Some(pk_value) = record.get(pk_field) {
                                    let update_query = format!(
                                        "UPDATE {} SET archived = ? WHERE {} = ?",
                                        table_name, pk_field
                                    );
                                    database
                                        .execute(
                                            &update_query,
                                            &[serde_json::Value::Bool(true), pk_value.clone()],
                                        )
                                        .await?;
                                    total_cleaned += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(total_cleaned)
    }

    /// Update timestamp fields
    ///
    /// Automatically updates `updated_at` fields when `auto_update_timestamps` is enabled.
    pub async fn update_timestamps(
        &self,
        database: &dyn crate::database::VirtualDatabase,
        table: &str,
        primary_key_field: &str,
        primary_key_value: &serde_json::Value,
    ) -> Result<()> {
        // Update updated_at field if it exists
        let now = chrono::Utc::now().to_rfc3339();
        let update_query =
            format!("UPDATE {} SET updated_at = ? WHERE {} = ?", table, primary_key_field);

        // Try to update, but ignore if column doesn't exist
        let _ = database
            .execute(&update_query, &[serde_json::Value::String(now), primary_key_value.clone()])
            .await;

        Ok(())
    }
}

impl Default for AgingManager {
    fn default() -> Self {
        Self::new()
    }
}
