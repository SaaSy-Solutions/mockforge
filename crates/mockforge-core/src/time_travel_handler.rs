//! # Time Travel Response Handler
//!
//! This module provides HTTP request handling logic for scheduled responses
//! that are triggered based on the virtual clock time.

use crate::time_travel::{ResponseScheduler, ScheduledResponse, VirtualClock};
use axum::{
    body::Body,
    http::{HeaderMap, HeaderValue, Response, StatusCode},
    response::IntoResponse,
};
use std::sync::Arc;
use tracing::{debug, info};

/// Handler that checks for and returns scheduled responses
pub struct TimeTravelHandler {
    /// Response scheduler
    scheduler: Arc<ResponseScheduler>,
    /// Virtual clock
    clock: Arc<VirtualClock>,
}

impl TimeTravelHandler {
    /// Create a new time travel handler
    pub fn new(scheduler: Arc<ResponseScheduler>, clock: Arc<VirtualClock>) -> Self {
        Self { scheduler, clock }
    }

    /// Check if there are any due responses and return the first one
    pub fn check_for_scheduled_response(&self) -> Option<ScheduledResponseWrapper> {
        if !self.clock.is_enabled() {
            return None;
        }

        let due_responses = self.scheduler.get_due_responses();
        if due_responses.is_empty() {
            return None;
        }

        // Return the first due response
        due_responses
            .into_iter()
            .next()
            .map(ScheduledResponseWrapper::new)
    }

    /// Get all due responses
    pub fn get_all_due_responses(&self) -> Vec<ScheduledResponseWrapper> {
        if !self.clock.is_enabled() {
            return Vec::new();
        }

        self.scheduler
            .get_due_responses()
            .into_iter()
            .map(ScheduledResponseWrapper::new)
            .collect()
    }

    /// Check if time travel is enabled
    pub fn is_enabled(&self) -> bool {
        self.clock.is_enabled()
    }
}

/// Wrapper around ScheduledResponse for converting to HTTP responses
#[derive(Debug, Clone)]
pub struct ScheduledResponseWrapper {
    inner: ScheduledResponse,
}

impl ScheduledResponseWrapper {
    /// Create a new wrapper
    pub fn new(response: ScheduledResponse) -> Self {
        Self { inner: response }
    }

    /// Get the inner scheduled response
    pub fn inner(&self) -> &ScheduledResponse {
        &self.inner
    }

    /// Convert to an Axum response
    pub fn into_response(self) -> Response<Body> {
        let mut response = Response::builder().status(self.inner.status);

        // Add headers
        if let Some(headers) = response.headers_mut() {
            for (key, value) in &self.inner.headers {
                if let Ok(header_name) = key.parse::<axum::http::HeaderName>() {
                    if let Ok(header_value) = HeaderValue::from_str(value) {
                        headers.insert(header_name, header_value);
                    }
                }
            }

            // Add custom header to indicate this is a scheduled response
            headers.insert(
                "X-MockForge-Scheduled-Response",
                HeaderValue::from_static("true"),
            );

            if let Some(name) = &self.inner.name {
                if let Ok(value) = HeaderValue::from_str(name) {
                    headers.insert("X-MockForge-Schedule-Name", value);
                }
            }
        }

        // Set body
        let body_str = serde_json::to_string(&self.inner.body).unwrap_or_else(|_| "{}".to_string());
        response
            .body(Body::from(body_str))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Failed to build response"))
                    .unwrap()
            })
    }
}

impl IntoResponse for ScheduledResponseWrapper {
    fn into_response(self) -> axum::response::Response {
        let mut response = Response::builder().status(self.inner.status);

        // Add headers
        let headers = response.headers_mut();
        if let Some(headers) = headers {
            for (key, value) in &self.inner.headers {
                if let Ok(header_name) = key.parse::<axum::http::HeaderName>() {
                    if let Ok(header_value) = HeaderValue::from_str(value) {
                        headers.insert(header_name, header_value);
                    }
                }
            }

            // Add custom header
            headers.insert(
                "X-MockForge-Scheduled-Response",
                HeaderValue::from_static("true"),
            );
        }

        // Set body
        let body_str = serde_json::to_string(&self.inner.body).unwrap_or_else(|_| "{}".to_string());
        response
            .body(Body::from(body_str))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Failed to build response"))
                    .unwrap()
            })
    }
}

/// Middleware layer for checking scheduled responses
pub async fn time_travel_middleware<B>(
    handler: Arc<TimeTravelHandler>,
    request: axum::http::Request<B>,
    next: axum::middleware::Next<B>,
) -> impl IntoResponse
where
    B: Send + 'static,
{
    // Check if there's a scheduled response
    if let Some(scheduled) = handler.check_for_scheduled_response() {
        info!("Returning scheduled response: {}", scheduled.inner().id);
        return scheduled.into_response();
    }

    // Otherwise, pass through to the next handler
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_travel::{ScheduledResponse, VirtualClock};
    use chrono::{Duration, Utc};
    use std::collections::HashMap;

    #[test]
    fn test_time_travel_handler_creation() {
        let clock = Arc::new(VirtualClock::new());
        let scheduler = Arc::new(ResponseScheduler::new(clock.clone()));
        let handler = TimeTravelHandler::new(scheduler, clock);

        assert!(!handler.is_enabled());
    }

    #[test]
    fn test_scheduled_response_wrapper() {
        let response = ScheduledResponse {
            id: "test-1".to_string(),
            trigger_time: Utc::now(),
            body: serde_json::json!({"message": "Hello"}),
            status: 200,
            headers: HashMap::new(),
            name: Some("test".to_string()),
            repeat: None,
        };

        let wrapper = ScheduledResponseWrapper::new(response.clone());
        assert_eq!(wrapper.inner().id, "test-1");
    }

    #[test]
    fn test_check_for_scheduled_response() {
        let clock = Arc::new(VirtualClock::new());
        let test_time = Utc::now();
        clock.enable_and_set(test_time);

        let scheduler = Arc::new(ResponseScheduler::new(clock.clone()));

        let response = ScheduledResponse {
            id: "test-1".to_string(),
            trigger_time: test_time + Duration::seconds(10),
            body: serde_json::json!({"message": "Hello"}),
            status: 200,
            headers: HashMap::new(),
            name: None,
            repeat: None,
        };

        scheduler.schedule(response).unwrap();

        let handler = TimeTravelHandler::new(scheduler, clock.clone());

        // Should not be due yet
        assert!(handler.check_for_scheduled_response().is_none());

        // Advance time
        clock.advance(Duration::seconds(15));

        // Should be due now
        assert!(handler.check_for_scheduled_response().is_some());
    }
}
