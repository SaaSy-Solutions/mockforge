//! The options say whether the random and time tokens of an example are
//! expanded. A route generates its own response, so each is told when the
//! registry is built: before, a route read the environment variable alone,
//! and `http.response_template_expand` in the configuration file reached no
//! spec that `mockforge serve` answered from.

use super::*;
use serde_json::json;

fn spec() -> OpenApiSpec {
    OpenApiSpec::from_json(json!({
        "openapi": "3.0.0",
        "info": {"title": "Tokens", "version": "1"},
        "paths": {
            "/token": {"get": {"responses": {"200": {"description": "ok"}}}},
            "/other": {"post": {"responses": {"200": {"description": "ok"}}}}
        }
    }))
    .expect("a spec")
}

#[test]
fn each_route_is_told_what_the_options_say_of_the_tokens() {
    let on = OpenApiRouteRegistry::new_with_options(
        spec(),
        ValidationOptions {
            response_template_expand: true,
            ..Default::default()
        },
    );
    assert_eq!(on.routes().len(), 2);
    assert!(on.routes().iter().all(|route| route.response_template_expand));
    // The copy a handler holds is told as well.
    assert!(on
        .clone_for_validation()
        .routes()
        .iter()
        .all(|route| route.response_template_expand));

    let off = OpenApiRouteRegistry::new_with_options(spec(), ValidationOptions::default());
    assert!(off.routes().iter().all(|route| !route.response_template_expand));
}
