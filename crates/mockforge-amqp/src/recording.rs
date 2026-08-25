//! #715 — traffic recording for the AMQP wire path (same pattern as
//! mockforge-mqtt's recording module): process-global recorder installed
//! from `MOCKFORGE_AMQP_RECORDING_DB`, read at the publish/deliver choke
//! points in `connection.rs`. Unset env → no-op everywhere.

use std::sync::{Arc, OnceLock};

use mockforge_recorder::protocols::async_brokers;
use mockforge_recorder::{models::RecordedRequest, Recorder, RecorderDatabase};

static RECORDER: OnceLock<Option<Arc<Recorder>>> = OnceLock::new();

/// Lazy, idempotent install from env. Cheap after the first call
/// (`OnceLock::get`); safe to invoke per publish/deliver.
pub async fn ensure_init() {
    if RECORDER.get().is_some() {
        return;
    }
    let recorder = match std::env::var("MOCKFORGE_AMQP_RECORDING_DB") {
        Ok(path) if !path.trim().is_empty() => {
            match RecorderDatabase::new(path.as_str()).await {
                Ok(db) => Some(Arc::new(Recorder::new(db))),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "MOCKFORGE_AMQP_RECORDING_DB unusable; continuing without recording"
                    );
                    None
                }
            }
        }
        _ => None,
    };
    let _ = RECORDER.set(recorder);
}

/// Attach a caller-built recorder directly (programmatic setup / tests).
pub fn set_recorder(recorder: Arc<Recorder>) {
    let _ = RECORDER.set(Some(recorder));
}

fn recorder() -> Option<Arc<Recorder>> {
    RECORDER.get().and_then(|o| o.clone())
}

fn spawn_record(event: RecordedRequest) {
    if let Some(rec) = recorder() {
        tokio::spawn(async move {
            if let Err(e) = rec.record_request(event).await {
                tracing::warn!(error = %e, "failed to record amqp exchange");
            }
        });
    }
}

/// Record a Basic.Publish crossing the wire (#715).
pub(crate) fn record_publish(
    exchange: &str,
    routing_key: &str,
    payload: &[u8],
) {
    spawn_record(async_brokers::amqp_event("publish", exchange, routing_key, payload));
}

/// Record a message delivered to one consumer (#715). The queue name
/// rides as the path (storage convention from #683).
pub(crate) fn record_deliver(queue_name: &str, payload: &[u8]) {
    spawn_record(async_brokers::amqp_event("deliver", "", queue_name, payload));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn recorder_roundtrip_stores_amqp_exchange() {
        let db = RecorderDatabase::new_in_memory().await.expect("in-memory db");
        let probe_db = db.clone();
        set_recorder(Arc::new(Recorder::new(db)));

        record_publish("amq.direct", "orders.created", br#"{"id":1}"#);

        for _ in 0..20 {
            let result = mockforge_recorder::query::execute_query(
                &probe_db,
                mockforge_recorder::query::QueryFilter {
                    protocol: Some(mockforge_recorder::models::Protocol::Amqp),
                    ..Default::default()
                },
            )
            .await
            .expect("query");
            if result.total >= 1 {
                let exchange = &result.exchanges[0];
                assert_eq!(exchange.request.method, "publish");
                assert_eq!(exchange.request.path, "amq.direct/orders.created");
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        panic!("amqp exchange was never recorded");
    }
}
