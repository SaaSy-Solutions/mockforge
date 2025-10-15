//! SMTP server implementation

use crate::{SmtpConfig, SmtpSpecRegistry};
use mockforge_core::protocol_abstraction::{
    MessagePattern, MiddlewareChain, Protocol, ProtocolRequest, SpecRegistry,
};
use mockforge_core::Result;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tracing::{debug, error, info, warn};

/// SMTP server
pub struct SmtpServer {
    config: SmtpConfig,
    spec_registry: Arc<SmtpSpecRegistry>,
    middleware_chain: Arc<MiddlewareChain>,
    #[allow(dead_code)]
    tls_acceptor: Option<TlsAcceptor>,
}

impl SmtpServer {
    /// Create a new SMTP server
    pub fn new(config: SmtpConfig, spec_registry: Arc<SmtpSpecRegistry>) -> Result<Self> {
        let middleware_chain = Arc::new(MiddlewareChain::new());

        let tls_acceptor = if config.enable_starttls {
            Some(Self::load_tls_acceptor(&config)?)
        } else {
            None
        };

        Ok(Self {
            config,
            spec_registry,
            middleware_chain,
            tls_acceptor,
        })
    }

    /// Load TLS acceptor from certificate and key files
    fn load_tls_acceptor(config: &SmtpConfig) -> Result<TlsAcceptor> {
        use rustls_pemfile::{certs, pkcs8_private_keys};
        use std::fs::File;
        use std::io::BufReader;

        let cert_path = config
            .tls_cert_path
            .as_ref()
            .ok_or_else(|| mockforge_core::Error::generic("TLS certificate path not configured"))?;
        let key_path = config
            .tls_key_path
            .as_ref()
            .ok_or_else(|| mockforge_core::Error::generic("TLS private key path not configured"))?;

        // Load certificate
        let cert_file = File::open(cert_path)?;
        let mut cert_reader = BufReader::new(cert_file);
        let certs: Vec<Vec<u8>> = certs(&mut cert_reader)?;
        let certs = certs.into_iter().map(rustls::Certificate).collect();

        // Load private key
        let key_file = File::open(key_path)?;
        let mut key_reader = BufReader::new(key_file);
        let mut keys: Vec<Vec<u8>> = pkcs8_private_keys(&mut key_reader)?;

        if keys.is_empty() {
            return Err(mockforge_core::Error::generic("No private keys found"));
        }

        let mut server_config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, rustls::PrivateKey(keys.remove(0)))
            .map_err(|e| mockforge_core::Error::generic(format!("TLS config error: {}", e)))?;

        server_config.alpn_protocols = vec![b"smtp".to_vec()];

        Ok(TlsAcceptor::from(Arc::new(server_config)))
    }

    /// Create a new SMTP server with custom middleware
    pub fn with_middleware(
        config: SmtpConfig,
        spec_registry: Arc<SmtpSpecRegistry>,
        middleware_chain: Arc<MiddlewareChain>,
    ) -> Result<Self> {
        let tls_acceptor = if config.enable_starttls {
            Some(Self::load_tls_acceptor(&config)?)
        } else {
            None
        };

        Ok(Self {
            config,
            spec_registry,
            middleware_chain,
            tls_acceptor,
        })
    }

    /// Start the SMTP server
    pub async fn start(&self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await?;

        info!("SMTP server listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    debug!("New SMTP connection from {}", peer_addr);

                    let registry = self.spec_registry.clone();
                    let middleware = self.middleware_chain.clone();
                    let hostname = self.config.hostname.clone();

                    tokio::spawn(async move {
                        if let Err(e) =
                            handle_smtp_session(stream, peer_addr, registry, middleware, hostname)
                                .await
                        {
                            error!("SMTP session error from {}: {}", peer_addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept SMTP connection: {}", e);
                }
            }
        }
    }
}

/// Handle a single SMTP session
async fn handle_smtp_session(
    stream: TcpStream,
    peer_addr: SocketAddr,
    registry: Arc<SmtpSpecRegistry>,
    middleware: Arc<MiddlewareChain>,
    hostname: String,
) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Send greeting
    let greeting = format!("220 {} ESMTP MockForge SMTP Server\r\n", hostname);
    writer.write_all(greeting.as_bytes()).await?;

    let mut session_state = SessionState::new();
    let mut line = String::new();

    while reader.read_line(&mut line).await? > 0 {
        let command = line.trim();
        debug!("SMTP command from {}: {}", peer_addr, command);

        if command.is_empty() {
            line.clear();
            continue;
        }

        // Parse and handle SMTP command
        match handle_smtp_command(
            command,
            &mut session_state,
            &mut writer,
            &hostname,
            &registry,
            &middleware,
            peer_addr,
        )
        .await
        {
            Ok(should_continue) => {
                if !should_continue {
                    debug!("SMTP session ended for {}", peer_addr);
                    break;
                }
            }
            Err(e) => {
                error!("Error handling SMTP command: {}", e);
                let error_response = "500 Internal server error\r\n";
                writer.write_all(error_response.as_bytes()).await?;
            }
        }

        line.clear();
    }

    Ok(())
}

