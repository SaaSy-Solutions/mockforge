//! Spec-level audits for the conformance self-test.
//!
//! Issue #79 round 17.4 — Srikanth's (17.4) ask: in addition to driving
//! a server with positive + negative *requests*, also audit the
//! OpenAPI document itself for things that will silently degrade
//! validator quality at runtime. These are not server-rejection
//! findings — they're spec-quality findings, surfaced before any
//! request is sent.
//!
//! Categories: `servers` (missing / localhost-only / relative-only),
//! `callbacks` (unsecured webhook operations), `polymorphism`
//! (`oneOf` / `anyOf` without a `discriminator`), `datatypes`
//! (coverage of every `(type, format)` combination in the spec).
//!
//! The audit is a pure function of `&openapiv3::OpenAPI`; no network
//! traffic, no server side-effects. Output ships alongside the
//! self-test JSON report.

use openapiv3::{
    OpenAPI, ReferenceOr, Schema, SchemaKind, StringFormat, Type, VariantOrUnknownOrEmpty,
};
use std::collections::BTreeMap;

/// Severity of a finding. Maps to traffic-light colours in the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational — not a defect, but useful coverage data.
    Info,
    /// Likely-degraded validator behaviour, but not a hard bug.
    Warning,
    /// Clear finding that the user should address.
    Error,
}

/// One audit finding.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SpecFinding {
    pub category: String,
    pub severity: Severity,
    /// JSON-pointer-ish location, e.g. `#/paths/~1users/post/callbacks/onCreated`.
    pub location: String,
    pub message: String,
}

/// Roll-up of all findings + the datatype coverage map.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct SpecAuditReport {
    pub findings: Vec<SpecFinding>,
    /// Per `(type, format)` count of how many schemas in the spec use it.
    /// Format `""` is used when no format is declared.
    pub datatype_coverage: BTreeMap<String, usize>,
    pub operations_audited: usize,
}

impl SpecAuditReport {
    /// Count of findings by severity. Useful for one-line summaries.
    pub fn counts_by_severity(&self) -> (usize, usize, usize) {
        let mut info = 0;
        let mut warn = 0;
        let mut err = 0;
        for f in &self.findings {
            match f.severity {
                Severity::Info => info += 1,
                Severity::Warning => warn += 1,
                Severity::Error => err += 1,
            }
        }
        (info, warn, err)
    }

    /// Human-readable single-paragraph summary.
    pub fn render_summary(&self) -> String {
        let (info, warn, err) = self.counts_by_severity();
        let coverage_kinds = self.datatype_coverage.len();
        format!(
            "Spec audit: {err} error(s), {warn} warning(s), {info} info; covered {coverage_kinds} datatype kind(s) across {} operation(s)",
            self.operations_audited
        )
    }
}

/// Walk the OpenAPI document and produce all findings + coverage.
/// Pure; no I/O.
pub fn audit_spec(spec: &OpenAPI) -> SpecAuditReport {
    let mut report = SpecAuditReport::default();
    audit_servers(spec, &mut report);
    audit_callbacks(spec, &mut report);
    audit_polymorphism_and_datatypes(spec, &mut report);
    report
}

fn audit_servers(spec: &OpenAPI, report: &mut SpecAuditReport) {
    if spec.servers.is_empty() {
        report.findings.push(SpecFinding {
            category: "servers".into(),
            severity: Severity::Warning,
            location: "#/servers".into(),
            message: "No `servers` declared — clients have to guess the base URL".into(),
        });
        return;
    }
    let mut all_localhost = true;
    let mut all_relative = true;
    for s in &spec.servers {
        let url = s.url.as_str();
        let is_local = url.contains("localhost") || url.contains("127.0.0.1");
        let is_rel = !url.starts_with("http://") && !url.starts_with("https://");
        if !is_local {
            all_localhost = false;
        }
        if !is_rel {
            all_relative = false;
        }
    }
    if all_localhost && !spec.servers.is_empty() {
        report.findings.push(SpecFinding {
            category: "servers".into(),
            severity: Severity::Warning,
            location: "#/servers".into(),
            message: format!(
                "All {} declared server(s) are localhost — production base URL missing",
                spec.servers.len()
            ),
        });
    }
    if all_relative && !spec.servers.is_empty() {
        report.findings.push(SpecFinding {
            category: "servers".into(),
            severity: Severity::Warning,
            location: "#/servers".into(),
            message: "All declared servers use relative URLs — clients must resolve against the spec's host".into(),
        });
    }
}

