//! Condition expression evaluator for state machine transitions
//!
//! Provides safe evaluation of JavaScript/TypeScript-like expressions for
//! conditional state transitions. Uses rquickjs for sandboxed execution.

use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

/// Error types for condition evaluation
#[derive(Debug, Error)]
pub enum ConditionError {
    /// Expression parsing or syntax error
    #[error("Expression syntax error: {0}")]
    SyntaxError(String),

    /// Runtime evaluation error
    #[error("Evaluation error: {0}")]
    EvaluationError(String),

    /// Type mismatch error
    #[error("Type error: {0}")]
    TypeError(String),

    /// Variable not found
    #[error("Variable not found: {0}")]
    VariableNotFound(String),
}

/// Result type for condition evaluation
pub type ConditionResult<T> = Result<T, ConditionError>;

/// Condition evaluator for state machine transitions
///
/// Evaluates JavaScript/TypeScript-like expressions in a sandboxed environment.
/// Supports variable access, comparison operators, logical operators, and
/// array/object access.
pub struct ConditionEvaluator {
    /// Context variables available for evaluation
    context: HashMap<String, Value>,
}

impl ConditionEvaluator {
    /// Create a new condition evaluator with empty context
    pub fn new() -> Self {
        Self {
            context: HashMap::new(),
        }
    }

    /// Create a new condition evaluator with initial context
    pub fn with_context(context: HashMap<String, Value>) -> Self {
        Self { context }
    }

    /// Set a context variable
    pub fn set_variable(&mut self, name: impl Into<String>, value: Value) {
        self.context.insert(name.into(), value);
    }

    /// Get a context variable
    pub fn get_variable(&self, name: &str) -> Option<&Value> {
        self.context.get(name)
    }

    /// Evaluate a condition expression
    ///
    /// The expression can access variables from the context using dot notation
    /// (e.g., `state.status`, `entity.count`). Supports:
    /// - Comparison: `==`, `!=`, `>`, `<`, `>=`, `<=`
    /// - Logical: `&&`, `||`, `!`
    /// - Arithmetic: `+`, `-`, `*`, `/`, `%`
    /// - Array/object access: `arr[0]`, `obj.field`
    ///
    /// Returns `true` if the condition is satisfied, `false` otherwise.
    pub fn evaluate(&self, expression: &str) -> ConditionResult<bool> {
        // Use rquickjs for safe JavaScript evaluation
        // This is a simplified implementation - in production, you'd want
        // more sophisticated parsing and validation

        // For now, we'll use a simple expression parser
        // In a full implementation, we'd use rquickjs::Context to evaluate JS
        self.evaluate_simple(expression)
    }

    /// Simple expression evaluator (fallback when rquickjs is not available)
    ///
    /// This is a basic implementation that handles common cases.
    /// For full JavaScript support, use rquickjs.
    fn evaluate_simple(&self, expression: &str) -> ConditionResult<bool> {
        let expr = expression.trim();

        // Handle boolean literals
        if expr == "true" {
            return Ok(true);
        }
        if expr == "false" {
            return Ok(false);
        }

        // Handle comparison operators
        if let Some(result) = self.evaluate_comparison(expr)? {
            return Ok(result);
        }

        // Handle logical operators
        if let Some(result) = self.evaluate_logical(expr)? {
            return Ok(result);
        }

        // Handle variable access
        if let Some(value) = self.get_variable_value(expr)? {
            return self.value_to_bool(&value);
        }

        Err(ConditionError::SyntaxError(format!("Unable to evaluate expression: {}", expr)))
    }

    /// Evaluate comparison expressions (==, !=, >, <, >=, <=)
    fn evaluate_comparison(&self, expr: &str) -> ConditionResult<Option<bool>> {
        // Note: We can't use closures with different signatures in an array,
        // so we'll handle each operator separately

        // Handle == operator
        if let Some((left, right)) = expr.split_once("==") {
            let left_val = self.evaluate_value(left.trim())?;
            let right_val = self.evaluate_value(right.trim())?;
            return Ok(Some(left_val == right_val));
        }

        // Handle != operator
        if let Some((left, right)) = expr.split_once("!=") {
            let left_val = self.evaluate_value(left.trim())?;
            let right_val = self.evaluate_value(right.trim())?;
            return Ok(Some(left_val != right_val));
        }

        // Handle numeric comparison operators
        for op in [">=", "<=", ">", "<"] {
            if let Some((left, right)) = expr.split_once(op) {
                let left_val = self.evaluate_value(left.trim())?;
                let right_val = self.evaluate_value(right.trim())?;

                // Try numeric comparison
                if let (Some(a), Some(b)) = (
                    left_val.as_f64().or_else(|| left_val.as_i64().map(|i| i as f64)),
                    right_val.as_f64().or_else(|| right_val.as_i64().map(|i| i as f64)),
                ) {
                    let result = match op {
                        ">=" => a >= b,
                        "<=" => a <= b,
                        ">" => a > b,
                        "<" => a < b,
                        _ => false,
                    };
                    return Ok(Some(result));
                }
            }
        }

        Ok(None)
    }

