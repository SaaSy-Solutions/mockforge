//! Spec-driven conformance testing
//!
//! Analyzes the user's OpenAPI spec to determine which features their API uses,
//! then generates k6 conformance tests against their real endpoints.

use super::generator::ConformanceConfig;
use super::schema_validator::SchemaValidatorGenerator;
use super::spec::ConformanceFeature;
use crate::error::Result;
use crate::request_gen::RequestGenerator;
use crate::spec_parser::ApiOperation;
use openapiv3::{
    OpenAPI, Operation, Parameter, ParameterSchemaOrContent, ReferenceOr, RequestBody, Response,
    Schema, SchemaKind, SecurityScheme, StringFormat, Type, VariantOrUnknownOrEmpty,
};
use std::collections::HashSet;

/// Resolve `$ref` references against the OpenAPI components
mod ref_resolver {
    use super::*;

    pub fn resolve_parameter<'a>(
        param_ref: &'a ReferenceOr<Parameter>,
        spec: &'a OpenAPI,
    ) -> Option<&'a Parameter> {
        match param_ref {
            ReferenceOr::Item(param) => Some(param),
            ReferenceOr::Reference { reference } => {
                let name = reference.strip_prefix("#/components/parameters/")?;
                let components = spec.components.as_ref()?;
                match components.parameters.get(name)? {
                    ReferenceOr::Item(param) => Some(param),
                    ReferenceOr::Reference { .. } => None, // No recursive ref chasing
                }
            }
        }
    }

    pub fn resolve_request_body<'a>(
        body_ref: &'a ReferenceOr<RequestBody>,
        spec: &'a OpenAPI,
    ) -> Option<&'a RequestBody> {
        match body_ref {
            ReferenceOr::Item(body) => Some(body),
            ReferenceOr::Reference { reference } => {
                let name = reference.strip_prefix("#/components/requestBodies/")?;
                let components = spec.components.as_ref()?;
                match components.request_bodies.get(name)? {
                    ReferenceOr::Item(body) => Some(body),
                    ReferenceOr::Reference { .. } => None,
                }
            }
        }
    }

    pub fn resolve_schema<'a>(
        schema_ref: &'a ReferenceOr<Schema>,
        spec: &'a OpenAPI,
    ) -> Option<&'a Schema> {
        resolve_schema_with_visited(schema_ref, spec, &mut HashSet::new())
    }

    fn resolve_schema_with_visited<'a>(
        schema_ref: &'a ReferenceOr<Schema>,
        spec: &'a OpenAPI,
        visited: &mut HashSet<String>,
    ) -> Option<&'a Schema> {
        match schema_ref {
            ReferenceOr::Item(schema) => Some(schema),
            ReferenceOr::Reference { reference } => {
                if !visited.insert(reference.clone()) {
                    return None; // Cycle detected
                }
                let name = reference.strip_prefix("#/components/schemas/")?;
                let components = spec.components.as_ref()?;
                let nested = components.schemas.get(name)?;
                resolve_schema_with_visited(nested, spec, visited)
            }
        }
    }

    /// Resolve a boxed schema reference (used by array items and object properties)
    pub fn resolve_boxed_schema<'a>(
        schema_ref: &'a ReferenceOr<Box<Schema>>,
        spec: &'a OpenAPI,
    ) -> Option<&'a Schema> {
        match schema_ref {
            ReferenceOr::Item(schema) => Some(schema.as_ref()),
            ReferenceOr::Reference { reference } => {
                // Delegate to the regular schema resolver
                let name = reference.strip_prefix("#/components/schemas/")?;
                let components = spec.components.as_ref()?;
                let nested = components.schemas.get(name)?;
                resolve_schema_with_visited(nested, spec, &mut HashSet::new())
            }
        }
    }

    pub fn resolve_response<'a>(
        resp_ref: &'a ReferenceOr<Response>,
        spec: &'a OpenAPI,
    ) -> Option<&'a Response> {
        match resp_ref {
            ReferenceOr::Item(resp) => Some(resp),
            ReferenceOr::Reference { reference } => {
                let name = reference.strip_prefix("#/components/responses/")?;
                let components = spec.components.as_ref()?;
                match components.responses.get(name)? {
                    ReferenceOr::Item(resp) => Some(resp),
                    ReferenceOr::Reference { .. } => None,
                }
            }
        }
    }
}

/// An API operation annotated with the conformance features it exercises
#[derive(Debug, Clone)]
pub struct AnnotatedOperation {
    pub path: String,
    pub method: String,
    pub features: Vec<ConformanceFeature>,
    pub request_body_content_type: Option<String>,
    pub sample_body: Option<String>,
    pub query_params: Vec<(String, String)>,
    pub header_params: Vec<(String, String)>,
    pub path_params: Vec<(String, String)>,
    /// Response schema for validation (JSON string of the schema)
    pub response_schema: Option<Schema>,
}

/// Generates spec-driven conformance k6 scripts
pub struct SpecDrivenConformanceGenerator {
    config: ConformanceConfig,
    operations: Vec<AnnotatedOperation>,
}

impl SpecDrivenConformanceGenerator {
    pub fn new(config: ConformanceConfig, operations: Vec<AnnotatedOperation>) -> Self {
        Self { config, operations }
    }

    /// Annotate a list of API operations with conformance features
    pub fn annotate_operations(
        operations: &[ApiOperation],
        spec: &OpenAPI,
    ) -> Vec<AnnotatedOperation> {
        operations.iter().map(|op| Self::annotate_operation(op, spec)).collect()
    }

