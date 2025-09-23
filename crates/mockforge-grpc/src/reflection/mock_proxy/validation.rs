//! Request validation and routing
//!
//! This module provides validation functionality for gRPC requests,
//! including service/method validation and request routing.

use crate::reflection::mock_proxy::proxy::MockReflectionProxy;
use prost_reflect::{DynamicMessage, MessageDescriptor};
use prost_types::value::Kind;
use serde_json::Value;
use tonic::{Request, Status};
use tracing::{debug, warn};
use mockforge_core::openapi_routes::ValidationMode;

use prost_reflect::prost::Message;

impl MockReflectionProxy {
    /// Validate a request against the service method schema
    pub async fn validate_request<T>(
        &self,
        request: &Request<T>,
        service_name: &str,
        method_name: &str,
    ) -> Result<(), Status>
    where
        T: Message,
    {
        debug!("Validating request for {}/{}", service_name, method_name);

        // Get method descriptor for validation
        let method_descriptor = self
            .cache
            .get_method_descriptor(service_name, method_name)
            .ok_or_else(|| Status::not_found("Method not found in cache"))?;

        // Get expected input descriptor
        let expected_descriptor = method_descriptor.input();

        // Get actual descriptor from the request message
        let actual_descriptor = request.get_ref().descriptor();

        // Check if the request descriptor matches the expected input type
        if actual_descriptor.full_name() != expected_descriptor.full_name() {
            return Err(Status::invalid_argument(format!(
                "Request type mismatch: expected {}, got {}",
                expected_descriptor.full_name(),
                actual_descriptor.full_name()
            )));
        }

        // Convert the typed message to DynamicMessage for field validation
        let dynamic_message = DynamicMessage::decode(
            expected_descriptor.clone(),
            &request.get_ref().encode_to_vec(),
        )
        .map_err(|e| Status::invalid_argument(format!("Failed to decode request as DynamicMessage: {}", e)))?;

        // Validate field types and presence
        Self::validate_dynamic_message_fields(&dynamic_message, expected_descriptor, "request")?;

        debug!("Request validation passed for {}/{}", service_name, method_name);
        Ok(())
    }

    /// Validate response data against the method's response schema
    pub async fn validate_response(
        &self,
        response: &DynamicMessage,
        service_name: &str,
        method_name: &str,
    ) -> Result<(), Status> {
        debug!("Validating response for {}/{}", service_name, method_name);

        // Get method descriptor for validation
        let method_descriptor = self
            .cache
            .get_method_descriptor(service_name, method_name)
            .ok_or_else(|| Status::not_found("Method not found in cache"))?;

        // Validate response against protobuf schema
        let expected_descriptor = method_descriptor.output();

        // Check if the response descriptor matches
        if response.descriptor().full_name() != expected_descriptor.full_name() {
            return Err(Status::invalid_argument(format!(
                "Response type mismatch: expected {}, got {}",
                expected_descriptor.full_name(),
                response.descriptor().full_name()
            )));
        }

        // Validate field types and presence
        Self::validate_dynamic_message_fields(response, expected_descriptor, "response")?;

        debug!("Response validation passed for {}/{}", service_name, method_name);
        Ok(())
    }

    /// Route a request to the appropriate handler
    pub async fn route_request<T>(
        &self,
        request: Request<T>,
    ) -> Result<(String, String, Request<T>), Status> {
        // Extract service and method from request metadata
        let (service_name, method_name) = self.extract_service_method_from_request(&request)?;

        // Validate that the service and method exist
        if !self.cache.has_service(&service_name) {
            return Err(Status::not_found(format!("Service {} not found", service_name)));
        }

        if !self.cache.has_method(&service_name, &method_name) {
            return Err(Status::not_found(format!("Method {} not found in service {}", method_name, service_name)));
        }

        Ok((service_name, method_name, request))
    }

    /// Check if a service method should be processed by this proxy
    pub fn can_handle_service_method(&self, service_name: &str, method_name: &str) -> bool {
        // Check if service exists in cache
        if !self.cache.has_service(service_name) {
            return false;
        }

        // Check if method exists in service
        if !self.cache.has_method(service_name, method_name) {
            return false;
        }

        true
    }

    /// Validate service method signature compatibility
    pub async fn validate_service_method_signature(
        &self,
        service_name: &str,
        method_name: &str,
        input_descriptor: MessageDescriptor,
        output_descriptor: MessageDescriptor,
    ) -> Result<(), Status> {
        debug!("Validating signature for {}/{}", service_name, method_name);

        // Check if method exists in cache
        let cached_descriptor = self
            .cache
            .get_method_descriptor(service_name, method_name)
            .ok_or_else(|| Status::not_found("Method not found in cache"))?;

        // Compare input/output types
        if input_descriptor.full_name() != cached_descriptor.input().full_name() {
            return Err(Status::invalid_argument(format!(
                "Input type mismatch: expected {}, got {}",
                cached_descriptor.input().full_name(),
                input_descriptor.full_name()
            )));
        }

        if output_descriptor.full_name() != cached_descriptor.output().full_name() {
            return Err(Status::invalid_argument(format!(
                "Output type mismatch: expected {}, got {}",
                cached_descriptor.output().full_name(),
                output_descriptor.full_name()
            )));
        }

        // Validate field compatibility and check for breaking changes
        Self::check_message_compatibility(cached_descriptor.input(), &input_descriptor, "input")?;
        Self::check_message_compatibility(cached_descriptor.output(), &output_descriptor, "output")?;

        debug!("Signature validation passed for {}/{}", service_name, method_name);
        Ok(())
    }

