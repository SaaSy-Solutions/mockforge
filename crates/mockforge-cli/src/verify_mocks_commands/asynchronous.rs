//! AsyncAPI definition, captured-message and complete fixture-set fidelity.
use super::{load_fidelity_document, DriftFinding, Value, VerifyMocksArgs};
use mockforge_contracts::fidelity::{
    async_sample::{self, AsyncMessage, AsyncSampleReport},
    compare_mock_spec, FidelityReport,
};
use mockforge_core::{Error, Result};
use mockforge_recorder::{
    models::{Protocol, RecordedExchange},
    RecorderDatabase,
};
use serde::Serialize;
use std::{collections::BTreeSet, path::Path};

mod fixtures;

#[derive(Serialize)]
struct Report {
    spec: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    definition: Option<FidelityReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sample: Option<AsyncSampleReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fixtures: Option<fixtures::Report>,
    exchanges_checked: usize,
    endpoints_with_drift: usize,
    endpoints_unverified: usize,
    findings: Vec<DriftFinding>,
}

pub(super) async fn handle(args: VerifyMocksArgs, source: Value) -> Result<()> {
    // Also validates a capture/fixture-only source before opening any database.
    compare_mock_spec(&source, &source).map_err(Error::config)?;
    let definition = args
        .mock_spec
        .as_ref()
        .map(|path| {
            compare_mock_spec(&load_fidelity_document(path)?, &source).map_err(Error::config)
        })
        .transpose()?;
    let sample = if let Some(path) = &args.capture_db {
        let records = capture(path, args.limit).await?;
        Some(check(&source, &records)?)
    } else {
        None
    };
    let fixtures = args
        .fixtures
        .as_deref()
        .map(|path| fixtures::verify(path, &source))
        .transpose()?;
    let mut findings = Vec::new();
    if let Some(definition) = &definition {
        for route in definition.routes.iter().filter(|r| r.status == "drift-detected") {
            findings.push(DriftFinding {
                endpoint: route.endpoint.clone(),
                kind: "definition".into(),
                detail: format!(
                    "{} definition; {} field delta(s)",
                    route.change,
                    route.deltas.len()
                ),
            });
        }
    }
    if let Some(sample) = &sample {
        collect_findings(sample, "sample", &mut findings);
    }
    if let Some(fixtures) = &fixtures {
        collect_findings(&fixtures.assessment, "fixture", &mut findings);
    }
    let drift: BTreeSet<_> = findings
        .iter()
        .filter(|f| !f.kind.ends_with("unverified"))
        .map(|f| &f.endpoint)
        .collect();
    let unverified: BTreeSet<_> = findings
        .iter()
        .filter(|f| f.kind.ends_with("unverified"))
        .map(|f| &f.endpoint)
        .collect();
    let report = Report {
        spec: args.spec.display().to_string(),
        exchanges_checked: sample.as_ref().map_or(0, |r| r.exchanges_checked),
        endpoints_with_drift: drift.len(),
        endpoints_unverified: unverified.len(),
        definition,
        sample,
        fixtures,
        findings,
    };
    println!("\n=== AsyncAPI mock fidelity / drift report ===");
    println!("messages checked: {}", report.exchanges_checked);
    println!("definitions/messages with drift: {}", report.endpoints_with_drift);
    println!("message routes not verified: {}", report.endpoints_unverified);
    for finding in &report.findings {
        println!("[{}] {}: {}", finding.kind, finding.endpoint, finding.detail);
    }
    if let Some(output) = &args.output {
        let json =
            serde_json::to_string_pretty(&report).map_err(|e| Error::config(e.to_string()))?;
        std::fs::write(output, json)
            .map_err(|e| Error::config(format!("cannot write {}: {e}", output.display())))?;
    }
    if args.fail_on_drift && !report.findings.is_empty() {
        eprintln!(
            "verify-mocks: AsyncAPI fidelity could not be established; failing (--fail-on-drift)"
        );
        std::process::exit(1);
    }
    Ok(())
}

fn collect_findings(report: &AsyncSampleReport, scope: &str, findings: &mut Vec<DriftFinding>) {
    for route in &report.routes {
        for (kind, details) in [
            (scope.to_string(), &route.findings),
            (format!("{scope}-unverified"), &route.notes),
        ] {
            for detail in details {
                findings.push(DriftFinding {
                    endpoint: route.endpoint.clone(),
                    kind: kind.clone(),
                    detail: detail.clone(),
                });
            }
        }
    }
}

fn check(source: &Value, records: &[RecordedExchange]) -> Result<AsyncSampleReport> {
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

async fn capture(path: &Path, limit: i32) -> Result<Vec<RecordedExchange>> {
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
