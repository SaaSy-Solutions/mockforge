//! Protocol-agnostic contract abstractions for multi-protocol drift detection
//!
//! This module provides a unified interface for contract definitions across different
//! protocols (HTTP/OpenAPI, gRPC, WebSocket, MQTT, Kafka), enabling consistent drift
//! detection and analysis regardless of the transport layer.
//!
//! The trait and data types are re-exported from
//! `mockforge-foundation::protocol_contract_types` so consumers can implement
//! and use them without depending on deprecated core modules.

use crate::ai_contract_diff::{ContractDiffResult, Mismatch};
use mockforge_foundation::protocol::Protocol;
pub use mockforge_foundation::protocol_contract_types::{
    classify_change, extract_breaking_changes, ChangeClassification, ContractError,
    ContractMetadata, ContractOperation, ContractRequest, OperationType, ProtocolContract,
    ValidationError, ValidationResult,
};
use std::collections::HashMap;

/// Registry for managing protocol contracts
pub struct ProtocolContractRegistry {
    contracts: HashMap<String, Box<dyn ProtocolContract>>,
}

impl ProtocolContractRegistry {
    /// Create a new contract registry
    pub fn new() -> Self {
        Self {
            contracts: HashMap::new(),
        }
    }

    /// Register a contract
    pub fn register(&mut self, contract: Box<dyn ProtocolContract>) {
        let id = contract.contract_id().to_string();
        self.contracts.insert(id, contract);
    }

    /// Get a contract by ID
    pub fn get(&self, contract_id: &str) -> Option<&dyn ProtocolContract> {
        self.contracts.get(contract_id).map(|c| c.as_ref())
    }

    /// List all contracts
    pub fn list(&self) -> Vec<&dyn ProtocolContract> {
        self.contracts.values().map(|c| c.as_ref()).collect()
    }

    /// List contracts by protocol
    pub fn list_by_protocol(&self, protocol: Protocol) -> Vec<&dyn ProtocolContract> {
        self.contracts
            .values()
            .filter(|c| c.protocol() == protocol)
            .map(|c| c.as_ref())
            .collect()
    }

    /// Remove a contract
    pub fn remove(&mut self, contract_id: &str) -> Option<Box<dyn ProtocolContract>> {
        self.contracts.remove(contract_id)
    }
}

impl Default for ProtocolContractRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to compare two contracts and generate drift analysis
pub async fn compare_contracts(
    old_contract: &dyn ProtocolContract,
    new_contract: &dyn ProtocolContract,
) -> Result<ContractDiffResult, ContractError> {
    // Ensure protocols match
    if old_contract.protocol() != new_contract.protocol() {
        return Err(ContractError::Other(format!(
            "Cannot compare contracts of different protocols: {:?} vs {:?}",
            old_contract.protocol(),
            new_contract.protocol()
        )));
    }

    // Use the contract's diff method
    old_contract.diff(new_contract).await
}

/// Generate a per-service+method drift report for gRPC contracts
///
/// Groups mismatches by service and method, showing additive vs breaking changes
pub fn generate_grpc_drift_report(diff: &ContractDiffResult) -> serde_json::Value {
    use std::collections::HashMap;

    // Group mismatches by service and method
    let mut service_reports: HashMap<String, HashMap<String, Vec<&Mismatch>>> = HashMap::new();

    for mismatch in &diff.mismatches {
        // Extract service and method from context or path
        let service = mismatch
            .context
            .get("service")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                // Fallback: try to extract from path (format: "service.method" or "service")
                mismatch.path.split('.').next().map(|s| s.to_string())
            })
            .unwrap_or_else(|| "unknown".to_string());

        let method = mismatch
            .context
            .get("method")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| mismatch.method.clone())
            .or_else(|| {
                // Fallback: try to extract from path
                mismatch.path.split('.').nth(1).map(|s| s.to_string())
            })
            .unwrap_or_else(|| "unknown".to_string());

        service_reports
            .entry(service)
            .or_default()
            .entry(method)
            .or_default()
            .push(mismatch);
    }

    // Build report structure
    let mut report = serde_json::Map::new();
    let mut services_json = serde_json::Map::new();

    for (service_name, methods) in service_reports {
        let mut service_json = serde_json::Map::new();
        let mut methods_json = serde_json::Map::new();
        let mut service_additive = 0;
        let mut service_breaking = 0;

        for (method_name, mismatches) in methods {
            let mut method_json = serde_json::Map::new();
            let mut method_additive = 0;
            let mut method_breaking = 0;
            let mut changes = Vec::new();

            // Save length before consuming mismatches in the loop
            let total_changes = mismatches.len();

            for mismatch in mismatches {
                let classification = classify_change(mismatch);
                if classification.is_additive {
                    method_additive += 1;
                }
                if classification.is_breaking {
                    method_breaking += 1;
                }

                changes.push(serde_json::json!({
                    "description": mismatch.description,
                    "path": mismatch.path,
                    "severity": format!("{:?}", mismatch.severity),
                    "is_additive": classification.is_additive,
                    "is_breaking": classification.is_breaking,
                    "change_category": classification.change_category,
                }));
            }

            method_json.insert("additive_changes".to_string(), serde_json::json!(method_additive));
            method_json.insert("breaking_changes".to_string(), serde_json::json!(method_breaking));
            method_json.insert("total_changes".to_string(), serde_json::json!(total_changes));
            method_json.insert("changes".to_string(), serde_json::json!(changes));

            service_additive += method_additive;
            service_breaking += method_breaking;

            methods_json.insert(method_name, serde_json::Value::Object(method_json));
        }

        service_json.insert("additive_changes".to_string(), serde_json::json!(service_additive));
        service_json.insert("breaking_changes".to_string(), serde_json::json!(service_breaking));
        service_json.insert("methods".to_string(), serde_json::Value::Object(methods_json));

        services_json.insert(service_name, serde_json::Value::Object(service_json));
    }

    report.insert("services".to_string(), serde_json::Value::Object(services_json));
    report.insert("total_mismatches".to_string(), serde_json::json!(diff.mismatches.len()));

    serde_json::Value::Object(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_type_serialization() {
        let op_type = OperationType::HttpEndpoint {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
        };
        let json = serde_json::to_string(&op_type).unwrap();
        assert!(json.contains("http_endpoint"));
        assert!(json.contains("GET"));
        assert!(json.contains("/api/users"));
    }

    #[test]
    fn test_contract_registry() {
        // This test would require a mock implementation of ProtocolContract
        // For now, just test the registry structure
        let registry = ProtocolContractRegistry::new();
        assert_eq!(registry.list().len(), 0);
    }
}