    /// Analyze an operation and determine which conformance features it exercises
    fn annotate_operation(op: &ApiOperation, spec: &OpenAPI) -> AnnotatedOperation {
        let mut features = Vec::new();
        let mut query_params = Vec::new();
        let mut header_params = Vec::new();
        let mut path_params = Vec::new();

        // Detect HTTP method feature
        match op.method.to_uppercase().as_str() {
            "GET" => features.push(ConformanceFeature::MethodGet),
            "POST" => features.push(ConformanceFeature::MethodPost),
            "PUT" => features.push(ConformanceFeature::MethodPut),
            "PATCH" => features.push(ConformanceFeature::MethodPatch),
            "DELETE" => features.push(ConformanceFeature::MethodDelete),
            "HEAD" => features.push(ConformanceFeature::MethodHead),
            "OPTIONS" => features.push(ConformanceFeature::MethodOptions),
            _ => {}
        }

        // Detect parameter features (resolves $ref)
        for param_ref in &op.operation.parameters {
            if let Some(param) = ref_resolver::resolve_parameter(param_ref, spec) {
                Self::annotate_parameter(
                    param,
                    spec,
                    &mut features,
                    &mut query_params,
                    &mut header_params,
                    &mut path_params,
                );
            }
        }

        // Detect path parameters from the path template itself
        for segment in op.path.split('/') {
            if segment.starts_with('{') && segment.ends_with('}') {
                let name = &segment[1..segment.len() - 1];
                // Only add if not already found from parameters
                if !path_params.iter().any(|(n, _)| n == name) {
                    path_params.push((name.to_string(), "test-value".to_string()));
                    // Determine type from path params we didn't already handle
                    if !features.contains(&ConformanceFeature::PathParamString)
                        && !features.contains(&ConformanceFeature::PathParamInteger)
                    {
                        features.push(ConformanceFeature::PathParamString);
                    }
                }
            }
        }

        // Detect request body features (resolves $ref)
        let mut request_body_content_type = None;
        let mut sample_body = None;

        let resolved_body = op
            .operation
            .request_body
            .as_ref()
            .and_then(|b| ref_resolver::resolve_request_body(b, spec));

        if let Some(body) = resolved_body {
            for (content_type, _media) in &body.content {
                match content_type.as_str() {
                    "application/json" => {
                        features.push(ConformanceFeature::BodyJson);
                        request_body_content_type = Some("application/json".to_string());
                        // Generate sample body from schema
                        if let Ok(template) = RequestGenerator::generate_template(op) {
                            if let Some(body_val) = &template.body {
                                sample_body = Some(body_val.to_string());
                            }
                        }
                    }
                    "application/x-www-form-urlencoded" => {
                        features.push(ConformanceFeature::BodyFormUrlencoded);
                        request_body_content_type =
                            Some("application/x-www-form-urlencoded".to_string());
                    }
                    "multipart/form-data" => {
                        features.push(ConformanceFeature::BodyMultipart);
                        request_body_content_type = Some("multipart/form-data".to_string());
                    }
                    _ => {}
                }
            }

            // Detect schema features in request body (resolves $ref in schema)
            if let Some(media) = body.content.get("application/json") {
                if let Some(schema_ref) = &media.schema {
                    if let Some(schema) = ref_resolver::resolve_schema(schema_ref, spec) {
                        Self::annotate_schema(schema, spec, &mut features);
                    }
                }
            }
        }

        // Detect response code features
        Self::annotate_responses(&op.operation, spec, &mut features);

        // Extract response schema for validation (resolves $ref)
        let response_schema = Self::extract_response_schema(&op.operation, spec);
        if response_schema.is_some() {
            features.push(ConformanceFeature::ResponseValidation);
        }

        // Detect content negotiation (response with multiple content types)
        Self::annotate_content_negotiation(&op.operation, spec, &mut features);

        // Detect security features
        Self::annotate_security(&op.operation, spec, &mut features);

        // Deduplicate features
        features.sort_by_key(|f| f.check_name());
        features.dedup_by_key(|f| f.check_name());

        AnnotatedOperation {
            path: op.path.clone(),
            method: op.method.to_uppercase(),
            features,
            request_body_content_type,
            sample_body,
            query_params,
            header_params,
            path_params,
            response_schema,
        }
    }

    /// Annotate parameter features
    fn annotate_parameter(
        param: &Parameter,
        spec: &OpenAPI,
        features: &mut Vec<ConformanceFeature>,
        query_params: &mut Vec<(String, String)>,
        header_params: &mut Vec<(String, String)>,
        path_params: &mut Vec<(String, String)>,
    ) {
        let (location, data) = match param {
            Parameter::Query { parameter_data, .. } => ("query", parameter_data),
            Parameter::Path { parameter_data, .. } => ("path", parameter_data),
            Parameter::Header { parameter_data, .. } => ("header", parameter_data),
            Parameter::Cookie { .. } => {
                features.push(ConformanceFeature::CookieParam);
                return;
            }
        };

        // Detect type from schema
        let is_integer = Self::param_schema_is_integer(data, spec);
        let is_array = Self::param_schema_is_array(data, spec);

        // Generate sample value
        let sample = if is_integer {
            "42".to_string()
        } else if is_array {
            "a,b".to_string()
        } else {
            "test-value".to_string()
        };

        match location {
            "path" => {
                if is_integer {
                    features.push(ConformanceFeature::PathParamInteger);
                } else {
                    features.push(ConformanceFeature::PathParamString);
                }
                path_params.push((data.name.clone(), sample));
            }
            "query" => {
                if is_array {
                    features.push(ConformanceFeature::QueryParamArray);
                } else if is_integer {
                    features.push(ConformanceFeature::QueryParamInteger);
                } else {
                    features.push(ConformanceFeature::QueryParamString);
                }
                query_params.push((data.name.clone(), sample));
            }
            "header" => {
                features.push(ConformanceFeature::HeaderParam);
                header_params.push((data.name.clone(), sample));
            }
            _ => {}
        }

        // Check for constraint features on the parameter (resolves $ref)
        if let ParameterSchemaOrContent::Schema(schema_ref) = &data.format {
            if let Some(schema) = ref_resolver::resolve_schema(schema_ref, spec) {
                Self::annotate_schema(schema, spec, features);
            }
        }

        // Required/optional
        if data.required {
            features.push(ConformanceFeature::ConstraintRequired);
        } else {
            features.push(ConformanceFeature::ConstraintOptional);
        }
    }

