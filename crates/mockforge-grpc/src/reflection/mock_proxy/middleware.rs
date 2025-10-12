//! Request processing middleware
//!
//! This module provides middleware for processing gRPC requests,
//! including request transformation, logging, and metrics collection.

use crate::reflection::metrics::{record_error, record_success};
use crate::reflection::mock_proxy::proxy::MockReflectionProxy;
use prost_reflect::{DynamicMessage, Kind, ReflectMessage};
use std::time::Instant;
use tonic::{
    metadata::{Ascii, MetadataKey, MetadataValue},
    Code, Request, Status,
};
use tracing::error;

impl MockReflectionProxy {
    /// Apply request preprocessing middleware
    pub async fn preprocess_request<T>(&self, request: &mut Request<T>) -> Result<(), Status>
    where
        T: prost_reflect::ReflectMessage,
    {
        // Extract metadata
        let mut metadata_log = Vec::new();
        for kv in request.metadata().iter() {
            match kv {
                tonic::metadata::KeyAndValueRef::Ascii(key, value) => {
                    metadata_log.push(format!("{}: {}", key, value.to_str().unwrap_or("<binary>")));
                }
                tonic::metadata::KeyAndValueRef::Binary(key, _) => {
                    metadata_log.push(format!("{}: <binary>", key));
                }
            }
        }
        tracing::debug!("Extracted request metadata: [{}]", metadata_log.join(", "));

        // Validate request format
        let descriptor = request.get_ref().descriptor();
        let mut buf = Vec::new();
        request
            .get_ref()
            .encode(&mut buf)
            .map_err(|_e| Status::internal("Failed to encode request".to_string()))?;
        let dynamic_message = DynamicMessage::decode(descriptor.clone(), &buf[..])
            .map_err(|_e| Status::internal("Failed to decode request".to_string()))?;
        if let Err(e) = self.validate_request_message(&dynamic_message) {
            return Err(Status::internal(format!("Request validation failed: {}", e)));
        }
        tracing::debug!("Request format validation passed");

        // Apply request transformations
        // Add mock-specific request headers
        request.metadata_mut().insert("x-mockforge-processed", "true".parse().unwrap());
        request
            .metadata_mut()
            .insert("x-mockforge-timestamp", chrono::Utc::now().to_rfc3339().parse().unwrap());

        tracing::debug!("Applied request transformations: added processed and timestamp headers");

        Ok(())
    }

    /// Apply request logging middleware
    pub async fn log_request<T>(&self, request: &Request<T>, service_name: &str, method_name: &str)
    where
        T: prost_reflect::ReflectMessage,
    {
        let start_time = std::time::Instant::now();

        // Log request metadata
        let mut metadata_log = Vec::new();
        for kv in request.metadata().iter() {
            match kv {
                tonic::metadata::KeyAndValueRef::Ascii(key, value) => {
                    metadata_log.push(format!("{}: {}", key, value.to_str().unwrap_or("<binary>")));
                }
                tonic::metadata::KeyAndValueRef::Binary(key, _) => {
                    metadata_log.push(format!("{}: <binary>", key));
                }
            }
        }
        tracing::debug!(
            "Request metadata for {}/{}: [{}]",
            service_name,
            method_name,
            metadata_log.join(", ")
        );

        // Log request size
        let request_size = request.get_ref().encoded_len();
        tracing::debug!(
            "Request size for {}/{}: {} bytes",
            service_name,
            method_name,
            request_size
        );

        // Log request timing (start time)
        tracing::debug!(
            "Request start time for {}/{}: {:?}",
            service_name,
            method_name,
            start_time
        );
    }

    /// Apply response postprocessing middleware
    pub async fn postprocess_response<T>(
        &self,
        response: &mut tonic::Response<T>,
        service_name: &str,
        method_name: &str,
    ) -> Result<(), Status> {
        let start = Instant::now();
        // Add mock-specific response headers
        response.metadata_mut().insert("x-mockforge-processed", "true".parse().unwrap());
        response
            .metadata_mut()
            .insert("x-mockforge-timestamp", chrono::Utc::now().to_rfc3339().parse().unwrap());

        // // Add processing timestamp for performance monitoring
        // let processing_time = std::time::SystemTime::now()
        //     .duration_since(std::time::UNIX_EPOCH)
        //     .unwrap()
        //     .as_millis();
        // response
        //     .metadata_mut()
        //     .insert("x-mockforge-processing-time", MetadataValue::<Ascii>::from(processing_time.to_string()));

        // Apply response transformations based on configuration
        if self.config.response_transform.enabled {
            // Add custom headers from configuration
            for (key, value) in &self.config.response_transform.custom_headers {
                let key: MetadataKey<Ascii> = key.parse().unwrap();
                let value: MetadataValue<Ascii> = value.parse().unwrap();
                response.metadata_mut().insert(key, value);
            }
        }

        // Log response processing
        let processing_time = start.elapsed().as_millis();
        // Add processing timestamp for performance monitoring
        response
            .metadata_mut()
            .insert("x-mockforge-processing-time", processing_time.to_string().parse().unwrap());
        tracing::debug!("Postprocessed response for {}/{}", service_name, method_name);

        Ok(())
    }