fn audit_callbacks(spec: &OpenAPI, report: &mut SpecAuditReport) {
    for (path, path_item_ref) in &spec.paths.paths {
        let path_item = match path_item_ref {
            ReferenceOr::Item(p) => p,
            ReferenceOr::Reference { .. } => continue,
        };
        for (method, op) in operations_of(path_item) {
            for (cb_name, cb) in &op.callbacks {
                // `Callback = IndexMap<String, PathItem>` — no ReferenceOr
                // on the value, so we walk directly.
                for (cb_path, cb_path_item) in cb {
                    for (cb_method, cb_op) in operations_of(cb_path_item) {
                        if cb_op.security.as_ref().is_none_or(|s| s.is_empty()) {
                            report.findings.push(SpecFinding {
                                category: "callbacks".into(),
                                severity: Severity::Warning,
                                location: format!(
                                    "#/paths/{}/{}/callbacks/{}/{}/{}",
                                    path, method, cb_name, cb_path, cb_method
                                ),
                                message: format!(
                                    "Callback `{}` on `{} {}` has no security requirement — webhook deliveries are unauthenticated",
                                    cb_name, method.to_uppercase(), path
                                ),
                            });
                        }
                    }
                }
            }
        }
    }
}

fn audit_polymorphism_and_datatypes(spec: &OpenAPI, report: &mut SpecAuditReport) {
    // Walk all schemas in components + every inline request/response
    // body schema. Single pass — we count datatype coverage and find
    // polymorphism findings at the same time.
    if let Some(components) = &spec.components {
        for (name, schema_ref) in &components.schemas {
            if let ReferenceOr::Item(schema) = schema_ref {
                walk_schema(schema, &format!("#/components/schemas/{}", name), report);
            }
        }
    }
    for (path, path_item_ref) in &spec.paths.paths {
        let path_item = match path_item_ref {
            ReferenceOr::Item(p) => p,
            ReferenceOr::Reference { .. } => continue,
        };
        report.operations_audited += operations_of(path_item).len();
        for (method, op) in operations_of(path_item) {
            if let Some(ReferenceOr::Item(rb)) = &op.request_body {
                for (ct, mt) in &rb.content {
                    if let Some(ReferenceOr::Item(schema)) = &mt.schema {
                        walk_schema(
                            schema,
                            &format!("#/paths/{}/{}/requestBody/{}", path, method, ct),
                            report,
                        );
                    }
                }
            }
            for (status, resp_ref) in &op.responses.responses {
                if let ReferenceOr::Item(resp) = resp_ref {
                    for (ct, mt) in &resp.content {
                        if let Some(ReferenceOr::Item(schema)) = &mt.schema {
                            walk_schema(
                                schema,
                                &format!(
                                    "#/paths/{}/{}/responses/{:?}/{}",
                                    path, method, status, ct
                                ),
                                report,
                            );
                        }
                    }
                }
            }
        }
    }
}

