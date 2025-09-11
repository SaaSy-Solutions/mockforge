use serde_json::json;
use mockforge_core::{OpenApiSpec, OpenApiRouteRegistry};

#[tokio::test]
async fn validate_pattern_and_array_size() {
    let spec = json!({
        "openapi":"3.0.0",
        "info": {"title":"T","version":"1"},
        "paths": {
            "/users": {"post": {
                "requestBody": {"content": {"application/json": {"schema": {
                    "type":"object","required":["name","tags"],
                    "properties":{
                        "name": {"type":"string","pattern":"^[A-Z][a-z]+$"},
                        "tags": {"type":"array","minItems":2,"maxItems":3,"items":{"type":"string"}}
                    }
                }}}},
                "responses": {
                    "200": {"description":"ok","content":{"application/json":{"schema":{"type":"object"}}}}
                }
            }}
        }
    });
    let spec = OpenApiSpec::from_json(spec).unwrap();
    let reg = OpenApiRouteRegistry::new(spec);

    // valid
    let body = json!({"name":"Alice","tags":["a","b"]});
    reg.validate_request_with("/users","POST", &serde_json::Map::new(), &serde_json::Map::new(), Some(&body)).unwrap();

    // pattern fail
    let body = json!({"name":"alice","tags":["a","b"]});
    assert!(reg.validate_request_with("/users","POST", &serde_json::Map::new(), &serde_json::Map::new(), Some(&body)).is_err());

    // minItems fail
    let body = json!({"name":"Alice","tags":["only_one", "two", "three", "four"]});
    assert!(reg.validate_request_with("/users","POST", &serde_json::Map::new(), &serde_json::Map::new(), Some(&body)).is_err());
}
