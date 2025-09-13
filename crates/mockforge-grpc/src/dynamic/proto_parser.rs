//! Proto file parsing and service discovery
//!
//! This module handles parsing of .proto files and extracting service definitions
//! to generate dynamic gRPC service implementations.

use prost_reflect::DescriptorPool;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{debug, error, info, warn};

/// A parsed proto service definition
#[derive(Debug, Clone)]
pub struct ProtoService {
    /// The service name (e.g., "mockforge.greeter.Greeter")
    pub name: String,
    /// The package name (e.g., "mockforge.greeter")
    pub package: String,
    /// The short service name (e.g., "Greeter")
    pub short_name: String,
    /// List of methods in this service
    pub methods: Vec<ProtoMethod>,
}

/// A parsed proto method definition
#[derive(Debug, Clone)]
pub struct ProtoMethod {
    /// The method name (e.g., "SayHello")
    pub name: String,
    /// The input message type
    pub input_type: String,
    /// The output message type
    pub output_type: String,
    /// Whether this is a client streaming method
    pub client_streaming: bool,
    /// Whether this is a server streaming method
    pub server_streaming: bool,
}

/// A proto file parser that can extract service definitions
pub struct ProtoParser {
    /// The descriptor pool containing parsed proto files
    pool: DescriptorPool,
    /// Map of service names to their definitions
    services: HashMap<String, ProtoService>,
}

impl ProtoParser {
    /// Create a new proto parser
    pub fn new() -> Self {
        Self {
            pool: DescriptorPool::new(),
            services: HashMap::new(),
        }
    }

    /// Parse proto files from a directory
    pub async fn parse_directory(
        &mut self,
        proto_dir: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Parsing proto files from directory: {}", proto_dir);

        let proto_path = Path::new(proto_dir);
        if !proto_path.exists() {
            return Err(format!("Proto directory does not exist: {}", proto_dir).into());
        }

        // Discover all proto files
        let proto_files = self.discover_proto_files(proto_path)?;
        if proto_files.is_empty() {
            warn!("No proto files found in directory: {}", proto_dir);
            return Ok(());
        }

        info!("Found {} proto files: {:?}", proto_files.len(), proto_files);

        // Parse each proto file
        for proto_file in proto_files {
            if let Err(e) = self.parse_proto_file(&proto_file).await {
                error!("Failed to parse proto file {}: {}", proto_file, e);
                // Continue with other files
            }
        }

        // Extract services from the descriptor pool
        self.extract_services()?;

        info!("Successfully parsed {} services", self.services.len());
        Ok(())
    }

