//! Query API for recorded requests

use crate::{database::RecorderDatabase, models::*, Result};
use serde::{Deserialize, Serialize};

/// Query filter for searching recorded requests
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryFilter {
    /// Filter by protocol
    pub protocol: Option<Protocol>,
    /// Filter by HTTP method or gRPC method
    pub method: Option<String>,
    /// Filter by path (supports wildcards)
    pub path: Option<String>,
    /// Filter by status code
    pub status_code: Option<i32>,
    /// Filter by trace ID
    pub trace_id: Option<String>,
    /// Filter by minimum duration (ms)
    pub min_duration_ms: Option<i64>,
    /// Filter by maximum duration (ms)
    pub max_duration_ms: Option<i64>,
    /// Filter by tags
    pub tags: Option<Vec<String>>,
    /// Limit number of results
    pub limit: Option<i32>,
    /// Offset for pagination
    pub offset: Option<i32>,
}

/// Query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub total: i64,
    pub offset: i32,
    pub limit: i32,
    pub exchanges: Vec<RecordedExchange>,
}

/// Execute a query against the database
pub async fn execute_query(db: &RecorderDatabase, filter: QueryFilter) -> Result<QueryResult> {
    let limit = filter.limit.unwrap_or(100);
    let offset = filter.offset.unwrap_or(0);

    // Fetch a sufficiently large recent window and apply filters in memory.
    // This avoids the previous placeholder behavior where filters were ignored.
    let fetch_window = std::cmp::max(limit + offset, 1000);
    let requests = db.list_recent(fetch_window).await?;

    let mut filtered: Vec<RecordedRequest> = requests
        .into_iter()
        .filter(|request| request_matches_filter(request, &filter))
        .collect();

    let total = filtered.len() as i64;
    filtered = filtered.into_iter().skip(offset as usize).take(limit as usize).collect();

    // Fetch responses for each request
    let mut exchanges = Vec::new();
    for request in filtered {
        let response = db.get_response(&request.id).await?;
        exchanges.push(RecordedExchange { request, response });
    }

    Ok(QueryResult {
        total,
        offset,
        limit,
        exchanges,
    })
}

fn request_matches_filter(request: &RecordedRequest, filter: &QueryFilter) -> bool {
    if let Some(protocol) = &filter.protocol {
        if &request.protocol != protocol {
            return false;
        }
    }

    if let Some(method) = &filter.method {
        if request.method != *method {
            return false;
        }
    }

    if let Some(path_filter) = &filter.path {
        let request_path = request.path.as_str();
        if path_filter.contains('*') {
            let pattern = path_filter.replace('*', "");
            if !request_path.contains(&pattern) {
                return false;
            }
        } else if request_path != path_filter {
            return false;
        }
    }

    if let Some(status_code) = filter.status_code {
        if request.status_code != Some(status_code) {
            return false;
        }
    }

    if let Some(trace_id) = &filter.trace_id {
        if request.trace_id.as_deref() != Some(trace_id.as_str()) {
            return false;
        }
    }

    if let Some(min_duration) = filter.min_duration_ms {
        let duration = request.duration_ms.unwrap_or_default();
        if duration < min_duration {
            return false;
        }
    }

    if let Some(max_duration) = filter.max_duration_ms {
        let duration = request.duration_ms.unwrap_or_default();
        if duration > max_duration {
            return false;
        }
    }

    if let Some(required_tags) = &filter.tags {
        let request_tags = request.tags_vec();
        if required_tags
            .iter()
            .any(|required| !request_tags.iter().any(|actual| actual == required))
        {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_filter_creation() {
        let filter = QueryFilter {
            protocol: Some(Protocol::Http),
            method: Some("GET".to_string()),
            path: Some("/api/*".to_string()),
            ..Default::default()
        };

        assert_eq!(filter.protocol, Some(Protocol::Http));
        assert_eq!(filter.method, Some("GET".to_string()));
    }

    #[test]
    fn test_request_matches_filter() {
        let request = RecordedRequest {
            id: "req-1".to_string(),
            protocol: Protocol::Http,
            timestamp: chrono::Utc::now(),
            method: "GET".to_string(),
            path: "/api/users/123".to_string(),
            query_params: None,
            headers: "{}".to_string(),
            body: None,
            body_encoding: "utf8".to_string(),
            client_ip: None,
            trace_id: Some("trace-1".to_string()),
            span_id: None,
            duration_ms: Some(42),
            status_code: Some(200),
            tags: Some(r#"["users","read"]"#.to_string()),
        };

        let filter = QueryFilter {
            protocol: Some(Protocol::Http),
            method: Some("GET".to_string()),
            path: Some("/api/users/*".to_string()),
            status_code: Some(200),
            trace_id: Some("trace-1".to_string()),
            min_duration_ms: Some(40),
            max_duration_ms: Some(100),
            tags: Some(vec!["users".to_string()]),
            limit: Some(10),
            offset: Some(0),
        };

        assert!(request_matches_filter(&request, &filter));
    }
}
