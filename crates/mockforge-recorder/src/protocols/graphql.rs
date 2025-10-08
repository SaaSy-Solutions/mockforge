//! GraphQL recording helpers

use crate::{models::*, recorder::Recorder};
use chrono::Utc;
use std::collections::HashMap;
use tracing::debug;
use uuid::Uuid;

/// Record a GraphQL query/mutation
pub async fn record_graphql_request(
    recorder: &Recorder,
    operation_type: &str, // "query", "mutation", "subscription"
    operation_name: Option<&str>,
    query: &str,
    variables: Option<&str>,
    headers: &HashMap<String, String>,
    client_ip: Option<&str>,
    trace_id: Option<&str>,
    span_id: Option<&str>,
) -> Result<String, crate::RecorderError> {
    let request_id = Uuid::new_v4().to_string();

    // Build request body as GraphQL JSON format
    let mut body_json = serde_json::json!({
        "query": query,
    });

    if let Some(vars) = variables {
        if let Ok(vars_json) = serde_json::from_str::<serde_json::Value>(vars) {
            body_json["variables"] = vars_json;
        }
    }

    if let Some(name) = operation_name {
        body_json["operationName"] = serde_json::json!(name);
    }

    let body_str = serde_json::to_string(&body_json)?;
    let method = format!("GraphQL {}", operation_type.to_uppercase());

    let mut tags = vec!["graphql".to_string(), operation_type.to_string()];
    if let Some(name) = operation_name {
        tags.push(name.to_string());
    }

    let request = RecordedRequest {
        id: request_id.clone(),
        protocol: Protocol::GraphQL,
        timestamp: Utc::now(),
        method,
        path: "/graphql".to_string(),
        query_params: None,
        headers: serde_json::to_string(&headers)?,
        body: Some(body_str),
        body_encoding: "utf8".to_string(),
        client_ip: client_ip.map(|s| s.to_string()),
        trace_id: trace_id.map(|s| s.to_string()),
        span_id: span_id.map(|s| s.to_string()),
        duration_ms: None,
        status_code: None,
        tags: Some(serde_json::to_string(&tags)?),
    };

    recorder.record_request(request).await?;
    debug!(
        "Recorded GraphQL request: {} {} {}",
        request_id,
        operation_type,
        operation_name.unwrap_or("anonymous")
    );

    Ok(request_id)
}

/// Record a GraphQL response
pub async fn record_graphql_response(
    recorder: &Recorder,
    request_id: &str,
    response_json: &str,
    has_errors: bool,
    duration_ms: i64,
) -> Result<(), crate::RecorderError> {
    let status_code = if has_errors { 400 } else { 200 };
    let size_bytes = response_json.len() as i64;

    let response = RecordedResponse {
        request_id: request_id.to_string(),
        status_code,
        headers: serde_json::to_string(&HashMap::from([(
            "content-type".to_string(),
            "application/json".to_string(),
        )]))?,
        body: Some(response_json.to_string()),
        body_encoding: "utf8".to_string(),
        size_bytes,
        timestamp: Utc::now(),
    };

    recorder.record_response(response).await?;
    debug!(
        "Recorded GraphQL response: {} status={} duration={}ms",
        request_id, status_code, duration_ms
    );

    Ok(())
}

/// Record a GraphQL subscription event
pub async fn record_graphql_subscription_event(
    recorder: &Recorder,
    subscription_id: &str,
    event_data: &str,
    trace_id: Option<&str>,
    span_id: Option<&str>,
) -> Result<String, crate::RecorderError> {
    let event_id = Uuid::new_v4().to_string();

    let request = RecordedRequest {
        id: event_id.clone(),
        protocol: Protocol::GraphQL,
        timestamp: Utc::now(),
        method: "GraphQL SUBSCRIPTION_EVENT".to_string(),
        path: format!("/graphql/subscriptions/{}", subscription_id),
        query_params: None,
        headers: serde_json::to_string(&HashMap::from([(
            "subscription-id".to_string(),
            subscription_id.to_string(),
        )]))?,
        body: Some(event_data.to_string()),
        body_encoding: "utf8".to_string(),
        client_ip: None,
        trace_id: trace_id.map(|s| s.to_string()),
        span_id: span_id.map(|s| s.to_string()),
        duration_ms: None,
        status_code: Some(200),
        tags: Some(serde_json::to_string(&vec!["graphql", "subscription", "event"])?),
    };

    recorder.record_request(request).await?;
    debug!(
        "Recorded GraphQL subscription event: {} subscription={}",
        event_id, subscription_id
    );

    Ok(event_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::RecorderDatabase;

    #[tokio::test]
    async fn test_record_graphql_query() {
        let db = RecorderDatabase::new_in_memory().await.unwrap();
        let recorder = Recorder::new(db);

        let headers = HashMap::from([(
            "content-type".to_string(),
            "application/json".to_string(),
        )]);

        let query = "query GetUser($id: ID!) { user(id: $id) { id name email } }";
        let variables = r#"{"id": "123"}"#;

        let request_id = record_graphql_request(
            &recorder,
            "query",
            Some("GetUser"),
            query,
            Some(variables),
            &headers,
            Some("127.0.0.1"),
            None,
            None,
        )
        .await
        .unwrap();

        let response_json = r#"{"data": {"user": {"id": "123", "name": "John", "email": "john@example.com"}}}"#;

        record_graphql_response(&recorder, &request_id, response_json, false, 42)
            .await
            .unwrap();

        // Verify it was recorded
        let exchange = recorder.database().get_exchange(&request_id).await.unwrap();
        assert!(exchange.is_some());

        let exchange = exchange.unwrap();
        assert_eq!(exchange.request.protocol, Protocol::GraphQL);
        assert_eq!(exchange.request.method, "GraphQL QUERY");
    }

    #[tokio::test]
    async fn test_record_graphql_mutation() {
        let db = RecorderDatabase::new_in_memory().await.unwrap();
        let recorder = Recorder::new(db);

        let headers = HashMap::from([(
            "content-type".to_string(),
            "application/json".to_string(),
        )]);

        let mutation = "mutation CreateUser($input: UserInput!) { createUser(input: $input) { id name } }";

        let request_id = record_graphql_request(
            &recorder,
            "mutation",
            Some("CreateUser"),
            mutation,
            None,
            &headers,
            Some("127.0.0.1"),
            None,
            None,
        )
        .await
        .unwrap();

        // Verify it was recorded
        let exchange = recorder.database().get_exchange(&request_id).await.unwrap();
        assert!(exchange.is_some());

        let exchange = exchange.unwrap();
        assert_eq!(exchange.request.protocol, Protocol::GraphQL);
        assert_eq!(exchange.request.method, "GraphQL MUTATION");
    }
}
