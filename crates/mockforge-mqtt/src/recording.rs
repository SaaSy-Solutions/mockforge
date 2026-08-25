//! #715 — traffic recording for the live MQTT wire path.
//!
//! The recorder is process-global (same pattern as
//! `mockforge_analytics::set_global_db`) because the wire path threads
//! free functions and per-client tasks that have no shared state handle:
//! `init_from_env` installs it once at server startup from
//! `MOCKFORGE_MQTT_RECORDING_DB`, and the publish/deliver choke points in
//! `SessionManager` read it thereafter. Unset env → no-op everywhere.

use std::sync::{Arc, OnceLock};

use mockforge_recorder::protocols::async_brokers;
use mockforge_recorder::{models::RecordedRequest, Recorder, RecorderDatabase};

static RECORDER: OnceLock<Option<Arc<Recorder>>> = OnceLock::new();

/// Install the recorder from `MOCKFORGE_MQTT_RECORDING_DB`. Idempotent:
/// only the first call wins (server startup may race listener setup).
/// A missing/unusable path records nothing — behaviour is unchanged.
pub async fn init_from_env() {
    if RECORDER.get().is_some() {
        return;
    }
    let recorder = match std::env::var("MOCKFORGE_MQTT_RECORDING_DB") {
        Ok(path) if !path.trim().is_empty() => {
            match RecorderDatabase::new(path.as_str()).await {
                Ok(db) => Some(Arc::new(Recorder::new(db))),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "MOCKFORGE_MQTT_RECORDING_DB unusable; continuing without recording"
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
                tracing::warn!(error = %e, "failed to record mqtt exchange");
            }
        });
    }
}

/// Record an inbound client PUBLISH crossing the wire (#715).
pub(crate) fn record_publish(client_id: &str, topic: &str, qos: u8, retain: bool, payload: &[u8]) {
    let mut event = async_brokers::mqtt_event("publish", topic, qos, retain, payload);
    event.client_ip = Some(client_id.to_string());
    spawn_record(event);
}

/// Record a delivery of a message to one subscriber (#715).
pub(crate) fn record_delivery(
    subscriber_id: &str,
    topic: &str,
    qos: u8,
    retain: bool,
    payload: &[u8],
) {
    let mut event = async_brokers::mqtt_event("deliver", topic, qos, retain, payload);
    event.client_ip = Some(subscriber_id.to_string());
    spawn_record(event);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn recorder_roundtrip_stores_mqtt_exchange() {
        let db = RecorderDatabase::new_in_memory().await.expect("in-memory db");
        let probe_db = db.clone();
        set_recorder(Arc::new(Recorder::new(db)));

        record_publish("ci-client", "sensors/temp", 1, false, b"21.5");

        // Give the fire-and-forget task a moment to flush.
        for _ in 0..20 {
            let result = mockforge_recorder::query::execute_query(
                &probe_db,
                mockforge_recorder::query::QueryFilter {
                    protocol: Some(mockforge_recorder::models::Protocol::Mqtt),
                    ..Default::default()
                },
            )
            .await
            .expect("query");
            if result.total >= 1 {
                let exchange = &result.exchanges[0];
                assert_eq!(exchange.request.method, "publish");
                assert_eq!(exchange.request.path, "sensors/temp");
                assert_eq!(exchange.request.body.as_deref(), Some("21.5"));
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        panic!("mqtt exchange was never recorded");
    }
}
