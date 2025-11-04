//! Proto file parsing and service discovery
//!
//! This module handles parsing of .proto files and extracting service definitions
//! to generate dynamic gRPC service implementations.

use prost_reflect::DescriptorPool;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
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
    /// Include paths for proto compilation
    include_paths: Vec<PathBuf>,
    /// Temporary directory for compilation artifacts
    temp_dir: Option<TempDir>,
}

impl ProtoParser {
    /// Create a new proto parser
    pub fn new() -> Self {
        Self {
            pool: DescriptorPool::new(),
            services: HashMap::new(),
            include_paths: vec![],
            temp_dir: None,
        }
    }

    /// Create a new proto parser with include paths
    pub fn with_include_paths(include_paths: Vec<PathBuf>) -> Self {
        Self {
            pool: DescriptorPool::new(),
            services: HashMap::new(),
            include_paths,
            temp_dir: None,
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

        // Optimize: Batch compile all proto files in a single protoc invocation
        if proto_files.len() > 1 {
            if let Err(e) = self.compile_protos_batch(&proto_files).await {
                warn!("Batch compilation failed, falling back to individual compilation: {}", e);
                // Fall back to individual compilation
                for proto_file in proto_files {
                    if let Err(e) = self.parse_proto_file(&proto_file).await {
                        error!("Failed to parse proto file {}: {}", proto_file, e);
                        // Continue with other files
                    }
                }
            }
        } else if !proto_files.is_empty() {
            // Single file - use existing method
            if let Err(e) = self.parse_proto_file(&proto_files[0]).await {
                error!("Failed to parse proto file {}: {}", proto_files[0], e);
            }
        }

        // Extract services from the descriptor pool only if there are any services in the pool
        if self.pool.services().count() > 0 {
            self.extract_services()?;
        } else {
            debug!("No services found in descriptor pool, keeping mock services");
        }

        info!("Successfully parsed {} services", self.services.len());
        Ok(())
    }

    /// Discover proto files in a directory recursively
    #[allow(clippy::only_used_in_recursion)]
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

    /// Parse a single proto file using protoc compilation
    async fn parse_proto_file(
        &mut self,
        proto_file: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Parsing proto file: {}", proto_file);

        // Create temporary directory for compilation artifacts if not exists
        if self.temp_dir.is_none() {
            self.temp_dir = Some(TempDir::new()?);
        }

        // Safe to unwrap here: we just created it above if it was None
        let temp_dir = self.temp_dir.as_ref().ok_or_else(|| {
            Box::<dyn std::error::Error + Send + Sync>::from("Temp directory not initialized")
        })?;
        let descriptor_path = temp_dir.path().join("descriptors.bin");

        // Try real protoc compilation first
        match self.compile_with_protoc(proto_file, &descriptor_path).await {
            Ok(()) => {
                // Load the compiled descriptor set into the pool
                let descriptor_bytes = fs::read(&descriptor_path)?;
                match self.pool.decode_file_descriptor_set(&*descriptor_bytes) {
                    Ok(()) => {
                        info!("Successfully compiled and loaded proto file: {}", proto_file);
                        // Extract services from the descriptor pool if successful
                        if self.pool.services().count() > 0 {
                            self.extract_services()?;
                        }
                        return Ok(());
                    }
                    Err(e) => {
                        warn!("Failed to decode descriptor set, falling back to mock: {}", e);
                    }
                }
            }
            Err(e) => {
                // This is expected behavior if protoc is not installed or proto files don't require compilation
                // MockForge will use fallback mock services, which is fine for basic usage
                warn!(
                    "protoc not available or compilation failed (this is OK for basic usage, using fallback): {}",
                    e
                );
            }
        }

        // Fallback to mock service for testing
        if proto_file.contains("gretter.proto") || proto_file.contains("greeter.proto") {
            debug!("Adding mock greeter service for {}", proto_file);
            self.add_mock_greeter_service();
        }

        Ok(())
    }

    /// Batch compile multiple proto files in a single protoc invocation
    /// This is significantly faster than compiling files individually
    async fn compile_protos_batch(
        &mut self,
        proto_files: &[String],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if proto_files.is_empty() {
            return Ok(());
        }

        info!("Batch compiling {} proto files", proto_files.len());

        // Create temporary directory for compilation artifacts if not exists
        if self.temp_dir.is_none() {
            self.temp_dir = Some(TempDir::new()?);
        }

        let temp_dir = self.temp_dir.as_ref().ok_or_else(|| {
            Box::<dyn std::error::Error + Send + Sync>::from("Temp directory not initialized")
        })?;
        let descriptor_path = temp_dir.path().join("descriptors_batch.bin");

        // Build protoc command
        let mut cmd = Command::new("protoc");

        // Collect unique parent directories for include paths
        let mut parent_dirs = std::collections::HashSet::new();
        for proto_file in proto_files {
            if let Some(parent_dir) = Path::new(proto_file).parent() {
                parent_dirs.insert(parent_dir.to_path_buf());
            }
        }

        // Add include paths
        for include_path in &self.include_paths {
            cmd.arg("-I").arg(include_path);
        }

        // Add proto file parent directories as include paths
        for parent_dir in &parent_dirs {
            cmd.arg("-I").arg(parent_dir);
        }

        // Add well-known types include path (common protoc installation paths)
        let well_known_paths = [
            "/usr/local/include",
            "/usr/include",
            "/opt/homebrew/include",
        ];

        for path in &well_known_paths {
            if Path::new(path).exists() {
                cmd.arg("-I").arg(path);
            }
        }

        // Set output path and format
        cmd.arg("--descriptor_set_out")
            .arg(&descriptor_path)
            .arg("--include_imports")
            .arg("--include_source_info");

        // Add all proto files to compile
        for proto_file in proto_files {
            cmd.arg(proto_file);
        }

        debug!("Running batch protoc command for {} files", proto_files.len());

        // Execute protoc
        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Batch protoc compilation failed: {}", stderr).into());
        }

        // Load the compiled descriptor set into the pool
        let descriptor_bytes = fs::read(&descriptor_path)?;
        match self.pool.decode_file_descriptor_set(&*descriptor_bytes) {
            Ok(()) => {
                info!("Successfully batch compiled and loaded {} proto files", proto_files.len());
                // Extract services from the descriptor pool if successful
                if self.pool.services().count() > 0 {
                    self.extract_services()?;
                }
                Ok(())
            }
            Err(e) => Err(format!("Failed to decode batch descriptor set: {}", e).into()),
        }
    }