fn walk_schema(schema: &Schema, location: &str, report: &mut SpecAuditReport) {
    match &schema.schema_kind {
        SchemaKind::Type(t) => {
            count_datatype(t, &mut report.datatype_coverage);
            // Recurse into object properties + array items.
            match t {
                Type::Object(obj) => {
                    for (k, v) in &obj.properties {
                        if let ReferenceOr::Item(inner) = v {
                            walk_schema(inner, &format!("{}.{}", location, k), report);
                        }
                    }
                }
                Type::Array(arr) => {
                    if let Some(ReferenceOr::Item(inner)) = &arr.items {
                        walk_schema(inner, &format!("{}[]", location), report);
                    }
                }
                _ => {}
            }
        }
        SchemaKind::OneOf { one_of } | SchemaKind::AnyOf { any_of: one_of } => {
            let kind = if matches!(schema.schema_kind, SchemaKind::OneOf { .. }) {
                "oneOf"
            } else {
                "anyOf"
            };
            if schema.schema_data.discriminator.is_none() {
                report.findings.push(SpecFinding {
                    category: "polymorphism".into(),
                    severity: Severity::Warning,
                    location: location.to_string(),
                    message: format!(
                        "{} composition has no `discriminator` — validator cannot pick the variant deterministically",
                        kind
                    ),
                });
            }
            for (i, variant) in one_of.iter().enumerate() {
                if let ReferenceOr::Item(inner) = variant {
                    walk_schema(inner, &format!("{}/{}/{}", location, kind, i), report);
                }
            }
        }
        SchemaKind::AllOf { all_of } => {
            for (i, variant) in all_of.iter().enumerate() {
                if let ReferenceOr::Item(inner) = variant {
                    walk_schema(inner, &format!("{}/allOf/{}", location, i), report);
                }
            }
        }
        _ => {}
    }
}

fn count_datatype(t: &Type, coverage: &mut BTreeMap<String, usize>) {
    let key = match t {
        Type::String(s) => match &s.format {
            VariantOrUnknownOrEmpty::Item(StringFormat::Date) => "string:date".to_string(),
            VariantOrUnknownOrEmpty::Item(StringFormat::DateTime) => "string:date-time".to_string(),
            VariantOrUnknownOrEmpty::Item(StringFormat::Password) => "string:password".to_string(),
            VariantOrUnknownOrEmpty::Item(StringFormat::Byte) => "string:byte".to_string(),
            VariantOrUnknownOrEmpty::Item(StringFormat::Binary) => "string:binary".to_string(),
            VariantOrUnknownOrEmpty::Unknown(f) => format!("string:{}", f),
            VariantOrUnknownOrEmpty::Empty => "string".to_string(),
        },
        Type::Number(_) => "number".to_string(),
        Type::Integer(_) => "integer".to_string(),
        Type::Boolean(_) => "boolean".to_string(),
        Type::Object(_) => "object".to_string(),
        Type::Array(_) => "array".to_string(),
    };
    *coverage.entry(key).or_insert(0) += 1;
}

