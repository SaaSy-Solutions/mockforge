//! Assemble deterministic CLI results and apply the explicit CI gate.
use super::{fixtures, DriftFinding};
use mockforge_contracts::fidelity::{async_sample::AsyncSampleReport, FidelityReport};
use mockforge_core::{Error, Result};
use serde::Serialize;
use std::{collections::BTreeSet, path::Path};

#[derive(Serialize)]
pub(super) struct Report {
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

impl Report {
    pub(super) fn new(
        spec: String,
        definition: Option<FidelityReport>,
        sample: Option<AsyncSampleReport>,
        fixtures: Option<fixtures::Report>,
    ) -> Self {
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
        Self {
            spec,
            exchanges_checked: sample.as_ref().map_or(0, |r| r.exchanges_checked),
            endpoints_with_drift: drift.len(),
            endpoints_unverified: unverified.len(),
            definition,
            sample,
            fixtures,
            findings,
        }
    }
    pub(super) fn emit(&self, output: Option<&Path>, fail_on_drift: bool) -> Result<()> {
        println!("\n=== AsyncAPI mock fidelity / drift report ===");
        println!("messages checked: {}", self.exchanges_checked);
        println!("definitions/messages with drift: {}", self.endpoints_with_drift);
        println!("message routes not verified: {}", self.endpoints_unverified);
        for finding in &self.findings {
            println!("[{}] {}: {}", finding.kind, finding.endpoint, finding.detail);
        }
        if let Some(output) = &output {
            let json =
                serde_json::to_string_pretty(self).map_err(|e| Error::config(e.to_string()))?;
            std::fs::write(output, json)
                .map_err(|e| Error::config(format!("cannot write {}: {e}", output.display())))?;
        }
        if fail_on_drift && !self.findings.is_empty() {
            eprintln!(
            "verify-mocks: AsyncAPI fidelity could not be established; failing (--fail-on-drift)"
        );
            std::process::exit(1);
        }
        Ok(())
    }
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