    /// Apply response postprocessing with body transformations for DynamicMessage responses
    pub async fn postprocess_dynamic_response(
        &self,
        response: &mut tonic::Response<prost_reflect::DynamicMessage>,
        service_name: &str,
        method_name: &str,
    ) -> Result<(), Status> {
        // First apply basic postprocessing
        self.postprocess_response(response, service_name, method_name).await?;

        // Apply body transformations if enabled
        if self.config.response_transform.enabled {
            if let Some(ref overrides) = self.config.response_transform.overrides {
                match self
                    .transform_dynamic_message(
                        &response.get_ref().clone(),
                        service_name,
                        method_name,
                        overrides,
                    )
                    .await
                {
                    Ok(transformed_message) => {
                        // Replace the response body
                        *response.get_mut() = transformed_message;
                        tracing::debug!(
                            "Applied body transformations to response for {}/{}",
                            service_name,
                            method_name
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to transform response body for {}/{}: {}",
                            service_name,
                            method_name,
                            e
                        );
                    }
                }
            }

            // Response validation
            if self.config.response_transform.validate_responses {
                if let Err(validation_error) = self
                    .validate_dynamic_message(response.get_ref(), service_name, method_name)
                    .await
                {
                    tracing::warn!(
                        "Response validation failed for {}/{}: {}",
                        service_name,
                        method_name,
                        validation_error
                    );
                }
            }
        }

        Ok(())
    }

    /// Transform a DynamicMessage using JSON overrides
    async fn transform_dynamic_message(
        &self,
        message: &prost_reflect::DynamicMessage,
        service_name: &str,
        method_name: &str,
        overrides: &mockforge_core::overrides::Overrides,
    ) -> Result<prost_reflect::DynamicMessage, Box<dyn std::error::Error + Send + Sync>> {
        use crate::dynamic::http_bridge::converters::ProtobufJsonConverter;

        // Get descriptor pool from service registry
        let descriptor_pool = self.service_registry.descriptor_pool();

        // Create a converter for JSON transformations
        let converter = ProtobufJsonConverter::new(descriptor_pool.clone());

        // Convert protobuf message to JSON
        let json_value = converter.protobuf_to_json(&message.descriptor(), message)?;

        // Apply overrides to the JSON
        let mut json_value = serde_json::Value::Object(json_value.as_object().unwrap().clone());
        overrides.apply_with_context(
            &format!("{}/{}", service_name, method_name),
            &[service_name.to_string()],
            &format!("{}/{}", service_name, method_name),
            &mut json_value,
            &mockforge_core::conditions::ConditionContext::new(),
        );

        // Convert back to protobuf message
        let transformed_message = converter.json_to_protobuf(&message.descriptor(), &json_value)?;

        Ok(transformed_message)
    }

    /// Apply response postprocessing for streaming DynamicMessage responses
    pub async fn postprocess_streaming_dynamic_response(
        &self,
        response: &mut tonic::Response<
            tokio_stream::wrappers::ReceiverStream<
                Result<prost_reflect::DynamicMessage, tonic::Status>,
            >,
        >,
        service_name: &str,
        method_name: &str,
    ) -> Result<(), Status> {
        // Apply basic postprocessing (headers only for streaming responses)
        self.postprocess_response(response, service_name, method_name).await?;

        // Note: Body transformation for streaming responses is complex and not yet implemented
        // It would require creating a new stream that transforms each message individually,
        // which involves significant async complexity and descriptor pool management.

        if self.config.response_transform.enabled {
            if self.config.response_transform.overrides.is_some() {
                tracing::debug!(
                    "Body transformation for streaming responses not yet implemented for {}/{}",
                    service_name,
                    method_name
                );
            }

            if self.config.response_transform.validate_responses {
                tracing::debug!(
                    "Response validation for streaming responses not yet implemented for {}/{}",
                    service_name,
                    method_name
                );
            }
        }

        Ok(())
    }