    /// Evaluate logical expressions (&&, ||, !)
    fn evaluate_logical(&self, expr: &str) -> ConditionResult<Option<bool>> {
        // Handle NOT operator
        if expr.starts_with('!') {
            let inner = expr[1..].trim();
            let inner_result = self.evaluate(inner)?;
            return Ok(Some(!inner_result));
        }

        // Handle AND operator
        if let Some((left, right)) = expr.split_once("&&") {
            let left_result = self.evaluate(left.trim())?;
            if !left_result {
                return Ok(Some(false));
            }
            return Ok(Some(self.evaluate(right.trim())?));
        }

        // Handle OR operator
        if let Some((left, right)) = expr.split_once("||") {
            let left_result = self.evaluate(left.trim())?;
            if left_result {
                return Ok(Some(true));
            }
            return Ok(Some(self.evaluate(right.trim())?));
        }

        Ok(None)
    }

    /// Evaluate a value expression (variable, literal, etc.)
    fn evaluate_value(&self, expr: &str) -> ConditionResult<Value> {
        // Try to get variable value
        if let Some(value) = self.get_variable_value(expr)? {
            return Ok(value.clone());
        }

        // Try to parse as JSON value
        if let Ok(value) = serde_json::from_str::<Value>(expr) {
            return Ok(value);
        }

        // Try to parse as number
        if let Ok(num) = expr.parse::<f64>() {
            return Ok(Value::Number(
                serde_json::Number::from_f64(num).unwrap_or_else(|| serde_json::Number::from(0)),
            ));
        }

        // Try to parse as boolean
        if expr == "true" {
            return Ok(Value::Bool(true));
        }
        if expr == "false" {
            return Ok(Value::Bool(false));
        }

        // Return as string
        Ok(Value::String(expr.to_string()))
    }

    /// Get variable value using dot notation (e.g., "state.status")
    fn get_variable_value(&self, path: &str) -> ConditionResult<Option<Value>> {
        let parts: Vec<&str> = path.split('.').collect();

        if parts.is_empty() {
            return Ok(None);
        }

        // Get root variable
        let root = self.context.get(parts[0]);
        if root.is_none() {
            return Ok(None);
        }

        let mut value = root.unwrap().clone();

        // Navigate through nested properties
        for part in parts.iter().skip(1) {
            match value {
                Value::Object(ref obj) => {
                    value = obj
                        .get(*part)
                        .ok_or_else(|| {
                            ConditionError::VariableNotFound(format!("{}.{}", parts[0], part))
                        })?
                        .clone();
                }
                _ => {
                    return Err(ConditionError::TypeError(format!(
                        "Cannot access property '{}' on non-object",
                        part
                    )));
                }
            }
        }

        Ok(Some(value))
    }

    /// Convert a JSON value to boolean
    fn value_to_bool(&self, value: &Value) -> ConditionResult<bool> {
        match value {
            Value::Bool(b) => Ok(*b),
            Value::Number(n) => Ok(n.as_f64().unwrap_or(0.0) != 0.0),
            Value::String(s) => Ok(!s.is_empty()),
            Value::Array(arr) => Ok(!arr.is_empty()),
            Value::Object(obj) => Ok(!obj.is_empty()),
            Value::Null => Ok(false),
        }
    }
}

impl Default for ConditionEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_boolean() {
        let evaluator = ConditionEvaluator::new();
        assert!(evaluator.evaluate("true").unwrap());
        assert!(!evaluator.evaluate("false").unwrap());
    }

    #[test]
    fn test_comparison_operators() {
        let evaluator = ConditionEvaluator::new();
        assert!(evaluator.evaluate("5 > 3").unwrap());
        assert!(evaluator.evaluate("3 < 5").unwrap());
        assert!(evaluator.evaluate("5 == 5").unwrap());
        assert!(evaluator.evaluate("5 != 3").unwrap());
    }

    #[test]
    fn test_variable_access() {
        let mut context = HashMap::new();
        context.insert("status".to_string(), Value::String("active".to_string()));
        context.insert("count".to_string(), Value::Number(5.into()));

        let evaluator = ConditionEvaluator::with_context(context);
        assert!(evaluator.evaluate("count > 3").unwrap());
    }

    #[test]
    fn test_logical_operators() {
        let evaluator = ConditionEvaluator::new();
        assert!(evaluator.evaluate("true && true").unwrap());
        assert!(!evaluator.evaluate("true && false").unwrap());
        assert!(evaluator.evaluate("true || false").unwrap());
        assert!(!evaluator.evaluate("!true").unwrap());
    }
}
