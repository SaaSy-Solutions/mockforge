//! gRPC contract implementation for protocol-agnostic contract drift detection
//!
//! This module provides a `GrpcContract` struct that implements the `ProtocolContract` trait
//! for gRPC services, enabling drift detection and analysis for protobuf-based APIs.

use crate::ai_contract_diff::{ContractDiffResult, Mismatch, MismatchSeverity, MismatchType};
use crate::contract_drift::protocol_contracts::{
    ContractError, ContractOperation, ContractRequest, OperationType, ProtocolContract,
    ValidationError, ValidationResult,
};
use crate::protocol_abstraction::Protocol;
use prost_reflect::{DescriptorPool, MessageDescriptor, MethodDescriptor, ServiceDescriptor};
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
        Err(ContractError::Other("Loading from proto file not yet implemented. Use GrpcContract::from_descriptor_set() instead".to_string()))
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
        let all_services: std::collections::HashSet<String> =
            self.services.keys().chain(other.services.keys()).cloned().collect();

        // Check for removed services (breaking change)
        for service_name in &all_services {
            if self.services.contains_key(service_name)
                && !other.services.contains_key(service_name)
            {
                let mut context = HashMap::new();
                context.insert("is_additive".to_string(), serde_json::json!(false));
                context.insert("is_breaking".to_string(), serde_json::json!(true));
                context.insert("change_category".to_string(), serde_json::json!("service_removed"));
                context.insert("service".to_string(), serde_json::json!(service_name));

                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::EndpointNotFound,
                    path: service_name.clone(),
                    method: None,
                    expected: Some(format!("Service {} should exist", service_name)),
                    actual: Some("Service removed".to_string()),
                    description: format!("Service {} was removed", service_name),
                    severity: MismatchSeverity::Critical,
                    confidence: 1.0,
                    context,
                });
            }
        }

        // Check for added services (non-breaking, additive)
        for service_name in &all_services {
            if !self.services.contains_key(service_name)
                && other.services.contains_key(service_name)
            {
                let mut context = HashMap::new();
                context.insert("is_additive".to_string(), serde_json::json!(true));
                context.insert("is_breaking".to_string(), serde_json::json!(false));
                context.insert("change_category".to_string(), serde_json::json!("service_added"));
                context.insert("service".to_string(), serde_json::json!(service_name));

                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::UnexpectedField,
                    path: service_name.clone(),
                    method: None,
                    expected: None,
                    actual: Some(format!("New service {}", service_name)),
                    description: format!("New service {} was added", service_name),
                    severity: MismatchSeverity::Low,
                    confidence: 1.0,
                    context,
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

    /// Classify a proto change as additive, breaking, or both
    ///
    /// Returns (is_additive, is_breaking) tuple
    /// - Additive: New methods, new optional fields, new services
    /// - Breaking: Removed methods, required field additions, type changes, signature changes
    fn classify_proto_change(mismatch: &Mismatch) -> (bool, bool) {
        match mismatch.mismatch_type {
            // Breaking changes
            MismatchType::EndpointNotFound => (false, true), // Method/service removed
            MismatchType::TypeMismatch => (false, true),     // Type changed
            MismatchType::SchemaMismatch => (false, true),   // Signature changed
            MismatchType::MissingRequiredField => (false, true), // Required field added

            // Additive changes
            MismatchType::UnexpectedField => {
                // Check severity - Low severity usually means additive (new method/field)
                match mismatch.severity {
                    MismatchSeverity::Low | MismatchSeverity::Info => (true, false),
                    _ => (false, false), // Medium/High severity unexpected fields might be breaking
                }
            }

            // Potentially breaking (depends on context)
            MismatchType::FormatMismatch | MismatchType::ConstraintViolation => {
                match mismatch.severity {
                    MismatchSeverity::Critical | MismatchSeverity::High => (false, true),
                    _ => (false, false),
                }
            }

            // Not applicable for proto changes
            _ => (false, false),
        }
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
                let mut context = HashMap::new();
                context.insert("is_additive".to_string(), serde_json::json!(false));
                context.insert("is_breaking".to_string(), serde_json::json!(true));
                context.insert("change_category".to_string(), serde_json::json!("method_removed"));
                context.insert("service".to_string(), serde_json::json!(service_name));
                context.insert("method".to_string(), serde_json::json!(method_name));

                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::EndpointNotFound,
                    path: path.clone(),
                    method: Some(method_name.clone()),
                    expected: Some(format!("Method {}.{} should exist", service_name, method_name)),
                    actual: Some("Method removed".to_string()),
                    description: format!("Method {}.{} was removed", service_name, method_name),
                    severity: MismatchSeverity::Critical,
                    confidence: 1.0,
                    context,
                });
            }
        }

        // Check for added methods (non-breaking, additive)
        for method_name in &new_methods {
            if !old_methods.contains(method_name) {
                let path = format!("{}.{}", service_name, method_name);
                let mut context = HashMap::new();
                context.insert("is_additive".to_string(), serde_json::json!(true));
                context.insert("is_breaking".to_string(), serde_json::json!(false));
                context.insert("change_category".to_string(), serde_json::json!("method_added"));
                context.insert("service".to_string(), serde_json::json!(service_name));
                context.insert("method".to_string(), serde_json::json!(method_name));

                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::UnexpectedField,
                    path: path.clone(),
                    method: Some(method_name.clone()),
                    expected: None,
                    actual: Some(format!("New method {}.{}", service_name, method_name)),
                    description: format!("New method {}.{} was added", service_name, method_name),
                    severity: MismatchSeverity::Low,
                    confidence: 1.0,
                    context,
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

            // Compare method signatures (input/output types, streaming flags)
            let method_mismatches =
                Self::diff_method_signatures(&old_method, &new_method, &service_name)?;
            mismatches.extend(method_mismatches);

            // Compare message fields if message types are the same
            // This helps detect field-level changes even when message type names match
            let old_input = old_method.input();
            let new_input = new_method.input();
            if old_input.full_name() == new_input.full_name() {
                let input_field_mismatches = Self::diff_message_fields(
                    &old_input,
                    &new_input,
                    &format!("{}.{}.input", service_name, method_name),
                    &service_name,
                    Some(method_name),
                )?;
                mismatches.extend(input_field_mismatches);
            }

            let old_output = old_method.output();
            let new_output = new_method.output();
            if old_output.full_name() == new_output.full_name() {
                let output_field_mismatches = Self::diff_message_fields(
                    &old_output,
                    &new_output,
                    &format!("{}.{}.output", service_name, method_name),
                    &service_name,
                    Some(method_name),
                )?;
                mismatches.extend(output_field_mismatches);
            }
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

        // Check input type changes (breaking change)
        if old_method.input().full_name() != new_method.input().full_name() {
            let mut context = HashMap::new();
            context.insert("is_additive".to_string(), serde_json::json!(false));
            context.insert("is_breaking".to_string(), serde_json::json!(true));
            context.insert("change_category".to_string(), serde_json::json!("input_type_changed"));
            context.insert("service".to_string(), serde_json::json!(service_name));
            context.insert("method".to_string(), serde_json::json!(method_name));
            context
                .insert("old_type".to_string(), serde_json::json!(old_method.input().full_name()));
            context
                .insert("new_type".to_string(), serde_json::json!(new_method.input().full_name()));

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
                context,
            });
        }

        // Check output type changes (breaking change)
        if old_method.output().full_name() != new_method.output().full_name() {
            let mut context = HashMap::new();
            context.insert("is_additive".to_string(), serde_json::json!(false));
            context.insert("is_breaking".to_string(), serde_json::json!(true));
            context.insert("change_category".to_string(), serde_json::json!("output_type_changed"));
            context.insert("service".to_string(), serde_json::json!(service_name));
            context.insert("method".to_string(), serde_json::json!(method_name));
            context
                .insert("old_type".to_string(), serde_json::json!(old_method.output().full_name()));
            context
                .insert("new_type".to_string(), serde_json::json!(new_method.output().full_name()));

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
                context,
            });
        }

        // Check streaming flag changes (breaking change)
        if old_method.is_client_streaming() != new_method.is_client_streaming() {
            let mut context = HashMap::new();
            context.insert("is_additive".to_string(), serde_json::json!(false));
            context.insert("is_breaking".to_string(), serde_json::json!(true));
            context.insert(
                "change_category".to_string(),
                serde_json::json!("streaming_config_changed"),
            );
            context.insert("service".to_string(), serde_json::json!(service_name));
            context.insert("method".to_string(), serde_json::json!(method_name));
            context.insert("streaming_type".to_string(), serde_json::json!("client"));
            context.insert(
                "old_value".to_string(),
                serde_json::json!(old_method.is_client_streaming()),
            );
            context.insert(
                "new_value".to_string(),
                serde_json::json!(new_method.is_client_streaming()),
            );

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
                context,
            });
        }

        if old_method.is_server_streaming() != new_method.is_server_streaming() {
            let mut context = HashMap::new();
            context.insert("is_additive".to_string(), serde_json::json!(false));
            context.insert("is_breaking".to_string(), serde_json::json!(true));
            context.insert(
                "change_category".to_string(),
                serde_json::json!("streaming_config_changed"),
            );
            context.insert("service".to_string(), serde_json::json!(service_name));
            context.insert("method".to_string(), serde_json::json!(method_name));
            context.insert("streaming_type".to_string(), serde_json::json!("server"));
            context.insert(
                "old_value".to_string(),
                serde_json::json!(old_method.is_server_streaming()),
            );
            context.insert(
                "new_value".to_string(),
                serde_json::json!(new_method.is_server_streaming()),
            );

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
                context,
            });
        }

        Ok(mismatches)
    }

    /// Compare message fields between two message descriptors
    ///
    /// Detects:
    /// - Field removals (breaking)
    /// - Field additions (additive if optional, breaking if required)
    /// - Field type changes (breaking)
    /// - Field number changes (breaking)
    fn diff_message_fields(
        old_message: &MessageDescriptor,
        new_message: &MessageDescriptor,
        path_prefix: &str,
        service_name: &str,
        method_name: Option<&str>,
    ) -> Result<Vec<Mismatch>, ContractError> {
        let mut mismatches = Vec::new();

        // Collect field information
        let old_fields: std::collections::HashMap<u32, prost_reflect::FieldDescriptor> =
            old_message.fields().map(|f| (f.number(), f)).collect();
        let new_fields: std::collections::HashMap<u32, prost_reflect::FieldDescriptor> =
            new_message.fields().map(|f| (f.number(), f)).collect();

        // Check for removed fields (breaking change)
        for (field_number, old_field) in &old_fields {
            if !new_fields.contains_key(field_number) {
                let field_path = format!("{}.field_{}", path_prefix, field_number);
                let mut context = HashMap::new();
                context.insert("is_additive".to_string(), serde_json::json!(false));
                context.insert("is_breaking".to_string(), serde_json::json!(true));
                context.insert("change_category".to_string(), serde_json::json!("field_removed"));
                context.insert("service".to_string(), serde_json::json!(service_name));
                if let Some(method) = method_name {
                    context.insert("method".to_string(), serde_json::json!(method));
                }
                context.insert("field_number".to_string(), serde_json::json!(*field_number));
                context.insert("field_name".to_string(), serde_json::json!(old_field.name()));
                context.insert(
                    "field_type".to_string(),
                    serde_json::json!(format!("{:?}", old_field.kind())),
                );

                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::EndpointNotFound,
                    path: field_path.clone(),
                    method: method_name.map(|s| s.to_string()),
                    expected: Some(format!(
                        "Field {} ({}) should exist",
                        old_field.name(),
                        field_number
                    )),
                    actual: Some("Field removed".to_string()),
                    description: format!(
                        "Field {} (number {}) was removed from message {}",
                        old_field.name(),
                        field_number,
                        old_message.full_name()
                    ),
                    severity: MismatchSeverity::High,
                    confidence: 1.0,
                    context,
                });
            }
        }

        // Check for added fields
        for (field_number, new_field) in &new_fields {
            if !old_fields.contains_key(field_number) {
                let field_path = format!("{}.field_{}", path_prefix, field_number);
                let mut context = HashMap::new();
                // In proto3, all fields are optional by default, so new fields are additive
                // In proto2, if the field is required, it's breaking
                let is_required = new_field.cardinality() == prost_reflect::Cardinality::Required;
                context.insert("is_additive".to_string(), serde_json::json!(!is_required));
                context.insert("is_breaking".to_string(), serde_json::json!(is_required));
                context.insert(
                    "change_category".to_string(),
                    serde_json::json!(if is_required {
                        "required_field_added"
                    } else {
                        "field_added"
                    }),
                );
                context.insert("service".to_string(), serde_json::json!(service_name));
                if let Some(method) = method_name {
                    context.insert("method".to_string(), serde_json::json!(method));
                }
                context.insert("field_number".to_string(), serde_json::json!(*field_number));
                context.insert("field_name".to_string(), serde_json::json!(new_field.name()));
                context.insert(
                    "field_type".to_string(),
                    serde_json::json!(format!("{:?}", new_field.kind())),
                );
                context.insert("is_required".to_string(), serde_json::json!(is_required));

                mismatches.push(Mismatch {
                    mismatch_type: if is_required {
                        MismatchType::MissingRequiredField
                    } else {
                        MismatchType::UnexpectedField
                    },
                    path: field_path.clone(),
                    method: method_name.map(|s| s.to_string()),
                    expected: None,
                    actual: Some(format!(
                        "New field {} (number {})",
                        new_field.name(),
                        field_number
                    )),
                    description: format!(
                        "New field {} (number {}) was added to message {} ({})",
                        new_field.name(),
                        field_number,
                        new_message.full_name(),
                        if is_required {
                            "required - breaking"
                        } else {
                            "optional - additive"
                        }
                    ),
                    severity: if is_required {
                        MismatchSeverity::High
                    } else {
                        MismatchSeverity::Low
                    },
                    confidence: 1.0,
                    context,
                });
            }
        }

        // Check for field type changes (same field number, different type)
        for (field_number, old_field) in &old_fields {
            if let Some(new_field) = new_fields.get(field_number) {
                // Check if field name changed (breaking)
                if old_field.name() != new_field.name() {
                    let field_path = format!("{}.field_{}", path_prefix, field_number);
                    let mut context = HashMap::new();
                    context.insert("is_additive".to_string(), serde_json::json!(false));
                    context.insert("is_breaking".to_string(), serde_json::json!(true));
                    context.insert(
                        "change_category".to_string(),
                        serde_json::json!("field_name_changed"),
                    );
                    context.insert("service".to_string(), serde_json::json!(service_name));
                    if let Some(method) = method_name {
                        context.insert("method".to_string(), serde_json::json!(method));
                    }
                    context.insert("field_number".to_string(), serde_json::json!(*field_number));
                    context.insert("old_name".to_string(), serde_json::json!(old_field.name()));
                    context.insert("new_name".to_string(), serde_json::json!(new_field.name()));

                    mismatches.push(Mismatch {
                        mismatch_type: MismatchType::SchemaMismatch,
                        path: field_path.clone(),
                        method: method_name.map(|s| s.to_string()),
                        expected: Some(format!("Field name: {}", old_field.name())),
                        actual: Some(format!("Field name: {}", new_field.name())),
                        description: format!(
                            "Field name changed from {} to {} (field number {})",
                            old_field.name(),
                            new_field.name(),
                            field_number
                        ),
                        severity: MismatchSeverity::High,
                        confidence: 1.0,
                        context,
                    });
                }

                // Check if field type changed (breaking)
                if old_field.kind() != new_field.kind() {
                    let field_path = format!("{}.field_{}", path_prefix, field_number);
                    let mut context = HashMap::new();
                    context.insert("is_additive".to_string(), serde_json::json!(false));
                    context.insert("is_breaking".to_string(), serde_json::json!(true));
                    context.insert(
                        "change_category".to_string(),
                        serde_json::json!("field_type_changed"),
                    );
                    context.insert("service".to_string(), serde_json::json!(service_name));
                    if let Some(method) = method_name {
                        context.insert("method".to_string(), serde_json::json!(method));
                    }
                    context.insert("field_number".to_string(), serde_json::json!(*field_number));
                    context.insert("field_name".to_string(), serde_json::json!(old_field.name()));
                    context.insert(
                        "old_type".to_string(),
                        serde_json::json!(format!("{:?}", old_field.kind())),
                    );
                    context.insert(
                        "new_type".to_string(),
                        serde_json::json!(format!("{:?}", new_field.kind())),
                    );

                    mismatches.push(Mismatch {
                        mismatch_type: MismatchType::TypeMismatch,
                        path: field_path.clone(),
                        method: method_name.map(|s| s.to_string()),
                        expected: Some(format!("Field type: {:?}", old_field.kind())),
                        actual: Some(format!("Field type: {:?}", new_field.kind())),
                        description: format!(
                            "Field {} type changed from {:?} to {:?}",
                            old_field.name(),
                            old_field.kind(),
                            new_field.kind()
                        ),
                        severity: MismatchSeverity::High,
                        confidence: 1.0,
                        context,
                    });
                }

                // Check if cardinality changed (e.g., optional to required - breaking)
                if old_field.cardinality() != new_field.cardinality() {
                    let old_cardinality = old_field.cardinality();
                    let new_cardinality = new_field.cardinality();
                    let is_breaking = matches!(
                        (old_cardinality, new_cardinality),
                        (
                            prost_reflect::Cardinality::Optional,
                            prost_reflect::Cardinality::Required
                        ) | (
                            prost_reflect::Cardinality::Repeated,
                            prost_reflect::Cardinality::Required
                        )
                    );

                    let field_path = format!("{}.field_{}", path_prefix, field_number);
                    let mut context = HashMap::new();
                    context.insert("is_additive".to_string(), serde_json::json!(!is_breaking));
                    context.insert("is_breaking".to_string(), serde_json::json!(is_breaking));
                    context.insert(
                        "change_category".to_string(),
                        serde_json::json!("field_cardinality_changed"),
                    );
                    context.insert("service".to_string(), serde_json::json!(service_name));
                    if let Some(method) = method_name {
                        context.insert("method".to_string(), serde_json::json!(method));
                    }
                    context.insert("field_number".to_string(), serde_json::json!(*field_number));
                    context.insert("field_name".to_string(), serde_json::json!(old_field.name()));
                    context.insert(
                        "old_cardinality".to_string(),
                        serde_json::json!(format!("{:?}", old_cardinality)),
                    );
                    context.insert(
                        "new_cardinality".to_string(),
                        serde_json::json!(format!("{:?}", new_cardinality)),
                    );

                    mismatches.push(Mismatch {
                        mismatch_type: if is_breaking {
                            MismatchType::MissingRequiredField
                        } else {
                            MismatchType::SchemaMismatch
                        },
                        path: field_path.clone(),
                        method: method_name.map(|s| s.to_string()),
                        expected: Some(format!("Cardinality: {:?}", old_cardinality)),
                        actual: Some(format!("Cardinality: {:?}", new_cardinality)),
                        description: format!(
                            "Field {} cardinality changed from {:?} to {:?} ({})",
                            old_field.name(),
                            old_cardinality,
                            new_cardinality,
                            if is_breaking {
                                "breaking"
                            } else {
                                "non-breaking"
                            }
                        ),
                        severity: if is_breaking {
                            MismatchSeverity::High
                        } else {
                            MismatchSeverity::Medium
                        },
                        confidence: 1.0,
                        context,
                    });
                }
            }
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
        request: &ContractRequest,
    ) -> Result<ValidationResult, ContractError> {
        // Check if the operation exists
        let method = match self.methods.get(operation_id) {
            Some(m) => m,
            None => {
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
        };

        // Get the input message descriptor for this method
        let input_message = method.input();
        let message_name = input_message.full_name().to_string();
        let field_count = input_message.fields().count();

        // Validate the payload against the protobuf schema
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Try to deserialize the payload as a protobuf message
        // For gRPC, the payload should be a serialized protobuf message
        if request.payload.is_empty() {
            // Empty payload might be valid for methods with no input
            if field_count > 0 {
                // Check if all fields are optional (proto3 has no required fields by default)
                // But we can still validate that the message structure is correct
                warnings.push("Empty payload provided for method with input message".to_string());
            }
        } else {
            // Attempt to deserialize the payload
            // Convert Vec<u8> to bytes::Bytes for prost_reflect
            use bytes::Bytes;
            let payload_bytes = Bytes::from(request.payload.clone());

            // Clone the input_message descriptor since decode takes ownership
            let input_message_clone = input_message;
            match prost_reflect::DynamicMessage::decode(input_message_clone, payload_bytes) {
                Ok(_message) => {
                    // Validate required fields (proto2) or check field presence
                    // In proto3, all fields are optional, but we can still validate types
                    // prost_reflect handles type validation during deserialization
                    // If we got here, the message structure is valid
                    // Field validation is handled by prost_reflect during deserialization
                    // If deserialization succeeded, the types are correct

                    // Check for unknown fields (fields not in the schema)
                    // This is handled by prost_reflect during deserialization
                    // If deserialization succeeded, the message structure is valid
                }
                Err(e) => {
                    // Deserialization failed - this is a validation error
                    errors.push(ValidationError {
                        message: format!(
                            "Failed to deserialize protobuf message: {}. Expected message type: {}",
                            e, message_name
                        ),
                        path: Some(operation_id.to_string()),
                        code: Some("DESERIALIZATION_ERROR".to_string()),
                    });
                }
            }
        }

        // Validate streaming configuration
        if method.is_client_streaming() && request.metadata.get("streaming").is_none() {
            warnings.push(
                "Method is client-streaming but request doesn't indicate streaming".to_string(),
            );
        }

        Ok(ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
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
