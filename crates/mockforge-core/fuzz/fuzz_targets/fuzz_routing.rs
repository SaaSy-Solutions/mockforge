#![no_main]

use libfuzzer_sys::fuzz_target;
use mockforge_core::routing::{HttpMethod, Route, RouteRegistry};

fuzz_target!(|data: &[u8]| {
    // Try to use the fuzz input as a path string
    if let Ok(path_str) = std::str::from_utf8(data) {
        let mut registry = RouteRegistry::new();

        // Add various routes with different patterns
        let routes = vec![
            Route::new(HttpMethod::GET, "/api/users".to_string()),
            Route::new(HttpMethod::GET, "/api/users/*".to_string()),
            Route::new(HttpMethod::POST, "/api/users".to_string()),
            Route::new(HttpMethod::GET, "/*".to_string()),
            Route::new(HttpMethod::GET, path_str.to_string()),
        ];

        for route in routes {
            let _ = registry.add_http_route(route);
        }

        // Attempt to find matching routes
        // Should never panic, even with malformed paths
        let _ = registry.find_http_routes(&HttpMethod::GET, path_str);
        let _ = registry.find_http_routes(&HttpMethod::POST, path_str);
        let _ = registry.find_ws_routes(path_str);
    }
});
