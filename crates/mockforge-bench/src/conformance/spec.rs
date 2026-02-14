//! Conformance feature definitions and bundled reference spec

/// OpenAPI 3.0.0 feature categories for conformance testing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConformanceFeature {
    // Parameters
    PathParamString,
    PathParamInteger,
    QueryParamString,
    QueryParamInteger,
    QueryParamArray,
    HeaderParam,
    CookieParam,
    // Request Bodies
    BodyJson,
    BodyFormUrlencoded,
    BodyMultipart,
    // Schema Types
    SchemaString,
    SchemaInteger,
    SchemaNumber,
    SchemaBoolean,
    SchemaArray,
    SchemaObject,
    // Composition
    CompositionOneOf,
    CompositionAnyOf,
    CompositionAllOf,
    // String Formats
    FormatDate,
    FormatDateTime,
    FormatEmail,
    FormatUuid,
    FormatUri,
    FormatIpv4,
    FormatIpv6,
    // Constraints
    ConstraintRequired,
    ConstraintOptional,
    ConstraintMinMax,
    ConstraintPattern,
    ConstraintEnum,
    // Response Codes
    Response200,
    Response201,
    Response204,
    Response400,
    Response404,
    // HTTP Methods
    MethodGet,
    MethodPost,
    MethodPut,
    MethodPatch,
    MethodDelete,
    MethodHead,
    MethodOptions,
    // Content Negotiation
    ContentNegotiation,
    // Security
    SecurityBearer,
    SecurityApiKey,
    SecurityBasic,
    // Response Validation (spec-driven mode)
    ResponseValidation,
}

