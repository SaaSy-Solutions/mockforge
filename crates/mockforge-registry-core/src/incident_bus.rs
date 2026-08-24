//! #720 — IncidentBus: the shared front door for raising incidents.
//!
//! Before this trait, every producer (external HTTP `POST /incidents`,
//! the runner's `diff_finding` handler, hosted-mock health checks)
//! called [`crate::models::incident::Incident::raise`] directly and each
//! re-encoded its own severity mapping. The bus centralises:
//!
//! - **severity mapping** — one canonical translation from
//!   producer-specific vocabularies to the incident severities routing
//!   rules understand (`map_finding_severity`);
//! - **the raise pipeline** — dedupe-keyed insert through the partial-
//!   unique index, so callers cannot accidentally bypass it.
//!
//! Producers construct a [`PgIncidentBus`] over their pool (or inject any
//! other impl in tests).

use std::future::Future;

#[cfg(feature = "postgres")]
use crate::models::incident::{Incident, RaiseIncidentInput};

/// Map a contract-diff finding severity to an incident severity.
///
/// Returns `None` for findings too mild to page anyone ("medium",
/// "low", "unknown", …) — those stay as `test_run_events` only. The
/// synonyms exist because the auditor's LLM pass uses "critical" where
/// the structural diff uses "breaking".
pub fn map_finding_severity(severity: &str) -> Option<&'static str> {
    match severity {
        "breaking" | "critical" => Some("critical"),
        "high" => Some("high"),
        _ => None,
    }
}

/// Producer-facing interface to the incident pipeline.
///
/// RPITIT keeps the trait object-safe enough for injection without an
/// `async_trait` dependency; implementors return a future that outlives
/// `&self`.
#[cfg(feature = "postgres")]
pub trait IncidentBus: Send + Sync {
    fn raise(
        &self,
        input: RaiseIncidentInput<'_>,
    ) -> impl Future<Output = sqlx::Result<Incident>> + Send;
}

/// Production implementation over a Postgres pool. Delegates to
/// [`Incident::raise`], whose `ON CONFLICT DO NOTHING` against the open-
/// incident dedupe index makes repeated fires idempotent.
#[cfg(feature = "postgres")]
pub struct PgIncidentBus {
    pool: sqlx::PgPool,
}

#[cfg(feature = "postgres")]
impl PgIncidentBus {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "postgres")]
impl IncidentBus for PgIncidentBus {
    async fn raise(&self, input: RaiseIncidentInput<'_>) -> sqlx::Result<Incident> {
        Incident::raise(&self.pool, input).await
    }
}

#[cfg(test)]
mod tests {
    use super::map_finding_severity;

    #[test]
    fn breaking_and_critical_map_to_critical() {
        assert_eq!(map_finding_severity("breaking"), Some("critical"));
        assert_eq!(map_finding_severity("critical"), Some("critical"));
    }

    #[test]
    fn high_maps_to_high() {
        assert_eq!(map_finding_severity("high"), Some("high"));
    }

    #[test]
    fn milder_findings_do_not_page() {
        for s in ["medium", "low", "unknown", "", "info"] {
            assert_eq!(map_finding_severity(s), None, "{s} must not page");
        }
    }
}
