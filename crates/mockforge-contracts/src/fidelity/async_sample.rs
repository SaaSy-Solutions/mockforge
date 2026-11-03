//! Protocol-aware recorded-message fidelity shared by CLI and admin consumers.

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

mod catalog;
mod schema;
mod validation;

/// A decoded recorded message, observed from the mock server's perspective.
pub struct AsyncMessage {
    /// Recorder protocol name: mqtt, kafka, amqp, or websocket.
    pub protocol: String,
    /// Recorder operation (publish/deliver, produce/consume, inbound/outbound).
    pub operation: String,
    /// Topic, AMQP exchange/routing-key, or original WebSocket channel path.
    pub address: Option<String>,
    /// Actual message headers, or None when the recorder did not capture them.
    pub headers: Option<Value>,
    /// Decoded message bytes, or None when the payload was not captured.
    pub payload: Option<Vec<u8>>,
    /// Capture time used to identify freshness.
    pub timestamp: DateTime<Utc>,
}

/// Per-route results use the same statuses as HTTP sample verification.
#[derive(Serialize)]
pub struct AsyncSampleRoute {
    /// Channel or directional operation identifier.
    pub endpoint: String,
    /// verified, drift-detected, not-verified, or not-sampled.
    pub status: &'static str,
    /// Number of matching captured messages.
    pub exchanges_checked: usize,
    /// Proven message contract violations.
    pub findings: Vec<String>,
    /// Reasons fidelity could not be established.
    pub notes: Vec<String>,
}

/// Structured sample report; unobserved operations remain explicitly unsampled.
#[derive(Serialize)]
pub struct AsyncSampleReport {
    /// Number of decoded messages assessed.
    pub exchanges_checked: usize,
    /// Most recent capture timestamp in RFC3339 format.
    pub latest_capture_at: String,
    /// Deterministically ordered per-route outcomes.
    pub routes: Vec<AsyncSampleRoute>,
}

/// Check message payloads and captured headers without fetching schema resources.
/// The expansion budget applies to source definitions. Excessive result details
/// fail instead of producing a partial report. No response record is required for
/// one-way broker messages.
pub fn verify(
    source: &Value,
    records: &[AsyncMessage],
    max_bytes: usize,
) -> Result<AsyncSampleReport, String> {
    let latest = records
        .iter()
        .map(|r| r.timestamp)
        .max()
        .ok_or("The sample contains no asynchronous messages")?;
    let (definitions, operations) = catalog::build(source, max_bytes)?;
    let mut routes: BTreeMap<_, _> =
        definitions.keys().map(|name| (name.clone(), new_route(name.clone()))).collect();
    let mut checker = validation::Checker::new(source);
    let mut detail_bytes = 0;
    for record in records {
        assess_record(record, &operations, &mut checker, &mut routes, &mut detail_bytes)?;
    }
    Ok(AsyncSampleReport {
        exchanges_checked: records.len(),
        latest_capture_at: latest.to_rfc3339(),
        routes: routes.into_values().collect(),
    })
}

