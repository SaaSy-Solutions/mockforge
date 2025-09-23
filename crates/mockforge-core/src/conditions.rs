//! Condition evaluation system for override rules
//!
//! This module provides support for conditional application of overrides based on
//! JSONPath and XPath queries, as well as other conditional expressions.

use jsonpath::Selector;
use roxmltree::{Document, Node};
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during condition evaluation
#[derive(Debug, Error)]
pub enum ConditionError {
    #[error("Invalid JSONPath expression: {0}")]
    InvalidJsonPath(String),

    #[error("Invalid XPath expression: {0}")]
    InvalidXPath(String),

    #[error("Invalid XML: {0}")]
    InvalidXml(String),

    #[error("Unsupported condition type: {0}")]
    UnsupportedCondition(String),

    #[error("Condition evaluation failed: {0}")]
    EvaluationFailed(String),
}

/// Context for evaluating conditions
#[derive(Debug, Clone)]
pub struct ConditionContext {
    /// Request body (JSON)
    pub request_body: Option<Value>,
    /// Response body (JSON)
    pub response_body: Option<Value>,
    /// Request body as XML string
    pub request_xml: Option<String>,
    /// Response body as XML string
    pub response_xml: Option<String>,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Query parameters
    pub query_params: HashMap<String, String>,
    /// Request path
    pub path: String,
    /// HTTP method
    pub method: String,
    /// Operation ID
    pub operation_id: Option<String>,
    /// Tags
    pub tags: Vec<String>,
}

impl Default for ConditionContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionContext {
    pub fn new() -> Self {
        Self {
            request_body: None,
            response_body: None,
            request_xml: None,
            response_xml: None,
            headers: HashMap::new(),
            query_params: HashMap::new(),
            path: String::new(),
            method: String::new(),
            operation_id: None,
            tags: Vec::new(),
        }
    }

    pub fn with_request_body(mut self, body: Value) -> Self {
        self.request_body = Some(body);
        self
    }

    pub fn with_response_body(mut self, body: Value) -> Self {
        self.response_body = Some(body);
        self
    }

    pub fn with_request_xml(mut self, xml: String) -> Self {
        self.request_xml = Some(xml);
        self
    }

    pub fn with_response_xml(mut self, xml: String) -> Self {
        self.response_xml = Some(xml);
        self
    }

    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    pub fn with_query_params(mut self, params: HashMap<String, String>) -> Self {
        self.query_params = params;
        self
    }

    pub fn with_path(mut self, path: String) -> Self {
        self.path = path;
        self
    }

    pub fn with_method(mut self, method: String) -> Self {
        self.method = method;
        self
    }

