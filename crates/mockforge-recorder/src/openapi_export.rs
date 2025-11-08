//! OpenAPI export utilities for recorder
//!
//! This module provides conversion from recorded HTTP exchanges to
//! the HttpExchange format used by the OpenAPI generator.

use crate::{
    database::RecorderDatabase,
    models::{Protocol, RecordedExchange, RecordedRequest, RecordedResponse},
    query::{execute_query, QueryFilter},
    Result,
};
use chrono::{DateTime, Utc};
use mockforge_core::intelligent_behavior::openapi_generator::HttpExchange;

/// Converter from recorded exchanges to HttpExchange format
pub struct RecordingsToOpenApi;

impl RecordingsToOpenApi {
    /// Convert RecordedExchange to HttpExchange
    pub fn convert_exchange(exchange: &RecordedExchange) -> HttpExchange {
        HttpExchange {
            method: exchange.request.method.clone(),
            path: exchange.request.path.clone(),
            query_params: exchange.request.query_params.clone(),
            headers: exchange.request.headers.clone(),
            body: exchange.request.body.clone(),
            body_encoding: exchange.request.body_encoding.clone(),
            status_code: exchange.response.as_ref().map(|r| r.status_code),
            response_headers: exchange.response.as_ref().map(|r| r.headers.clone()),
            response_body: exchange.response.as_ref().and_then(|r| r.body.clone()),
            response_body_encoding: exchange.response.as_ref().map(|r| r.body_encoding.clone()),
            timestamp: exchange.request.timestamp,
        }
    }

    /// Convert a single RecordedRequest/RecordedResponse pair to HttpExchange
    pub fn convert_request_response(
        request: &RecordedRequest,
        response: Option<&RecordedResponse>,
    ) -> HttpExchange {
        HttpExchange {
            method: request.method.clone(),
            path: request.path.clone(),
            query_params: request.query_params.clone(),
            headers: request.headers.clone(),
            body: request.body.clone(),
            body_encoding: request.body_encoding.clone(),
            status_code: response.map(|r| r.status_code),
            response_headers: response.map(|r| r.headers.clone()),
            response_body: response.and_then(|r| r.body.clone()),
            response_body_encoding: response.map(|r| r.body_encoding.clone()),
            timestamp: request.timestamp,
        }
    }

    /// Convert multiple RecordedExchange to HttpExchange
    pub fn convert_exchanges(exchanges: &[RecordedExchange]) -> Vec<HttpExchange> {
        exchanges.iter().map(Self::convert_exchange).collect()
    }

    /// Query HTTP exchanges from database and convert to HttpExchange format
    ///
    /// This method queries the recorder database for HTTP requests/responses
    /// and converts them to the format expected by the OpenAPI generator.
    pub async fn query_http_exchanges(
        db: &RecorderDatabase,
        filters: Option<QueryFilters>,
    ) -> Result<Vec<HttpExchange>> {
        let mut query_filter = QueryFilter {
            protocol: Some(Protocol::Http),
            limit: filters.as_ref().and_then(|f| f.max_requests).map(|n| n as i32).or(Some(1000)),
            ..Default::default()
        };

        // Apply path pattern filters
        if let Some(ref filters) = filters {
            if let Some(ref path_pattern) = filters.path_pattern {
                query_filter.path = Some(path_pattern.clone());
            }

            if let Some(status) = filters.min_status_code {
                query_filter.status_code = Some(status);
            }
        }

        // Execute query
        let query_result = execute_query(db, query_filter).await?;

        // Convert to HttpExchange format
        let mut exchanges = Self::convert_exchanges(&query_result.exchanges);

        // Apply time range filters if specified
        if let Some(ref filters) = filters {
            if let Some(since) = filters.since {
                exchanges.retain(|e| e.timestamp >= since);
            }

            if let Some(until) = filters.until {
                exchanges.retain(|e| e.timestamp <= until);
            }
        }

        Ok(exchanges)
    }
}

/// Query filters for OpenAPI generation
#[derive(Debug, Clone)]
pub struct QueryFilters {
    /// Time range filter: start timestamp (optional)
    pub since: Option<DateTime<Utc>>,

    /// Time range filter: end timestamp (optional)
    pub until: Option<DateTime<Utc>>,

    /// Path pattern filter (supports wildcards)
    pub path_pattern: Option<String>,

    /// Minimum status code filter
    pub min_status_code: Option<i32>,

    /// Maximum number of requests to analyze
    pub max_requests: Option<usize>,
}

impl Default for QueryFilters {
    fn default() -> Self {
        Self {
            since: None,
            until: None,
            path_pattern: None,
            min_status_code: None,
            max_requests: Some(1000),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{RecordedRequest, RecordedResponse};

    #[test]
    fn test_convert_exchange() {
        let request = RecordedRequest {
            id: "test-123".to_string(),
            protocol: Protocol::Http,
            timestamp: Utc::now(),
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            query_params: None,
            headers: "{}".to_string(),
            body: None,
            body_encoding: "utf8".to_string(),
            client_ip: None,
            trace_id: None,
            span_id: None,
            duration_ms: None,
            status_code: Some(200),
            tags: None,
        };

        let response = Some(RecordedResponse {
            request_id: "test-123".to_string(),
            status_code: 200,
            headers: "{}".to_string(),
            body: Some(r#"{"result": "ok"}"#.to_string()),
            body_encoding: "utf8".to_string(),
            size_bytes: 15,
            timestamp: Utc::now(),
        });

        let exchange = RecordedExchange { request, response };
        let http_exchange = RecordingsToOpenApi::convert_exchange(&exchange);

        assert_eq!(http_exchange.method, "GET");
        assert_eq!(http_exchange.path, "/api/test");
        assert_eq!(http_exchange.status_code, Some(200));
    }
}
