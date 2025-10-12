//! Request validation and routing
//!
//! This module provides validation functionality for gRPC requests,
//! including service/method validation and request routing.

use crate::reflection::mock_proxy::proxy::MockReflectionProxy;
use mockforge_core::openapi_routes::ValidationMode;
use prost::bytes::Bytes as ProstBytes;
use prost_reflect::ReflectMessage;
use prost_reflect::{DynamicMessage, Kind, MessageDescriptor, Value};
use tonic::{Request, Status};
use tracing::debug;

use prost_reflect::prost::Message;

impl MockReflectionProxy {
    /// Validate a request against the service method schema
    pub async fn validate_request(
        &self,
        request: &Request<DynamicMessage>,
        service_name: &str,
        method_name: &str,
    ) -> Result<(), Status> {
        debug!("Validating request for {}/{}", service_name, method_name);

        // Get method descriptor for validation
        let method_descriptor = self.cache.get_method(service_name, method_name).await?;

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
        let method_descriptor = self.cache.get_method(service_name, method_name).await?;
        let expected_descriptor = method_descriptor.input();

        let encoded = request.get_ref().encode_to_vec();
        let dynamic_message =
            DynamicMessage::decode(expected_descriptor.clone(), ProstBytes::from(encoded))
                .map_err(|e| {
                    Status::invalid_argument(format!(
                        "Failed to decode request as DynamicMessage: {}",
                        e
                    ))
                })?;

        // Validate field types and presence
        Self::validate_dynamic_message_fields(&dynamic_message, &expected_descriptor, "request")?;

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
        let method_descriptor = self.cache.get_method(service_name, method_name).await?;

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
        Self::validate_dynamic_message_fields(response, &expected_descriptor, "response")?;

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
        let contains_service = self.cache.contains_service(&service_name).await;
        if !contains_service {
            return Err(Status::not_found(format!("Service {} not found", service_name)));
        }

        if self.cache.get_method(&service_name, &method_name).await.is_err() {
            return Err(Status::not_found(format!(
                "Method {} not found in service {}",
                method_name, service_name
            )));
        }

        Ok((service_name.to_string(), method_name.to_string(), request))
    }

    /// Check if a service method should be processed by this proxy
    pub async fn can_handle_service_method(&self, service_name: &str, method_name: &str) -> bool {
        // Check if service exists in cache
        if !self.cache.contains_service(service_name).await {
            return false;
        }

        // Check if method exists in service
        if !self.cache.contains_method(service_name, method_name).await {
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
        let cached_descriptor = self.cache.get_method(service_name, method_name).await?;

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
        Self::check_message_compatibility(&cached_descriptor.input(), &input_descriptor, "input")?;
        Self::check_message_compatibility(
            &cached_descriptor.output(),
            &output_descriptor,
            "output",
        )?;

        debug!("Signature validation passed for {}/{}", service_name, method_name);
        Ok(())
    }

    /// Check if two message descriptors are compatible (no breaking changes)
    fn check_message_compatibility(
        expected: &MessageDescriptor,
        provided: &MessageDescriptor,
        message_type: &str,
    ) -> Result<(), Status> {
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
                if let prost_reflect::Kind::Message(expected_msg) = expected_field.kind() {
                    if let prost_reflect::Kind::Message(provided_msg) = provided_field.kind() {
                        if expected_msg.full_name() != provided_msg.full_name() {
                            // Recursively check nested messages
                            Self::check_message_compatibility(
                                &expected_msg,
                                &provided_msg,
                                &format!("{}.{}", message_type, field_name),
                            )?;
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
        for field in descriptor.fields() {
            let field_name = field.name();

            let value = message.get_field(&field);
            let value_ref = value.as_ref();
            // Check if the value kind matches the field kind
            if !Self::value_matches_kind(value_ref, field.kind()) {
                return Err(Status::invalid_argument(format!(
                    "{} field '{}' has incorrect type: expected {:?}, got {:?}",
                    context,
                    field_name,
                    field.kind(),
                    value_ref
                )));
            }

            // For nested messages, recursively validate
            if let Kind::Message(expected_msg) = field.kind() {
                if let Value::Message(ref nested_msg) = *value_ref {
                    Self::validate_dynamic_message_fields(
                        nested_msg,
                        &expected_msg,
                        &format!("{}.{}", context, field_name),
                    )?;
                }
            }
        }

        Ok(())
    }

    /// Check if a Value matches a Kind
    pub fn value_matches_kind(value: &Value, kind: prost_reflect::Kind) -> bool {
        match *value {
            prost_reflect::Value::Bool(_) => kind == prost_reflect::Kind::Bool,
            prost_reflect::Value::I32(_) => matches!(
                kind,
                prost_reflect::Kind::Int32
                    | prost_reflect::Kind::Sint32
                    | prost_reflect::Kind::Sfixed32
            ),
            prost_reflect::Value::I64(_) => matches!(
                kind,
                prost_reflect::Kind::Int64
                    | prost_reflect::Kind::Sint64
                    | prost_reflect::Kind::Sfixed64
            ),
            prost_reflect::Value::U32(_) => {
                matches!(kind, prost_reflect::Kind::Uint32 | prost_reflect::Kind::Fixed32)
            }
            prost_reflect::Value::U64(_) => {
                matches!(kind, prost_reflect::Kind::Uint64 | prost_reflect::Kind::Fixed64)
            }
            prost_reflect::Value::F32(_) => kind == prost_reflect::Kind::Float,
            prost_reflect::Value::F64(_) => kind == prost_reflect::Kind::Double,
            prost_reflect::Value::String(_) => kind == prost_reflect::Kind::String,
            prost_reflect::Value::Bytes(_) => kind == prost_reflect::Kind::Bytes,
            prost_reflect::Value::Message(_) => matches!(kind, prost_reflect::Kind::Message(_)),
            prost_reflect::Value::List(_) => matches!(kind, prost_reflect::Kind::Message(_)), // Lists are for repeated messages
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
        if let Some(mode) = self.config.overrides.get(&format!("{}/{}", service_name, method_name))
        {
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

#[cfg(test)]
mod tests {

    #[test]
    fn test_module_compiles() {}
}
