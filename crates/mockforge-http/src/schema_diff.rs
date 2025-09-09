//! JSON schema diff utilities for 422 responses.
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct FieldError {
    pub path: String,
    pub expected: String,
    pub found: String,
    pub message: Option<String>
}

pub fn diff(expected_schema: &Value, actual: &Value) -> Vec<FieldError> {
    let mut out = Vec::new();
    walk(expected_schema, actual, "", &mut out);
    out
}

fn walk(expected: &Value, actual: &Value, path: &str, out: &mut Vec<FieldError>) {
    match (expected, actual) {
        (Value::Object(eo), Value::Object(ao)) => {
            for (k, ev) in eo {
                let np = format!("{}/{}", path, k);
                if let Some(av) = ao.get(k) {
                    walk(ev, av, &np, out);
                } else {
                    out.push(FieldError{
                        path: np, expected: type_of(ev), found: "missing".into(), message: Some("required".into())
                    });
                }
            }
        }
        (Value::Array(ea), Value::Array(aa)) => {
            if let Some(esample) = ea.get(0) {
                for (i, av) in aa.iter().enumerate() {
                    let np = format!("{}/{}", path, i);
                    walk(esample, av, &np, out);
                }
            }
        }
        (e, a) => {
            let et = type_of(e);
            let at = type_of(a);
            if et != at {
                out.push(FieldError{ path: path.into(), expected: et, found: at, message: None });
            }
        }
    }
}

fn type_of(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(_) => "bool".to_string(),
        Value::Number(n) => if n.is_i64() { "integer" } else { "number" }.to_string(),
        Value::String(_) => "string".to_string(),
        Value::Array(_) => "array".to_string(),
        Value::Object(_) => "object".to_string(),
    }
}

pub fn to_422_json(errors: Vec<FieldError>) -> Value {
    json!({
        "error": "Schema validation failed",
        "details": errors.into_iter().map(|e| json!({
            "path": e.path,
            "expected": e.expected,
            "found": e.found,
            "message": e.message
        })).collect::<Vec<_>>()
    })
}