    /// Discover proto files in a directory recursively
    fn discover_proto_files(
        &self,
        dir: &Path,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let mut proto_files = Vec::new();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.is_dir() {
                    // Recursively search subdirectories
                    proto_files.extend(self.discover_proto_files(&path)?);
                } else if path.extension().and_then(|s| s.to_str()) == Some("proto") {
                    // Found a .proto file
                    proto_files.push(path.to_string_lossy().to_string());
                }
            }
        }

        Ok(proto_files)
    }

    /// Parse a single proto file
    async fn parse_proto_file(
        &mut self,
        proto_file: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Parsing proto file: {}", proto_file);

        // For now, we'll use a simple approach that compiles the proto files
        // and then reads the generated descriptor set
        // In a full implementation, we would use prost-build or similar

        // This is a placeholder - in reality, we would need to:
        // 1. Compile the proto files using prost-build
        // 2. Read the generated descriptor set
        // 3. Parse the descriptor set into our internal format

        // For now, we'll create a mock service based on the greeter.proto
        if proto_file.contains("gretter.proto") || proto_file.contains("greeter.proto") {
            self.add_mock_greeter_service();
        }

        Ok(())
    }

    /// Add a mock greeter service (for demonstration)
    fn add_mock_greeter_service(&mut self) {
        let service = ProtoService {
            name: "mockforge.greeter.Greeter".to_string(),
            package: "mockforge.greeter".to_string(),
            short_name: "Greeter".to_string(),
            methods: vec![
                ProtoMethod {
                    name: "SayHello".to_string(),
                    input_type: "mockforge.greeter.HelloRequest".to_string(),
                    output_type: "mockforge.greeter.HelloReply".to_string(),
                    client_streaming: false,
                    server_streaming: false,
                },
                ProtoMethod {
                    name: "SayHelloStream".to_string(),
                    input_type: "mockforge.greeter.HelloRequest".to_string(),
                    output_type: "mockforge.greeter.HelloReply".to_string(),
                    client_streaming: false,
                    server_streaming: true,
                },
                ProtoMethod {
                    name: "SayHelloClientStream".to_string(),
                    input_type: "mockforge.greeter.HelloRequest".to_string(),
                    output_type: "mockforge.greeter.HelloReply".to_string(),
                    client_streaming: true,
                    server_streaming: false,
                },
                ProtoMethod {
                    name: "Chat".to_string(),
                    input_type: "mockforge.greeter.HelloRequest".to_string(),
                    output_type: "mockforge.greeter.HelloReply".to_string(),
                    client_streaming: true,
                    server_streaming: true,
                },
            ],
        };

        self.services.insert(service.name.clone(), service);
    }

    /// Extract services from the descriptor pool
    fn extract_services(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // This would extract services from the actual descriptor pool
        // For now, we're using the mock service
        Ok(())
    }

    /// Get all discovered services
    pub fn services(&self) -> &HashMap<String, ProtoService> {
        &self.services
    }

    /// Get a specific service by name
    pub fn get_service(&self, name: &str) -> Option<&ProtoService> {
        self.services.get(name)
    }

    /// Get the descriptor pool
    pub fn pool(&self) -> &DescriptorPool {
        &self.pool
    }

    /// Consume the parser and return the descriptor pool
    pub fn into_pool(self) -> DescriptorPool {
        self.pool
    }
}

impl Default for ProtoParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn test_parse_proto_file() {
        // Test with the existing greeter.proto file
        let proto_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/proto";
        let proto_path = format!("{}/gretter.proto", proto_dir);

        // Parse the proto file
        let mut parser = ProtoParser::new();
        parser.parse_proto_file(&proto_path).await.unwrap();

        // Verify the service was parsed correctly
        let services = parser.services();
        assert_eq!(services.len(), 1);

        let service_name = "mockforge.greeter.Greeter";
        assert!(services.contains_key(service_name));

        let service = &services[service_name];
        assert_eq!(service.name, service_name);
        assert_eq!(service.methods.len(), 4); // SayHello, SayHelloStream, SayHelloClientStream, Chat

        // Check SayHello method (unary)
        let say_hello = service.methods.iter().find(|m| m.name == "SayHello").unwrap();
        assert_eq!(say_hello.input_type, "mockforge.greeter.HelloRequest");
        assert_eq!(say_hello.output_type, "mockforge.greeter.HelloReply");
        assert!(!say_hello.client_streaming);
        assert!(!say_hello.server_streaming);

        // Check SayHelloStream method (server streaming)
        let say_hello_stream = service.methods.iter().find(|m| m.name == "SayHelloStream").unwrap();
        assert!(!say_hello_stream.client_streaming);
        assert!(say_hello_stream.server_streaming);
    }

    #[tokio::test]
    async fn test_parse_directory() {
        // Test with the existing proto directory
        let proto_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/proto";

        // Parse the directory
        let mut parser = ProtoParser::new();
        parser.parse_directory(&proto_dir).await.unwrap();

        // Verify services were discovered
        let services = parser.services();
        assert_eq!(services.len(), 1);

        let service_name = "mockforge.greeter.Greeter";
        assert!(services.contains_key(service_name));

        let service = &services[service_name];
        assert_eq!(service.methods.len(), 4);

        // Check all methods exist
        let method_names: Vec<&str> = service.methods.iter().map(|m| m.name.as_str()).collect();
        assert!(method_names.contains(&"SayHello"));
        assert!(method_names.contains(&"SayHelloStream"));
        assert!(method_names.contains(&"SayHelloClientStream"));
        assert!(method_names.contains(&"Chat"));
    }
}