    fn param_schema_is_integer(data: &openapiv3::ParameterData, spec: &OpenAPI) -> bool {
        if let ParameterSchemaOrContent::Schema(schema_ref) = &data.format {
            if let Some(schema) = ref_resolver::resolve_schema(schema_ref, spec) {
                return matches!(&schema.schema_kind, SchemaKind::Type(Type::Integer(_)));
            }
        }
        false
    }

    fn param_schema_is_array(data: &openapiv3::ParameterData, spec: &OpenAPI) -> bool {
        if let ParameterSchemaOrContent::Schema(schema_ref) = &data.format {
            if let Some(schema) = ref_resolver::resolve_schema(schema_ref, spec) {
                return matches!(&schema.schema_kind, SchemaKind::Type(Type::Array(_)));
            }
        }
        false
    }

    /// Annotate schema-level features (types, composition, formats, constraints)
    fn annotate_schema(schema: &Schema, spec: &OpenAPI, features: &mut Vec<ConformanceFeature>) {
        match &schema.schema_kind {
            SchemaKind::Type(Type::String(s)) => {
                features.push(ConformanceFeature::SchemaString);
                // Check format
                match &s.format {
                    VariantOrUnknownOrEmpty::Item(StringFormat::Date) => {
                        features.push(ConformanceFeature::FormatDate);
                    }
                    VariantOrUnknownOrEmpty::Item(StringFormat::DateTime) => {
                        features.push(ConformanceFeature::FormatDateTime);
                    }
                    VariantOrUnknownOrEmpty::Unknown(fmt) => match fmt.as_str() {
                        "email" => features.push(ConformanceFeature::FormatEmail),
                        "uuid" => features.push(ConformanceFeature::FormatUuid),
                        "uri" | "url" => features.push(ConformanceFeature::FormatUri),
                        "ipv4" => features.push(ConformanceFeature::FormatIpv4),
                        "ipv6" => features.push(ConformanceFeature::FormatIpv6),
                        _ => {}
                    },
                    _ => {}
                }
                // Check constraints
                if s.pattern.is_some() {
                    features.push(ConformanceFeature::ConstraintPattern);
                }
                if !s.enumeration.is_empty() {
                    features.push(ConformanceFeature::ConstraintEnum);
                }
                if s.min_length.is_some() || s.max_length.is_some() {
                    features.push(ConformanceFeature::ConstraintMinMax);
                }
            }
            SchemaKind::Type(Type::Integer(i)) => {
                features.push(ConformanceFeature::SchemaInteger);
                if i.minimum.is_some() || i.maximum.is_some() {
                    features.push(ConformanceFeature::ConstraintMinMax);
                }
                if !i.enumeration.is_empty() {
                    features.push(ConformanceFeature::ConstraintEnum);
                }
            }
            SchemaKind::Type(Type::Number(n)) => {
                features.push(ConformanceFeature::SchemaNumber);
                if n.minimum.is_some() || n.maximum.is_some() {
                    features.push(ConformanceFeature::ConstraintMinMax);
                }
            }
            SchemaKind::Type(Type::Boolean(_)) => {
                features.push(ConformanceFeature::SchemaBoolean);
            }
            SchemaKind::Type(Type::Array(arr)) => {
                features.push(ConformanceFeature::SchemaArray);
                if let Some(item_ref) = &arr.items {
                    if let Some(item_schema) = ref_resolver::resolve_boxed_schema(item_ref, spec) {
                        Self::annotate_schema(item_schema, spec, features);
                    }
                }
            }
            SchemaKind::Type(Type::Object(obj)) => {
                features.push(ConformanceFeature::SchemaObject);
                // Check required fields
                if !obj.required.is_empty() {
                    features.push(ConformanceFeature::ConstraintRequired);
                }
                // Walk properties (resolves $ref)
                for (_name, prop_ref) in &obj.properties {
                    if let Some(prop_schema) = ref_resolver::resolve_boxed_schema(prop_ref, spec) {
                        Self::annotate_schema(prop_schema, spec, features);
                    }
                }
            }
            SchemaKind::OneOf { .. } => {
                features.push(ConformanceFeature::CompositionOneOf);
            }
            SchemaKind::AnyOf { .. } => {
                features.push(ConformanceFeature::CompositionAnyOf);
            }
            SchemaKind::AllOf { .. } => {
                features.push(ConformanceFeature::CompositionAllOf);
            }
            _ => {}
        }
    }

    /// Detect response code features (resolves $ref in responses)
    fn annotate_responses(
        operation: &Operation,
        spec: &OpenAPI,
        features: &mut Vec<ConformanceFeature>,
    ) {
        for (status_code, resp_ref) in &operation.responses.responses {
            // Only count features for responses that actually resolve
            if ref_resolver::resolve_response(resp_ref, spec).is_some() {
                match status_code {
                    openapiv3::StatusCode::Code(200) => {
                        features.push(ConformanceFeature::Response200)
                    }
                    openapiv3::StatusCode::Code(201) => {
                        features.push(ConformanceFeature::Response201)
                    }
                    openapiv3::StatusCode::Code(204) => {
                        features.push(ConformanceFeature::Response204)
                    }
                    openapiv3::StatusCode::Code(400) => {
                        features.push(ConformanceFeature::Response400)
                    }
                    openapiv3::StatusCode::Code(404) => {
                        features.push(ConformanceFeature::Response404)
                    }
                    _ => {}
                }
            }
        }
    }