    /// Check if two message descriptors are compatible (no breaking changes)
    fn check_message_compatibility(
        expected: &MessageDescriptor,
        provided: &MessageDescriptor,
        message_type: &str,
    ) -> Result<(), Status> {
        use prost_reflect::Kind;

        for expected_field in expected.fields() {
            let field_name = expected_field.name();
            if let Some(provided_field) = provided.get_field_by_name(field_name) {
                // Check if kinds match
                if expected_field.kind() != provided_field.kind() {
                    return Err(Status::invalid_argument(format!(
                        "{} field '{}' type mismatch: expected {:?}, got {:?}",
                        message_type,
                        field_name,
                        expected_field.kind(),
                        provided_field.kind()
                    )));
                }

                // For message types, check nested compatibility if full names differ
                if let Kind::Message(expected_msg) = expected_field.kind() {
                    if let Kind::Message(provided_msg) = provided_field.kind() {
                        if expected_msg.full_name() != provided_msg.full_name() {
                            // Recursively check nested messages
                            Self::check_message_compatibility(&expected_msg, &provided_msg, &format!("{}.{}", message_type, field_name))?;
                        }
                    }
                }
            } else {
                return Err(Status::invalid_argument(format!(
                    "Missing {} field '{}' in provided descriptor",
                    message_type, field_name
                )));
            }
        }

        Ok(())
    }

    /// Validate fields of a DynamicMessage against its descriptor
    fn validate_dynamic_message_fields(
        message: &DynamicMessage,
        descriptor: &MessageDescriptor,
        context: &str,
    ) -> Result<(), Status> {
        use prost_reflect::{Kind, Value};

        for field in descriptor.fields() {
            let field_name = field.name();
            let field_number = field.number();

            if let Some(value) = message.get_field(field_number) {
                // Check if the value kind matches the field kind
                if !Self::value_matches_kind(&value, field.kind()) {
                    return Err(Status::invalid_argument(format!(
                        "{} field '{}' has incorrect type: expected {:?}, got {:?}",
                        context, field_name, field.kind(), value
                    )));
                }

                // For nested messages, recursively validate
                if let (Kind::Message(expected_msg), Value::Message(nested_msg)) = (field.kind(), &value) {
                    Self::validate_dynamic_message_fields(nested_msg, &expected_msg, &format!("{}.{}", context, field_name))?;
                }
            } else {
                // In proto3, fields are optional, so missing is ok
                // But if we want to check for required, we could, but proto3 has no required
            }
        }

        Ok(())
    }

    /// Check if a Value matches a Kind
    fn value_matches_kind(value: &Value, kind: Kind) -> bool {
        match (value, kind) {
            (Value::Bool(_), Kind::Bool) => true,
            (Value::I32(_) | Value::I64(_), Kind::Int32 | Kind::Int64 | Kind::Sint32 | Kind::Sint64 | Kind::Sfixed32 | Kind::Sfixed64) => true,
            (Value::U32(_) | Value::U64(_), Kind::Uint32 | Kind::Uint64 | Kind::Fixed32 | Kind::Fixed64) => true,
            (Value::F32(_), Kind::Float) => true,
            (Value::F64(_), Kind::Double) => true,
            (Value::String(_), Kind::String) => true,
            (Value::Bytes(_), Kind::Bytes) => true,
            (Value::Message(_), Kind::Message(_)) => true,
            (Value::Enum(_, _), Kind::Enum(_)) => true,
            (Value::List(_), Kind::Message(_)) => false, // Lists are for repeated, but Kind::Message is for nested
            _ => false,
        }
    }

    /// Validate request size limits
    pub async fn validate_request_size<T>(
        &self,
        request: &Request<T>,
        max_size: usize,
    ) -> Result<(), Status>
    where
        T: Message,
    {
        // Encode the request to get its serialized size
        let encoded_size = request.get_ref().encode_to_vec().len();

        // Check if the request size exceeds the configured limit
        if encoded_size > max_size {
            return Err(Status::resource_exhausted(format!(
                "Request size {} bytes exceeds maximum allowed size of {} bytes",
                encoded_size, max_size
            )));
        }

        Ok(())
    }

    /// Validate response size limits
    pub async fn validate_response_size(
        &self,
        response: &DynamicMessage,
        max_size: usize,
    ) -> Result<(), Status> {
        // Encode the response to get its serialized size
        let encoded_size = response.encode_to_vec().len();

        // Check if the response size exceeds the configured limit
        if encoded_size > max_size {
            return Err(Status::resource_exhausted(format!(
                "Response size {} bytes exceeds maximum allowed size of {} bytes",
                encoded_size, max_size
            )));
        }

        Ok(())
    }

    /// Check if request should be skipped for validation (admin endpoints, etc.)
    pub fn should_skip_validation(&self, service_name: &str, method_name: &str) -> bool {
        // Check admin skip prefixes from config
        for prefix in &self.config.admin_skip_prefixes {
            if service_name.starts_with(prefix) || method_name.starts_with(prefix) {
                return true;
            }
        }

        false
    }

    /// Apply validation mode for a service method
    pub fn get_validation_mode_for_method(
        &self,
        service_name: &str,
        method_name: &str,
    ) -> ValidationMode {
        // Check for method-specific overrides
        if let Some(mode) = self.config.overrides.get(&format!("{}/{}", service_name, method_name)) {
            return mode.clone();
        }

        // Check for service-specific overrides
        if let Some(mode) = self.config.overrides.get(service_name) {
            return mode.clone();
        }

        // Return default mode
        self.config.request_mode.clone()
    }
}
