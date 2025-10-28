//! Integration tests for token-based response generation in HTTP endpoints

use axum::http::StatusCode;
use mockforge_http::token_response::{resolve_response_tokens, TokenResolvedResponse};
use serde_json::json;

#[tokio::test]
async fn test_resolve_response_tokens_integration() {
    // Test basic token resolution
    let body = json!({
        "id": "$random.uuid",
        "name": "$faker.name",
        "email": "$faker.email"
    });

    let result = resolve_response_tokens(body).await;
    assert!(result.is_ok());

    let resolved = result.unwrap();
    assert!(resolved["id"].is_string());
    assert!(resolved["name"].is_string());
    assert!(resolved["email"].is_string());

    // Verify that the values are different from the token strings
    assert_ne!(resolved["id"].as_str().unwrap(), "$random.uuid");
    assert_ne!(resolved["name"].as_str().unwrap(), "$faker.name");
}

#[tokio::test]
async fn test_token_resolved_response_builder() {
    let body = json!({
        "message": "Hello",
        "user": {
            "id": "$random.uuid",
            "name": "$faker.name"
        }
    });

    let response = TokenResolvedResponse::new(StatusCode::OK, body).build().await;

    assert_eq!(response.status(), StatusCode::OK);

    // Extract and parse the body
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&body_str).unwrap();

    assert_eq!(parsed["message"], "Hello");
    assert!(parsed["user"]["id"].is_string());
    assert!(parsed["user"]["name"].is_string());
}

#[tokio::test]
async fn test_nested_token_resolution() {
    let body = json!({
        "level1": {
            "level2": {
                "level3": {
                    "id": "$random.uuid",
                    "value": "$random.int"
                }
            }
        }
    });

    let resolved = resolve_response_tokens(body).await.unwrap();
    assert!(resolved["level1"]["level2"]["level3"]["id"].is_string());
    assert!(resolved["level1"]["level2"]["level3"]["value"].is_number());
}

#[tokio::test]
async fn test_array_token_resolution() {
    let body = json!({
        "users": [
            {
                "id": "$random.uuid",
                "name": "$faker.name",
                "email": "$faker.email"
            },
            {
                "id": "$random.uuid",
                "name": "$faker.name",
                "email": "$faker.email"
            },
            {
                "id": "$random.uuid",
                "name": "$faker.name",
                "email": "$faker.email"
            }
        ]
    });

    let resolved = resolve_response_tokens(body).await.unwrap();
    let users = resolved["users"].as_array().unwrap();

    assert_eq!(users.len(), 3);
    for user in users {
        assert!(user["id"].is_string());
        assert!(user["name"].is_string());
        assert!(user["email"].is_string());

        // Verify UUIDs are valid
        let uuid_str = user["id"].as_str().unwrap();
        assert!(uuid::Uuid::parse_str(uuid_str).is_ok());
    }
}

#[tokio::test]
async fn test_mixed_static_and_token_values() {
    let body = json!({
        "static_field": "static_value",
        "static_number": 42,
        "static_bool": true,
        "dynamic_id": "$random.uuid",
        "dynamic_name": "$faker.name",
        "nested": {
            "static": "value",
            "dynamic": "$random.int"
        }
    });

    let resolved = resolve_response_tokens(body).await.unwrap();

    // Static values should remain unchanged
    assert_eq!(resolved["static_field"], "static_value");
    assert_eq!(resolved["static_number"], 42);
    assert_eq!(resolved["static_bool"], true);
    assert_eq!(resolved["nested"]["static"], "value");

    // Dynamic values should be resolved
    assert!(resolved["dynamic_id"].is_string());
    assert_ne!(resolved["dynamic_id"].as_str().unwrap(), "$random.uuid");
    assert!(resolved["dynamic_name"].is_string());
    assert_ne!(resolved["dynamic_name"].as_str().unwrap(), "$faker.name");
    assert!(resolved["nested"]["dynamic"].is_number());
}

#[tokio::test]
async fn test_all_random_token_types() {
    let body = json!({
        "uuid": "$random.uuid",
        "int": "$random.int",
        "int_small": "$random.int.small",
        "int_large": "$random.int.large",
        "float": "$random.float",
        "bool": "$random.bool",
        "hex": "$random.hex",
        "hex_short": "$random.hex.short",
        "alphanumeric": "$random.alphanumeric",
        "choice": "$random.choice"
    });

    let resolved = resolve_response_tokens(body).await.unwrap();

    // Verify UUID format
    let uuid_str = resolved["uuid"].as_str().unwrap();
    assert!(uuid::Uuid::parse_str(uuid_str).is_ok());

    // Verify numbers
    assert!(resolved["int"].is_number());
    assert!(resolved["int_small"].is_number());
    assert!(resolved["int_large"].is_number());
    assert!(resolved["float"].is_number());

    // Verify boolean
    assert!(resolved["bool"].is_boolean());

    // Verify hex strings
    assert!(resolved["hex"].is_string());
    assert!(resolved["hex_short"].is_string());
    let hex_str = resolved["hex"].as_str().unwrap();
    assert!(hex_str.len() > 0);

    // Verify alphanumeric
    assert!(resolved["alphanumeric"].is_string());

    // Verify choice
    assert!(resolved["choice"].is_string());
}

