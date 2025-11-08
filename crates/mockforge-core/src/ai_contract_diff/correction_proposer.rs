//! Schema correction proposal generator
//!
//! This module generates JSON Patch (RFC 6902) files for correcting contract specifications
//! based on detected mismatches and recommendations.

use super::types::{CorrectionProposal, Mismatch, PatchOperation, Recommendation};
use crate::openapi::OpenApiSpec;
use crate::Result;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Correction proposal generator
pub struct CorrectionProposer;

impl CorrectionProposer {
    /// Generate correction proposals from mismatches and recommendations
    pub fn generate_proposals(
        mismatches: &[Mismatch],
        recommendations: &[Recommendation],
        spec: &OpenApiSpec,
    ) -> Vec<CorrectionProposal> {
        let mut proposals = Vec::new();

        // Group recommendations by mismatch
        let mut rec_by_mismatch: HashMap<String, Vec<&Recommendation>> = HashMap::new();
        for rec in recommendations {
            rec_by_mismatch
                .entry(rec.mismatch_id.clone())
                .or_insert_with(Vec::new)
                .push(rec);
        }

        // Generate proposals for each mismatch
        for (idx, mismatch) in mismatches.iter().enumerate() {
            let mismatch_id = format!("mismatch_{}", idx);
            let recommendations_for_mismatch =
                rec_by_mismatch.get(&mismatch_id).map(|v| v.as_slice()).unwrap_or(&[]);

            let proposals_for_mismatch =
                Self::generate_proposals_for_mismatch(mismatch, recommendations_for_mismatch, spec);

            proposals.extend(proposals_for_mismatch);
        }

        proposals
    }

    /// Generate correction proposals for a specific mismatch
    fn generate_proposals_for_mismatch(
        mismatch: &Mismatch,
        recommendations: &[&Recommendation],
        spec: &OpenApiSpec,
    ) -> Vec<CorrectionProposal> {
        let mut proposals = Vec::new();

        match mismatch.mismatch_type {
            super::types::MismatchType::MissingRequiredField => {
                if let Some(proposal) = Self::propose_add_required_field(mismatch, spec) {
                    proposals.push(proposal);
                }
            }
            super::types::MismatchType::TypeMismatch => {
                if let Some(proposal) = Self::propose_fix_type(mismatch, spec) {
                    proposals.push(proposal);
                }
            }
            super::types::MismatchType::UnexpectedField => {
                if let Some(proposal) = Self::propose_remove_field(mismatch, spec) {
                    proposals.push(proposal);
                } else if let Some(proposal) = Self::propose_add_optional_field(mismatch, spec) {
                    // If field is being sent but not in spec, maybe it should be added as optional
                    proposals.push(proposal);
                }
            }
            super::types::MismatchType::FormatMismatch => {
                if let Some(proposal) = Self::propose_add_format(mismatch, spec) {
                    proposals.push(proposal);
                }
            }
            super::types::MismatchType::EndpointNotFound => {
                if let Some(proposal) = Self::propose_add_endpoint(mismatch, spec) {
                    proposals.push(proposal);
                }
            }
            _ => {
                // Generic proposal for other mismatch types
                if let Some(proposal) = Self::propose_generic_fix(mismatch, recommendations, spec) {
                    proposals.push(proposal);
                }
            }
        }

        proposals
    }

    /// Propose adding a required field to the schema
    fn propose_add_required_field(
        mismatch: &Mismatch,
        _spec: &OpenApiSpec,
    ) -> Option<CorrectionProposal> {
        // Extract field path and type from mismatch
        let path_parts: Vec<&str> = mismatch.path.split('/').filter(|s| !s.is_empty()).collect();
        if path_parts.is_empty() {
            return None;
        }

        let field_name = path_parts.last().unwrap();
        let expected_type = mismatch.expected.as_ref()?;

        // Build JSON Patch path (RFC 6902 format with ~1 for / and ~0 for ~)
        let patch_path = Self::build_patch_path(&mismatch.path, mismatch.method.as_deref());

        // Determine field schema based on expected type
        let field_schema = match expected_type.as_str() {
            "string" => json!({
                "type": "string"
            }),
            "integer" => json!({
                "type": "integer"
            }),
            "number" => json!({
                "type": "number"
            }),
            "boolean" => json!({
                "type": "boolean"
            }),
            "array" => json!({
                "type": "array",
                "items": {}
            }),
            "object" => json!({
                "type": "object",
                "properties": {}
            }),
            _ => json!({
                "type": "string"
            }),
        };

        // Get before value (current schema state)
        let before = json!(null); // Would need to extract from actual spec

        // Get after value (proposed schema)
        let after = field_schema.clone();

        Some(CorrectionProposal {
            id: format!("correction_{}", mismatch.path.replace('/', "_")),
            patch_path: format!("{}/properties/{}", patch_path, field_name),
            operation: PatchOperation::Add,
            value: Some(field_schema),
            from: None,
            confidence: mismatch.confidence,
            description: format!(
                "Add required field '{}' of type '{}' to schema",
                field_name, expected_type
            ),
            reasoning: Some(format!(
                "Front-end consistently sends '{}' but it's not defined in the contract",
                field_name
            )),
            affected_endpoints: mismatch
                .method
                .as_ref()
                .map(|m| vec![format!("{} {}", m, mismatch.path)])
                .unwrap_or_default(),
            before: Some(before),
            after: Some(after),
        })
    }