/// `(method_name, &Operation)` for each declared HTTP method on a path
/// item. Mirrors what `openapiv3::PathItem` exposes individually.
fn operations_of(p: &openapiv3::PathItem) -> Vec<(&'static str, &openapiv3::Operation)> {
    let mut out = Vec::new();
    if let Some(o) = &p.get {
        out.push(("get", o));
    }
    if let Some(o) = &p.post {
        out.push(("post", o));
    }
    if let Some(o) = &p.put {
        out.push(("put", o));
    }
    if let Some(o) = &p.patch {
        out.push(("patch", o));
    }
    if let Some(o) = &p.delete {
        out.push(("delete", o));
    }
    if let Some(o) = &p.head {
        out.push(("head", o));
    }
    if let Some(o) = &p.options {
        out.push(("options", o));
    }
    if let Some(o) = &p.trace {
        out.push(("trace", o));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use openapiv3::{ObjectType, SchemaData, Server};

    fn empty_spec() -> OpenAPI {
        OpenAPI {
            openapi: "3.0.0".into(),
            info: Default::default(),
            ..Default::default()
        }
    }

    #[test]
    fn no_servers_yields_servers_warning() {
        let spec = empty_spec();
        let report = audit_spec(&spec);
        assert!(report
            .findings
            .iter()
            .any(|f| f.category == "servers" && f.severity == Severity::Warning));
    }

    #[test]
    fn localhost_only_servers_warn() {
        let mut spec = empty_spec();
        spec.servers = vec![
            Server {
                url: "http://localhost:3000".into(),
                ..Default::default()
            },
            Server {
                url: "http://127.0.0.1:8080".into(),
                ..Default::default()
            },
        ];
        let report = audit_spec(&spec);
        assert!(report
            .findings
            .iter()
            .any(|f| f.category == "servers" && f.message.contains("localhost")));
    }

    #[test]
    fn relative_only_servers_warn() {
        let mut spec = empty_spec();
        spec.servers = vec![Server {
            url: "/v1".into(),
            ..Default::default()
        }];
        let report = audit_spec(&spec);
        assert!(report
            .findings
            .iter()
            .any(|f| f.category == "servers" && f.message.contains("relative URLs")));
    }

    #[test]
    fn production_servers_no_warning() {
        let mut spec = empty_spec();
        spec.servers = vec![Server {
            url: "https://api.example.com".into(),
            ..Default::default()
        }];
        let report = audit_spec(&spec);
        assert!(!report.findings.iter().any(|f| f.category == "servers"));
    }

    #[test]
    fn datatype_coverage_records_string_format() {
        use openapiv3::{Components, StringType};
        let mut spec = empty_spec();
        let mut components = Components::default();
        let mut email_schema = Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::Type(Type::String(StringType {
                format: VariantOrUnknownOrEmpty::Unknown("email".into()),
                ..Default::default()
            })),
        };
        // Reuse the same shape with no format for a second schema.
        components
            .schemas
            .insert("Email".into(), ReferenceOr::Item(email_schema.clone()));
        email_schema.schema_kind = SchemaKind::Type(Type::String(Default::default()));
        components.schemas.insert("Plain".into(), ReferenceOr::Item(email_schema));
        spec.components = Some(components);
        let report = audit_spec(&spec);
        assert_eq!(report.datatype_coverage.get("string:email"), Some(&1));
        assert_eq!(report.datatype_coverage.get("string"), Some(&1));
    }

    #[test]
    fn oneof_without_discriminator_flags_polymorphism() {
        use openapiv3::Components;
        let mut spec = empty_spec();
        let mut components = Components::default();
        let one_of_schema = Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::OneOf {
                one_of: vec![
                    ReferenceOr::Item(Schema {
                        schema_data: SchemaData::default(),
                        schema_kind: SchemaKind::Type(Type::Object(ObjectType::default())),
                    }),
                    ReferenceOr::Item(Schema {
                        schema_data: SchemaData::default(),
                        schema_kind: SchemaKind::Type(Type::Object(ObjectType::default())),
                    }),
                ],
            },
        };
        components.schemas.insert("Shape".into(), ReferenceOr::Item(one_of_schema));
        spec.components = Some(components);
        let report = audit_spec(&spec);
        assert!(report
            .findings
            .iter()
            .any(|f| f.category == "polymorphism" && f.message.contains("oneOf")));
    }

    #[test]
    fn summary_counts_severities() {
        let report = SpecAuditReport {
            findings: vec![
                SpecFinding {
                    category: "servers".into(),
                    severity: Severity::Warning,
                    location: "#/servers".into(),
                    message: "x".into(),
                },
                SpecFinding {
                    category: "callbacks".into(),
                    severity: Severity::Error,
                    location: "#/x".into(),
                    message: "y".into(),
                },
            ],
            datatype_coverage: BTreeMap::from([("string".into(), 5)]),
            operations_audited: 3,
        };
        let (info, warn, err) = report.counts_by_severity();
        assert_eq!((info, warn, err), (0, 1, 1));
        let s = report.render_summary();
        assert!(s.contains("1 error"));
        assert!(s.contains("1 warning"));
        assert!(s.contains("3 operation"));
    }
}
