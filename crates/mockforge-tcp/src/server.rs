//! TCP server implementation

use crate::{TcpConfig, TcpSpecRegistry};
use mockforge_core::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{sleep, timeout, Duration};
use tracing::{debug, error, info, warn};

/// TCP server
pub struct TcpServer {
    config: TcpConfig,
    spec_registry: Arc<TcpSpecRegistry>,
}

impl TcpServer {
    /// Create a new TCP server
    pub fn new(config: TcpConfig, spec_registry: Arc<TcpSpecRegistry>) -> Result<Self> {
        Ok(Self {
            config,
            spec_registry,
        })
    }

    /// Start the TCP server
    pub async fn start(&self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await?;

        info!("TCP server listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    debug!("New TCP connection from {}", peer_addr);

                    let registry = self.spec_registry.clone();
                    let config = self.config.clone();

                    tokio::spawn(async move {
                        if let Err(e) =
                            handle_tcp_connection(stream, peer_addr, registry, config).await
                        {
                            error!("TCP connection error from {}: {}", peer_addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept TCP connection: {}", e);
                }
            }
        }
    }
}

/// Handle a single TCP connection
async fn handle_tcp_connection(
    mut stream: TcpStream,
    peer_addr: SocketAddr,
    registry: Arc<TcpSpecRegistry>,
    config: TcpConfig,
) -> Result<()> {
    debug!("Handling TCP connection from {}", peer_addr);

    let mut buffer = vec![0u8; config.read_buffer_size];
    let mut accumulated_data = Vec::new();

    loop {
        // Set read timeout
        let read_timeout = Duration::from_secs(config.timeout_secs);

        match timeout(read_timeout, stream.read(&mut buffer)).await {
            Ok(Ok(0)) => {
                // Connection closed by client
                debug!("TCP connection closed by client: {}", peer_addr);
                break;
            }
            Ok(Ok(n)) => {
                let received_data = &buffer[..n];
                accumulated_data.extend_from_slice(received_data);

                debug!("Received {} bytes from {}", n, peer_addr);

                // Try to find matching fixture
                let response_data =
                    if let Some(fixture) = registry.find_matching_fixture(&accumulated_data) {
                        debug!("Found matching fixture: {}", fixture.identifier);

                        // Apply delay if configured
                        if fixture.response.delay_ms > 0 {
                            sleep(Duration::from_millis(fixture.response.delay_ms)).await;
                        }

                        // Generate response data
                        generate_response_data(&fixture.response)?
                    } else if config.echo_mode {
                        // Echo mode: echo back received data
                        debug!("No fixture match, echoing data back");
                        accumulated_data.clone()
                    } else {
                        // No match and echo mode disabled - close connection
                        warn!("No fixture match and echo mode disabled, closing connection");
                        break;
                    };

                // Send response
                if !response_data.is_empty() {
                    if let Err(e) = stream.write_all(&response_data).await {
                        error!("Failed to write response to {}: {}", peer_addr, e);
                        break;
                    }

                    if let Err(e) = stream.flush().await {
                        error!("Failed to flush response to {}: {}", peer_addr, e);
                        break;
                    }
                }

                // Check if we should close after response
                if let Some(fixture) = registry.find_matching_fixture(&accumulated_data) {
                    if fixture.response.close_after_response {
                        debug!("Closing connection after response as configured");
                        break;
                    }

                    if !fixture.response.keep_alive {
                        debug!("Closing connection (keep_alive=false)");
                        break;
                    }
                } else if !config.echo_mode {
                    // Close if echo mode disabled and no fixture matched
                    break;
                }

                // If delimiter is configured, check if we've received complete message
                if let Some(ref delimiter) = config.delimiter {
                    if accumulated_data.ends_with(delimiter) {
                        debug!("Received complete message (matched delimiter), resetting buffer");
                        accumulated_data.clear();
                    }
                } else {
                    // Stream mode: reset buffer for next read
                    accumulated_data.clear();
                }
            }
            Ok(Err(e)) => {
                error!("TCP read error from {}: {}", peer_addr, e);
                break;
            }
            Err(_) => {
                warn!("TCP read timeout from {}", peer_addr);
                break;
            }
        }
    }

    debug!("TCP connection handler finished for {}", peer_addr);
    Ok(())
}

/// Generate response data from fixture configuration
fn generate_response_data(response: &crate::fixtures::TcpResponse) -> Result<Vec<u8>> {
    match response.encoding.as_str() {
        "hex" => hex::decode(&response.data)
            .map_err(|e| mockforge_core::Error::generic(format!("Invalid hex data: {}", e))),
        "base64" => base64::decode(&response.data)
            .map_err(|e| mockforge_core::Error::generic(format!("Invalid base64 data: {}", e))),
        "text" => Ok(response.data.as_bytes().to_vec()),
        "file" => {
            let file_path = response.file_path.as_ref().ok_or_else(|| {
                mockforge_core::Error::generic("file_path not specified for file encoding")
            })?;

            std::fs::read(file_path).map_err(|e| {
                mockforge_core::Error::generic(format!(
                    "Failed to read file {:?}: {}",
                    file_path, e
                ))
            })
        }
        _ => Err(mockforge_core::Error::generic(format!(
            "Unknown encoding: {}. Supported: hex, base64, text, file",
            response.encoding
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::TcpResponse;
    use std::io::Write;
    use std::path::PathBuf;

    fn create_test_response(data: &str, encoding: &str) -> TcpResponse {
        TcpResponse {
            data: data.to_string(),
            encoding: encoding.to_string(),
            file_path: None,
            delay_ms: 0,
            close_after_response: false,
            keep_alive: true,
        }
    }

    #[test]
    fn test_tcp_server_new() {
        let config = TcpConfig::default();
        let registry = Arc::new(TcpSpecRegistry::new());

        let server = TcpServer::new(config.clone(), registry.clone());
        assert!(server.is_ok());

        let server = server.unwrap();
        assert_eq!(server.config.port, config.port);
        assert_eq!(server.config.host, config.host);
    }

    #[test]
    fn test_tcp_server_new_with_custom_config() {
        let config = TcpConfig {
            port: 8080,
            host: "127.0.0.1".to_string(),
            timeout_secs: 60,
            echo_mode: false,
            ..Default::default()
        };
        let registry = Arc::new(TcpSpecRegistry::new());

        let server = TcpServer::new(config.clone(), registry).unwrap();
        assert_eq!(server.config.port, 8080);
        assert_eq!(server.config.host, "127.0.0.1");
        assert_eq!(server.config.timeout_secs, 60);
        assert!(!server.config.echo_mode);
    }

    #[test]
    fn test_generate_response_data_text_encoding() {
        let response = create_test_response("Hello, World!", "text");
        let result = generate_response_data(&response);

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data, b"Hello, World!");
        assert_eq!(String::from_utf8(data).unwrap(), "Hello, World!");
    }

    #[test]
    fn test_generate_response_data_text_encoding_empty() {
        let response = create_test_response("", "text");
        let result = generate_response_data(&response);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"");
    }

    #[test]
    fn test_generate_response_data_text_encoding_unicode() {
        let response = create_test_response("Hello 世界 🌍", "text");
        let result = generate_response_data(&response);

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(String::from_utf8(data).unwrap(), "Hello 世界 🌍");
    }

    #[test]
    fn test_generate_response_data_hex_encoding() {
        let response = create_test_response("48656c6c6f", "hex"); // "Hello" in hex
        let result = generate_response_data(&response);

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data, b"Hello");
    }

    #[test]
    fn test_generate_response_data_hex_encoding_uppercase() {
        let response = create_test_response("48656C6C6F", "hex"); // "Hello" in hex (uppercase)
        let result = generate_response_data(&response);

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data, b"Hello");
    }

    #[test]
    fn test_generate_response_data_hex_encoding_mixed_case() {
        let response = create_test_response("48656c6C6f", "hex"); // "Hello" in hex (mixed case)
        let result = generate_response_data(&response);

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data, b"Hello");
    }

    #[test]
    fn test_generate_response_data_hex_encoding_invalid() {
        let response = create_test_response("GGGG", "hex"); // Invalid hex
        let result = generate_response_data(&response);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Invalid hex data"));
    }

    #[test]
    fn test_generate_response_data_hex_encoding_odd_length() {
        let response = create_test_response("123", "hex"); // Odd length hex string
        let result = generate_response_data(&response);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Invalid hex data"));
    }

    #[test]
    fn test_generate_response_data_base64_encoding() {
        let response = create_test_response("SGVsbG8gV29ybGQ=", "base64"); // "Hello World" in base64
        let result = generate_response_data(&response);

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data, b"Hello World");
    }

