//! Read-only message selection and recorder adaptation.
use super::Value;
use mockforge_contracts::fidelity::async_sample::{self, AsyncMessage, AsyncSampleReport};
use mockforge_core::{Error, Result};
use mockforge_recorder::{
    models::{Protocol, RecordedExchange},
    RecorderDatabase,
};
use std::{collections::BTreeSet, path::Path};

pub(super) fn check(source: &Value, records: &[RecordedExchange]) -> Result<AsyncSampleReport> {
    let messages = mockforge_recorder::async_messages::decode(records).map_err(Error::config)?;
    let messages: Vec<_> = messages
        .into_iter()
        .map(|r| AsyncMessage {
            protocol: r.protocol.as_str().into(),
            operation: r.operation,
            address: r.address,
            headers: r.headers,
            payload: r.payload,
            timestamp: r.timestamp,
        })
        .collect();
    async_sample::verify(source, &messages, usize::MAX).map_err(Error::config)
}

pub(super) async fn capture(path: &Path, limit: i32) -> Result<Vec<RecordedExchange>> {
    let db = RecorderDatabase::open_read_only(path)
        .await
        .map_err(|e| Error::config(format!("cannot open capture db: {e}")))?;
    let requests = db
        .list_recent_async_messages(limit)
        .await
        .map_err(|e| Error::config(e.to_string()))?;
    if requests.is_empty() {
        return Err(Error::config(
            "No asynchronous message events to validate; capture traffic first",
        ));
    }
    let mut records: Vec<_> = requests
        .into_iter()
        .map(|request| RecordedExchange {
            request,
            response: None,
        })
        .collect();
    let mut connections = BTreeSet::new();
    for exchange in &records {
        if exchange.request.protocol == Protocol::WebSocket {
            let headers: Value = serde_json::from_str(&exchange.request.headers)
                .map_err(|_| Error::config("Invalid recorded message headers"))?;
            if let Some(id) = headers["ws-connection-id"].as_str() {
                connections.insert(id.to_string());
            }
        }
    }
    for id in connections {
        if let Some(request) =
            db.get_request(&id).await.map_err(|e| Error::config(e.to_string()))?
        {
            if request.protocol == Protocol::WebSocket
                && request.method.eq_ignore_ascii_case("CONNECT")
            {
                records.push(RecordedExchange {
                    request,
                    response: None,
                });
            }
        }
    }
    Ok(records)
}
