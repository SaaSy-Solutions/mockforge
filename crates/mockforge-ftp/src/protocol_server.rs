//! Unified protocol server implementation for the FTP mock server.

use async_trait::async_trait;
use mockforge_core::config::FtpConfig;
use mockforge_core::protocol_abstraction::Protocol;
use mockforge_core::protocol_server::MockProtocolServer;

use crate::server::FtpServer;

/// A `MockProtocolServer` wrapper around [`FtpServer`].
///
/// Constructs the FTP server (including VFS and spec registry), then
/// delegates to [`FtpServer::start`] with shutdown-signal integration.
pub struct FtpMockServer {
    config: FtpConfig,
}

impl FtpMockServer {
    /// Create a new `FtpMockServer` with the given configuration.
    pub fn new(config: FtpConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl MockProtocolServer for FtpMockServer {
    fn protocol(&self) -> Protocol {
        Protocol::Ftp
    }

    async fn start(
        &self,
        mut shutdown: tokio::sync::watch::Receiver<()>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let server = FtpServer::new(self.config.clone());

        tokio::select! {
            result = server.start() => {
                result.map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    Box::new(std::io::Error::other(e.to_string()))
                })
            }
            _ = shutdown.changed() => {
                tracing::info!("Shutting down FTP server on port {}", self.config.port);
                Ok(())
            }
        }
    }

    fn port(&self) -> u16 {
        self.config.port
    }

    fn description(&self) -> String {
        format!("FTP server on {}:{}", self.config.host, self.config.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ftp_mock_server_protocol() {
        let server = FtpMockServer::new(FtpConfig::default());
        assert_eq!(server.protocol(), Protocol::Ftp);
    }

    #[test]
    fn test_ftp_mock_server_port() {
        let server = FtpMockServer::new(FtpConfig::default());
        assert_eq!(server.port(), server.config.port);
    }

    #[test]
    fn test_ftp_mock_server_description() {
        let config = FtpConfig {
            host: "127.0.0.1".to_string(),
            port: 2121,
            ..Default::default()
        };
        let server = FtpMockServer::new(config);
        assert_eq!(server.description(), "FTP server on 127.0.0.1:2121");
    }
}