impl ConformanceFeature {
    /// Get the category name for this feature
    pub fn category(&self) -> &'static str {
        match self {
            Self::PathParamString
            | Self::PathParamInteger
            | Self::QueryParamString
            | Self::QueryParamInteger
            | Self::QueryParamArray
            | Self::HeaderParam
            | Self::CookieParam => "Parameters",
            Self::BodyJson | Self::BodyFormUrlencoded | Self::BodyMultipart => "Request Bodies",
            Self::SchemaString
            | Self::SchemaInteger
            | Self::SchemaNumber
            | Self::SchemaBoolean
            | Self::SchemaArray
            | Self::SchemaObject => "Schema Types",
            Self::CompositionOneOf | Self::CompositionAnyOf | Self::CompositionAllOf => {
                "Composition"
            }
            Self::FormatDate
            | Self::FormatDateTime
            | Self::FormatEmail
            | Self::FormatUuid
            | Self::FormatUri
            | Self::FormatIpv4
            | Self::FormatIpv6 => "String Formats",
            Self::ConstraintRequired
            | Self::ConstraintOptional
            | Self::ConstraintMinMax
            | Self::ConstraintPattern
            | Self::ConstraintEnum => "Constraints",
            Self::Response200
            | Self::Response201
            | Self::Response204
            | Self::Response400
            | Self::Response404 => "Response Codes",
            Self::MethodGet
            | Self::MethodPost
            | Self::MethodPut
            | Self::MethodPatch
            | Self::MethodDelete
            | Self::MethodHead
            | Self::MethodOptions => "HTTP Methods",
            Self::ContentNegotiation => "Content Types",
            Self::SecurityBearer | Self::SecurityApiKey | Self::SecurityBasic => "Security",
            Self::ResponseValidation => "Response Validation",
        }
    }

    /// Get the check name used in k6 scripts (maps back from k6 output)
    pub fn check_name(&self) -> &'static str {
        match self {
            Self::PathParamString => "param:path:string",
            Self::PathParamInteger => "param:path:integer",
            Self::QueryParamString => "param:query:string",
            Self::QueryParamInteger => "param:query:integer",
            Self::QueryParamArray => "param:query:array",
            Self::HeaderParam => "param:header",
            Self::CookieParam => "param:cookie",
            Self::BodyJson => "body:json",
            Self::BodyFormUrlencoded => "body:form-urlencoded",
            Self::BodyMultipart => "body:multipart",
            Self::SchemaString => "schema:string",
            Self::SchemaInteger => "schema:integer",
            Self::SchemaNumber => "schema:number",
            Self::SchemaBoolean => "schema:boolean",
            Self::SchemaArray => "schema:array",
            Self::SchemaObject => "schema:object",
            Self::CompositionOneOf => "composition:oneOf",
            Self::CompositionAnyOf => "composition:anyOf",
            Self::CompositionAllOf => "composition:allOf",
            Self::FormatDate => "format:date",
            Self::FormatDateTime => "format:date-time",
            Self::FormatEmail => "format:email",
            Self::FormatUuid => "format:uuid",
            Self::FormatUri => "format:uri",
            Self::FormatIpv4 => "format:ipv4",
            Self::FormatIpv6 => "format:ipv6",
            Self::ConstraintRequired => "constraint:required",
            Self::ConstraintOptional => "constraint:optional",
            Self::ConstraintMinMax => "constraint:minmax",
            Self::ConstraintPattern => "constraint:pattern",
            Self::ConstraintEnum => "constraint:enum",
            Self::Response200 => "response:200",
            Self::Response201 => "response:201",
            Self::Response204 => "response:204",
            Self::Response400 => "response:400",
            Self::Response404 => "response:404",
            Self::MethodGet => "method:GET",
            Self::MethodPost => "method:POST",
            Self::MethodPut => "method:PUT",
            Self::MethodPatch => "method:PATCH",
            Self::MethodDelete => "method:DELETE",
            Self::MethodHead => "method:HEAD",
            Self::MethodOptions => "method:OPTIONS",
            Self::ContentNegotiation => "content:negotiation",
            Self::SecurityBearer => "security:bearer",
            Self::SecurityApiKey => "security:apikey",
            Self::SecurityBasic => "security:basic",
            Self::ResponseValidation => "response:schema:validation",
        }
    }

    /// All feature variants
    pub fn all() -> &'static [ConformanceFeature] {
        &[
            Self::PathParamString,
            Self::PathParamInteger,
            Self::QueryParamString,
            Self::QueryParamInteger,
            Self::QueryParamArray,
            Self::HeaderParam,
            Self::CookieParam,
            Self::BodyJson,
            Self::BodyFormUrlencoded,
            Self::BodyMultipart,
            Self::SchemaString,
            Self::SchemaInteger,
            Self::SchemaNumber,
            Self::SchemaBoolean,
            Self::SchemaArray,
            Self::SchemaObject,
            Self::CompositionOneOf,
            Self::CompositionAnyOf,
            Self::CompositionAllOf,
            Self::FormatDate,
            Self::FormatDateTime,
            Self::FormatEmail,
            Self::FormatUuid,
            Self::FormatUri,
            Self::FormatIpv4,
            Self::FormatIpv6,
            Self::ConstraintRequired,
            Self::ConstraintOptional,
            Self::ConstraintMinMax,
            Self::ConstraintPattern,
            Self::ConstraintEnum,
            Self::Response200,
            Self::Response201,
            Self::Response204,
            Self::Response400,
            Self::Response404,
            Self::MethodGet,
            Self::MethodPost,
            Self::MethodPut,
            Self::MethodPatch,
            Self::MethodDelete,
            Self::MethodHead,
            Self::MethodOptions,
            Self::ContentNegotiation,
            Self::SecurityBearer,
            Self::SecurityApiKey,
            Self::SecurityBasic,
            Self::ResponseValidation,
        ]
    }

    /// All unique categories
    pub fn categories() -> &'static [&'static str] {
        &[
            "Parameters",
            "Request Bodies",
            "Schema Types",
            "Composition",
            "String Formats",
            "Constraints",
            "Response Codes",
            "HTTP Methods",
            "Content Types",
            "Security",
            "Response Validation",
        ]
    }

    /// Convert a CLI category name (lowercase, hyphenated) to the canonical category name
    pub fn category_from_cli_name(name: &str) -> Option<&'static str> {
        match name.to_lowercase().replace('_', "-").as_str() {
            "parameters" => Some("Parameters"),
            "request-bodies" => Some("Request Bodies"),
            "schema-types" => Some("Schema Types"),
            "composition" => Some("Composition"),
            "string-formats" => Some("String Formats"),
            "constraints" => Some("Constraints"),
            "response-codes" => Some("Response Codes"),
            "http-methods" => Some("HTTP Methods"),
            "content-types" => Some("Content Types"),
            "security" => Some("Security"),
            "response-validation" => Some("Response Validation"),
            _ => None,
        }
    }

    /// All valid CLI category names with their canonical names
    pub fn cli_category_names() -> Vec<(&'static str, &'static str)> {
        vec![
            ("parameters", "Parameters"),
            ("request-bodies", "Request Bodies"),
            ("schema-types", "Schema Types"),
            ("composition", "Composition"),
            ("string-formats", "String Formats"),
            ("constraints", "Constraints"),
            ("response-codes", "Response Codes"),
            ("http-methods", "HTTP Methods"),
            ("content-types", "Content Types"),
            ("security", "Security"),
            ("response-validation", "Response Validation"),
        ]
    }

    /// Get the OpenAPI spec URL for this feature's category (used in SARIF reports)
    pub fn spec_url(&self) -> &'static str {
        match self.category() {
            "Parameters" => "https://spec.openapis.org/oas/v3.0.0#parameter-object",
            "Request Bodies" => "https://spec.openapis.org/oas/v3.0.0#request-body-object",
            "Schema Types" => "https://spec.openapis.org/oas/v3.0.0#schema-object",
            "Composition" => "https://spec.openapis.org/oas/v3.0.0#schema-object",
            "String Formats" => "https://spec.openapis.org/oas/v3.0.0#data-types",
            "Constraints" => "https://spec.openapis.org/oas/v3.0.0#schema-object",
            "Response Codes" => "https://spec.openapis.org/oas/v3.0.0#responses-object",
            "HTTP Methods" => "https://spec.openapis.org/oas/v3.0.0#path-item-object",
            "Content Types" => "https://spec.openapis.org/oas/v3.0.0#media-type-object",
            "Security" => "https://spec.openapis.org/oas/v3.0.0#security-scheme-object",
            "Response Validation" => "https://spec.openapis.org/oas/v3.0.0#response-object",
            _ => "https://spec.openapis.org/oas/v3.0.0",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_features_have_categories() {
        for feature in ConformanceFeature::all() {
            assert!(!feature.category().is_empty());
            assert!(!feature.check_name().is_empty());
        }
    }

    #[test]
    fn test_all_categories_covered() {
        let categories: std::collections::HashSet<&str> =
            ConformanceFeature::all().iter().map(|f| f.category()).collect();
        for cat in ConformanceFeature::categories() {
            assert!(categories.contains(cat), "Category '{}' has no features", cat);
        }
    }

    #[test]
    fn test_category_from_cli_name() {
        assert_eq!(ConformanceFeature::category_from_cli_name("parameters"), Some("Parameters"));
        assert_eq!(
            ConformanceFeature::category_from_cli_name("request-bodies"),
            Some("Request Bodies")
        );
        assert_eq!(
            ConformanceFeature::category_from_cli_name("schema-types"),
            Some("Schema Types")
        );
        assert_eq!(ConformanceFeature::category_from_cli_name("PARAMETERS"), Some("Parameters"));
        assert_eq!(
            ConformanceFeature::category_from_cli_name("Request-Bodies"),
            Some("Request Bodies")
        );
        assert_eq!(ConformanceFeature::category_from_cli_name("invalid"), None);
    }

    #[test]
    fn test_cli_category_names_complete() {
        let cli_names = ConformanceFeature::cli_category_names();
        let categories = ConformanceFeature::categories();
        assert_eq!(cli_names.len(), categories.len());
        for (_, canonical) in &cli_names {
            assert!(
                categories.contains(canonical),
                "CLI name maps to unknown category: {}",
                canonical
            );
        }
    }

    #[test]
    fn test_spec_urls_not_empty() {
        for feature in ConformanceFeature::all() {
            assert!(!feature.spec_url().is_empty(), "Feature {:?} has empty spec URL", feature);
        }
    }
}
