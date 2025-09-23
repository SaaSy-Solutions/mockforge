use mockforge_core::import::openapi_command_generator::{
    generate_commands_from_openapi, CommandGenerationOptions, CommandFormat
};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simple OpenAPI spec for testing with security schemes
    let openapi_spec = r#"{
        "openapi": "3.0.3",
        "info": {"title": "Test API", "version": "1.0.0"},
        "servers": [{"url": "https://api.example.com"}],
        "security": [{"bearerAuth": []}],
        "components": {
            "securitySchemes": {
                "bearerAuth": {
                    "type": "http",
                    "scheme": "bearer"
                },
                "apiKeyAuth": {
                    "type": "apiKey",
                    "in": "header",
                    "name": "X-API-Key"
                }
            }
        },
        "paths": {
             "/users": {
                 "get": {
                     "operationId": "getUsers",
                     "summary": "Get all users",
                     "parameters": [
                         {"name": "limit", "in": "query", "schema": {"type": "integer"}},
                         {"name": "offset", "in": "query", "schema": {"type": "integer"}}
                     ],
                     "security": [{"bearerAuth": []}],
                     "responses": {"200": {"description": "Success"}}
                 }
             },
             "/users/{userId}": {
                 "post": {
                     "operationId": "createUser",
                     "summary": "Create user",
                     "parameters": [
                         {"name": "userId", "in": "path", "required": true, "schema": {"type": "string"}}
                     ],
                     "security": [{"apiKeyAuth": []}],
                     "requestBody": {
                         "content": {
                             "application/json": {
                                 "schema": {"type": "object", "properties": {"name": {"type": "string"}}}
                             }
                         }
                     },
                     "responses": {"201": {"description": "Created"}}
                 }
             }
        }
    }"#;

    let options = CommandGenerationOptions {
        base_url: Some("https://api.example.com".to_string()),
        format: CommandFormat::Both,
        include_auth: true,
        all_operations: false,
        include_examples: true,
        custom_headers: HashMap::new(),
        max_examples_per_operation: 1,
    };

    println!("🧪 Testing OpenAPI Command Generator...");

    match generate_commands_from_openapi(openapi_spec, options) {
        Ok(result) => {
            println!("✅ Successfully generated {} commands!", result.commands.len());

            for command in &result.commands {
                println!("\n🔧 Operation: {}", command.operation_id);
                println!("📍 URL: {}", command.url);
                println!("🌐 Method: {}", command.method);

                if !command.curl_command.is_empty() {
                    println!("🐚 curl command:");
                    println!("{}", command.curl_command);
                }

                if !command.httpie_command.is_empty() {
                    println!("🌐 httpie command:");
                    println!("{}", command.httpie_command);
                }
            }

            println!("\n🎉 OpenAPI command generator is working correctly!");
        }
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