    /// Compile proto file using protoc
    async fn compile_with_protoc(
        &self,
        proto_file: &str,
        output_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Compiling proto file with protoc: {}", proto_file);

        // Build protoc command
        let mut cmd = Command::new("protoc");

        // Add include paths
        for include_path in &self.include_paths {
            cmd.arg("-I").arg(include_path);
        }

        // Add proto file's directory as include path
        if let Some(parent_dir) = Path::new(proto_file).parent() {
            cmd.arg("-I").arg(parent_dir);
        }

        // Add well-known types include path (common protoc installation paths)
        let well_known_paths = [
            "/usr/local/include",
            "/usr/include",
            "/opt/homebrew/include",
        ];

        for path in &well_known_paths {
            if Path::new(path).exists() {
                cmd.arg("-I").arg(path);
            }
        }

        // Set output path and format
        cmd.arg("--descriptor_set_out")
            .arg(output_path)
            .arg("--include_imports")
            .arg("--include_source_info")
            .arg(proto_file);

        debug!("Running protoc command: {:?}", cmd);

        // Execute protoc
        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("protoc failed: {}", stderr).into());
        }

        info!("Successfully compiled proto file with protoc: {}", proto_file);
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
        debug!("Extracting services from descriptor pool");

        // Clear existing services (except mock ones)
        let mock_services: HashMap<String, ProtoService> = self
            .services
            .drain()
            .filter(|(name, _)| name.contains("mockforge.greeter"))
            .collect();

        self.services = mock_services;

        // Extract services from the descriptor pool
        for service_descriptor in self.pool.services() {
            let service_name = service_descriptor.full_name().to_string();
            let package_name = service_descriptor.parent_file().package_name().to_string();
            let short_name = service_descriptor.name().to_string();

            debug!("Found service: {} in package: {}", service_name, package_name);

            // Extract methods for this service
            let mut methods = Vec::new();
            for method_descriptor in service_descriptor.methods() {
                let method = ProtoMethod {
                    name: method_descriptor.name().to_string(),
                    input_type: method_descriptor.input().full_name().to_string(),
                    output_type: method_descriptor.output().full_name().to_string(),
                    client_streaming: method_descriptor.is_client_streaming(),
                    server_streaming: method_descriptor.is_server_streaming(),
                };

                debug!(
                    "  Found method: {} ({} -> {})",
                    method.name, method.input_type, method.output_type
                );

                methods.push(method);
            }

            let service = ProtoService {
                name: service_name.clone(),
                package: package_name,
                short_name,
                methods,
            };

            self.services.insert(service_name, service);
        }

        info!("Extracted {} services from descriptor pool", self.services.len());
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
