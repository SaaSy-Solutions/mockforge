//! `TraceQueryProvider` bridge from the recorder DB to behavioral cloning (#675).
//!
//! The trait lives in `mockforge-intelligence` so the sequence learner can
//! be tested without depending on the recorder. This module implements the
//! trait for `mockforge_recorder::RecorderDatabase`, closing the
//! recorder→cloning loop that the audit called out:
//!
//! > "TraceQueryProvider trait is defined, but no automatic plumbing from
//! > recorder → behavioral model → mock generation. Manual integration
//! > required."
//!
//! With this impl, callers can pass an `&RecorderDatabase` straight into
//! `SequenceLearner::discover_sequences_from_traces` without writing a
//! shim.

use async_trait::async_trait;
use mockforge_foundation::Result;
use mockforge_intelligence::behavioral_cloning::{TraceQueryProvider, TraceRequest};

use crate::database::RecorderDatabase;

#[async_trait]
impl TraceQueryProvider for RecorderDatabase {
    async fn get_requests_by_trace(
        &self,
        min_requests_per_trace: Option<usize>,
    ) -> Result<Vec<(String, Vec<TraceRequest>)>> {
        // The recorder API takes i32 for min; clamp from usize. Anything
        // beyond i32::MAX is nonsense for this filter — treat it as "no
        // minimum" rather than panicking.
        let min_i32 = min_requests_per_trace.map(|n| i32::try_from(n).unwrap_or(i32::MAX));

        let groups = self.get_requests_by_trace(min_i32).await.map_err(|e| {
            mockforge_foundation::Error::internal(format!(
                "recorder get_requests_by_trace failed: {e}"
            ))
        })?;

        let mut out: Vec<(String, Vec<TraceRequest>)> = Vec::with_capacity(groups.len());
        for (trace_id, requests) in groups {
            let mapped: Vec<TraceRequest> = requests
                .into_iter()
                .map(|r| {
                    // RecorderDB stores duration_ms as i64; the sequence
                    // learner wants u64. Negative durations shouldn't
                    // happen for recorded traffic, but if they do we drop
                    // them rather than wrap around.
                    let duration_ms =
                        r.duration_ms.and_then(|d| if d >= 0 { Some(d as u64) } else { None });
                    TraceRequest {
                        id: r.id,
                        method: r.method,
                        path: r.path,
                        timestamp: r.timestamp,
                        duration_ms,
                    }
                })
                .collect();
            out.push((trace_id, mapped));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::RecorderDatabase;
    use crate::models::{Protocol, RecordedRequest};
    use mockforge_intelligence::behavioral_cloning::SequenceLearner;

    async fn make_db() -> RecorderDatabase {
        RecorderDatabase::new_in_memory().await.expect("in-memory recorder DB")
    }

    fn req(trace: &str, method: &str, path: &str, ts_ms: i64) -> RecordedRequest {
        RecordedRequest {
            id: uuid::Uuid::new_v4().to_string(),
            protocol: Protocol::Http,
            timestamp: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ts_ms).unwrap(),
            method: method.to_string(),
            path: path.to_string(),
            query_params: None,
            headers: "{}".to_string(),
            body: None,
            body_encoding: "utf8".to_string(),
            client_ip: None,
            trace_id: Some(trace.to_string()),
            span_id: None,
            duration_ms: Some(10),
            status_code: Some(200),
            tags: None,
        }
    }

    #[tokio::test]
    async fn impl_returns_grouped_requests_via_trait() {
        let db = make_db().await;
        // Two traces, three requests each — sequence learner uses min 2 by default.
        for r in [
            req("t-1", "POST", "/login", 1000),
            req("t-1", "GET", "/users", 2000),
            req("t-1", "GET", "/users/1", 3000),
            req("t-2", "POST", "/login", 4000),
            req("t-2", "GET", "/orders", 5000),
        ] {
            db.insert_request(&r).await.unwrap();
        }

        let provider: &dyn TraceQueryProvider = &db;
        let groups = provider.get_requests_by_trace(Some(2)).await.unwrap();
        assert_eq!(groups.len(), 2, "expected 2 traces, got {}", groups.len());
        let t1 = groups.iter().find(|(id, _)| id == "t-1").expect("t-1 present");
        assert_eq!(t1.1.len(), 3);
        assert_eq!(t1.1[0].method, "POST");
        assert_eq!(t1.1[0].path, "/login");
    }

    #[tokio::test]
    async fn impl_feeds_sequence_learner_end_to_end() {
        let db = make_db().await;
        // Three near-identical traces so the learner's frequency
        // threshold (default 0.5) is reachable.
        for trace_id in ["t-A", "t-B", "t-C"] {
            for r in [
                req(trace_id, "POST", "/login", 1000),
                req(trace_id, "GET", "/users", 2000),
                req(trace_id, "GET", "/users/1", 3000),
            ] {
                db.insert_request(&r).await.unwrap();
            }
        }

        // Discover sequences via the trait — this is the "one-call"
        // recorder → cloning path that #675 was asking for.
        let sequences = SequenceLearner::discover_sequences_from_traces(&db, 0.5, Some(2))
            .await
            .expect("sequence discovery succeeds");

        // We can't assert the exact shape of discovered sequences without
        // depending on internal heuristics, but the cardinality should be
        // non-zero given three matching traces of length 3.
        assert!(
            !sequences.is_empty(),
            "expected at least one behavioral sequence from 3 matching traces"
        );
    }
}