    /// Validate a DynamicMessage response
    async fn validate_dynamic_message(
        &self,
        message: &prost_reflect::DynamicMessage,
        service_name: &str,
        method_name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Basic validation: check that required fields are present
        let _descriptor = message.descriptor();

        // Note: In proto3, all fields are effectively optional
        // Required field validation removed as is_required() method is no longer available

        // Schema validation against expected message structure
        // For protobuf, the message structure is validated by the descriptor,
        // but we can check field constraints
        self.validate_message_schema(message, service_name, method_name)?;

        // Business rule validation (e.g., email format, date ranges)
        self.validate_business_rules(message, service_name, method_name)?;

        // Cross-field validation
        self.validate_cross_field_rules(message, service_name, method_name)?;

        // Custom validation rules from configuration
        self.validate_custom_rules(message, service_name, method_name)?;

        tracing::debug!("Response validation passed for {}/{}", service_name, method_name);

        Ok(())
    }

    /// Validate a request DynamicMessage
    fn validate_request_message(
        &self,
        message: &DynamicMessage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Schema validation
        self.validate_message_schema(message, "", "")?;
        // Business rule validation
        self.validate_business_rules(message, "", "")?;
        // Cross-field validation
        self.validate_cross_field_rules(message, "", "")?;
        // Custom validation
        self.validate_custom_rules(message, "", "")?;
        tracing::debug!("Request validation passed");
        Ok(())
    }

