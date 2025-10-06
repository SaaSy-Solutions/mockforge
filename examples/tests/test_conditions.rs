use mockforge_core::conditions::{evaluate_condition, ConditionContext};
use serde_json::json;

#[test]
fn test_jsonpath_functionality() {
    let context = ConditionContext::new()
        .with_response_body(json!({
            "user": {
                "name": "John",
                "role": "admin"
            },
            "items": [1, 2, 3]
        }));

    // Test simple path existence
    assert!(evaluate_condition("$.user", &context).unwrap());

    // Test specific value matching
    assert!(evaluate_condition("$.user.role", &context).unwrap());

    // Test array access
    assert!(evaluate_condition("$.items[0]", &context).unwrap());

    // Test non-existent path
    assert!(!evaluate_condition("$.nonexistent", &context).unwrap());

    println!("JSONPath tests passed!");
}

#[test]
fn test_xpath_functionality() {
    let xml_content = r#"
        <user id="123">
            <name>John Doe</name>
            <role>admin</role>
            <preferences>
                <theme>dark</theme>
                <notifications>true</notifications>
            </preferences>
        </user>
    "#;

    let context = ConditionContext::new()
        .with_response_xml(xml_content.to_string());

    // Test basic element existence
    assert!(evaluate_condition("/user", &context).unwrap());

    // Test nested element
    assert!(evaluate_condition("/user/name", &context).unwrap());

    // Test attribute query
    assert!(evaluate_condition("/user[@id='123']", &context).unwrap());
    assert!(!evaluate_condition("/user[@id='456']", &context).unwrap());

    // Test text content
    assert!(evaluate_condition("/user/name/text()", &context).unwrap());

    // Test descendant axis
    assert!(evaluate_condition("//theme", &context).unwrap());

    // Test non-existent element
    assert!(!evaluate_condition("/nonexistent", &context).unwrap());

    println!("XPath tests passed!");
}

#[test]
fn test_simple_conditions() {
    let mut headers = std::collections::HashMap::new();
    headers.insert("authorization".to_string(), "Bearer token123".to_string());

    let mut query_params = std::collections::HashMap::new();
    query_params.insert("limit".to_string(), "10".to_string());

    let context = ConditionContext::new()
        .with_headers(headers)
        .with_query_params(query_params)
        .with_method("POST".to_string())
        .with_path("/api/users".to_string());

    // Test header condition
    assert!(evaluate_condition("header[authorization]=Bearer token123", &context).unwrap());
    assert!(!evaluate_condition("header[authorization]=Bearer wrong", &context).unwrap());

    // Test query parameter condition
    assert!(evaluate_condition("query[limit]=10", &context).unwrap());
    assert!(!evaluate_condition("query[limit]=20", &context).unwrap());

    // Test method condition
    assert!(evaluate_condition("method=POST", &context).unwrap());
    assert!(!evaluate_condition("method=GET", &context).unwrap());

    // Test path condition
    assert!(evaluate_condition("path=/api/users", &context).unwrap());
    assert!(!evaluate_condition("path=/api/posts", &context).unwrap());

    println!("Simple condition tests passed!");
}

fn main() {
    println!("Running condition evaluation tests...");

    test_jsonpath_functionality();
    test_xpath_functionality();
    test_simple_conditions();

    println!("All tests passed! ✅");
}
