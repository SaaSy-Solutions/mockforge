use mockforge_core::import::har_import;
use std::fs;

fn main() {
    println!("MockForge HAR Import Demo");
    println!("==========================");

    // Read the test HAR file
    let har_content = fs::read_to_string("test_har.har").expect("Failed to read HAR file");

    // Import the HAR archive
    match har_import::import_har_archive(&har_content, Some("https://api.example.com")) {
        Ok(result) => {
            println!("✅ Successfully imported {} routes", result.routes.len());

            if !result.warnings.is_empty() {
                println!("\n⚠️  Warnings:");
                for warning in &result.warnings {
                    println!("  - {}", warning);
                }
            }

            println!("\n📋 Imported Routes:");
            for (i, route) in result.routes.iter().enumerate() {
                println!("{}. {} {}", i + 1, route.method, route.path);
                if !route.headers.is_empty() {
                    println!("   Headers: {} header(s)", route.headers.len());
                }
                if let Some(body) = &route.body {
                    println!("   Body: {} characters", body.len());
                }
                println!("   Response: {} ({})", route.response.status, route.response.body);
            }

            // Show sample output format
            println!("\n📄 Generated MockForge Config:");
            let config = serde_json::json!({
                "routes": result.routes.iter().map(|route| {
                    serde_json::json!({
                        "method": route.method,
                        "path": route.path,
                        "headers": route.headers,
                        "body": route.body,
                        "response": {
                            "status": route.response.status,
                            "headers": route.response.headers,
                            "body": route.response.body
                        }
                    })
                }).collect::<Vec<_>>()
            });

            println!("{}", serde_json::to_string_pretty(&config).unwrap());
        }
        Err(e) => {
            println!("❌ Failed to import HAR file: {}", e);
        }
    }
}
