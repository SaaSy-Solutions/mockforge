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
