//! Field count tracking for percentage-based drift budgets
//!
//! This module provides functionality to track field counts over time
//! for calculating percentage-based drift budgets.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Field count record for an endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldCountRecord {
    /// Workspace ID (optional)
    pub workspace_id: Option<String>,
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// Number of fields in the contract
    pub field_count: u32,
    /// When this count was recorded
    pub recorded_at: DateTime<Utc>,
}

/// In-memory field count tracker
#[derive(Debug, Clone)]
pub struct FieldCountTracker {
    /// Field count records (key: "{workspace_id}:{method} {endpoint}")
    records: HashMap<String, Vec<FieldCountRecord>>,
}

impl FieldCountTracker {
    /// Create a new field count tracker
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }

    /// Record a field count for an endpoint
    pub fn record_count(
        &mut self,
        workspace_id: Option<&str>,
        endpoint: &str,
        method: &str,
        field_count: u32,
    ) {
        let key = Self::make_key(workspace_id, endpoint, method);
        let record = FieldCountRecord {
            workspace_id: workspace_id.map(|s| s.to_string()),
            endpoint: endpoint.to_string(),
            method: method.to_string(),
            field_count,
            recorded_at: Utc::now(),
        };

        self.records.entry(key).or_default().push(record);
    }

    /// Get the baseline field count for an endpoint
    ///
    /// Returns the most recent field count recorded before the specified time,
    /// or the most recent count if no time is specified.
    pub fn get_baseline_count(
        &self,
        workspace_id: Option<&str>,
        endpoint: &str,
        method: &str,
        before: Option<DateTime<Utc>>,
    ) -> Option<u32> {
        let key = Self::make_key(workspace_id, endpoint, method);
        let records = self.records.get(&key)?;

        // Filter by time if specified
        let filtered: Vec<&FieldCountRecord> = if let Some(before_time) = before {
            records.iter().filter(|r| r.recorded_at <= before_time).collect()
        } else {
            records.iter().collect()
        };

        // Get the most recent record
        filtered.iter().max_by_key(|r| r.recorded_at).map(|r| r.field_count)
    }

    /// Get average field count over a time window
    ///
    /// Returns the average field count for records within the specified time window.
    pub fn get_average_count(
        &self,
        workspace_id: Option<&str>,
        endpoint: &str,
        method: &str,
        window_days: u32,
    ) -> Option<f64> {
        let key = Self::make_key(workspace_id, endpoint, method);
        let records = self.records.get(&key)?;

        let cutoff = Utc::now() - chrono::Duration::days(window_days as i64);
        let window_records: Vec<&FieldCountRecord> =
            records.iter().filter(|r| r.recorded_at >= cutoff).collect();

        if window_records.is_empty() {
            return None;
        }

        let sum: u32 = window_records.iter().map(|r| r.field_count).sum();
        Some(sum as f64 / window_records.len() as f64)
    }

    /// Calculate field churn percentage
    ///
    /// Returns the percentage change in field count compared to the baseline.
    /// Positive values indicate growth, negative values indicate reduction.
    pub fn calculate_churn_percent(
        &self,
        workspace_id: Option<&str>,
        endpoint: &str,
        method: &str,
        current_count: u32,
        window_days: Option<u32>,
    ) -> Option<f64> {
        let baseline = if let Some(days) = window_days {
            // Use average over time window as baseline
            self.get_average_count(workspace_id, endpoint, method, days)?
        } else {
            // Use most recent count as baseline
            self.get_baseline_count(workspace_id, endpoint, method, None)? as f64
        };

        if baseline == 0.0 {
            return None;
        }

        let change = current_count as f64 - baseline;
        Some((change / baseline) * 100.0)
    }

    /// Clean up old records beyond retention period
    pub fn cleanup_old_records(&mut self, retention_days: u32) {
        let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);

        for records in self.records.values_mut() {
            records.retain(|r| r.recorded_at >= cutoff);
        }

        // Remove empty entries
        self.records.retain(|_, records| !records.is_empty());
    }

    /// Make a key for indexing records
    fn make_key(workspace_id: Option<&str>, endpoint: &str, method: &str) -> String {
        if let Some(ws_id) = workspace_id {
            format!("{}:{} {}", ws_id, method, endpoint)
        } else {
            format!("{} {}", method, endpoint)
        }
    }
}

impl Default for FieldCountTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_retrieve_count() {
        let mut tracker = FieldCountTracker::new();
        tracker.record_count(None, "/api/users", "GET", 10);

        let count = tracker.get_baseline_count(None, "/api/users", "GET", None);
        assert_eq!(count, Some(10));
    }

    #[test]
    fn test_calculate_churn_percent() {
        let mut tracker = FieldCountTracker::new();
        tracker.record_count(None, "/api/users", "GET", 10);

        // Record a new count with 20% increase
        let churn = tracker.calculate_churn_percent(None, "/api/users", "GET", 12, None);
        assert!(churn.is_some());
        let churn_value = churn.unwrap();
        assert!((churn_value - 20.0).abs() < 0.1); // 20% increase
    }

    #[test]
    fn test_average_count_over_window() {
        let mut tracker = FieldCountTracker::new();

        // Record counts at different times
        tracker.record_count(None, "/api/users", "GET", 10);
        tracker.record_count(None, "/api/users", "GET", 12);
        tracker.record_count(None, "/api/users", "GET", 14);

        let avg = tracker.get_average_count(None, "/api/users", "GET", 30);
        assert!(avg.is_some());
        let avg_value = avg.unwrap();
        assert!((avg_value - 12.0).abs() < 0.1); // Average of 10, 12, 14
    }
}