    #[test]
    fn test_generate_response_data_base64_encoding_with_padding() {
        let response = create_test_response("SGVsbG8=", "base64"); // "Hello" in base64 with padding
        let result = generate_response_data(&response);

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data, b"Hello");
    }

    #[test]
    fn test_generate_response_data_base64_url_safe() {
        let response = create_test_response("PEJPRA==", "base64"); // base64 standard encoding
        let result = generate_response_data(&response);

        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_generate_response_data_base64_encoding_invalid() {
        let response = create_test_response("!!!invalid@@@", "base64"); // Invalid base64
        let result = generate_response_data(&response);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Invalid base64 data"));
    }

    #[test]
    fn test_generate_response_data_file_encoding() {
        // Create a temporary file
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        temp_file.write_all(b"File content").unwrap();
        temp_file.flush().unwrap();

        let mut response = create_test_response("", "file");
        response.file_path = Some(temp_file.path().to_path_buf());

        let result = generate_response_data(&response);

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data, b"File content");
    }

    #[test]
    fn test_generate_response_data_file_encoding_binary() {
        // Create a temporary file with binary data
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        let binary_data = vec![0x00, 0x01, 0x02, 0xFF, 0xFE, 0xFD];
        temp_file.write_all(&binary_data).unwrap();
        temp_file.flush().unwrap();

        let mut response = create_test_response("", "file");
        response.file_path = Some(temp_file.path().to_path_buf());

        let result = generate_response_data(&response);

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data, binary_data);
    }

    #[test]
    fn test_generate_response_data_file_encoding_no_path() {
        let response = create_test_response("", "file");
        // file_path is None

        let result = generate_response_data(&response);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("file_path not specified"));
    }

    #[test]
    fn test_generate_response_data_file_encoding_nonexistent_file() {
        let mut response = create_test_response("", "file");
        response.file_path = Some(PathBuf::from("/nonexistent/path/to/file.txt"));

        let result = generate_response_data(&response);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Failed to read file"));
    }

    #[test]
    fn test_generate_response_data_unknown_encoding() {
        let response = create_test_response("data", "unknown");
        let result = generate_response_data(&response);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Unknown encoding: unknown"));
        assert!(error.to_string().contains("Supported: hex, base64, text, file"));
    }

    #[test]
    fn test_generate_response_data_case_sensitive_encoding() {
        // Test that encoding is case-sensitive
        let response = create_test_response("SGVsbG8=", "BASE64"); // uppercase encoding
        let result = generate_response_data(&response);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown encoding"));
    }

    #[test]
    fn test_generate_response_data_text_with_special_chars() {
        let response = create_test_response("Line1\nLine2\r\nLine3\t\0End", "text");
        let result = generate_response_data(&response);

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data, b"Line1\nLine2\r\nLine3\t\0End");
    }

    #[test]
    fn test_generate_response_data_hex_empty() {
        let response = create_test_response("", "hex");
        let result = generate_response_data(&response);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"");
    }

    #[test]
    fn test_generate_response_data_base64_empty() {
        let response = create_test_response("", "base64");
        let result = generate_response_data(&response);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"");
    }

    #[test]
    fn test_generate_response_data_hex_with_spaces() {
        // Hex decoder doesn't handle spaces, should fail
        let response = create_test_response("48 65 6c 6c 6f", "hex");
        let result = generate_response_data(&response);

        assert!(result.is_err());
    }

    #[test]
    fn test_tcp_server_config_fields() {
        let config = TcpConfig {
            port: 9000,
            host: "localhost".to_string(),
            fixtures_dir: Some(PathBuf::from("/tmp/fixtures")),
            timeout_secs: 120,
            max_connections: 50,
            read_buffer_size: 4096,
            write_buffer_size: 4096,
            enable_tls: true,
            tls_cert_path: Some(PathBuf::from("/path/to/cert.pem")),
            tls_key_path: Some(PathBuf::from("/path/to/key.pem")),
            echo_mode: false,
            delimiter: Some(b"\r\n".to_vec()),
        };

        let registry = Arc::new(TcpSpecRegistry::new());
        let server = TcpServer::new(config, registry).unwrap();

        assert_eq!(server.config.port, 9000);
        assert_eq!(server.config.host, "localhost");
        assert_eq!(server.config.timeout_secs, 120);
        assert_eq!(server.config.max_connections, 50);
        assert_eq!(server.config.read_buffer_size, 4096);
        assert_eq!(server.config.write_buffer_size, 4096);
        assert!(server.config.enable_tls);
        assert!(!server.config.echo_mode);
        assert_eq!(server.config.delimiter, Some(b"\r\n".to_vec()));
    }

    #[test]
    fn test_tcp_response_with_delay() {
        let response = TcpResponse {
            data: "delayed".to_string(),
            encoding: "text".to_string(),
            file_path: None,
            delay_ms: 500,
            close_after_response: true,
            keep_alive: false,
        };

        let result = generate_response_data(&response);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"delayed");
        // Note: delay is applied in handle_tcp_connection, not in generate_response_data
    }

    #[test]
    fn test_tcp_response_close_after_response() {
        let response = TcpResponse {
            data: "close me".to_string(),
            encoding: "text".to_string(),
            file_path: None,
            delay_ms: 0,
            close_after_response: true,
            keep_alive: false,
        };

        assert!(response.close_after_response);
        assert!(!response.keep_alive);

        let result = generate_response_data(&response);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_response_data_large_text() {
        let large_text = "x".repeat(100_000);
        let response = create_test_response(&large_text, "text");
        let result = generate_response_data(&response);

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.len(), 100_000);
        assert_eq!(data, large_text.as_bytes());
    }

    #[test]
    fn test_generate_response_data_large_hex() {
        // Generate 10000 bytes of hex data (20000 hex chars)
        let hex_data = "00".repeat(10_000);
        let response = create_test_response(&hex_data, "hex");
        let result = generate_response_data(&response);

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.len(), 10_000);
        assert!(data.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_file_encoding_empty_file() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        // Don't write anything, leave it empty

        let mut response = create_test_response("", "file");
        response.file_path = Some(temp_file.path().to_path_buf());

        let result = generate_response_data(&response);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"");
    }

    #[test]
    fn test_file_encoding_large_file() {
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        let large_data = vec![0xAB; 50_000]; // 50KB of data
        temp_file.write_all(&large_data).unwrap();
        temp_file.flush().unwrap();

        let mut response = create_test_response("", "file");
        response.file_path = Some(temp_file.path().to_path_buf());

        let result = generate_response_data(&response);

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.len(), 50_000);
        assert_eq!(data, large_data);
    }
}