/// Handle a single SMTP command
async fn handle_smtp_command<W: AsyncWriteExt + Unpin>(
    command: &str,
    state: &mut SessionState,
    writer: &mut W,
    hostname: &str,
    registry: &Arc<SmtpSpecRegistry>,
    middleware: &Arc<MiddlewareChain>,
    peer_addr: SocketAddr,
) -> Result<bool> {
    let parts: Vec<&str> = command.splitn(2, ' ').collect();
    let cmd = parts[0].to_uppercase();

    match cmd.as_str() {
        "HELLO" | "EHLO" => {
            let domain = parts.get(1).unwrap_or(&hostname);
            let response = if cmd == "EHLO" {
                format!(
                    "250-{} Hello {}\r\n250-SIZE 10485760\r\n250-8BITMIME\r\n250-STARTTLS\r\n250 HELP\r\n",
                    hostname, domain
                )
            } else {
                format!("250 {} Hello {}\r\n", hostname, domain)
            };
            writer.write_all(response.as_bytes()).await?;
            Ok(true)
        }

        "MAIL" => {
            if let Some(from_part) = parts.get(1) {
                // Parse MAIL FROM:<address>
                let from = extract_email_address(from_part);
                state.mail_from = Some(from);
                writer.write_all(b"250 OK\r\n").await?;
            } else {
                writer.write_all(b"501 Syntax error in parameters\r\n").await?;
            }
            Ok(true)
        }

        "RCPT" => {
            if let Some(to_part) = parts.get(1) {
                // Parse RCPT TO:<address>
                let to = extract_email_address(to_part);
                state.rcpt_to.push(to);
                writer.write_all(b"250 OK\r\n").await?;
            } else {
                writer.write_all(b"501 Syntax error in parameters\r\n").await?;
            }
            Ok(true)
        }

        "DATA" => {
            writer.write_all(b"354 Start mail input; end with <CRLF>.<CRLF>\r\n").await?;
            state.in_data_mode = true;
            Ok(true)
        }

        "RSET" => {
            state.reset();
            writer.write_all(b"250 OK\r\n").await?;
            Ok(true)
        }

        "NOOP" => {
            writer.write_all(b"250 OK\r\n").await?;
            Ok(true)
        }

        "QUIT" => {
            writer.write_all(b"221 Bye\r\n").await?;
            Ok(false) // End session
        }

        "STARTTLS" => {
            // Mock STARTTLS implementation - accept but don't actually upgrade
            writer.write_all(b"220 Ready to start TLS\r\n").await?;
            Ok(true)
        }

        "HELP" => {
            let help_text = "214-Commands supported:\r\n\
                            214-  HELLO EHLO MAIL RCPT DATA\r\n\
                            214-  RSET NOOP QUIT HELP STARTTLS\r\n\
                            214 End of HELP info\r\n";
            writer.write_all(help_text.as_bytes()).await?;
            Ok(true)
        }

        _ => {
            // Handle data mode or unknown command
            if state.in_data_mode {
                if command == "." {
                    // End of data
                    state.in_data_mode = false;

                    // Process the email
                    let response = process_email(state, registry, middleware, peer_addr).await?;

                    writer.write_all(response.as_bytes()).await?;
                    state.reset();
                } else {
                    // Accumulate email data
                    state.data.push_str(command);
                    state.data.push('\n');
                }
                Ok(true)
            } else {
                warn!("Unknown SMTP command: {}", command);
                writer.write_all(b"502 Command not implemented\r\n").await?;
                Ok(true)
            }
        }
    }
}

