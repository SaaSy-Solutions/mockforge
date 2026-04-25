//! Federation-wide scenario activation model.
//!
//! Tracks which scenario is currently active on a federation and the
//! per-service override state that has been pushed to workspaces. Snapshotting
//! the manifest here (rather than storing only a `scenario_id`) ensures
//! deactivation/rollback still works after the source scenario is edited or
//! removed.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Lifecycle status for a federation-wide activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FederationScenarioActivationStatus {
    /// Scenario is active — workspaces should observe the overrides.
    Active,
    /// Scenario was deactivated by a user; overrides should be reverted.
    Deactivated,
    /// Activation failed mid-apply; overrides may be in an inconsistent state.
    Failed,
}

impl FederationScenarioActivationStatus {
    /// Serialized form used when writing to the database.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Deactivated => "deactivated",
            Self::Failed => "failed",
        }
    }

    /// Parse a status string from the database.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "deactivated" => Some(Self::Deactivated),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// Per-service runtime state for a federation activation.
///
/// One entry per service in the federation. The registry writes these as
/// `pending`, then flips them to `applied` / `failed` as the runtime poll
/// endpoint confirms workspaces have observed the overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerServiceActivationState {
    /// Service name (matches `ServiceBoundary.name`).
    pub service_name: String,
    /// Workspace the service is bound to.
    pub workspace_id: Uuid,
    /// State machine: `pending` → `applied` | `failed`.
    pub status: String,
    /// Error message when `status == "failed"`.
    #[serde(default)]
    pub error: Option<String>,
    /// Timestamp the workspace last confirmed the activation.
    #[serde(default)]
    pub last_observed_at: Option<DateTime<Utc>>,
}

/// Federation-wide scenario activation record.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct FederationScenarioActivation {
    pub id: Uuid,
    pub federation_id: Uuid,
    /// Source scenario — nullable because an admin may activate an inline
    /// manifest that is not stored in the scenarios table.
    pub scenario_id: Option<Uuid>,
    /// Human-readable name captured for audit even if `scenario_id` is later
    /// nulled out.
    pub scenario_name: String,
    /// Full manifest JSON snapshot at activation time.
    pub manifest_snapshot: serde_json::Value,
    /// Per-service overrides applied on top of the manifest; shape is
    /// `{ service_name: { ... } }`.
    pub service_overrides: serde_json::Value,
    /// Lifecycle status. Stored as TEXT in both Postgres and SQLite — see
    /// `FederationScenarioActivationStatus::as_str`.
    pub status: String,
    /// Per-service state; JSON array of `PerServiceActivationState`.
    pub per_service_state: serde_json::Value,
    pub activated_by: Uuid,
    pub activated_at: DateTime<Utc>,
    pub deactivated_at: Option<DateTime<Utc>>,
}

impl FederationScenarioActivation {
    /// Parse `status` into a typed enum.
    #[must_use]
    pub fn typed_status(&self) -> Option<FederationScenarioActivationStatus> {
        FederationScenarioActivationStatus::parse(&self.status)
    }

    /// Parse `per_service_state` into typed entries.
    ///
    /// # Errors
    ///
    /// Returns an error if the stored JSON is not an array of
    /// `PerServiceActivationState` records.
    pub fn parse_per_service_state(
        &self,
    ) -> Result<Vec<PerServiceActivationState>, serde_json::Error> {
        serde_json::from_value(self.per_service_state.clone())
    }
}
