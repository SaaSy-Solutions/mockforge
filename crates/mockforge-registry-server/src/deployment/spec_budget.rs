//! Response-generation budget for hosted OpenAPI documents on the shared runtime.

/// Bound recursive schema expansion before the synchronous response generator runs.
/// Each array may synthesize up to 100 items, including nested arrays and refs.
pub(crate) fn validate_document_resources(document: &serde_json::Value) -> Result<(), String> {
    ResourceBudget {
        document,
        references: Vec::new(),
        remaining: 1024 * 1024,
    }
    .visit(document, 0, 1)
}

struct ResourceBudget<'a> {
    document: &'a serde_json::Value,
    /// `$ref`s on the current path, to reject recursive schemas.
    references: Vec<String>,
    remaining: usize,
}

impl ResourceBudget<'_> {
    fn visit(
        &mut self,
        value: &serde_json::Value,
        depth: usize,
        copies: usize,
    ) -> Result<(), String> {
        let node_bytes = 64 + value.as_str().map_or(0, str::len);
        let cost = copies.saturating_mul(node_bytes);
        if depth > 32 || cost > self.remaining {
            return Err("Hosted specification exceeds the response generation budget".into());
        }
        self.remaining -= cost;
        match value {
            serde_json::Value::Object(object) => self.visit_object(object, depth, copies),
            serde_json::Value::Array(values) => {
                values.iter().try_for_each(|child| self.visit(child, depth + 1, copies))
            }
            _ => Ok(()),
        }
    }

    fn visit_object(
        &mut self,
        object: &serde_json::Map<String, serde_json::Value>,
        depth: usize,
        copies: usize,
    ) -> Result<(), String> {
        if let Some(reference) = object.get("$ref").and_then(|value| value.as_str()) {
            self.visit_reference(reference, depth, copies)?;
        }
        for (key, child) in object.iter().filter(|(key, _)| key.as_str() != "$ref") {
            // Pagination may also expand an example's items array.
            let multiplier = if key == "items" { 100 } else { 1 };
            self.visit(child, depth + 1, copies.saturating_mul(multiplier))?;
        }
        Ok(())
    }

    fn visit_reference(
        &mut self,
        reference: &str,
        depth: usize,
        copies: usize,
    ) -> Result<(), String> {
        let pointer = reference
            .strip_prefix('#')
            .ok_or_else(|| "Hosted specifications require local references".to_string())?;
        if self.references.iter().any(|active| active == reference) {
            return Err("Recursive schemas require an isolated deployment provider".into());
        }
        let target = self
            .document
            .pointer(pointer)
            .ok_or_else(|| "Unresolved specification reference".to_string())?;
        self.references.push(reference.to_string());
        self.visit(target, depth + 1, copies)?;
        self.references.pop();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::validate_document_resources;
    use serde_json::json;

    #[test]
    fn generation_budget_rejects_cycles_and_nested_array_amplification() {
        let recursive = json!({"components":{"schemas":{"Node":{"type":"object","properties":{"next":{"$ref":"#/components/schemas/Node"}}}}}});
        assert!(validate_document_resources(&recursive).unwrap_err().contains("Recursive"));
        let amplified = json!({"type":"array","items":{"type":"array","items":{"type":"array","items":{"type":"string"}}}});
        assert!(validate_document_resources(&amplified).unwrap_err().contains("budget"));
        let normal = json!({"components":{"schemas":{"Item":{"type":"object","properties":{"name":{"type":"string"}}}}},"schema":{"type":"array","items":{"$ref":"#/components/schemas/Item"}}});
        assert!(validate_document_resources(&normal).is_ok());
        assert!(
            validate_document_resources(&json!({"$ref":"https://fixture.invalid/spec"})).is_err()
        );
    }
}