/// Process received email and generate response
async fn process_email(
    state: &SessionState,
    registry: &Arc<SmtpSpecRegistry>,
    middleware: &Arc<MiddlewareChain>,
    peer_addr: SocketAddr,
) -> Result<String> {
    let from = state
        .mail_from
        .as_ref()
        .ok_or_else(|| mockforge_core::Error::generic("Missing MAIL FROM"))?;
    let to = state.rcpt_to.join(", ");

    // Extract subject from data
    let subject = extract_subject(&state.data);

    // Create protocol request
    let mut request = ProtocolRequest {
        protocol: Protocol::Smtp,
        pattern: MessagePattern::OneWay,
        operation: "SEND".to_string(),
        path: from.clone(),
        topic: None,
        routing_key: None,
        partition: None,
        qos: None,
        metadata: HashMap::from([
            ("from".to_string(), from.clone()),
            ("to".to_string(), to.clone()),
            ("subject".to_string(), subject.clone()),
        ]),
        body: Some(state.data.as_bytes().to_vec()),
        client_ip: Some(peer_addr.ip().to_string()),
    };

    // Process through middleware
    middleware.process_request(&mut request).await?;

    // Generate response
    let mut response = registry.generate_mock_response(&request)?;

    // Process response through middleware
    middleware.process_response(&request, &mut response).await?;

    // Return SMTP response
    Ok(String::from_utf8_lossy(&response.body).to_string())
}

/// Extract email address from SMTP command parameter
fn extract_email_address(param: &str) -> String {
    // Handle formats like "FROM:<user@example.com>" or "TO:<user@example.com>"
    if let Some(start) = param.find('<') {
        if let Some(end) = param.find('>') {
            return param[start + 1..end].to_string();
        }
    }

    // If no angle brackets, just trim and return
    param.trim().to_string()
}

/// Extract subject from email data
fn extract_subject(data: &str) -> String {
    for line in data.lines() {
        if line.to_lowercase().starts_with("subject:") {
            return line[8..].trim().to_string();
        }
    }
    String::new()
}

/// Session state for SMTP connection
struct SessionState {
    mail_from: Option<String>,
    rcpt_to: Vec<String>,
    data: String,
    in_data_mode: bool,
}

impl SessionState {
    fn new() -> Self {
        Self {
            mail_from: None,
            rcpt_to: Vec::new(),
            data: String::new(),
            in_data_mode: false,
        }
    }

    fn reset(&mut self) {
        self.mail_from = None;
        self.rcpt_to.clear();
        self.data.clear();
        self.in_data_mode = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_email_address() {
        assert_eq!(extract_email_address("FROM:<user@example.com>"), "user@example.com");
        assert_eq!(extract_email_address("TO:<admin@test.com>"), "admin@test.com");
        assert_eq!(extract_email_address("user@example.com"), "user@example.com");
    }

    #[test]
    fn test_extract_subject() {
        let data =
            "From: sender@example.com\nSubject: Test Email\nTo: recipient@example.com\n\nBody text";
        assert_eq!(extract_subject(data), "Test Email");
    }

    #[test]
    fn test_session_state() {
        let mut state = SessionState::new();
        assert!(state.mail_from.is_none());
        assert_eq!(state.rcpt_to.len(), 0);

        state.mail_from = Some("sender@example.com".to_string());
        state.rcpt_to.push("recipient@example.com".to_string());

        state.reset();
        assert!(state.mail_from.is_none());
        assert_eq!(state.rcpt_to.len(), 0);
    }
}