    /// Extract the response schema for the primary success response (200 or 201)
    /// Resolves $ref for both the response and the schema within it.
    fn extract_response_schema(operation: &Operation, spec: &OpenAPI) -> Option<Schema> {
        // Try 200 first, then 201
        for code in [200u16, 201] {
            if let Some(resp_ref) =
                operation.responses.responses.get(&openapiv3::StatusCode::Code(code))
            {
                if let Some(response) = ref_resolver::resolve_response(resp_ref, spec) {
                    if let Some(media) = response.content.get("application/json") {
                        if let Some(schema_ref) = &media.schema {
                            if let Some(schema) = ref_resolver::resolve_schema(schema_ref, spec) {
                                return Some(schema.clone());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Detect content negotiation: response supports multiple content types
    fn annotate_content_negotiation(
        operation: &Operation,
        spec: &OpenAPI,
        features: &mut Vec<ConformanceFeature>,
    ) {
        for (_status_code, resp_ref) in &operation.responses.responses {
            if let Some(response) = ref_resolver::resolve_response(resp_ref, spec) {
                if response.content.len() > 1 {
                    features.push(ConformanceFeature::ContentNegotiation);
                    return; // Only need to detect once per operation
                }
            }
        }
    }

    /// Detect security scheme features.
    /// Checks operation-level security first, falling back to global security requirements.
    /// Resolves scheme names against SecurityScheme definitions in components.
    fn annotate_security(
        operation: &Operation,
        spec: &OpenAPI,
        features: &mut Vec<ConformanceFeature>,
    ) {
        // Use operation-level security if present, otherwise fall back to global
        let security_reqs = operation.security.as_ref().or(spec.security.as_ref());

        if let Some(security) = security_reqs {
            for security_req in security {
                for scheme_name in security_req.keys() {
                    // Try to resolve the scheme from components for accurate type detection
                    if let Some(resolved) = Self::resolve_security_scheme(scheme_name, spec) {
                        match resolved {
                            SecurityScheme::HTTP { ref scheme, .. } => {
                                if scheme.eq_ignore_ascii_case("bearer") {
                                    features.push(ConformanceFeature::SecurityBearer);
                                } else if scheme.eq_ignore_ascii_case("basic") {
                                    features.push(ConformanceFeature::SecurityBasic);
                                }
                            }
                            SecurityScheme::APIKey { .. } => {
                                features.push(ConformanceFeature::SecurityApiKey);
                            }
                            // OAuth2 and OpenIDConnect don't map to our current feature set
                            _ => {}
                        }
                    } else {
                        // Fallback: heuristic name matching for unresolvable schemes
                        let name_lower = scheme_name.to_lowercase();
                        if name_lower.contains("bearer") || name_lower.contains("jwt") {
                            features.push(ConformanceFeature::SecurityBearer);
                        } else if name_lower.contains("api") && name_lower.contains("key") {
                            features.push(ConformanceFeature::SecurityApiKey);
                        } else if name_lower.contains("basic") {
                            features.push(ConformanceFeature::SecurityBasic);
                        }
                    }
                }
            }
        }
    }

    /// Resolve a security scheme name to its SecurityScheme definition
    fn resolve_security_scheme<'a>(name: &str, spec: &'a OpenAPI) -> Option<&'a SecurityScheme> {
        let components = spec.components.as_ref()?;
        match components.security_schemes.get(name)? {
            ReferenceOr::Item(scheme) => Some(scheme),
            ReferenceOr::Reference { .. } => None,
        }
    }

    /// Returns the number of operations being tested
    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }

    /// Generate the k6 conformance script.
    /// Returns (script, check_count) where check_count is the number of unique checks emitted.
    pub fn generate(&self) -> Result<(String, usize)> {
        let mut script = String::with_capacity(16384);

        // Imports
        script.push_str("import http from 'k6/http';\n");
        script.push_str("import { check, group } from 'k6';\n\n");

        // Options
        script.push_str("export const options = {\n");
        script.push_str("  vus: 1,\n");
        script.push_str("  iterations: 1,\n");
        if self.config.skip_tls_verify {
            script.push_str("  insecureSkipTLSVerify: true,\n");
        }
        if self.config.has_cookie_header() {
            script.push_str("  noCookies: true,\n");
        }
        script.push_str("  thresholds: {\n");
        script.push_str("    checks: ['rate>0'],\n");
        script.push_str("  },\n");
        script.push_str("};\n\n");

        // Base URL (includes base_path if configured)
        script.push_str(&format!("const BASE_URL = '{}';\n\n", self.config.effective_base_url()));
        script.push_str("const JSON_HEADERS = { 'Content-Type': 'application/json' };\n\n");

        // Default function
        script.push_str("export default function () {\n");

        // Group operations by category
        let mut category_ops: std::collections::BTreeMap<
            &'static str,
            Vec<(&AnnotatedOperation, &ConformanceFeature)>,
        > = std::collections::BTreeMap::new();

        for op in &self.operations {
            for feature in &op.features {
                let category = feature.category();
                if self.config.should_include_category(category) {
                    category_ops.entry(category).or_default().push((op, feature));
                }
            }
        }

        // Emit grouped tests
        let mut total_checks = 0usize;
        for (category, ops) in &category_ops {
            script.push_str(&format!("  group('{}', function () {{\n", category));

            if self.config.all_operations {
                // All-operations mode: test every operation, using path-qualified check names
                let mut emitted_checks: HashSet<String> = HashSet::new();
                for (op, feature) in ops {
                    let qualified = format!("{}:{}", feature.check_name(), op.path);
                    if emitted_checks.insert(qualified.clone()) {
                        self.emit_check_named(&mut script, op, feature, &qualified);
                        total_checks += 1;
                    }
                }
            } else {
                // Default: one representative operation per feature check name
                let mut emitted_checks: HashSet<&str> = HashSet::new();
                for (op, feature) in ops {
                    if emitted_checks.insert(feature.check_name()) {
                        self.emit_check(&mut script, op, feature);
                        total_checks += 1;
                    }
                }
            }

            script.push_str("  });\n\n");
        }

        script.push_str("}\n\n");

        // handleSummary
        self.generate_handle_summary(&mut script);

        Ok((script, total_checks))
    }

    /// Emit a single k6 check for an operation + feature using the feature's default check name
    fn emit_check(
        &self,
        script: &mut String,
        op: &AnnotatedOperation,
        feature: &ConformanceFeature,
    ) {
        self.emit_check_named(script, op, feature, feature.check_name());
    }

    /// Emit a single k6 check for an operation + feature with a custom check name
    fn emit_check_named(
        &self,
        script: &mut String,
        op: &AnnotatedOperation,
        feature: &ConformanceFeature,
        check_name: &str,
    ) {
        script.push_str("    {\n");

        // Build the URL path with parameters substituted
        let mut url_path = op.path.clone();
        for (name, value) in &op.path_params {
            url_path = url_path.replace(&format!("{{{}}}", name), value);
        }

        // Build query string
        if !op.query_params.is_empty() {
            let qs: Vec<String> =
                op.query_params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            url_path = format!("{}?{}", url_path, qs.join("&"));
        }

        let full_url = format!("${{BASE_URL}}{}", url_path);

        // Build effective headers: merge spec-derived headers with custom headers.
        // Custom headers override spec-derived ones with the same name.
        let effective_headers = self.effective_headers(&op.header_params);
        let has_headers = !effective_headers.is_empty();
        let headers_obj = if has_headers {
            Self::format_headers(&effective_headers)
        } else {
            String::new()
        };

        // Determine HTTP method and emit request
        match op.method.as_str() {
            "GET" => {
                if has_headers {
                    script.push_str(&format!(
                        "      let res = http.get(`{}`, {{ headers: {} }});\n",
                        full_url, headers_obj
                    ));
                } else {
                    script.push_str(&format!("      let res = http.get(`{}`);\n", full_url));
                }
            }
            "POST" => {
                self.emit_request_with_body(script, "post", &full_url, op, &effective_headers);
            }
            "PUT" => {
                self.emit_request_with_body(script, "put", &full_url, op, &effective_headers);
            }
            "PATCH" => {
                self.emit_request_with_body(script, "patch", &full_url, op, &effective_headers);
            }
            "DELETE" => {
                if has_headers {
                    script.push_str(&format!(
                        "      let res = http.del(`{}`, null, {{ headers: {} }});\n",
                        full_url, headers_obj
                    ));
                } else {
                    script.push_str(&format!("      let res = http.del(`{}`);\n", full_url));
                }
            }
            "HEAD" => {
                if has_headers {
                    script.push_str(&format!(
                        "      let res = http.head(`{}`, {{ headers: {} }});\n",
                        full_url, headers_obj
                    ));
                } else {
                    script.push_str(&format!("      let res = http.head(`{}`);\n", full_url));
                }
            }
            "OPTIONS" => {
                if has_headers {
                    script.push_str(&format!(
                        "      let res = http.options(`{}`, null, {{ headers: {} }});\n",
                        full_url, headers_obj
                    ));
                } else {
                    script.push_str(&format!("      let res = http.options(`{}`);\n", full_url));
                }
            }
            _ => {
                if has_headers {
                    script.push_str(&format!(
                        "      let res = http.get(`{}`, {{ headers: {} }});\n",
                        full_url, headers_obj
                    ));
                } else {
                    script.push_str(&format!("      let res = http.get(`{}`);\n", full_url));
                }
            }
        }

        // Check: emit assertion based on feature type
        if matches!(
            feature,
            ConformanceFeature::Response200
                | ConformanceFeature::Response201
                | ConformanceFeature::Response204
                | ConformanceFeature::Response400
                | ConformanceFeature::Response404
        ) {
            let expected_code = match feature {
                ConformanceFeature::Response200 => 200,
                ConformanceFeature::Response201 => 201,
                ConformanceFeature::Response204 => 204,
                ConformanceFeature::Response400 => 400,
                ConformanceFeature::Response404 => 404,
                _ => 200,
            };
            script.push_str(&format!(
                "      check(res, {{ '{}': (r) => r.status === {} }});\n",
                check_name, expected_code
            ));
        } else if matches!(feature, ConformanceFeature::ResponseValidation) {
            // Response schema validation — validate the response body against the schema
            if let Some(schema) = &op.response_schema {
                let validation_js = SchemaValidatorGenerator::generate_validation(schema);
                script.push_str(&format!(
                    "      try {{ let body = res.json(); check(res, {{ '{}': (r) => {{ {} }} }}); }} catch(e) {{ check(res, {{ '{}': () => false }}); }}\n",
                    check_name, validation_js, check_name
                ));
            }
        } else {
            script.push_str(&format!(
                "      check(res, {{ '{}': (r) => r.status >= 200 && r.status < 500 }});\n",
                check_name
            ));
        }

        script.push_str("    }\n");
    }

    /// Emit an HTTP request with a body
    fn emit_request_with_body(
        &self,
        script: &mut String,
        method: &str,
        url: &str,
        op: &AnnotatedOperation,
        effective_headers: &[(String, String)],
    ) {
        if let Some(body) = &op.sample_body {
            let escaped_body = body.replace('\'', "\\'");
            let headers = if !effective_headers.is_empty() {
                format!(
                    "Object.assign({{}}, JSON_HEADERS, {})",
                    Self::format_headers(effective_headers)
                )
            } else {
                "JSON_HEADERS".to_string()
            };
            script.push_str(&format!(
                "      let res = http.{}(`{}`, '{}', {{ headers: {} }});\n",
                method, url, escaped_body, headers
            ));
        } else if !effective_headers.is_empty() {
            script.push_str(&format!(
                "      let res = http.{}(`{}`, null, {{ headers: {} }});\n",
                method,
                url,
                Self::format_headers(effective_headers)
            ));
        } else {
            script.push_str(&format!("      let res = http.{}(`{}`, null);\n", method, url));
        }
    }

    /// Build effective headers by merging spec-derived headers with custom headers.
    /// Custom headers override spec-derived ones with the same name (case-insensitive).
    /// Custom headers not in the spec are appended.
    fn effective_headers(&self, spec_headers: &[(String, String)]) -> Vec<(String, String)> {
        let custom = &self.config.custom_headers;
        if custom.is_empty() {
            return spec_headers.to_vec();
        }

        let mut result: Vec<(String, String)> = Vec::new();

        // Start with spec headers, replacing values if a custom header matches
        for (name, value) in spec_headers {
            if let Some((_, custom_val)) =
                custom.iter().find(|(cn, _)| cn.eq_ignore_ascii_case(name))
            {
                result.push((name.clone(), custom_val.clone()));
            } else {
                result.push((name.clone(), value.clone()));
            }
        }

        // Append custom headers that aren't already in spec headers
        for (name, value) in custom {
            if !spec_headers.iter().any(|(sn, _)| sn.eq_ignore_ascii_case(name)) {
                result.push((name.clone(), value.clone()));
            }
        }

        result
    }

    /// Format header params as a JS object literal
    fn format_headers(headers: &[(String, String)]) -> String {
        let entries: Vec<String> =
            headers.iter().map(|(k, v)| format!("'{}': '{}'", k, v)).collect();
        format!("{{ {} }}", entries.join(", "))
    }

    /// handleSummary — same format as reference mode for report compatibility
    fn generate_handle_summary(&self, script: &mut String) {
        // Use absolute path for report output so k6 writes where the CLI expects
        let report_path = match &self.config.output_dir {
            Some(dir) => {
                let abs = std::fs::canonicalize(dir)
                    .unwrap_or_else(|_| dir.clone())
                    .join("conformance-report.json");
                abs.to_string_lossy().to_string()
            }
            None => "conformance-report.json".to_string(),
        };

        script.push_str("export function handleSummary(data) {\n");
        script.push_str("  let checks = {};\n");
        script.push_str("  if (data.metrics && data.metrics.checks) {\n");
        script.push_str("    checks.overall_pass_rate = data.metrics.checks.values.rate;\n");
        script.push_str("  }\n");
        script.push_str("  let checkResults = {};\n");
        script.push_str("  function walkGroups(group) {\n");
        script.push_str("    if (group.checks) {\n");
        script.push_str("      for (let checkObj of group.checks) {\n");
        script.push_str("        checkResults[checkObj.name] = {\n");
        script.push_str("          passes: checkObj.passes,\n");
        script.push_str("          fails: checkObj.fails,\n");
        script.push_str("        };\n");
        script.push_str("      }\n");
        script.push_str("    }\n");
        script.push_str("    if (group.groups) {\n");
        script.push_str("      for (let subGroup of group.groups) {\n");
        script.push_str("        walkGroups(subGroup);\n");
        script.push_str("      }\n");
        script.push_str("    }\n");
        script.push_str("  }\n");
        script.push_str("  if (data.root_group) {\n");
        script.push_str("    walkGroups(data.root_group);\n");
        script.push_str("  }\n");
        script.push_str("  return {\n");
        script.push_str(&format!(
            "    '{}': JSON.stringify({{ checks: checkResults, overall: checks }}, null, 2),\n",
            report_path
        ));
        script.push_str("    stdout: textSummary(data, { indent: '  ', enableColors: true }),\n");
        script.push_str("  };\n");
        script.push_str("}\n\n");
        script.push_str("function textSummary(data, opts) {\n");
        script.push_str("  return JSON.stringify(data, null, 2);\n");
        script.push_str("}\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openapiv3::{
        Operation, ParameterData, ParameterSchemaOrContent, PathStyle, Response, Schema,
        SchemaData, SchemaKind, StringType, Type,
    };

    fn make_op(method: &str, path: &str, operation: Operation) -> ApiOperation {
        ApiOperation {
            method: method.to_string(),
            path: path.to_string(),
            operation,
            operation_id: None,
        }
    }

    fn empty_spec() -> OpenAPI {
        OpenAPI::default()
    }

    #[test]
    fn test_annotate_get_with_path_param() {
        let mut op = Operation::default();
        op.parameters.push(ReferenceOr::Item(Parameter::Path {
            parameter_data: ParameterData {
                name: "id".to_string(),
                description: None,
                required: true,
                deprecated: None,
                format: ParameterSchemaOrContent::Schema(ReferenceOr::Item(Schema {
                    schema_data: SchemaData::default(),
                    schema_kind: SchemaKind::Type(Type::String(StringType::default())),
                })),
                example: None,
                examples: Default::default(),
                explode: None,
                extensions: Default::default(),
            },
            style: PathStyle::Simple,
        }));

        let api_op = make_op("get", "/users/{id}", op);
        let annotated = SpecDrivenConformanceGenerator::annotate_operation(&api_op, &empty_spec());

        assert!(annotated.features.contains(&ConformanceFeature::MethodGet));
        assert!(annotated.features.contains(&ConformanceFeature::PathParamString));
        assert!(annotated.features.contains(&ConformanceFeature::ConstraintRequired));
        assert_eq!(annotated.path_params.len(), 1);
        assert_eq!(annotated.path_params[0].0, "id");
    }

    #[test]
    fn test_annotate_post_with_json_body() {
        let mut op = Operation::default();
        let mut body = openapiv3::RequestBody::default();
        body.required = true;
        body.content
            .insert("application/json".to_string(), openapiv3::MediaType::default());
        op.request_body = Some(ReferenceOr::Item(body));

        let api_op = make_op("post", "/items", op);
        let annotated = SpecDrivenConformanceGenerator::annotate_operation(&api_op, &empty_spec());

        assert!(annotated.features.contains(&ConformanceFeature::MethodPost));
        assert!(annotated.features.contains(&ConformanceFeature::BodyJson));
    }

    #[test]
    fn test_annotate_response_codes() {
        let mut op = Operation::default();
        op.responses
            .responses
            .insert(openapiv3::StatusCode::Code(200), ReferenceOr::Item(Response::default()));
        op.responses
            .responses
            .insert(openapiv3::StatusCode::Code(404), ReferenceOr::Item(Response::default()));

        let api_op = make_op("get", "/items", op);
        let annotated = SpecDrivenConformanceGenerator::annotate_operation(&api_op, &empty_spec());

        assert!(annotated.features.contains(&ConformanceFeature::Response200));
        assert!(annotated.features.contains(&ConformanceFeature::Response404));
    }

    #[test]
    fn test_generate_spec_driven_script() {
        let config = ConformanceConfig {
            target_url: "http://localhost:3000".to_string(),
            api_key: None,
            basic_auth: None,
            skip_tls_verify: false,
            categories: None,
            base_path: None,
            custom_headers: vec![],
            output_dir: None,
            all_operations: false,
        };

        let operations = vec![AnnotatedOperation {
            path: "/users/{id}".to_string(),
            method: "GET".to_string(),
            features: vec![
                ConformanceFeature::MethodGet,
                ConformanceFeature::PathParamString,
            ],
            request_body_content_type: None,
            sample_body: None,
            query_params: vec![],
            header_params: vec![],
            path_params: vec![("id".to_string(), "test-value".to_string())],
            response_schema: None,
        }];

        let gen = SpecDrivenConformanceGenerator::new(config, operations);
        let (script, _check_count) = gen.generate().unwrap();

        assert!(script.contains("import http from 'k6/http'"));
        assert!(script.contains("/users/test-value"));
        assert!(script.contains("param:path:string"));
        assert!(script.contains("method:GET"));
        assert!(script.contains("handleSummary"));
    }

    #[test]
    fn test_generate_with_category_filter() {
        let config = ConformanceConfig {
            target_url: "http://localhost:3000".to_string(),
            api_key: None,
            basic_auth: None,
            skip_tls_verify: false,
            categories: Some(vec!["Parameters".to_string()]),
            base_path: None,
            custom_headers: vec![],
            output_dir: None,
            all_operations: false,
        };

        let operations = vec![AnnotatedOperation {
            path: "/users/{id}".to_string(),
            method: "GET".to_string(),
            features: vec![
                ConformanceFeature::MethodGet,
                ConformanceFeature::PathParamString,
            ],
            request_body_content_type: None,
            sample_body: None,
            query_params: vec![],
            header_params: vec![],
            path_params: vec![("id".to_string(), "1".to_string())],
            response_schema: None,
        }];

        let gen = SpecDrivenConformanceGenerator::new(config, operations);
        let (script, _check_count) = gen.generate().unwrap();

        assert!(script.contains("group('Parameters'"));
        assert!(!script.contains("group('HTTP Methods'"));
    }

    #[test]
    fn test_annotate_response_validation() {
        use openapiv3::ObjectType;

        // Operation with a 200 response that has a JSON schema
        let mut op = Operation::default();
        let mut response = Response::default();
        let mut media = openapiv3::MediaType::default();
        let mut obj_type = ObjectType::default();
        obj_type.properties.insert(
            "name".to_string(),
            ReferenceOr::Item(Box::new(Schema {
                schema_data: SchemaData::default(),
                schema_kind: SchemaKind::Type(Type::String(StringType::default())),
            })),
        );
        obj_type.required = vec!["name".to_string()];
        media.schema = Some(ReferenceOr::Item(Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::Type(Type::Object(obj_type)),
        }));
        response.content.insert("application/json".to_string(), media);
        op.responses
            .responses
            .insert(openapiv3::StatusCode::Code(200), ReferenceOr::Item(response));

        let api_op = make_op("get", "/users", op);
        let annotated = SpecDrivenConformanceGenerator::annotate_operation(&api_op, &empty_spec());

        assert!(
            annotated.features.contains(&ConformanceFeature::ResponseValidation),
            "Should detect ResponseValidation when response has a JSON schema"
        );
        assert!(annotated.response_schema.is_some(), "Should extract the response schema");

        // Verify generated script includes schema validation with try-catch
        let config = ConformanceConfig {
            target_url: "http://localhost:3000".to_string(),
            api_key: None,
            basic_auth: None,
            skip_tls_verify: false,
            categories: None,
            base_path: None,
            custom_headers: vec![],
            output_dir: None,
            all_operations: false,
        };
        let gen = SpecDrivenConformanceGenerator::new(config, vec![annotated]);
        let (script, _check_count) = gen.generate().unwrap();

        assert!(
            script.contains("response:schema:validation"),
            "Script should contain the validation check name"
        );
        assert!(script.contains("try {"), "Script should wrap validation in try-catch");
        assert!(script.contains("res.json()"), "Script should parse response as JSON");
    }

    #[test]
    fn test_annotate_global_security() {
        // Spec with global security requirement, operation without its own security
        let op = Operation::default();
        let mut spec = OpenAPI::default();
        let mut global_req = openapiv3::SecurityRequirement::new();
        global_req.insert("bearerAuth".to_string(), vec![]);
        spec.security = Some(vec![global_req]);
        // Define the security scheme in components
        let mut components = openapiv3::Components::default();
        components.security_schemes.insert(
            "bearerAuth".to_string(),
            ReferenceOr::Item(SecurityScheme::HTTP {
                scheme: "bearer".to_string(),
                bearer_format: Some("JWT".to_string()),
                description: None,
                extensions: Default::default(),
            }),
        );
        spec.components = Some(components);

        let api_op = make_op("get", "/protected", op);
        let annotated = SpecDrivenConformanceGenerator::annotate_operation(&api_op, &spec);

        assert!(
            annotated.features.contains(&ConformanceFeature::SecurityBearer),
            "Should detect SecurityBearer from global security + components"
        );
    }

    #[test]
    fn test_annotate_security_scheme_resolution() {
        // Test that security scheme type is resolved from components, not just name heuristic
        let mut op = Operation::default();
        // Use a generic name that wouldn't match name heuristics
        let mut req = openapiv3::SecurityRequirement::new();
        req.insert("myAuth".to_string(), vec![]);
        op.security = Some(vec![req]);

        let mut spec = OpenAPI::default();
        let mut components = openapiv3::Components::default();
        components.security_schemes.insert(
            "myAuth".to_string(),
            ReferenceOr::Item(SecurityScheme::APIKey {
                location: openapiv3::APIKeyLocation::Header,
                name: "X-API-Key".to_string(),
                description: None,
                extensions: Default::default(),
            }),
        );
        spec.components = Some(components);

        let api_op = make_op("get", "/data", op);
        let annotated = SpecDrivenConformanceGenerator::annotate_operation(&api_op, &spec);

        assert!(
            annotated.features.contains(&ConformanceFeature::SecurityApiKey),
            "Should detect SecurityApiKey from SecurityScheme::APIKey, not name heuristic"
        );
    }

    #[test]
    fn test_annotate_content_negotiation() {
        let mut op = Operation::default();
        let mut response = Response::default();
        // Response with multiple content types
        response
            .content
            .insert("application/json".to_string(), openapiv3::MediaType::default());
        response
            .content
            .insert("application/xml".to_string(), openapiv3::MediaType::default());
        op.responses
            .responses
            .insert(openapiv3::StatusCode::Code(200), ReferenceOr::Item(response));

        let api_op = make_op("get", "/items", op);
        let annotated = SpecDrivenConformanceGenerator::annotate_operation(&api_op, &empty_spec());

        assert!(
            annotated.features.contains(&ConformanceFeature::ContentNegotiation),
            "Should detect ContentNegotiation when response has multiple content types"
        );
    }

    #[test]
    fn test_no_content_negotiation_for_single_type() {
        let mut op = Operation::default();
        let mut response = Response::default();
        response
            .content
            .insert("application/json".to_string(), openapiv3::MediaType::default());
        op.responses
            .responses
            .insert(openapiv3::StatusCode::Code(200), ReferenceOr::Item(response));

        let api_op = make_op("get", "/items", op);
        let annotated = SpecDrivenConformanceGenerator::annotate_operation(&api_op, &empty_spec());

        assert!(
            !annotated.features.contains(&ConformanceFeature::ContentNegotiation),
            "Should NOT detect ContentNegotiation for a single content type"
        );
    }

    #[test]
    fn test_spec_driven_with_base_path() {
        let annotated = AnnotatedOperation {
            path: "/users".to_string(),
            method: "GET".to_string(),
            features: vec![ConformanceFeature::MethodGet],
            path_params: vec![],
            query_params: vec![],
            header_params: vec![],
            request_body_content_type: None,
            sample_body: None,
            response_schema: None,
        };
        let config = ConformanceConfig {
            target_url: "https://192.168.2.86/".to_string(),
            api_key: None,
            basic_auth: None,
            skip_tls_verify: true,
            categories: None,
            base_path: Some("/api".to_string()),
            custom_headers: vec![],
            output_dir: None,
            all_operations: false,
        };
        let gen = SpecDrivenConformanceGenerator::new(config, vec![annotated]);
        let (script, _check_count) = gen.generate().unwrap();

        assert!(
            script.contains("const BASE_URL = 'https://192.168.2.86/api'"),
            "BASE_URL should include the base_path. Got: {}",
            script.lines().find(|l| l.contains("BASE_URL")).unwrap_or("not found")
        );
    }

    #[test]
    fn test_spec_driven_with_custom_headers() {
        let annotated = AnnotatedOperation {
            path: "/users".to_string(),
            method: "GET".to_string(),
            features: vec![ConformanceFeature::MethodGet],
            path_params: vec![],
            query_params: vec![],
            header_params: vec![
                ("X-Avi-Tenant".to_string(), "test-value".to_string()),
                ("X-CSRFToken".to_string(), "test-value".to_string()),
            ],
            request_body_content_type: None,
            sample_body: None,
            response_schema: None,
        };
        let config = ConformanceConfig {
            target_url: "https://192.168.2.86/".to_string(),
            api_key: None,
            basic_auth: None,
            skip_tls_verify: true,
            categories: None,
            base_path: Some("/api".to_string()),
            custom_headers: vec![
                ("X-Avi-Tenant".to_string(), "admin".to_string()),
                ("X-CSRFToken".to_string(), "real-csrf-token".to_string()),
                ("Cookie".to_string(), "sessionid=abc123".to_string()),
            ],
            output_dir: None,
            all_operations: false,
        };
        let gen = SpecDrivenConformanceGenerator::new(config, vec![annotated]);
        let (script, _check_count) = gen.generate().unwrap();

        // Custom headers should override spec-derived test-value placeholders
        assert!(
            script.contains("'X-Avi-Tenant': 'admin'"),
            "Should use custom value for X-Avi-Tenant, not test-value"
        );
        assert!(
            script.contains("'X-CSRFToken': 'real-csrf-token'"),
            "Should use custom value for X-CSRFToken, not test-value"
        );
        // Custom headers not in spec should be appended
        assert!(
            script.contains("'Cookie': 'sessionid=abc123'"),
            "Should include Cookie header from custom_headers"
        );
        // test-value should NOT appear
        assert!(
            !script.contains("'test-value'"),
            "test-value placeholders should be replaced by custom values"
        );
    }

    #[test]
    fn test_effective_headers_merging() {
        let config = ConformanceConfig {
            target_url: "http://localhost".to_string(),
            api_key: None,
            basic_auth: None,
            skip_tls_verify: false,
            categories: None,
            base_path: None,
            custom_headers: vec![
                ("X-Auth".to_string(), "real-token".to_string()),
                ("Cookie".to_string(), "session=abc".to_string()),
            ],
            output_dir: None,
            all_operations: false,
        };
        let gen = SpecDrivenConformanceGenerator::new(config, vec![]);

        // Spec headers with a matching custom header
        let spec_headers = vec![
            ("X-Auth".to_string(), "test-value".to_string()),
            ("X-Other".to_string(), "keep-this".to_string()),
        ];
        let effective = gen.effective_headers(&spec_headers);

        // X-Auth should be overridden
        assert_eq!(effective[0], ("X-Auth".to_string(), "real-token".to_string()));
        // X-Other should be kept as-is
        assert_eq!(effective[1], ("X-Other".to_string(), "keep-this".to_string()));
        // Cookie should be appended
        assert_eq!(effective[2], ("Cookie".to_string(), "session=abc".to_string()));
    }
}
