//! `mockforge validate-spec` — validate one local OpenAPI document, offline.
//!
//! `generate` and `serve` already validate a spec before they use it, but
//! only as a side effect, and only as text. Tools that drive MockForge from
//! outside (editors, CI, agent hosts such as Aether) need the same check on
//! its own, with findings they can read: `--format json` prints one
//! versioned document ([`VALIDATE_SPEC_SCHEMA_VERSION`]). Only a local file is
//! read; nothing is fetched, written or started.
//!
//! Exit codes: 0 when the spec is valid (warnings allowed), 1 when it has
//! errors, 2 when it could not be read as an OpenAPI document at all. The
//! JSON document is printed in every case.

use mockforge_openapi::spec_parser::{OpenApiValidator, SpecFormat};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The version of the JSON document `validate-spec --format json` prints.
pub(crate) const VALIDATE_SPEC_SCHEMA_VERSION: u32 = 1;
/// The largest spec read; a larger file is reported, not read.
const MAX_SPEC_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum ValidateSpecFormat {
    Text,
    Json,
}

/// Validate one local OpenAPI document without changing anything.
#[derive(clap::Args, Debug)]
pub(crate) struct ValidateSpecArgs {
    /// The OpenAPI 2.0 or 3.x document (JSON or YAML). A local file only:
    /// URLs are not fetched.
    pub spec: PathBuf,