    /// Propose fixing a type mismatch
    fn propose_fix_type(mismatch: &Mismatch, _spec: &OpenApiSpec) -> Option<CorrectionProposal> {
        let expected_type = mismatch.expected.as_ref()?;
        let actual_type = mismatch.actual.as_ref()?;

        let patch_path = Self::build_patch_path(&mismatch.path, mismatch.method.as_deref());

        // Build new type schema
        let new_type_schema = match expected_type.as_str() {
            "string" => json!({"type": "string"}),
            "integer" => json!({"type": "integer"}),
            "number" => json!({"type": "number"}),
            "boolean" => json!({"type": "boolean"}),
            "array" => json!({"type": "array", "items": {}}),
            "object" => json!({"type": "object", "properties": {}}),
            _ => json!({"type": "string"}),
        };

        let before = json!({"type": actual_type});
        let after = new_type_schema.clone();

        Some(CorrectionProposal {
            id: format!("correction_type_{}", mismatch.path.replace('/', "_")),
            patch_path: format!("{}/type", patch_path),
            operation: PatchOperation::Replace,
            value: Some(Value::String(expected_type.clone())),
            from: None,
            confidence: mismatch.confidence,
            description: format!("Change field type from '{}' to '{}'", actual_type, expected_type),
            reasoning: Some(format!(
                "Front-end sends '{}' as '{}' but contract expects '{}'",
                mismatch.path, actual_type, expected_type
            )),
            affected_endpoints: mismatch
                .method
                .as_ref()
                .map(|m| vec![format!("{} {}", m, mismatch.path)])
                .unwrap_or_default(),
            before: Some(before),
            after: Some(after),
        })
    }

    /// Propose removing an unexpected field
    fn propose_remove_field(
        mismatch: &Mismatch,
        _spec: &OpenApiSpec,
    ) -> Option<CorrectionProposal> {
        let patch_path = Self::build_patch_path(&mismatch.path, mismatch.method.as_deref());

        Some(CorrectionProposal {
            id: format!("correction_remove_{}", mismatch.path.replace('/', "_")),
            patch_path: patch_path.clone(),
            operation: PatchOperation::Remove,
            value: None,
            from: None,
            confidence: mismatch.confidence * 0.8, // Lower confidence for removal
            description: format!("Remove unexpected field '{}' from request", mismatch.path),
            reasoning: Some(format!(
                "Field '{}' is sent by front-end but not defined in contract",
                mismatch.path
            )),
            affected_endpoints: mismatch
                .method
                .as_ref()
                .map(|m| vec![format!("{} {}", m, mismatch.path)])
                .unwrap_or_default(),
            before: Some(json!({"field": mismatch.path})),
            after: Some(json!(null)),
        })
    }

    /// Propose adding an optional field (alternative to removal)
    fn propose_add_optional_field(
        mismatch: &Mismatch,
        _spec: &OpenApiSpec,
    ) -> Option<CorrectionProposal> {
        // Similar to add_required_field but with required: false
        Self::propose_add_required_field(mismatch, _spec).map(|mut proposal| {
            proposal.operation = PatchOperation::Add;
            proposal.confidence = mismatch.confidence * 0.7; // Lower confidence for optional
            proposal.description = format!(
                "Add optional field '{}' to schema (alternative to removing from request)",
                mismatch.path
            );
            proposal.reasoning = Some(format!(
                "Front-end sends '{}' but it's not in contract. Consider adding as optional field.",
                mismatch.path
            ));
            proposal
        })
    }

    /// Propose adding format constraint
    fn propose_add_format(mismatch: &Mismatch, _spec: &OpenApiSpec) -> Option<CorrectionProposal> {
        // Extract format from context if available
        let format_value =
            mismatch.context.get("format").and_then(|v| v.as_str()).map(|s| s.to_string());

        let format_value_clone = format_value.clone();
        if format_value.is_none() {
            return None;
        }

        let patch_path = Self::build_patch_path(&mismatch.path, mismatch.method.as_deref());

        Some(CorrectionProposal {
            id: format!("correction_format_{}", mismatch.path.replace('/', "_")),
            patch_path: format!("{}/format", patch_path),
            operation: PatchOperation::Add,
            value: Some(Value::String(format_value.unwrap())),
            from: None,
            confidence: mismatch.confidence,
            description: format!("Add format constraint to field '{}'", mismatch.path),
            reasoning: Some(format!("Field '{}' should have format validation", mismatch.path)),
            affected_endpoints: mismatch
                .method
                .as_ref()
                .map(|m| vec![format!("{} {}", m, mismatch.path)])
                .unwrap_or_default(),
            before: Some(json!({"format": null})),
            after: Some(json!({"format": format_value_clone})),
        })
    }

