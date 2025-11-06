//! Time-based data evolution
//!
//! This module handles data aging rules, automatic cleanup of expired data,
//! and time-based field updates.

use crate::Result;
use mockforge_core::VirtualClock;
use std::sync::Arc;

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
    /// Virtual clock for time travel (optional)
    virtual_clock: Option<Arc<VirtualClock>>,
}

impl AgingManager {
    /// Create a new aging manager
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            virtual_clock: None,
        }
    }

    /// Create a new aging manager with virtual clock
    pub fn with_virtual_clock(clock: Arc<VirtualClock>) -> Self {
        Self {
            rules: Vec::new(),
            virtual_clock: Some(clock),
        }
    }

    /// Set the virtual clock
    pub fn set_virtual_clock(&mut self, clock: Option<Arc<VirtualClock>>) {
        self.virtual_clock = clock;
    }

    /// Get the current time (virtual or real)
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        if let Some(ref clock) = self.virtual_clock {
            clock.now()
        } else {
            chrono::Utc::now()
        }
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
            let now = self.now();

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
                                    .unwrap_or_else(|| self.now())
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
        let now = self.now().to_rfc3339();
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

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_core::VirtualClock;
    use std::sync::Arc;

    #[test]
    fn test_aging_with_virtual_clock() {
        // Create aging manager with virtual clock
        let clock = Arc::new(VirtualClock::new());
        let initial_time = chrono::Utc::now();
        clock.enable_and_set(initial_time);

        let aging_manager = AgingManager::with_virtual_clock(clock.clone());

        // Verify that aging manager uses virtual clock
        let now = aging_manager.now();
        assert!((now - initial_time).num_seconds().abs() < 1);

        // Advance virtual clock by 2 hours
        clock.advance(chrono::Duration::hours(2));

        // Verify aging manager now sees the advanced time
        let advanced_now = aging_manager.now();
        let elapsed = advanced_now - initial_time;
        assert!(elapsed.num_hours() >= 1 && elapsed.num_hours() <= 3);
    }

    #[test]
    fn test_aging_timestamps_with_virtual_clock() {
        let clock = Arc::new(VirtualClock::new());
        let initial_time = chrono::Utc::now();
        clock.enable_and_set(initial_time);

        let aging_manager = AgingManager::with_virtual_clock(clock.clone());

        // Advance time by 1 month
        clock.advance(chrono::Duration::days(30));

        // Update timestamps should use virtual clock
        // This is tested indirectly through the now() method
        let now = aging_manager.now();
        let elapsed = now - initial_time;
        assert!(elapsed.num_days() >= 29 && elapsed.num_days() <= 31);
    }

    #[test]
    fn test_one_month_aging_scenario() {
        // Simulate "1 month later" scenario with data aging
        let clock = Arc::new(VirtualClock::new());
        let initial_time = chrono::Utc::now();
        clock.enable_and_set(initial_time);

        let aging_manager = AgingManager::with_virtual_clock(clock.clone());

        // Initial time check
        let start_time = aging_manager.now();
        assert!((start_time - initial_time).num_seconds().abs() < 1);

        // Advance by 1 month (30 days)
        clock.advance(chrono::Duration::days(30));

        // Verify aging manager sees the advanced time
        let month_later = aging_manager.now();
        let elapsed = month_later - start_time;

        // Should be approximately 30 days
        assert!(
            elapsed.num_days() >= 29 && elapsed.num_days() <= 31,
            "Expected ~30 days, got {} days",
            elapsed.num_days()
        );
    }
}