fn assess_record(
    record: &AsyncMessage,
    operations: &[catalog::Operation],
    checker: &mut validation::Checker<'_>,
    routes: &mut BTreeMap<String, AsyncSampleRoute>,
    detail_bytes: &mut usize,
) -> Result<(), String> {
    let action = direction(record);
    let candidates: Vec<_> = operations
        .iter()
        .filter(|operation| {
            action == Some(operation.action.as_str())
                && operation.address.as_ref().is_some_and(|r| {
                    record.address.as_deref().is_some_and(|address| r.is_match(address))
                })
                && protocol_matches(&operation.protocols, &record.protocol)
        })
        .collect();
    if candidates.is_empty() {
        return unattributed(record, operations, routes, detail_bytes);
    }
    let ambiguous = candidates.len() > 1;
    let outcomes = candidates
        .into_iter()
        .map(|operation| {
            checker
                .alternatives(&operation.messages, record)
                .map(|outcome| (operation, outcome))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let all_invalid = outcomes.iter().all(|(_, outcome)| !outcome.findings.is_empty());
    let mut channels: BTreeMap<String, validation::Outcome> = BTreeMap::new();
    for (operation, mut outcome) in outcomes {
        let known_failures = if all_invalid {
            outcome.findings.clone()
        } else {
            Vec::new()
        };
        if ambiguous {
            outcome.findings.clear();
            outcome.notes.push("Multiple source operations share this channel and direction; the capture does not identify which operation ran".into());
        }
        if operation.protocols.is_empty() {
            outcome.notes.push("The source does not identify the channel protocol".into());
        }
        update(
            routes.get_mut(&operation.endpoint).ok_or("Missing sample operation")?,
            &outcome,
            detail_bytes,
        )?;
        let channel = channels.entry(operation.channel_endpoint.clone()).or_default();
        channel.findings.extend(outcome.findings);
        channel.findings.extend(known_failures);
        channel.notes.extend(outcome.notes);
    }
    for (endpoint, outcome) in channels {
        update(
            routes.get_mut(&endpoint).ok_or("Missing sample channel")?,
            &outcome,
            detail_bytes,
        )?;
    }
    Ok(())
}

fn unattributed(
    record: &AsyncMessage,
    operations: &[catalog::Operation],
    routes: &mut BTreeMap<String, AsyncSampleRoute>,
    detail_bytes: &mut usize,
) -> Result<(), String> {
    let endpoint = format!(
        "{} {} {}",
        record.protocol,
        record.operation,
        record.address.as_deref().unwrap_or("<unknown>")
    );
    let route = routes.entry(endpoint.clone()).or_insert_with(|| new_route(endpoint));
    let uncertain = record.address.is_none()
        || direction(record).is_none()
        || operations.is_empty()
        || operations.iter().any(|o| o.address.is_none());
    let outcome = if uncertain {
        validation::Outcome { notes:vec!["Recorded operation or a dynamic channel address cannot be attributed to a source operation".into()], ..Default::default() }
    } else {
        validation::Outcome { findings:vec!["Recorded message channel, protocol or direction is absent from the source definition".into()], ..Default::default() }
    };
    update(route, &outcome, detail_bytes)
}

fn new_route(endpoint: String) -> AsyncSampleRoute {
    AsyncSampleRoute {
        endpoint,
        status: "not-sampled",
        exchanges_checked: 0,
        findings: Vec::new(),
        notes: Vec::new(),
    }
}

fn update(
    route: &mut AsyncSampleRoute,
    outcome: &validation::Outcome,
    detail_bytes: &mut usize,
) -> Result<(), String> {
    route.exchanges_checked += 1;
    for (target, incoming) in [
        (&mut route.findings, &outcome.findings),
        (&mut route.notes, &outcome.notes),
    ] {
        for detail in incoming {
            if !target.contains(detail) {
                *detail_bytes += detail.len();
                if *detail_bytes > 1024 * 1024 {
                    return Err(
                        "Sample comparison produced too many details; export a smaller sample"
                            .into(),
                    );
                }
                target.push(detail.clone());
            }
        }
    }
    route.status = if !route.findings.is_empty() {
        "drift-detected"
    } else if !route.notes.is_empty() {
        "not-verified"
    } else {
        "verified"
    };
    Ok(())
}

fn direction(record: &AsyncMessage) -> Option<&'static str> {
    match (
        record.protocol.to_ascii_lowercase().as_str(),
        record.operation.to_ascii_lowercase().as_str(),
    ) {
        ("mqtt" | "amqp", "publish") | ("kafka", "produce") | ("websocket", "inbound") => {
            Some("RECEIVE")
        }
        ("mqtt" | "amqp", "deliver") | ("kafka", "consume") | ("websocket", "outbound") => {
            Some("SEND")
        }
        _ => None,
    }
}

fn protocol_matches(protocols: &[String], protocol: &str) -> bool {
    protocols.is_empty()
        || protocols.iter().any(|declared| match protocol.to_ascii_lowercase().as_str() {
            "mqtt" => matches!(declared.as_str(), "mqtt" | "mqtts" | "secure-mqtt"),
            "kafka" => matches!(declared.as_str(), "kafka" | "kafka-secure"),
            "amqp" => matches!(declared.as_str(), "amqp" | "amqps"),
            "websocket" => matches!(declared.as_str(), "ws" | "wss"),
            _ => false,
        })
}

#[cfg(test)]
mod tests;