    /// Validate message schema constraints
    fn validate_message_schema(
        &self,
        message: &DynamicMessage,
        _service_name: &str,
        _method_name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let descriptor = message.descriptor();

        // Check field types and constraints
        for field in descriptor.fields() {
            let value = message.get_field(&field);
            let value_ref = value.as_ref();

            // Check if the value kind matches the field kind
            if !Self::value_matches_kind(value_ref, field.kind()) {
                return Err(format!(
                    "{} field '{}' has incorrect type: expected {:?}, got {:?}",
                    "Message validation",
                    field.name(),
                    field.kind(),
                    value_ref
                )
                .into());
            }

            // For nested messages, recursively validate
            if let Kind::Message(expected_msg) = field.kind() {
                if let prost_reflect::Value::Message(ref nested_msg) = *value_ref {
                    // Basic nested message validation - could be expanded
                    if nested_msg.descriptor() != expected_msg {
                        return Err(format!(
                            "{} field '{}' has incorrect message type",
                            "Message validation",
                            field.name()
                        )
                        .into());
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate business rules (email format, date ranges, etc.)
    fn validate_business_rules(
        &self,
        message: &DynamicMessage,
        service_name: &str,
        method_name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let descriptor = message.descriptor();

        for field in descriptor.fields() {
            let value = message.get_field(&field);
            let field_value = value.as_ref();
            let field_name = field.name().to_lowercase();

            // Email validation
            if field_name.contains("email") && field.kind() == Kind::String {
                if let Some(email_str) = field_value.as_str() {
                    if !self.is_valid_email(email_str) {
                        return Err(format!(
                            "Invalid email format '{}' for field '{}' in {}/{}",
                            email_str,
                            field.name(),
                            service_name,
                            method_name
                        )
                        .into());
                    }
                }
            }

            // Date/timestamp validation
            if field_name.contains("date") || field_name.contains("timestamp") {
                match field.kind() {
                    Kind::String => {
                        if let Some(date_str) = field_value.as_str() {
                            if !self.is_valid_iso8601_date(date_str) {
                                return Err(format!(
                                    "Invalid date format '{}' for field '{}' in {}/{}",
                                    date_str,
                                    field.name(),
                                    service_name,
                                    method_name
                                )
                                .into());
                            }
                        }
                    }
                    Kind::Int64 | Kind::Uint64 => {
                        // For timestamp fields, check reasonable range (1970-2100)
                        if let Some(timestamp) = field_value.as_i64() {
                            if !(0..=4102444800).contains(&timestamp) {
                                // 2100-01-01
                                return Err(format!(
                                    "Timestamp {} out of reasonable range for field '{}' in {}/{}",
                                    timestamp,
                                    field.name(),
                                    service_name,
                                    method_name
                                )
                                .into());
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Phone number validation (basic)
            if field_name.contains("phone") && field.kind() == Kind::String {
                if let Some(phone_str) = field_value.as_str() {
                    if !self.is_valid_phone_number(phone_str) {
                        return Err(format!(
                            "Invalid phone number format '{}' for field '{}' in {}/{}",
                            phone_str,
                            field.name(),
                            service_name,
                            method_name
                        )
                        .into());
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate cross-field rules
    fn validate_cross_field_rules(
        &self,
        message: &DynamicMessage,
        service_name: &str,
        method_name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let descriptor = message.descriptor();

        // Collect date/time fields for cross-validation
        let mut date_fields = Vec::new();
        let mut timestamp_fields = Vec::new();

        for field in descriptor.fields() {
            let value = message.get_field(&field);
            let field_value = value.as_ref();
            let field_name = field.name().to_lowercase();

            if field_name.contains("start")
                && (field_name.contains("date") || field_name.contains("time"))
            {
                if let Some(value) = field_value.as_i64() {
                    date_fields.push(("start", value));
                }
            } else if field_name.contains("end")
                && (field_name.contains("date") || field_name.contains("time"))
            {
                if let Some(value) = field_value.as_i64() {
                    date_fields.push(("end", value));
                }
            } else if field_name.contains("timestamp") {
                if let Some(value) = field_value.as_i64() {
                    timestamp_fields.push((field.name().to_string(), value));
                }
            }
        }

        // Validate start_date < end_date
        if date_fields.len() >= 2 {
            let start_dates: Vec<_> = date_fields.iter().filter(|(t, _)| *t == "start").collect();
            let end_dates: Vec<_> = date_fields.iter().filter(|(t, _)| *t == "end").collect();

            for &(_, start_val) in &start_dates {
                for &(_, end_val) in &end_dates {
                    if start_val >= end_val {
                        return Err(format!(
                            "Start date/time {} must be before end date/time {} in {}/{}",
                            start_val, end_val, service_name, method_name
                        )
                        .into());
                    }
                }
            }
        }

        // Validate timestamp ranges (e.g., created_at <= updated_at)
        if timestamp_fields.len() >= 2 {
            let created_at = timestamp_fields
                .iter()
                .find(|(name, _)| name.to_lowercase().contains("created"));
            let updated_at = timestamp_fields
                .iter()
                .find(|(name, _)| name.to_lowercase().contains("updated"));

            if let (Some((_, created)), Some((_, updated))) = (created_at, updated_at) {
                if created > updated {
                    return Err(format!(
                        "Created timestamp {} cannot be after updated timestamp {} in {}/{}",
                        created, updated, service_name, method_name
                    )
                    .into());
                }
            }
        }

        Ok(())
    }

    /// Validate custom rules from configuration
    fn validate_custom_rules(
        &self,
        message: &DynamicMessage,
        service_name: &str,
        method_name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // For now, implement basic custom validation based on field names and values
        // In a full implementation, this would read from a configuration file

        let descriptor = message.descriptor();

        for field in descriptor.fields() {
            let value = message.get_field(&field);
            let field_value = value.as_ref();
            let field_name = field.name().to_lowercase();

            // Custom rule: ID fields should be positive
            if field_name.ends_with("_id") || field_name == "id" {
                match field.kind() {
                    Kind::Int32 | Kind::Int64 => {
                        if let Some(id_val) = field_value.as_i64() {
                            if id_val <= 0 {
                                return Err(format!(
                                    "ID field '{}' must be positive, got {} in {}/{}",
                                    field.name(),
                                    id_val,
                                    service_name,
                                    method_name
                                )
                                .into());
                            }
                        }
                    }
                    Kind::Uint32 | Kind::Uint64 => {
                        if let Some(id_val) = field_value.as_u64() {
                            if id_val == 0 {
                                return Err(format!(
                                    "ID field '{}' must be non-zero, got {} in {}/{}",
                                    field.name(),
                                    id_val,
                                    service_name,
                                    method_name
                                )
                                .into());
                            }
                        }
                    }
                    Kind::String => {
                        if let Some(id_str) = field_value.as_str() {
                            if id_str.trim().is_empty() {
                                return Err(format!(
                                    "ID field '{}' cannot be empty in {}/{}",
                                    field.name(),
                                    service_name,
                                    method_name
                                )
                                .into());
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Custom rule: Amount/price fields should be non-negative
            if field_name.contains("amount")
                || field_name.contains("price")
                || field_name.contains("cost")
            {
                if let Some(numeric_val) = field_value.as_f64() {
                    if numeric_val < 0.0 {
                        return Err(format!(
                            "Amount/price field '{}' cannot be negative, got {} in {}/{}",
                            field.name(),
                            numeric_val,
                            service_name,
                            method_name
                        )
                        .into());
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate email format (basic)
    fn is_valid_email(&self, email: &str) -> bool {
        // Basic email validation: contains @ and . with reasonable structure
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            return false;
        }

        let local = parts[0];
        let domain = parts[1];

        if local.is_empty() || domain.is_empty() {
            return false;
        }

        // Domain should contain a dot
        domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
    }

    /// Validate phone number format (basic)
    fn is_valid_phone_number(&self, phone: &str) -> bool {
        // Basic phone validation: not empty and reasonable length
        !phone.is_empty() && phone.len() >= 7 && phone.len() <= 15
    }

    /// Validate ISO 8601 date format (basic)
    fn is_valid_iso8601_date(&self, date_str: &str) -> bool {
        // Basic ISO 8601 validation: YYYY-MM-DDTHH:MM:SSZ or similar
        // For simplicity, check if it parses as a date
        chrono::DateTime::parse_from_rfc3339(date_str).is_ok()
            || chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").is_ok()
            || chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S").is_ok()
    }

    /// Apply error handling middleware
    pub async fn handle_error(
        &self,
        error: Status,
        service_name: &str,
        method_name: &str,
    ) -> Status {
        // Log error details with context
        error!(
            "Error in {}/{}: {} (code: {:?})",
            service_name,
            method_name,
            error,
            error.code()
        );

        match error.code() {
            Code::InvalidArgument => Status::invalid_argument(format!(
                "Invalid arguments provided to {}/{}",
                service_name, method_name
            )),
            Code::NotFound => {
                Status::not_found(format!("Resource not found in {}/{}", service_name, method_name))
            }
            Code::AlreadyExists => Status::already_exists(format!(
                "Resource already exists in {}/{}",
                service_name, method_name
            )),
            Code::PermissionDenied => Status::permission_denied(format!(
                "Permission denied for {}/{}",
                service_name, method_name
            )),
            Code::FailedPrecondition => Status::failed_precondition(format!(
                "Precondition failed for {}/{}",
                service_name, method_name
            )),
            Code::Aborted => {
                Status::aborted(format!("Operation aborted for {}/{}", service_name, method_name))
            }
            Code::OutOfRange => Status::out_of_range(format!(
                "Value out of range in {}/{}",
                service_name, method_name
            )),
            Code::Unimplemented => Status::unimplemented(format!(
                "Method {}/{} not implemented",
                service_name, method_name
            )),
            Code::Internal => {
                Status::internal(format!("Internal error in {}/{}", service_name, method_name))
            }
            Code::Unavailable => Status::unavailable(format!(
                "Service {}/{} temporarily unavailable",
                service_name, method_name
            )),
            Code::DataLoss => {
                Status::data_loss(format!("Data loss occurred in {}/{}", service_name, method_name))
            }
            Code::Unauthenticated => Status::unauthenticated(format!(
                "Authentication required for {}/{}",
                service_name, method_name
            )),
            Code::DeadlineExceeded => Status::deadline_exceeded(format!(
                "Request to {}/{} timed out",
                service_name, method_name
            )),
            Code::ResourceExhausted => Status::resource_exhausted(format!(
                "Rate limit exceeded for {}/{}",
                service_name, method_name
            )),
            _ => {
                let message = error.message();
                if message.contains(service_name) && message.contains(method_name) {
                    error
                } else {
                    Status::new(
                        error.code(),
                        format!("{}/{}: {}", service_name, method_name, message),
                    )
                }
            }
        }
    }

    /// Apply metrics collection middleware
    pub async fn collect_metrics(
        &self,
        service_name: &str,
        method_name: &str,
        duration: std::time::Duration,
        success: bool,
    ) {
        let duration_ms = duration.as_millis() as u64;

        if success {
            record_success(service_name, method_name, duration_ms).await;
        } else {
            record_error(service_name, method_name).await;
        }

        tracing::debug!(
            "Request {}/{} completed in {:?}, success: {}",
            service_name,
            method_name,
            duration,
            success
        );
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_module_compiles() {}
}
