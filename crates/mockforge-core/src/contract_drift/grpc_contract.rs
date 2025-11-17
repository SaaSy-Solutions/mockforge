//! gRPC contract implementation for protocol-agnostic contract drift detection
//!
//! This module provides a `GrpcContract` struct that implements the `ProtocolContract` trait
//! for gRPC services, enabling drift detection and analysis for protobuf-based APIs.

use crate::ai_contract_diff::{ContractDiffResult, Mismatch, MismatchSeverity, MismatchType};
use crate::contract_drift::protocol_contracts::{
    compare_contracts, ContractError, ContractOperation, ContractRequest, OperationType,
    ProtocolContract, ValidationError, ValidationResult,
};
use crate::protocol_abstraction::Protocol;
use prost_reflect::{DescriptorPool, MethodDescriptor, ServiceDescriptor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// gRPC contract implementation
///
/// Wraps a protobuf descriptor pool and provides contract drift detection
/// capabilities for gRPC services and methods.
pub struct GrpcContract {
    /// Unique identifier for this contract
    contract_id: String,
    /// Contract version
    version: String,
    /// Descriptor pool containing the proto definitions
    descriptor_pool: Arc<DescriptorPool>,
    /// Map of service names to service descriptors
    services: HashMap<String, ServiceDescriptor>,
    /// Map of operation IDs (service.method) to method descriptors
    methods: HashMap<String, MethodDescriptor>,
    /// Cached contract operations for quick lookup
    operations_cache: HashMap<String, ContractOperation>,
    /// Contract metadata
    metadata: HashMap<String, String>,
}

impl GrpcContract {
    /// Create a new gRPC contract from a descriptor pool
    pub fn new(
        contract_id: String,
        version: String,
        descriptor_pool: Arc<DescriptorPool>,
    ) -> Result<Self, ContractError> {
        let mut services = HashMap::new();
        let mut methods = HashMap::new();

        let mut operations_cache = HashMap::new();

        // Extract all services and methods from the descriptor pool
        for service in descriptor_pool.services() {
            let service_name = service.full_name().to_string();
            services.insert(service_name.clone(), service.clone());

            // Extract methods from this service
            for method in service.methods() {
                let method_name = method.name().to_string();
                let operation_id = format!("{}.{}", service_name, method_name);
                methods.insert(operation_id.clone(), method.clone());

                // Cache the contract operation
                let operation = ContractOperation {
                    id: operation_id.clone(),
                    name: method_name.clone(),
                    operation_type: OperationType::GrpcMethod {
                        service: service_name.clone(),
                        method: method_name,
                    },
                    input_schema: Some(serde_json::json!({
                        "type": method.input().full_name(),
                        "streaming": method.is_client_streaming(),
                    })),
                    output_schema: Some(serde_json::json!({
                        "type": method.output().full_name(),
                        "streaming": method.is_server_streaming(),
                    })),
                    metadata: HashMap::new(),
                };
                operations_cache.insert(operation_id, operation);
            }
        }

        Ok(Self {
            contract_id,
            version,
            descriptor_pool,
            services,
            methods,
            operations_cache,
            metadata: HashMap::new(),
        })
    }

    /// Create a gRPC contract from a proto file path
    pub async fn from_proto_file(
        contract_id: String,
        version: String,
        proto_file: &str,
    ) -> Result<Self, ContractError> {
        // This would require compiling the proto file first
        // For now, we'll return an error indicating this needs to be implemented
        // In a full implementation, this would:
        // 1. Compile the proto file using protoc
        // 2. Load the descriptor set
        // 3. Create a DescriptorPool from it
        // 4. Call Self::new()
        Err(ContractError::Other(format!(
            "Loading from proto file not yet implemented. Use GrpcContract::from_descriptor_set() instead"
        )))
    }

    /// Create a gRPC contract from a compiled descriptor set (FileDescriptorSet bytes)
    pub fn from_descriptor_set(
        contract_id: String,
        version: String,
        descriptor_bytes: &[u8],
    ) -> Result<Self, ContractError> {
        let mut descriptor_pool = DescriptorPool::new();
        descriptor_pool.decode_file_descriptor_set(descriptor_bytes).map_err(|e| {
            ContractError::InvalidFormat(format!("Failed to decode descriptor set: {}", e))
        })?;

        Self::new(contract_id, version, Arc::new(descriptor_pool))
    }

    /// Compare two gRPC contracts and detect differences
    fn diff_services(&self, other: &GrpcContract) -> Result<ContractDiffResult, ContractError> {
        let mut mismatches = Vec::new();
        let mut all_services: std::collections::HashSet<String> =
            self.services.keys().chain(other.services.keys()).cloned().collect();

        // Check for removed services
        for service_name in &all_services {
            if self.services.contains_key(service_name)
                && !other.services.contains_key(service_name)
            {
                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::EndpointNotFound,
                    path: service_name.clone(),
                    method: None,
                    expected: Some(format!("Service {} should exist", service_name)),
                    actual: Some("Service removed".to_string()),
                    description: format!("Service {} was removed", service_name),
                    severity: MismatchSeverity::Critical,
                    confidence: 1.0,
                    context: HashMap::new(),
                });
            }
        }

        // Check for added services (non-breaking, but track it)
        for service_name in &all_services {
            if !self.services.contains_key(service_name)
                && other.services.contains_key(service_name)
            {
                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::UnexpectedField,
                    path: service_name.clone(),
                    method: None,
                    expected: None,
                    actual: Some(format!("New service {}", service_name)),
                    description: format!("New service {} was added", service_name),
                    severity: MismatchSeverity::Low,
                    confidence: 1.0,
                    context: HashMap::new(),
                });
            }
        }

        // Compare methods in common services
        for service_name in &all_services {
            if let (Some(old_service), Some(new_service)) =
                (self.services.get(service_name), other.services.get(service_name))
            {
                let method_diff = self.diff_methods(old_service, new_service)?;
                mismatches.extend(method_diff);
            }
        }

        let matches = mismatches.is_empty();
        let confidence = if matches { 1.0 } else { 0.8 };

        Ok(ContractDiffResult {
            matches,
            confidence,
            mismatches,
            recommendations: Vec::new(),
            corrections: Vec::new(),
            metadata: crate::ai_contract_diff::DiffMetadata {
                analyzed_at: chrono::Utc::now(),
                request_source: "grpc_contract_diff".to_string(),
                contract_version: Some(self.version.clone()),
                contract_format: "protobuf".to_string(),
                endpoint_path: "".to_string(),
                http_method: "".to_string(),
                request_count: 1,
                llm_provider: None,
                llm_model: None,
            },
        })
    }

    /// Compare methods between two service descriptors
    fn diff_methods(
        &self,
        old_service: &ServiceDescriptor,
        new_service: &ServiceDescriptor,
    ) -> Result<Vec<Mismatch>, ContractError> {
        let mut mismatches = Vec::new();
        let service_name = old_service.full_name().to_string();

        // Collect all method names
        let old_methods: std::collections::HashSet<String> =
            old_service.methods().map(|m| m.name().to_string()).collect();
        let new_methods: std::collections::HashSet<String> =
            new_service.methods().map(|m| m.name().to_string()).collect();

        // Check for removed methods (breaking change)
        for method_name in &old_methods {
            if !new_methods.contains(method_name) {
                let path = format!("{}.{}", service_name, method_name);
                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::EndpointNotFound,
                    path: path.clone(),
                    method: Some(method_name.clone()),
                    expected: Some(format!("Method {}.{} should exist", service_name, method_name)),
                    actual: Some("Method removed".to_string()),
                    description: format!("Method {}.{} was removed", service_name, method_name),
                    severity: MismatchSeverity::Critical,
                    confidence: 1.0,
                    context: HashMap::new(),
                });
            }
        }

        // Check for added methods (non-breaking)
        for method_name in &new_methods {
            if !old_methods.contains(method_name) {
                let path = format!("{}.{}", service_name, method_name);
                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::UnexpectedField,
                    path: path.clone(),
                    method: Some(method_name.clone()),
                    expected: None,
                    actual: Some(format!("New method {}.{}", service_name, method_name)),
                    description: format!("New method {}.{} was added", service_name, method_name),
                    severity: MismatchSeverity::Low,
                    confidence: 1.0,
                    context: HashMap::new(),
                });
            }
        }

        // Compare method signatures for methods that exist in both
        for method_name in old_methods.intersection(&new_methods) {
            let old_method = old_service
                .methods()
                .find(|m| m.name() == method_name)
                .ok_or_else(|| ContractError::OperationNotFound(method_name.clone()))?;
            let new_method = new_service
                .methods()
                .find(|m| m.name() == method_name)
                .ok_or_else(|| ContractError::OperationNotFound(method_name.clone()))?;

            let method_mismatches =
                Self::diff_method_signatures(&old_method, &new_method, &service_name)?;
            mismatches.extend(method_mismatches);
        }

        Ok(mismatches)
    }

    /// Compare method signatures (input/output types, streaming flags)
    fn diff_method_signatures(
        old_method: &MethodDescriptor,
        new_method: &MethodDescriptor,
        service_name: &str,
    ) -> Result<Vec<Mismatch>, ContractError> {
        let mut mismatches = Vec::new();
        let method_name = old_method.name();
        let path = format!("{}.{}", service_name, method_name);

        // Check input type changes
        if old_method.input().full_name() != new_method.input().full_name() {
            mismatches.push(Mismatch {
                mismatch_type: MismatchType::TypeMismatch,
                path: format!("{}.input", path),
                method: Some(method_name.to_string()),
                expected: Some(old_method.input().full_name().to_string()),
                actual: Some(new_method.input().full_name().to_string()),
                description: format!(
                    "Input type changed from {} to {}",
                    old_method.input().full_name(),
                    new_method.input().full_name()
                ),
                severity: MismatchSeverity::High,
                confidence: 1.0,
                context: HashMap::new(),
            });
        }

        // Check output type changes
        if old_method.output().full_name() != new_method.output().full_name() {
            mismatches.push(Mismatch {
                mismatch_type: MismatchType::TypeMismatch,
                path: format!("{}.output", path),
                method: Some(method_name.to_string()),
                expected: Some(old_method.output().full_name().to_string()),
                actual: Some(new_method.output().full_name().to_string()),
                description: format!(
                    "Output type changed from {} to {}",
                    old_method.output().full_name(),
                    new_method.output().full_name()
                ),
                severity: MismatchSeverity::High,
                confidence: 1.0,
                context: HashMap::new(),
            });
        }

        // Check streaming flag changes (breaking change)
        if old_method.is_client_streaming() != new_method.is_client_streaming() {
            mismatches.push(Mismatch {
                mismatch_type: MismatchType::SchemaMismatch,
                path: path.clone(),
                method: Some(method_name.to_string()),
                expected: Some(format!("Client streaming: {}", old_method.is_client_streaming())),
                actual: Some(format!("Client streaming: {}", new_method.is_client_streaming())),
                description: format!(
                    "Client streaming flag changed for {}.{}",
                    service_name, method_name
                ),
                severity: MismatchSeverity::Critical,
                confidence: 1.0,
                context: HashMap::new(),
            });
        }

        if old_method.is_server_streaming() != new_method.is_server_streaming() {
            mismatches.push(Mismatch {
                mismatch_type: MismatchType::SchemaMismatch,
                path: path.clone(),
                method: Some(method_name.to_string()),
                expected: Some(format!("Server streaming: {}", old_method.is_server_streaming())),
                actual: Some(format!("Server streaming: {}", new_method.is_server_streaming())),
                description: format!(
                    "Server streaming flag changed for {}.{}",
                    service_name, method_name
                ),
                severity: MismatchSeverity::Critical,
                confidence: 1.0,
                context: HashMap::new(),
            });
        }

        Ok(mismatches)
    }
}