    /// Output format.
    #[arg(long, value_enum, default_value = "text")]
    pub format: ValidateSpecFormat,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) enum SpecVersion {
    #[serde(rename = "2.0")]
    Openapi20,
    #[serde(rename = "3.0")]
    Openapi30,
    #[serde(rename = "3.1")]
    Openapi31,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FindingSeverity {
    Error,
    Warning,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct SpecFinding {
    pub severity: FindingSeverity,
    /// The validator's code, e.g. `MISSING_FIELD`; `UNREADABLE`,
    /// `INVALID_SYNTAX` or `NOT_OPENAPI` when the document could not be
    /// validated at all; `WARNING` for a warning without a code.
    pub rule: String,
    /// JSON pointer to the field, when the validator names one.
    pub path: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct SpecValidation {
    pub schema_version: u32,
    /// The version of this `mockforge` binary.
    pub mockforge_version: String,
    /// The spec as it was named on the command line.
    pub spec: String,
    /// The OpenAPI version detected; null when the document is not one.
    pub openapi: Option<SpecVersion>,
    /// True when the document was validated and has no errors.
    pub valid: bool,
    pub errors: u32,
    pub warnings: u32,
    pub findings: Vec<SpecFinding>,
}

impl SpecValidation {
    fn new(spec: &Path) -> Self {
        Self {
            schema_version: VALIDATE_SPEC_SCHEMA_VERSION,
            mockforge_version: env!("CARGO_PKG_VERSION").to_string(),
            spec: spec.display().to_string(),
            openapi: None,
            valid: false,
            errors: 0,
            warnings: 0,
            findings: Vec::new(),
        }
    }

    /// A document that could not be validated at all.
    fn unvalidated(mut self, rule: &str, message: String) -> Self {
        self.findings.push(SpecFinding {
            severity: FindingSeverity::Error,
            rule: rule.into(),
            path: None,
            message,
            suggestion: None,
        });
        self.errors = 1;
        self
    }

    /// The exit code: 0 valid, 1 invalid, 2 not validated.
    pub(crate) fn exit_code(&self) -> i32 {
        match (self.valid, self.openapi) {
            (true, _) => 0,
            (false, Some(_)) => 1,
            (false, None) => 2,
        }
    }
}

/// Validate `spec`, reading it and nothing else.
pub(crate) fn validate_spec(spec: &Path) -> SpecValidation {
    let report = SpecValidation::new(spec);
    let text = spec.to_string_lossy();
    if text.starts_with("http://") || text.starts_with("https://") {
        return report.unvalidated(
            "UNREADABLE",
            "validate-spec reads a local file only; download the spec first".into(),
        );
    }
    match std::fs::metadata(spec) {
        Ok(meta) if meta.len() > MAX_SPEC_BYTES => {
            return report.unvalidated(
                "UNREADABLE",
                format!("the spec is larger than {MAX_SPEC_BYTES} bytes"),
            );
        }
        Ok(meta) if !meta.is_file() => {
            return report.unvalidated("UNREADABLE", "the spec is not a file".into());
        }
        Ok(_) => {}
        Err(e) => return report.unvalidated("UNREADABLE", format!("cannot read the spec: {e}")),
    }
    let content = match std::fs::read_to_string(spec) {
        Ok(content) => content,
        Err(e) => return report.unvalidated("UNREADABLE", format!("cannot read the spec: {e}")),
    };
    let document: serde_json::Value = match serde_json::from_str(&content) {
        Ok(value) => value,
        Err(_) => match serde_yaml::from_str(&content) {
            Ok(value) => value,
            Err(e) => {
                return report.unvalidated("INVALID_SYNTAX", format!("neither JSON nor YAML: {e}"));
            }
        },
    };
    let (format, version) = match SpecFormat::detect(&content, Some(spec)) {
        Ok(SpecFormat::OpenApi20) => (SpecFormat::OpenApi20, SpecVersion::Openapi20),
        Ok(SpecFormat::OpenApi30) => (SpecFormat::OpenApi30, SpecVersion::Openapi30),
        Ok(SpecFormat::OpenApi31) => (SpecFormat::OpenApi31, SpecVersion::Openapi31),
        _ => {
            return report.unvalidated(
                "NOT_OPENAPI",
                "not an OpenAPI 2.0 or 3.x document (no `swagger: 2.x` or `openapi: 3.x`)".into(),
            );
        }
    };
    let result = OpenApiValidator::validate(&document, format);
    let mut report = report;
    report.openapi = Some(version);
    report.findings = result
        .errors
        .iter()
        .map(|e| SpecFinding {
            severity: FindingSeverity::Error,
            rule: e.code.clone().unwrap_or_else(|| "VALIDATION".into()),
            path: e.path.clone(),
            message: e.message.clone(),
            suggestion: e.suggestion.clone(),
        })
        .chain(result.warnings.iter().map(|w| SpecFinding {
            severity: FindingSeverity::Warning,
            rule: "WARNING".into(),
            path: None,
            message: w.clone(),
            suggestion: None,
        }))
        .collect();
    report.errors = u32::try_from(result.errors.len()).unwrap_or(u32::MAX);
    report.warnings = u32::try_from(result.warnings.len()).unwrap_or(u32::MAX);
    report.valid = result.errors.is_empty();
    report
}

fn print_text(report: &SpecValidation) {
    let verdict = match report.exit_code() {
        0 => "valid",
        1 => "invalid",
        _ => "not validated",
    };
    println!(
        "{} · {verdict} · {} error(s), {} warning(s)",
        report.spec, report.errors, report.warnings
    );
    for finding in &report.findings {
        let severity = match finding.severity {
            FindingSeverity::Error => "error",
            FindingSeverity::Warning => "warning",
        };
        let at = finding.path.as_deref().map(|p| format!(" (at {p})")).unwrap_or_default();
        println!("  {severity} {}: {}{at}", finding.rule, finding.message);
        if let Some(suggestion) = &finding.suggestion {
            println!("    hint: {suggestion}");
        }
    }
}

pub(crate) async fn handle_validate_spec_command(
    args: ValidateSpecArgs,
) -> mockforge_core::Result<()> {
    let spec = args.spec.clone();
    let report = tokio::task::spawn_blocking(move || validate_spec(&spec))
        .await
        .map_err(|e| mockforge_core::Error::config(e.to_string()))?;
    match args.format {
        ValidateSpecFormat::Json => {
            let json = serde_json::to_string_pretty(&report)
                .map_err(|e| mockforge_core::Error::config(e.to_string()))?;
            println!("{json}");
        }
        ValidateSpecFormat::Text => print_text(&report),
    }
    match report.exit_code() {
        0 => Ok(()),
        code => std::process::exit(code),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &tempfile::TempDir, name: &str, body: &str) -> PathBuf {
        let path = dir.path().join(name);
        std::fs::write(&path, body).unwrap();
        path
    }

    const VALID: &str = r#"openapi: 3.0.3
info:
  title: Users
  version: "1.0"
paths:
  /users:
    get:
      responses:
        "200":
          description: ok
"#;

    #[test]
    fn a_valid_yaml_spec_reads_valid_and_versioned() {
        let dir = tempfile::tempdir().unwrap();
        let spec = write(&dir, "api.yaml", VALID);
        let report = validate_spec(&spec);
        assert_eq!(report.schema_version, 1);
        assert_eq!(report.openapi, Some(SpecVersion::Openapi30));
        assert!(report.valid, "{report:?}");
        assert_eq!((report.errors, report.exit_code()), (0, 0));
        assert!(report.findings.iter().all(|f| f.severity == FindingSeverity::Warning));
        // Nothing was written beside it.
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn a_spec_missing_required_fields_lists_each_error_with_its_rule() {
        let dir = tempfile::tempdir().unwrap();
        let spec = write(&dir, "api.json", r#"{"openapi":"3.1.0","paths":{}}"#);
        let report = validate_spec(&spec);
        assert_eq!(report.openapi, Some(SpecVersion::Openapi31));
        assert!(!report.valid);
        assert_eq!(report.exit_code(), 1);
        assert!(report.errors >= 1, "{report:?}");
        assert_eq!(
            report.errors as usize,
            report.findings.iter().filter(|f| f.severity == FindingSeverity::Error).count()
        );
        // The missing `info` is named, with where it belongs.
        let missing = report
            .findings
            .iter()
            .find(|f| f.rule == "MISSING_INFO")
            .unwrap_or_else(|| panic!("no MISSING_INFO in {report:?}"));
        assert_eq!(missing.severity, FindingSeverity::Error);
        assert_eq!(missing.path.as_deref(), Some("/info"));
        assert!(missing.suggestion.is_some());
        assert!(report.findings.iter().all(|f| !f.rule.is_empty()));
    }

    #[test]
    fn what_cannot_be_validated_says_why_and_exits_two() {
        let dir = tempfile::tempdir().unwrap();
        let cases = [
            (dir.path().join("missing.yaml"), "UNREADABLE"),
            (write(&dir, "broken.json", "{\"openapi\": "), "INVALID_SYNTAX"),
            (write(&dir, "other.yaml", "name: not an api\n"), "NOT_OPENAPI"),
            (PathBuf::from("https://example.com/openapi.json"), "UNREADABLE"),
        ];
        for (spec, rule) in cases {
            let report = validate_spec(&spec);
            assert!(!report.valid);
            assert_eq!(report.openapi, None);
            assert_eq!(report.exit_code(), 2, "{report:?}");
            assert_eq!(report.findings[0].rule, rule, "{report:?}");
        }
        // A directory is not a spec.
        assert_eq!(validate_spec(dir.path()).findings[0].rule, "UNREADABLE");
    }

    #[test]
    fn the_json_document_keeps_its_field_names() {
        let dir = tempfile::tempdir().unwrap();
        let spec = write(&dir, "api.yaml", VALID);
        let value = serde_json::to_value(validate_spec(&spec)).unwrap();
        for field in [
            "schema_version",
            "mockforge_version",
            "spec",
            "openapi",
            "valid",
            "errors",
            "warnings",
            "findings",
        ] {
            assert!(value.get(field).is_some(), "{field} missing from {value}");
        }
        assert_eq!(value["openapi"], "3.0");
    }
}