    pub fn with_operation_id(mut self, operation_id: String) -> Self {
        self.operation_id = Some(operation_id);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

/// Evaluate a condition expression
pub fn evaluate_condition(condition: &str, context: &ConditionContext) -> Result<bool, ConditionError> {
    let condition = condition.trim();

    if condition.is_empty() {
        return Ok(true); // Empty condition always evaluates to true
    }

    // Handle logical operators
    if let Some(and_conditions) = condition.strip_prefix("AND(") {
        if let Some(inner) = and_conditions.strip_suffix(")") {
            return evaluate_and_condition(inner, context);
        }
    }

    if let Some(or_conditions) = condition.strip_prefix("OR(") {
        if let Some(inner) = or_conditions.strip_suffix(")") {
            return evaluate_or_condition(inner, context);
        }
    }

    if let Some(not_condition) = condition.strip_prefix("NOT(") {
        if let Some(inner) = not_condition.strip_suffix(")") {
            return evaluate_not_condition(inner, context);
        }
    }

    // Handle JSONPath queries
    if condition.starts_with("$.") || condition.starts_with("$[") {
        return evaluate_jsonpath(condition, context);
    }

    // Handle XPath queries
    if condition.starts_with("/") {
        return evaluate_xpath(condition, context);
    }

    // Handle simple comparisons
    evaluate_simple_condition(condition, context)
}

/// Evaluate AND condition with multiple sub-conditions
fn evaluate_and_condition(conditions: &str, context: &ConditionContext) -> Result<bool, ConditionError> {
    let parts: Vec<&str> = conditions.split(',').map(|s| s.trim()).collect();

    for part in parts {
        if !evaluate_condition(part, context)? {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Evaluate OR condition with multiple sub-conditions
fn evaluate_or_condition(conditions: &str, context: &ConditionContext) -> Result<bool, ConditionError> {
    let parts: Vec<&str> = conditions.split(',').map(|s| s.trim()).collect();

    for part in parts {
        if evaluate_condition(part, context)? {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Evaluate NOT condition
fn evaluate_not_condition(condition: &str, context: &ConditionContext) -> Result<bool, ConditionError> {
    Ok(!evaluate_condition(condition, context)?)
}

/// Evaluate JSONPath query
fn evaluate_jsonpath(query: &str, context: &ConditionContext) -> Result<bool, ConditionError> {
    // Determine if this is a request or response query
    let (_is_request, json_value) = if query.starts_with("$.request.") {
        let _query = query.replace("$.request.", "$.");
        (true, &context.request_body)
    } else if query.starts_with("$.response.") {
        let _query = query.replace("$.response.", "$.");
        (false, &context.response_body)
    } else {
        // Default to response body if not specified
        (false, &context.response_body)
    };

    let Some(json_value) = json_value else {
        return Ok(false); // No body to query
    };

    match Selector::new(query) {
        Ok(selector) => {
            let results: Vec<_> = selector.find(json_value).collect();
            Ok(!results.is_empty())
        }
        Err(_) => Err(ConditionError::InvalidJsonPath(query.to_string())),
    }
}

/// Evaluate XPath query
fn evaluate_xpath(query: &str, context: &ConditionContext) -> Result<bool, ConditionError> {
    // Determine if this is a request or response query
    let (_is_request, xml_content) = if query.starts_with("/request/") {
        let _query = query.replace("/request/", "/");
        (true, &context.request_xml)
    } else if query.starts_with("/response/") {
        let _query = query.replace("/response/", "/");
        (false, &context.response_xml)
    } else {
        // Default to response XML if not specified
        (false, &context.response_xml)
    };

    let Some(xml_content) = xml_content else {
        return Ok(false); // No XML content to query
    };

    match Document::parse(xml_content) {
        Ok(doc) => {
            // Simple XPath evaluation - check if any nodes match
            let root = doc.root_element();
            let matches = evaluate_xpath_simple(&root, query);
            Ok(matches)
        }
        Err(_) => Err(ConditionError::InvalidXml(xml_content.clone())),
    }
}

/// Simple XPath evaluator (basic implementation)
fn evaluate_xpath_simple(node: &Node, xpath: &str) -> bool {
    // This is a simplified XPath implementation
    // For production use, consider a more complete XPath library

    let xpath = xpath.trim_start_matches('/');

    if xpath.is_empty() {
        return true;
    }

    // Handle attribute queries: element[@attribute='value']
    if let Some((element_part, attr_part)) = xpath.split_once('[') {
        if let Some(attr_query) = attr_part.strip_suffix(']') {
            if let Some((attr_name, attr_value)) = attr_query.split_once("='") {
                if let Some(expected_value) = attr_value.strip_suffix('\'') {
                    if let Some(attr_val) = attr_name.strip_prefix('@') {
                        if node.tag_name().name() == element_part {
                            if let Some(attr) = node.attribute(attr_val) {
                                return attr == expected_value;
                            }
                        }
                    }
                }
            }
        }
        return false;
    }

    // Handle element name matching with optional predicates
    if let Some((element_name, rest)) = xpath.split_once('/') {
        if node.tag_name().name() == element_name {
            if rest.is_empty() {
                return true;
            }
            // Check child elements recursively
            for child in node.children() {
                if child.is_element() && evaluate_xpath_simple(&child, rest) {
                    return true;
                }
            }
        }
    } else if node.tag_name().name() == xpath {
        return true;
    }

    // Handle text content queries: element/text()
    if let Some(text_query) = xpath.strip_suffix("/text()") {
        if node.tag_name().name() == text_query {
            return node.text().is_some_and(|t| !t.trim().is_empty());
        }
    }

    // Handle descendant-or-self axis: //element
    if let Some(element_name) = xpath.strip_prefix("//") {
        if node.tag_name().name() == element_name {
            return true;
        }
        // Check descendants
        for child in node.children() {
            if child.is_element() && evaluate_xpath_simple(&child, xpath) {
                return true;
            }
        }
    }

    false
}

/// Evaluate simple conditions like header checks, query param checks, etc.
fn evaluate_simple_condition(condition: &str, context: &ConditionContext) -> Result<bool, ConditionError> {
    // Handle header conditions: header[name]=value
    if let Some(header_condition) = condition.strip_prefix("header[") {
        if let Some((header_name, expected_value)) = header_condition.split_once("]=") {
            let expected_value = expected_value.trim();
            if let Some(actual_value) = context.headers.get(header_name) {
                return Ok(actual_value == expected_value);
            }
            return Ok(false);
        }
    }

    // Handle query parameter conditions: query[name]=value
    if let Some(query_condition) = condition.strip_prefix("query[") {
        if let Some((param_name, expected_value)) = query_condition.split_once("]=") {
            let expected_value = expected_value.trim();
            if let Some(actual_value) = context.query_params.get(param_name) {
                return Ok(actual_value == expected_value);
            }
            return Ok(false);
        }
    }

    // Handle method conditions: method=POST
    if let Some(method_condition) = condition.strip_prefix("method=") {
        return Ok(context.method == method_condition);
    }

    // Handle path conditions: path=/api/users
    if let Some(path_condition) = condition.strip_prefix("path=") {
        return Ok(context.path == path_condition);
    }

    // Handle tag conditions: has_tag[admin]
    if let Some(tag_condition) = condition.strip_prefix("has_tag[") {
        if let Some(tag) = tag_condition.strip_suffix("]") {
            return Ok(context.tags.contains(&tag.to_string()));
        }
    }

    // Handle operation conditions: operation=getUser
    if let Some(op_condition) = condition.strip_prefix("operation=") {
        if let Some(operation_id) = &context.operation_id {
            return Ok(operation_id == op_condition);
        }
        return Ok(false);
    }

    Err(ConditionError::UnsupportedCondition(condition.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_jsonpath_condition() {
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
    }

    #[test]
    fn test_simple_conditions() {
        let mut headers = HashMap::new();
        headers.insert("authorization".to_string(), "Bearer token123".to_string());

        let mut query_params = HashMap::new();
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
    }

    #[test]
    fn test_logical_conditions() {
        let context = ConditionContext::new()
            .with_method("POST".to_string())
            .with_path("/api/users".to_string());

        // Test AND condition
        assert!(evaluate_condition("AND(method=POST,path=/api/users)", &context).unwrap());
        assert!(!evaluate_condition("AND(method=GET,path=/api/users)", &context).unwrap());

        // Test OR condition
        assert!(evaluate_condition("OR(method=POST,path=/api/posts)", &context).unwrap());
        assert!(!evaluate_condition("OR(method=GET,path=/api/posts)", &context).unwrap());

        // Test NOT condition
        assert!(!evaluate_condition("NOT(method=POST)", &context).unwrap());
        assert!(evaluate_condition("NOT(method=GET)", &context).unwrap());
    }

    #[test]
    fn test_xpath_condition() {
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
    }
}