#[async_trait::async_trait]
impl ProtocolContract for GrpcContract {
    fn protocol(&self) -> Protocol {
        Protocol::Grpc
    }

    fn contract_id(&self) -> &str {
        &self.contract_id
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn operations(&self) -> Vec<ContractOperation> {
        self.operations_cache.values().cloned().collect()
    }

    fn get_operation(&self, operation_id: &str) -> Option<&ContractOperation> {
        self.operations_cache.get(operation_id)
    }

    async fn diff(
        &self,
        other: &dyn ProtocolContract,
    ) -> Result<ContractDiffResult, ContractError> {
        // Ensure the other contract is also a gRPC contract
        if other.protocol() != Protocol::Grpc {
            return Err(ContractError::UnsupportedProtocol(other.protocol()));
        }

        // Try to downcast to GrpcContract
        // Since we can't use downcast_ref on trait objects, we'll need to use a different approach
        // For now, we'll require that contracts of the same protocol can be compared
        // In a full implementation, we might use a type-erased approach or require
        // contracts to provide a way to access their internal representation

        // This is a limitation of the current design - we need a way to compare
        // contracts of the same protocol type
        Err(ContractError::Other(
            "Direct comparison of GrpcContract instances requires type information. \
             Use GrpcContract::diff_services() for comparing two GrpcContract instances."
                .to_string(),
        ))
    }

    async fn validate(
        &self,
        operation_id: &str,
        _request: &ContractRequest,
    ) -> Result<ValidationResult, ContractError> {
        // Check if the operation exists
        if !self.methods.contains_key(operation_id) {
            return Ok(ValidationResult {
                valid: false,
                errors: vec![ValidationError {
                    message: format!("Method {} not found in contract", operation_id),
                    path: Some(operation_id.to_string()),
                    code: Some("METHOD_NOT_FOUND".to_string()),
                }],
                warnings: Vec::new(),
            });
        }

        // TODO: Implement actual message validation against the proto schema
        // This would require:
        // 1. Deserializing the request payload as protobuf
        // 2. Validating against the method's input message descriptor
        // 3. Checking required fields, types, etc.

        Ok(ValidationResult {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        })
    }

    fn get_schema(&self, operation_id: &str) -> Option<serde_json::Value> {
        self.methods.get(operation_id).map(|method| {
            serde_json::json!({
                "input": {
                    "type": method.input().full_name(),
                    "streaming": method.is_client_streaming(),
                },
                "output": {
                    "type": method.output().full_name(),
                    "streaming": method.is_server_streaming(),
                },
            })
        })
    }

    fn to_json(&self) -> Result<serde_json::Value, ContractError> {
        let operations: Vec<serde_json::Value> = self
            .operations()
            .iter()
            .map(|op| {
                serde_json::json!({
                    "id": op.id,
                    "name": op.name,
                    "type": op.operation_type,
                    "input_schema": op.input_schema,
                    "output_schema": op.output_schema,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "contract_id": self.contract_id,
            "version": self.version,
            "protocol": "grpc",
            "services": self.services.keys().collect::<Vec<_>>(),
            "operations": operations,
            "metadata": self.metadata,
        }))
    }
}

/// Helper function to compare two GrpcContract instances
pub fn diff_grpc_contracts(
    old_contract: &GrpcContract,
    new_contract: &GrpcContract,
) -> Result<ContractDiffResult, ContractError> {
    old_contract.diff_services(new_contract)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_contract_creation() {
        // This test would require a sample descriptor set
        // For now, we'll just test that the structure compiles
        // In a full implementation, we'd create a test proto file and compile it
    }
}