#[tokio::test]
async fn test_all_faker_token_types() {
    let body = json!({
        "name": "$faker.name",
        "email": "$faker.email",
        "phone": "$faker.phone",
        "address": "$faker.address",
        "company": "$faker.company",
        "url": "$faker.url",
        "datetime": "$faker.datetime",
        "word": "$faker.word",
        "sentence": "$faker.sentence",
        "paragraph": "$faker.paragraph",
        "uuid": "$faker.uuid"
    });

    let resolved = resolve_response_tokens(body).await.unwrap();

    // All faker fields should be strings
    assert!(resolved["name"].is_string());
    assert!(resolved["email"].is_string());
    assert!(resolved["phone"].is_string());
    assert!(resolved["address"].is_string());
    assert!(resolved["company"].is_string());
    assert!(resolved["url"].is_string());
    assert!(resolved["datetime"].is_string());
    assert!(resolved["word"].is_string());
    assert!(resolved["sentence"].is_string());
    assert!(resolved["paragraph"].is_string());
    assert!(resolved["uuid"].is_string());

    // Verify email format
    let email = resolved["email"].as_str().unwrap();
    assert!(email.contains('@'));

    // Verify UUID format
    let uuid_str = resolved["uuid"].as_str().unwrap();
    assert!(uuid::Uuid::parse_str(uuid_str).is_ok());
}

#[tokio::test]
async fn test_real_world_ecommerce_scenario() {
    let body = json!({
        "order_id": "$random.uuid",
        "customer": {
            "id": "$random.uuid",
            "name": "$faker.name",
            "email": "$faker.email",
            "phone": "$faker.phone"
        },
        "items": [
            {
                "id": "$random.uuid",
                "name": "$faker.word",
                "price": "$random.float",
                "quantity": "$random.int.small"
            },
            {
                "id": "$random.uuid",
                "name": "$faker.word",
                "price": "$random.float",
                "quantity": "$random.int.small"
            }
        ],
        "shipping": {
            "address": "$faker.address",
            "method": "standard"
        },
        "total": "$random.float",
        "status": "pending",
        "created_at": "$faker.datetime",
        "updated_at": "$faker.datetime"
    });

    let resolved = resolve_response_tokens(body).await.unwrap();

    // Verify structure
    assert!(resolved["order_id"].is_string());
    assert!(resolved["customer"]["id"].is_string());
    assert!(resolved["customer"]["name"].is_string());
    assert!(resolved["customer"]["email"].is_string());

    // Verify items array
    let items = resolved["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    for item in items {
        assert!(item["id"].is_string());
        assert!(item["name"].is_string());
        assert!(item["price"].is_number());
        assert!(item["quantity"].is_number());
    }

    // Verify static values remain
    assert_eq!(resolved["shipping"]["method"], "standard");
    assert_eq!(resolved["status"], "pending");
}

#[tokio::test]
async fn test_real_world_iot_scenario() {
    let body = json!({
        "device_id": "$random.uuid",
        "sensor_id": "$random.uuid",
        "readings": [
            {
                "temperature": "$random.float",
                "humidity": "$random.float",
                "pressure": "$random.float",
                "timestamp": "$faker.datetime"
            },
            {
                "temperature": "$random.float",
                "humidity": "$random.float",
                "pressure": "$random.float",
                "timestamp": "$faker.datetime"
            }
        ],
        "location": {
            "latitude": "$random.float",
            "longitude": "$random.float"
        },
        "status": "active"
    });

    let resolved = resolve_response_tokens(body).await.unwrap();

    // Verify device info
    assert!(resolved["device_id"].is_string());
    assert!(resolved["sensor_id"].is_string());

    // Verify readings
    let readings = resolved["readings"].as_array().unwrap();
    assert_eq!(readings.len(), 2);
    for reading in readings {
        assert!(reading["temperature"].is_number());
        assert!(reading["humidity"].is_number());
        assert!(reading["pressure"].is_number());
        assert!(reading["timestamp"].is_string());
    }

    // Verify location
    assert!(resolved["location"]["latitude"].is_number());
    assert!(resolved["location"]["longitude"].is_number());

    // Verify static value
    assert_eq!(resolved["status"], "active");
}

#[tokio::test]
async fn test_performance_large_object() {
    use std::time::Instant;

    let body = json!({
        "field1": "$random.uuid",
        "field2": "$faker.name",
        "field3": "$faker.email",
        "field4": "$random.int",
        "field5": "$random.float",
        "field6": "$faker.company",
        "field7": "$faker.address",
        "field8": "$faker.phone",
        "field9": "$random.bool",
        "field10": "$faker.datetime",
        "nested": {
            "field1": "$random.uuid",
            "field2": "$faker.name",
            "field3": "$faker.email",
            "field4": "$random.int",
            "field5": "$random.float"
        },
        "array": [
            {"id": "$random.uuid", "name": "$faker.name"},
            {"id": "$random.uuid", "name": "$faker.name"},
            {"id": "$random.uuid", "name": "$faker.name"}
        ]
    });

    let start = Instant::now();
    let resolved = resolve_response_tokens(body).await.unwrap();
    let duration = start.elapsed();

    // Should complete in under 1ms (way under 200ms requirement)
    assert!(duration.as_millis() < 1);

    // Verify all fields were resolved
    assert!(resolved["field1"].is_string());
    assert_ne!(resolved["field1"].as_str().unwrap(), "$random.uuid");
}