    /// Propose adding a missing endpoint
    fn propose_add_endpoint(
        mismatch: &Mismatch,
        _spec: &OpenApiSpec,
    ) -> Option<CorrectionProposal> {
        let method = mismatch.method.as_ref()?;
        let path = &mismatch.path;

        // Build OpenAPI path item structure
        let patch_path = format!("/paths/{}", Self::escape_json_pointer(path));

        let endpoint_schema = json!({
            method.to_lowercase(): {
                "summary": format!("{} {}", method, path),
                "responses": {
                    "200": {
                        "description": "Success"
                    }
                }
            }
        });

        let endpoint_schema_clone = endpoint_schema.clone();

        Some(CorrectionProposal {
            id: format!("correction_endpoint_{}_{}", method, path.replace('/', "_")),
            patch_path,
            operation: PatchOperation::Add,
            value: Some(endpoint_schema),
            from: None,
            confidence: mismatch.confidence,
            description: format!("Add endpoint {} {} to contract", method, path),
            reasoning: Some(format!(
                "Front-end calls {} {} but endpoint is not defined in contract",
                method, path
            )),
            affected_endpoints: vec![format!("{} {}", method, path)],
            before: Some(json!(null)),
            after: Some(endpoint_schema_clone),
        })
    }

    /// Propose a generic fix based on recommendations
    fn propose_generic_fix(
        mismatch: &Mismatch,
        recommendations: &[&Recommendation],
        _spec: &OpenApiSpec,
    ) -> Option<CorrectionProposal> {
        // Use the first recommendation if available
        let recommendation = recommendations.first()?;

        let patch_path = Self::build_patch_path(&mismatch.path, mismatch.method.as_deref());

        Some(CorrectionProposal {
            id: format!("correction_generic_{}", mismatch.path.replace('/', "_")),
            patch_path,
            operation: PatchOperation::Replace,
            value: recommendation.example.clone(),
            from: None,
            confidence: recommendation.confidence,
            description: recommendation.recommendation.clone(),
            reasoning: recommendation.reasoning.clone(),
            affected_endpoints: mismatch
                .method
                .as_ref()
                .map(|m| vec![format!("{} {}", m, mismatch.path)])
                .unwrap_or_default(),
            before: None,
            after: recommendation.example.clone(),
        })
    }

    /// Build JSON Patch path from request path and method
    fn build_patch_path(request_path: &str, method: Option<&str>) -> String {
        // Convert request path to OpenAPI schema path
        // Example: /api/users -> /paths/~1api~1users/post/requestBody/content/application~1json/schema
        let escaped_path = Self::escape_json_pointer(request_path);

        if let Some(m) = method {
            format!(
                "/paths/{}/{}/requestBody/content/application~1json/schema",
                escaped_path,
                m.to_lowercase()
            )
        } else {
            format!("/paths/{}/schema", escaped_path)
        }
    }

    /// Escape JSON Pointer special characters (RFC 6901)
    fn escape_json_pointer(path: &str) -> String {
        path.replace('~', "~0").replace('/', "~1")
    }

    /// Generate JSON Patch file from correction proposals
    pub fn generate_patch_file(proposals: &[CorrectionProposal], spec_version: &str) -> Value {
        let patch_operations: Vec<Value> = proposals
            .iter()
            .map(|proposal| {
                let mut op = json!({
                    "op": format!("{:?}", proposal.operation).to_lowercase(),
                    "path": proposal.patch_path,
                });

                match proposal.operation {
                    PatchOperation::Add | PatchOperation::Replace => {
                        if let Some(value) = &proposal.value {
                            op["value"] = value.clone();
                        }
                    }
                    PatchOperation::Move | PatchOperation::Copy => {
                        if let Some(from) = &proposal.from {
                            op["from"] = json!(from);
                        }
                    }
                    _ => {}
                }

                // Add metadata
                op["metadata"] = json!({
                    "id": proposal.id,
                    "confidence": proposal.confidence,
                    "description": proposal.description,
                    "reasoning": proposal.reasoning,
                    "affected_endpoints": proposal.affected_endpoints,
                });

                op
            })
            .collect();

        json!({
            "format": "json-patch",
            "spec_version": spec_version,
            "operations": patch_operations,
            "metadata": {
                "generated_at": chrono::Utc::now().to_rfc3339(),
                "total_corrections": proposals.len(),
                "average_confidence": proposals.iter().map(|p| p.confidence).sum::<f64>() / proposals.len() as f64,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_json_pointer() {
        assert_eq!(CorrectionProposer::escape_json_pointer("/api/users"), "~1api~1users");
        assert_eq!(CorrectionProposer::escape_json_pointer("~test"), "~0test");
    }

    #[test]
    fn test_build_patch_path() {
        let path = CorrectionProposer::build_patch_path("/api/users", Some("POST"));
        assert!(path.contains("~1api~1users"));
        assert!(path.contains("post"));
    }
}
