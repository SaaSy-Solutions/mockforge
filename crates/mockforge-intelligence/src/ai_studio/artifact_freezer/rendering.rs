//! Serialize snapshots while keeping payload hashes independent of provenance.
use super::{FreezeRequest, Result};
use chrono::Utc;
use serde_json::{json, Value};

pub(super) fn render(
    request: &FreezeRequest,
    output_hash: Option<&str>,
) -> Result<(Value, String)> {
    let mut content = request.content.clone();
    if let Some(object) = content.as_object_mut() {
        let mut provenance = json!({
            "frozen_at": Utc::now().to_rfc3339(),
            "artifact_type": request.artifact_type,
            "source": "ai_generated",
            "format": request.format,
        });
        if let Some(metadata) = &request.metadata {
            let fields = serde_json::to_value(metadata)?;
            provenance
                .as_object_mut()
                .expect("provenance is an object")
                .extend(fields.as_object().expect("metadata is an object").clone());
        }
        provenance["output_hash"] = json!(output_hash);
        object.insert("_frozen_metadata".into(), provenance);
    }
    let serialized = if request.format == "json" {
        serde_json::to_string_pretty(&content)?
    } else {
        serde_yaml::to_string(&content).map_err(|error| {
            mockforge_foundation::Error::internal(format!(
                "Failed to serialize frozen YAML: {error}"
            ))
        })?
    };
    Ok((content, serialized))
}
