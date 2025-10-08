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
    // Build SQL query based on filters
    let mut query = String::from(
        r#"
        SELECT id, protocol, timestamp, method, path, query_params,
               headers, body, body_encoding, client_ip, trace_id, span_id,
               duration_ms, status_code, tags
        FROM requests WHERE 1=1
        "#,
    );

    let mut params: Vec<Box<dyn sqlx::Encode<'_, sqlx::Sqlite> + Send>> = Vec::new();

    // Add filters
    if let Some(protocol) = filter.protocol {
        query.push_str(" AND protocol = ?");
        params.push(Box::new(protocol.as_str().to_string()));
    }

    if let Some(method) = &filter.method {
        query.push_str(" AND method = ?");
        params.push(Box::new(method.clone()));
    }

    if let Some(path) = &filter.path {
        if path.contains('*') {
            query.push_str(" AND path LIKE ?");
            params.push(Box::new(path.replace('*', "%")));
        } else {
            query.push_str(" AND path = ?");
            params.push(Box::new(path.clone()));
        }
    }

    if let Some(status) = filter.status_code {
        query.push_str(" AND status_code = ?");
        params.push(Box::new(status));
    }

    if let Some(trace_id) = &filter.trace_id {
        query.push_str(" AND trace_id = ?");
        params.push(Box::new(trace_id.clone()));
    }

    if let Some(min_duration) = filter.min_duration_ms {
        query.push_str(" AND duration_ms >= ?");
        params.push(Box::new(min_duration));
    }

    if let Some(max_duration) = filter.max_duration_ms {
        query.push_str(" AND duration_ms <= ?");
        params.push(Box::new(max_duration));
    }

    // Order by timestamp descending
    query.push_str(" ORDER BY timestamp DESC");

    // Add limit and offset
    let limit = filter.limit.unwrap_or(100);
    let offset = filter.offset.unwrap_or(0);
    query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

    // For now, use the list_recent method as a placeholder
    // Full query implementation would require dynamic query building
    let requests = db.list_recent(limit).await?;

    // Fetch responses for each request
    let mut exchanges = Vec::new();
    for request in requests {
        let response = db.get_response(&request.id).await?;
        exchanges.push(RecordedExchange { request, response });
    }

    Ok(QueryResult {
        total: exchanges.len() as i64,
        offset,
        limit,
        exchanges,
    })
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
}
