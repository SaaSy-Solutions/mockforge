//! HAR (HTTP Archive) export functionality

use crate::{models::*, Result};
use har::{Har, v1_2};
use std::collections::HashMap;

/// Export recorded exchanges to HAR format
pub fn export_to_har(exchanges: &[RecordedExchange]) -> Result<Har> {
    let mut entries = Vec::new();

    for exchange in exchanges {
        if exchange.request.protocol != Protocol::Http {
            continue; // HAR only supports HTTP
        }

        let entry = create_har_entry(exchange)?;
        entries.push(entry);
    }

    let log = v1_2::Log {
        creator: v1_2::Creator {
            name: "MockForge".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            comment: None,
        },
        browser: None,
        pages: None,
        entries,
        comment: None,
    };

    Ok(Har { log: har::Spec::V1_2(log) })
}

/// Create a HAR entry from a recorded exchange
fn create_har_entry(exchange: &RecordedExchange) -> Result<v1_2::Entries> {
    let request = &exchange.request;

    // Parse headers
    let headers_map: HashMap<String, String> = serde_json::from_str(&request.headers)?;

    let request_headers: Vec<v1_2::Headers> = headers_map
        .iter()
        .map(|(k, v)| v1_2::Headers {
            name: k.clone(),
            value: v.clone(),
            comment: None,
        })
        .collect();

    // Build query string
    let query_string: Vec<v1_2::QueryString> = if let Some(query) = &request.query_params {
        let params: HashMap<String, String> = serde_json::from_str(query)?;
        params
            .iter()
            .map(|(k, v)| v1_2::QueryString {
                name: k.clone(),
                value: v.clone(),
                comment: None,
            })
            .collect()
    } else {
        Vec::new()
    };

    // Build request
    let har_request = v1_2::Request {
        method: request.method.clone(),
        url: format!("http://mockforge{}", request.path),
        http_version: "HTTP/1.1".to_string(),
        cookies: Vec::new(),
        headers: request_headers,
        query_string,
        post_data: None,
        headers_size: -1,
        body_size: request.body.as_ref().map(|b| b.len() as i64).unwrap_or(0),
        comment: None,
    };

    // Build response if available
    let har_response = if let Some(response) = &exchange.response {
        let response_headers_map: HashMap<String, String> = serde_json::from_str(&response.headers)?;

        let response_headers: Vec<v1_2::Headers> = response_headers_map
            .iter()
            .map(|(k, v)| v1_2::Headers {
                name: k.clone(),
                value: v.clone(),
                comment: None,
            })
            .collect();

        v1_2::Response {
            status: response.status_code as i64,
            status_text: "OK".to_string(),
            http_version: "HTTP/1.1".to_string(),
            cookies: Vec::new(),
            headers: response_headers,
            content: v1_2::Content {
                size: response.size_bytes,
                compression: None,
                mime_type: Some("application/octet-stream".to_string()),
                text: response.body.clone(),
                encoding: Some(response.body_encoding.clone()),
                comment: None,
            },
            redirect_url: Some(String::new()),
            headers_size: -1,
            body_size: response.size_bytes,
            comment: None,
        }
    } else {
        // Default response if not recorded
        v1_2::Response {
            status: 0,
            status_text: "No Response".to_string(),
            http_version: "HTTP/1.1".to_string(),
            cookies: Vec::new(),
            headers: Vec::new(),
            content: v1_2::Content {
                size: 0,
                compression: None,
                mime_type: None,
                text: None,
                encoding: None,
                comment: None,
            },
            redirect_url: Some(String::new()),
            headers_size: -1,
            body_size: 0,
            comment: None,
        }
    };

    Ok(v1_2::Entries {
        pageref: None,
        started_date_time: request.timestamp.to_rfc3339(),
        time: request.duration_ms.unwrap_or(0) as f64,
        request: har_request,
        response: har_response,
        cache: v1_2::Cache::default(),
        timings: v1_2::Timings {
            blocked: None,
            dns: None,
            connect: None,
            send: 0.0,
            wait: request.duration_ms.unwrap_or(0) as f64,
            receive: 0.0,
            ssl: None,
            comment: None,
        },
        server_ip_address: request.client_ip.clone(),
        connection: None,
        comment: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_export_empty() {
        let exchanges = vec![];
        let har = export_to_har(&exchanges).unwrap();
        match &har.log {
            har::Spec::V1_2(log) => assert_eq!(log.entries.len(), 0),
            _ => panic!("Expected V1_2 spec"),
        }
    }

    #[test]
    fn test_export_single_exchange() {
        let exchange = RecordedExchange {
            request: RecordedRequest {
                id: "test-1".to_string(),
                protocol: Protocol::Http,
                timestamp: Utc::now(),
                method: "GET".to_string(),
                path: "/api/test".to_string(),
                query_params: None,
                headers: "{}".to_string(),
                body: None,
                body_encoding: "utf8".to_string(),
                client_ip: Some("127.0.0.1".to_string()),
                trace_id: None,
                span_id: None,
                duration_ms: Some(42),
                status_code: Some(200),
                tags: None,
            },
            response: Some(RecordedResponse {
                request_id: "test-1".to_string(),
                status_code: 200,
                headers: "{}".to_string(),
                body: Some("{\"ok\":true}".to_string()),
                body_encoding: "utf8".to_string(),
                size_bytes: 11,
                timestamp: Utc::now(),
            }),
        };

        let exchanges = vec![exchange];
        let har = export_to_har(&exchanges).unwrap();
        match &har.log {
            har::Spec::V1_2(log) => assert_eq!(log.entries.len(), 1),
            _ => panic!("Expected V1_2 spec"),
        }
    }
}
