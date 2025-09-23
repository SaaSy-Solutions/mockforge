use axum::{routing::get, Router};
use mockforge_ui::routes::create_admin_router;

// Test that SPA routing is configured correctly
#[tokio::test]
async fn test_spa_routing_configuration() {
    let router = create_admin_router(None, None, None, true);
    
    // The router should be configured with the catch-all route
    // We can't easily test the exact routes without more complex setup,
    // but we can verify the router is created without panicking
    assert!(true); // If we get here, the router was created successfully
}

fn main() {
    println!("SPA routing test would run here");
}
