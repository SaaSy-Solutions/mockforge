//! Select serve-time validation behavior or an isolated, strict contract policy.

use super::{OpenApiRouteRegistry, ValidationMode};
use serde_json::{Map, Value};

impl OpenApiRouteRegistry {
    pub(super) fn request_validation_policy(
        &self,
        path: &str,
        method: &str,
    ) -> Option<(bool, ValidationMode)> {
        if !self.honor_serve_validation_policy {
            return Some((true, ValidationMode::Enforce));
        }
        // Skip validation for any configured admin prefixes
        for pref in &self.options.admin_skip_prefixes {
            if !pref.is_empty() && path.starts_with(pref) {
                return None;
            }
        }
        // Runtime env overrides
        let env_mode = self.environment_value("MOCKFORGE_REQUEST_VALIDATION").map(|v| {
            match v.to_ascii_lowercase().as_str() {
                "off" | "disable" | "disabled" => ValidationMode::Disabled,
                "warn" | "warning" => ValidationMode::Warn,
                _ => ValidationMode::Enforce,
            }
        });
        let aggregate = self
            .environment_value("MOCKFORGE_AGGREGATE_ERRORS")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(self.options.aggregate_errors);
        // Per-route runtime overrides via JSON env var
        let env_overrides: Option<Map<String, Value>> = self
            .environment_value("MOCKFORGE_VALIDATION_OVERRIDES_JSON")
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .and_then(|v| v.as_object().cloned());
        // Response validation is handled in HTTP layer now
        let mut effective_mode = env_mode.unwrap_or(self.options.request_mode.clone());
        // Apply runtime overrides first if present
        if let Some(map) = &env_overrides {
            if let Some(v) = map.get(&format!("{} {}", method, path)) {
                if let Some(m) = v.as_str() {
                    effective_mode = match m {
                        "off" => ValidationMode::Disabled,
                        "warn" => ValidationMode::Warn,
                        _ => ValidationMode::Enforce,
                    };
                }
            }
        }
        // Then static options overrides
        if let Some(override_mode) = self.options.overrides.get(&format!("{} {}", method, path)) {
            effective_mode = override_mode.clone();
        }
        Some((aggregate, effective_mode))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{openapi_routes::ValidationOptions, OpenApiSpec};
    use serde_json::json;

    fn spec() -> OpenApiSpec {
        OpenApiSpec::from_json(json!({"openapi":"3.0.3","info":{"title":"policy","version":"1"},
        "paths":{"/policy-verification/items":{"post":{
            "requestBody":{"required":true,"content":{"application/json":{"schema":{
                "type":"object","required":["name"],"properties":{"name":{"type":"string"}}
            }}}},"responses":{"200":{"description":"ok"}}
        }}}}))
        .unwrap()
    }

    fn missing_body(registry: &OpenApiRouteRegistry) -> bool {
        registry
            .validate_request_with_all_ex(
                "/policy-verification/items",
                "POST",
                &Map::new(),
                &Map::new(),
                &Map::new(),
                &Map::new(),
                None,
                false,
            )
            .is_err()
    }

    #[test]
    fn contract_constructor_and_validation_clone_enforce_requests() {
        let registry = OpenApiRouteRegistry::new_for_contract_verification(spec());
        assert!(missing_body(&registry));
        assert!(missing_body(&registry.clone_for_validation()));
        assert_eq!(registry.options.validation_status, Some(400));
        let body = json!({"name":"valid"});
        assert!(registry
            .validate_request_with_all_ex(
                "/policy-verification/items",
                "POST",
                &Map::new(),
                &Map::new(),
                &Map::new(),
                &Map::new(),
                Some(&body),
                true
            )
            .is_ok());
    }

    #[test]
    fn serving_constructors_keep_modes_skip_prefixes_and_static_overrides() {
        for mode in [ValidationMode::Disabled, ValidationMode::Warn] {
            let options = ValidationOptions {
                request_mode: mode,
                ..Default::default()
            };
            assert!(!missing_body(&OpenApiRouteRegistry::new_with_options(spec(), options)));
        }
        assert!(missing_body(&OpenApiRouteRegistry::new_with_options(
            spec(),
            ValidationOptions::default()
        )));
        let options = ValidationOptions {
            admin_skip_prefixes: vec!["/policy-verification".into()],
            ..Default::default()
        };
        assert!(!missing_body(&OpenApiRouteRegistry::new_with_options(spec(), options)));
        let options = ValidationOptions {
            overrides: [("POST /policy-verification/items".into(), ValidationMode::Disabled)]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        assert!(!missing_body(&OpenApiRouteRegistry::new_with_options(spec(), options)));
    }
}
