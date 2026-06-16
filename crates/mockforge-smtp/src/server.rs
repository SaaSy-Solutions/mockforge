//! SMTP server implementation

use crate::{SmtpConfig, SmtpSpecRegistry};
use mockforge_core::protocol_abstraction::{
    MessagePattern, MiddlewareChain, Protocol, ProtocolRequest, SpecRegistry,
};
use mockforge_core::Result;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tokio_rustls::{rustls, server::TlsStream, TlsAcceptor};
use tracing::{debug, error, info, warn};

/// Hard cap on a single SMTP protocol line (command or DATA line) before the
/// connection is dropped. SMTP lines are spec-limited to ~1000 bytes, so 1 MiB
/// is generous headroom while still bounding a newline-less flood (#754).
const MAX_LINE_BYTES: usize = 1024 * 1024;

/// Outcome of a bounded line read.
enum LineRead {
    /// A full line (terminated by `\n` or by EOF after some bytes) was read.
    Ok,
    /// The stream ended with no further bytes.
    Eof,
    /// The line exceeded the byte cap before a newline arrived.
    TooLong,
}

/// Read a single `\n`-terminated line into `line`, but stop and report
/// [`LineRead::TooLong`] once the accumulated line exceeds `max_bytes`.
///
/// This wraps `read_until` one byte chunk at a time so an attacker can't make
/// the underlying buffer grow without bound by simply never sending a newline.
async fn read_line_capped<R>(
    reader: &mut R,
    line: &mut Vec<u8>,
    max_bytes: usize,
) -> std::io::Result<LineRead>
where
    R: AsyncBufReadExt + Unpin,
{
    loop {
        let before = line.len();
        let n = reader.read_until(b'\n', line).await?;
        if n == 0 {
            // EOF: if we'd already accumulated a partial line, treat it as a
            // final line; otherwise the stream is closed.
            return Ok(if line.is_empty() {
                LineRead::Eof
            } else {
                LineRead::Ok
            });
        }
        let ends_with_newline = line.last() == Some(&b'\n');
        if line.len() > max_bytes {
            return Ok(LineRead::TooLong);
        }
        if ends_with_newline {
            return Ok(LineRead::Ok);
        }
        // No newline yet and under the cap — `read_until` returned because the
        // buffered chunk was exhausted; keep reading. Guard against a stuck
        // reader that returns 0-growth without EOF.
        if line.len() == before {
            return Ok(LineRead::Ok);
        }
    }
}

/// Stream wrapper that can be either plaintext TCP or TLS-upgraded.
/// The session handler starts each connection as `Plain` and swaps to
/// `Tls` mid-stream when the client sends STARTTLS (RFC 3207). The
/// enum-plus-manual-AsyncRead/Write-impl pattern is what lets us put
/// the upgraded stream back into the existing `BufReader` without
/// rewriting the whole session loop.
pub enum SmtpStream {
    /// Plaintext TCP before STARTTLS (or when STARTTLS is disabled).
    Plain(TcpStream),
    /// TLS-encrypted after STARTTLS completes. Boxed to keep the
    /// enum size manageable — `TlsStream` is large.
    Tls(Box<TlsStream<TcpStream>>),
}

impl AsyncRead for SmtpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            SmtpStream::Plain(s) => Pin::new(s).poll_read(cx, buf),
            SmtpStream::Tls(s) => Pin::new(s.as_mut()).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for SmtpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            SmtpStream::Plain(s) => Pin::new(s).poll_write(cx, buf),
            SmtpStream::Tls(s) => Pin::new(s.as_mut()).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            SmtpStream::Plain(s) => Pin::new(s).poll_flush(cx),
            SmtpStream::Tls(s) => Pin::new(s.as_mut()).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            SmtpStream::Plain(s) => Pin::new(s).poll_shutdown(cx),
            SmtpStream::Tls(s) => Pin::new(s.as_mut()).poll_shutdown(cx),
        }
    }
}

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

        let cert_path = config.tls_cert_path.as_ref().ok_or_else(|| {
            mockforge_core::Error::internal("TLS certificate path not configured")
        })?;
        let key_path = config.tls_key_path.as_ref().ok_or_else(|| {
            mockforge_core::Error::internal("TLS private key path not configured")
        })?;

        // Load certificate
        let cert_file = File::open(cert_path)?;
        let mut cert_reader = BufReader::new(cert_file);
        let certs: Vec<Vec<u8>> = certs(&mut cert_reader)?;
        // Use rustls types from tokio-rustls for compatibility
        let certs: Vec<rustls::Certificate> = certs.into_iter().map(rustls::Certificate).collect();

        // Load private key
        let key_file = File::open(key_path)?;
        let mut key_reader = BufReader::new(key_file);
        let mut keys: Vec<Vec<u8>> = pkcs8_private_keys(&mut key_reader)?;

        if keys.is_empty() {
            return Err(mockforge_core::Error::internal("No private keys found"));
        }

        // Use rustls from tokio-rustls which has compatible API
        let mut server_config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, rustls::PrivateKey(keys.remove(0)))
            .map_err(|e| mockforge_core::Error::internal(format!("TLS config error: {}", e)))?;

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

        // Cap concurrent sessions at `max_connections` so a connection flood
        // can't spawn unbounded tasks (#755). A permit is acquired before each
        // session task and moved into it, freeing on disconnect.
        let max_connections = self.config.max_connections.max(1);
        let connection_limiter = Arc::new(Semaphore::new(max_connections));
        let max_message_bytes = self.config.max_message_bytes;

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    debug!("New SMTP connection from {}", peer_addr);

                    let permit = match connection_limiter.clone().acquire_owned().await {
                        Ok(permit) => permit,
                        Err(_) => {
                            // Semaphore closed — should not happen while the
                            // server runs, but bail cleanly if it does.
                            error!("SMTP connection limiter closed; stopping accept loop");
                            break Ok(());
                        }
                    };

                    let registry = self.spec_registry.clone();
                    let middleware = self.middleware_chain.clone();
                    let hostname = self.config.hostname.clone();
                    let tls_acceptor = self.tls_acceptor.clone();

                    tokio::spawn(async move {
                        // Hold the permit for the lifetime of the session.
                        let _permit = permit;
                        if let Err(e) = handle_smtp_session(
                            SmtpStream::Plain(stream),
                            peer_addr,
                            registry,
                            middleware,
                            hostname,
                            tls_acceptor,
                            max_message_bytes,
                        )
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

/// Handle a single SMTP session. Accepts an `SmtpStream` (plaintext
/// or TLS) so a STARTTLS mid-session upgrade can swap the underlying
/// transport without tearing down the connection.
async fn handle_smtp_session(
    stream: SmtpStream,
    peer_addr: SocketAddr,
    registry: Arc<SmtpSpecRegistry>,
    middleware: Arc<MiddlewareChain>,
    hostname: String,
    tls_acceptor: Option<TlsAcceptor>,
    max_message_bytes: usize,
) -> Result<()> {
    // Keep read + write on the same `SmtpStream` so STARTTLS can
    // upgrade both halves atomically. Writes go through
    // `reader.get_mut()` to avoid an extra split.
    let mut reader = BufReader::new(stream);

    // Send greeting
    let greeting = format!("220 {} ESMTP MockForge SMTP Server\r\n", hostname);
    reader.get_mut().write_all(greeting.as_bytes()).await?;

    let mut session_state = SessionState::with_max_message_bytes(max_message_bytes);
    // Byte-level accumulator so 8BITMIME bodies (Latin-1, UTF-8 with
    // explicit content-transfer-encoding, etc.) round-trip verbatim.
    // `read_line` would fail on non-UTF-8 inputs.
    let mut line: Vec<u8> = Vec::new();

    loop {
        // Bound each line read so a client that never sends a newline can't
        // make `read_until` accumulate an unbounded buffer (#754).
        match read_line_capped(&mut reader, &mut line, MAX_LINE_BYTES).await? {
            LineRead::Eof => break,
            LineRead::Ok => {}
            LineRead::TooLong => {
                warn!("SMTP line from {} exceeded {} bytes; closing", peer_addr, MAX_LINE_BYTES);
                reader.get_mut().write_all(b"500 line too long\r\n").await?;
                break;
            }
        }

        // DATA mode bypasses command parsing entirely. Without this, the
        // first word of every body line would be matched against the SMTP
        // verb table — any body beginning with "Hello ..." / "Data ..." /
        // "Quit ..." / etc. was being re-interpreted as a command and
        // dropped on the floor. Inside DATA mode, only the bare "." ends
        // the message; everything else (headers, blank separator, body
        // lines, verbatim) accumulates *as bytes*.
        if session_state.in_data_mode {
            // Strip trailing \r\n / \n. If the line is exactly "." +
            // newline, that terminates the DATA section per RFC 5321.
            let trimmed = strip_line_terminator(&line);
            if trimmed == b"." {
                session_state.in_data_mode = false;
                let response =
                    process_email(&session_state, &registry, &middleware, peer_addr).await?;
                reader.get_mut().write_all(response.as_bytes()).await?;
                session_state.reset();
            } else if session_state.data_would_overflow(trimmed.len() + 1) {
                // Reject oversized messages instead of buffering them to OOM
                // (#754). Per RFC 5321 the 552 response ends the DATA phase;
                // reset the transaction and leave DATA mode.
                warn!(
                    "SMTP DATA from {} exceeded max_message_bytes ({}); rejecting",
                    peer_addr, max_message_bytes
                );
                reader
                    .get_mut()
                    .write_all(b"552 message size exceeds fixed maximum message size\r\n")
                    .await?;
                session_state.reset();
            } else {
                session_state.data.extend_from_slice(trimmed);
                session_state.data.push(b'\n');
            }
            line.clear();
            continue;
        }

        // Outside DATA mode, SMTP verbs are ASCII-only per spec. Decode
        // lossily so a malformed UTF-8 byte doesn't crash the session,
        // then parse the verb table as before.
        let as_str = String::from_utf8_lossy(&line);
        let command = as_str.trim();
        debug!("SMTP command from {}: {}", peer_addr, command);

        // AUTH continuation: if the previous command opened an AUTH
        // dialog, this line is base64 credential material, not an
        // SMTP verb. Handle it here before the verb table would send
        // back "502 Command not implemented".
        if let Some(stage) = session_state.pending_auth.clone() {
            handle_auth_continuation(stage, command, &mut session_state, reader.get_mut()).await?;
            line.clear();
            continue;
        }

        // Skip blank lines outside DATA mode — otherwise idle keep-alive
        // newlines would reach the verb parser.
        if command.is_empty() {
            line.clear();
            continue;
        }

        // STARTTLS is handled at this outer level (not in the verb
        // table) because we need to swap out the BufReader's owned
        // stream. Only fires when (a) client asked for STARTTLS,
        // (b) we're still plaintext, and (c) a TLS acceptor is
        // configured. Per RFC 3207, the upgraded session starts
        // completely fresh — the client MUST re-EHLO.
        if command.eq_ignore_ascii_case("STARTTLS") {
            if !matches!(reader.get_ref(), SmtpStream::Plain(_)) {
                // Already TLS — refuse per spec.
                reader.get_mut().write_all(b"503 Command not allowed\r\n").await?;
            } else if let Some(acceptor) = tls_acceptor.clone() {
                reader.get_mut().write_all(b"220 Ready to start TLS\r\n").await?;
                reader.get_mut().flush().await?;

                let inner = reader.into_inner();
                let tcp = match inner {
                    SmtpStream::Plain(t) => t,
                    SmtpStream::Tls(_) => unreachable!("checked is Plain above"),
                };
                let tls_stream = acceptor.accept(tcp).await.map_err(|e| {
                    mockforge_core::Error::internal(format!("TLS accept failed: {e}"))
                })?;
                reader = BufReader::new(SmtpStream::Tls(Box::new(tls_stream)));
                session_state = SessionState::with_max_message_bytes(max_message_bytes);
                line.clear();
                continue;
            } else {
                // STARTTLS requested but no cert configured.
                reader
                    .get_mut()
                    .write_all(b"454 TLS not available due to temporary reason\r\n")
                    .await?;
            }
            line.clear();
            continue;
        }

        // Parse and handle SMTP command
        match handle_smtp_command(
            command,
            &mut session_state,
            reader.get_mut(),
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
                reader.get_mut().write_all(error_response.as_bytes()).await?;
            }
        }

        line.clear();
    }

    Ok(())
}

/// Decode `AUTH PLAIN`'s base64 credential blob. The SASL PLAIN
/// payload is `\0authzid\0authcid\0passwd` (authzid may be empty).
/// Returns the authcid (username) on success; `None` if the base64
/// is malformed or the structure is wrong. The mock accepts any
/// credentials — we only parse enough to pull out the username for
/// mailbox observability.
fn decode_plain_auth(b64: &str) -> Option<String> {
    use base64::Engine as _;
    let decoded = base64::engine::general_purpose::STANDARD.decode(b64.trim()).ok()?;
    // Split on NULs. Valid forms produce 3 segments; we just need the
    // middle one (authcid).
    let mut parts = decoded.split(|b| *b == 0);
    let _authzid = parts.next()?;
    let authcid = parts.next()?;
    let _passwd = parts.next()?;
    Some(String::from_utf8_lossy(authcid).into_owned())
}

/// Handle the line *following* an AUTH open. Advances or completes
/// the dialog depending on the stage we left off in.
async fn handle_auth_continuation<W: AsyncWriteExt + Unpin>(
    stage: AuthStage,
    line: &str,
    state: &mut SessionState,
    writer: &mut W,
) -> Result<()> {
    use base64::Engine as _;
    match stage {
        AuthStage::AwaitingPlainCredentials => {
            state.pending_auth = None;
            match decode_plain_auth(line) {
                Some(user) => {
                    state.authenticated_user = Some(user);
                    writer.write_all(b"235 2.7.0 Authentication successful\r\n").await?;
                }
                None => {
                    writer.write_all(b"535 5.7.8 Authentication credentials invalid\r\n").await?;
                }
            }
        }
        AuthStage::AwaitingLoginUsername => {
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(line.trim())
                .ok()
                .and_then(|b| String::from_utf8(b).ok());
            match decoded {
                Some(u) => {
                    state.authenticated_user = Some(u);
                    state.pending_auth = Some(AuthStage::AwaitingLoginPassword);
                    // "Password:" base64 = "UGFzc3dvcmQ6".
                    writer.write_all(b"334 UGFzc3dvcmQ6\r\n").await?;
                }
                None => {
                    state.pending_auth = None;
                    state.authenticated_user = None;
                    writer.write_all(b"535 5.7.8 Authentication credentials invalid\r\n").await?;
                }
            }
        }
        AuthStage::AwaitingLoginPassword => {
            state.pending_auth = None;
            // We don't actually verify the password — accept anything
            // that decodes as base64. On decode failure reject with
            // 535 so clients that intentionally pass junk (negative
            // tests) still get a sane response.
            if base64::engine::general_purpose::STANDARD.decode(line.trim()).is_ok() {
                writer.write_all(b"235 2.7.0 Authentication successful\r\n").await?;
            } else {
                state.authenticated_user = None;
                writer.write_all(b"535 5.7.8 Authentication credentials invalid\r\n").await?;
            }
        }
    }
    Ok(())
}

/// Strip trailing `\r\n` or `\n` from an SMTP line read via
/// `read_until(b'\n', ...)`. Keeps the rest of the bytes intact so
/// a non-UTF-8 body round-trips verbatim.
fn strip_line_terminator(line: &[u8]) -> &[u8] {
    let mut end = line.len();
    if end > 0 && line[end - 1] == b'\n' {
        end -= 1;
    }
    if end > 0 && line[end - 1] == b'\r' {
        end -= 1;
    }
    &line[..end]
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
                // Advertise AUTH PLAIN LOGIN so clients that gate on the
                // capability line (lettre with Credentials, Python's
                // smtplib.login, etc.) actually try AUTH instead of
                // skipping to MAIL FROM.
                format!(
                    "250-{} Hello {}\r\n\
                     250-SIZE 10485760\r\n\
                     250-8BITMIME\r\n\
                     250-STARTTLS\r\n\
                     250-AUTH PLAIN LOGIN\r\n\
                     250 HELP\r\n",
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

        "AUTH" => {
            // AUTH dispatch. Any credentials are accepted — this is a
            // mock, not an authenticator — but we do go through the full
            // handshake so clients that gate on the 334/235 responses
            // work as expected.
            //
            // `parts` uses `splitn(2, ' ')`, so for `AUTH PLAIN <b64>`
            // we get parts = ["AUTH", "PLAIN <b64>"]. Split the rest
            // into its own mechanism + optional initial-response piece.
            let rest = parts.get(1).copied().unwrap_or("");
            let mut auth_args = rest.splitn(2, ' ');
            let mechanism = auth_args.next().map(|s| s.to_ascii_uppercase()).unwrap_or_default();
            let initial_response = auth_args.next().map(str::trim).filter(|s| !s.is_empty());
            match mechanism.as_str() {
                "PLAIN" => {
                    // Two forms:
                    //   "AUTH PLAIN <base64>"          — single-shot
                    //   "AUTH PLAIN\r\n" then client → <base64>  — two-shot
                    if let Some(b64) = initial_response {
                        match decode_plain_auth(b64) {
                            Some(user) => {
                                state.authenticated_user = Some(user);
                                writer
                                    .write_all(b"235 2.7.0 Authentication successful\r\n")
                                    .await?;
                            }
                            None => {
                                writer
                                    .write_all(b"535 5.7.8 Authentication credentials invalid\r\n")
                                    .await?;
                            }
                        }
                    } else {
                        state.pending_auth = Some(AuthStage::AwaitingPlainCredentials);
                        // 334 with empty challenge = "send me the SASL initial response".
                        writer.write_all(b"334 \r\n").await?;
                    }
                    Ok(true)
                }
                "LOGIN" => {
                    state.pending_auth = Some(AuthStage::AwaitingLoginUsername);
                    // "Username:" base64-encoded = "VXNlcm5hbWU6".
                    writer.write_all(b"334 VXNlcm5hbWU6\r\n").await?;
                    Ok(true)
                }
                _ => {
                    writer
                        .write_all(b"504 5.5.4 Authentication mechanism not supported\r\n")
                        .await?;
                    Ok(true)
                }
            }
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
            // DATA-mode lines are short-circuited before `handle_smtp_command`
            // is called (see `handle_smtp_session`), so we only land here for
            // genuinely unknown verbs.
            if state.in_data_mode {
                // Defensive fallback in case the short-circuit is ever
                // bypassed — keep the original accumulator behavior so the
                // session doesn't derail.
                if command == "." {
                    state.in_data_mode = false;
                    let response = process_email(state, registry, middleware, peer_addr).await?;
                    writer.write_all(response.as_bytes()).await?;
                    state.reset();
                } else if state.data_would_overflow(command.len() + 1) {
                    // Mirror the primary DATA-path cap so the fallback can't be
                    // used to bypass max_message_bytes (#754).
                    warn!("SMTP DATA fallback exceeded max_message_bytes; rejecting");
                    writer
                        .write_all(b"552 message size exceeds fixed maximum message size\r\n")
                        .await?;
                    state.reset();
                } else {
                    // Command path: the caller already trimmed the line
                    // into a `&str`. Non-ASCII bodies are accumulated
                    // via the byte-level DATA-mode branch in
                    // `handle_smtp_session`; this fallback just
                    // preserves the UTF-8 subset.
                    state.data.extend_from_slice(command.as_bytes());
                    state.data.push(b'\n');
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
        .ok_or_else(|| mockforge_core::Error::internal("Missing MAIL FROM"))?;
    let to = state.rcpt_to.join(", ");

    // Extract subject from data
    let subject = extract_subject(&state.data);

    // Capture the delivered message in the in-memory mailbox before we
    // touch fixture-driven response generation. The spec-registry's
    // storage logic used to live inside `generate_mock_response` gated on
    // `fixture.storage.save_to_mailbox`, which meant that if no fixture
    // matched (the common case: users running the mock SMTP to inspect
    // outgoing mail from their application) the message would silently
    // disappear AND the server would return a 500 to the client. Capture
    // is the primary contract of a mock SMTP; fixture matching should only
    // affect the reply.
    let captured = crate::fixtures::StoredEmail {
        id: uuid::Uuid::new_v4().to_string(),
        from: from.clone(),
        to: state.rcpt_to.clone(),
        subject: subject.clone(),
        // `body` is `String`; decode lossy so callers that only want a
        // preview of a UTF-8 message aren't forced to round-trip
        // through `raw`. Byte-exact consumers use `raw`.
        body: String::from_utf8_lossy(&state.data).into_owned(),
        headers: HashMap::from([
            ("from".to_string(), from.clone()),
            ("to".to_string(), to.clone()),
            ("subject".to_string(), subject.clone()),
        ]),
        received_at: chrono::Utc::now(),
        raw: Some(state.data.clone()),
    };
    if let Err(e) = registry.store_email(captured) {
        warn!("Failed to store email in mailbox: {}", e);
    }

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
        body: Some(state.data.clone()),
        client_ip: Some(peer_addr.ip().to_string()),
    };

    // Process through middleware (may short-circuit, e.g., auth rejection)
    if let Some(short_circuit_response) = middleware.process_request(&mut request).await? {
        return Ok(String::from_utf8_lossy(&short_circuit_response.body).to_string());
    }

    // Ask the spec registry for a fixture-driven reply. When no fixture
    // matches, fall back to the standard 250 OK so clients accept the
    // message (the message has already been captured above).
    let response = match registry.generate_mock_response(&request) {
        Ok(mut resp) => {
            middleware.process_response(&request, &mut resp).await?;
            String::from_utf8_lossy(&resp.body).to_string()
        }
        Err(_) => "250 OK\r\n".to_string(),
    };

    Ok(response)
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

/// Extract subject from email data. Takes bytes so non-UTF-8 bodies
/// still succeed; the header zone (above the blank line) is ASCII per
/// RFC 5322, so lossy decoding for the search is safe.
fn extract_subject(data: &[u8]) -> String {
    let header_text = String::from_utf8_lossy(data);
    for line in header_text.lines() {
        // Headers end at the first blank line; stop searching there so we
        // don't accidentally match a "Subject:" that appears in the body.
        if line.is_empty() {
            break;
        }
        if line.to_lowercase().starts_with("subject:") {
            return line[8..].trim().to_string();
        }
    }
    String::new()
}

/// AUTH dialog state. SMTP's AUTH LOGIN / AUTH PLAIN both need
/// multi-round-trip exchanges where the *next* line from the client
/// is not an SMTP verb — it's base64-encoded credential material.
/// The main read loop consults `SessionState.pending_auth` before
/// dispatching to the verb table so that continuation data isn't
/// misrouted into `502 Command not implemented`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)] // all stages share the `Awaiting` prefix by design.
enum AuthStage {
    /// Client sent `AUTH LOGIN`. Next line should be the base64
    /// username; we respond with `334 <base64("Password:")>`.
    AwaitingLoginUsername,
    /// Client sent the AUTH LOGIN username. Next line should be the
    /// base64 password; we accept it and send 235.
    AwaitingLoginPassword,
    /// Client sent `AUTH PLAIN` on its own. Next line should be a
    /// single base64 blob decoding to `\0user\0pass`.
    AwaitingPlainCredentials,
}

/// Session state for SMTP connection.
///
/// `data` is `Vec<u8>` (not `String`) so the DATA body survives
/// byte-for-byte even when the sender negotiated 8BITMIME and
/// included non-ASCII / non-UTF-8 content. The mailbox API still
/// exposes a `String` body via lossy decoding; `raw` holds the
/// byte-accurate version for consumers that need it.
struct SessionState {
    mail_from: Option<String>,
    rcpt_to: Vec<String>,
    data: Vec<u8>,
    in_data_mode: bool,
    /// Mid-AUTH dialog state, `None` when not currently negotiating.
    pending_auth: Option<AuthStage>,
    /// Best-effort capture of the authenticated username so callers
    /// inspecting the mailbox can filter by who sent what. Populated
    /// on successful AUTH PLAIN / AUTH LOGIN; cleared by RSET/reset().
    authenticated_user: Option<String>,
    /// Maximum accepted DATA payload size in bytes. Carried on the
    /// session so both the primary DATA path and the defensive fallback
    /// can enforce the same cap (#754).
    max_message_bytes: usize,
}

impl SessionState {
    #[cfg(test)]
    fn new() -> Self {
        Self::with_max_message_bytes(crate::SmtpConfig::default().max_message_bytes)
    }

    fn with_max_message_bytes(max_message_bytes: usize) -> Self {
        Self {
            mail_from: None,
            rcpt_to: Vec::new(),
            data: Vec::new(),
            in_data_mode: false,
            pending_auth: None,
            authenticated_user: None,
            max_message_bytes,
        }
    }

    /// True if appending `additional` bytes to `data` would exceed the
    /// configured `max_message_bytes` cap.
    fn data_would_overflow(&self, additional: usize) -> bool {
        self.data.len().saturating_add(additional) > self.max_message_bytes
    }

    fn reset(&mut self) {
        self.mail_from = None;
        self.rcpt_to.clear();
        self.data.clear();
        self.in_data_mode = false;
        self.pending_auth = None;
        // `authenticated_user` intentionally survives RSET — RFC 4954:
        // the authenticated state is tied to the connection, not the
        // mail transaction.
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
    fn test_extract_email_address_whitespace() {
        assert_eq!(extract_email_address("  user@example.com  "), "user@example.com");
    }

    #[test]
    fn test_extract_email_address_no_brackets() {
        assert_eq!(extract_email_address("plain@email.com"), "plain@email.com");
    }

    #[test]
    fn test_extract_email_address_mail_from_format() {
        assert_eq!(extract_email_address("FROM:<sender@domain.com>"), "sender@domain.com");
    }

    #[test]
    fn test_extract_subject() {
        let data =
            "From: sender@example.com\nSubject: Test Email\nTo: recipient@example.com\n\nBody text";
        assert_eq!(extract_subject(data.as_bytes()), "Test Email");
    }

    #[test]
    fn test_extract_subject_not_found() {
        let data = "From: sender@example.com\nTo: recipient@example.com\n\nBody text";
        assert_eq!(extract_subject(data.as_bytes()), "");
    }

    #[test]
    fn test_extract_subject_lowercase() {
        let data = "subject: lowercase subject\nFrom: sender@example.com";
        assert_eq!(extract_subject(data.as_bytes()), "lowercase subject");
    }

    #[test]
    fn test_extract_subject_mixed_case() {
        let data = "SUBJECT: UPPERCASE SUBJECT\nFrom: sender@example.com";
        assert_eq!(extract_subject(data.as_bytes()), "UPPERCASE SUBJECT");
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

    #[test]
    fn test_session_state_new() {
        let state = SessionState::new();
        assert!(state.mail_from.is_none());
        assert!(state.rcpt_to.is_empty());
        assert!(state.data.is_empty());
        assert!(!state.in_data_mode);
    }

    #[test]
    fn test_session_state_reset() {
        let mut state = SessionState::new();
        state.mail_from = Some("test@example.com".to_string());
        state.rcpt_to.push("recipient1@example.com".to_string());
        state.rcpt_to.push("recipient2@example.com".to_string());
        state.data = b"Email body content".to_vec();
        state.in_data_mode = true;

        state.reset();

        assert!(state.mail_from.is_none());
        assert!(state.rcpt_to.is_empty());
        assert!(state.data.is_empty());
        assert!(!state.in_data_mode);
    }

    #[test]
    fn test_session_state_multiple_recipients() {
        let mut state = SessionState::new();
        state.rcpt_to.push("a@example.com".to_string());
        state.rcpt_to.push("b@example.com".to_string());
        state.rcpt_to.push("c@example.com".to_string());
        assert_eq!(state.rcpt_to.len(), 3);
    }

    #[test]
    fn test_session_state_data_accumulation() {
        let mut state = SessionState::new();
        state.data.extend_from_slice(b"Line 1\n");
        state.data.extend_from_slice(b"Line 2\n");
        state.data.extend_from_slice(b"Line 3\n");
        assert_eq!(state.data, b"Line 1\nLine 2\nLine 3\n");
    }

    #[test]
    fn test_strip_line_terminator() {
        assert_eq!(strip_line_terminator(b"hello\r\n"), b"hello");
        assert_eq!(strip_line_terminator(b"hello\n"), b"hello");
        assert_eq!(strip_line_terminator(b"hello"), b"hello");
        assert_eq!(strip_line_terminator(b""), b"");
        // Non-UTF-8 bytes survive intact through the strip.
        assert_eq!(strip_line_terminator(b"\xff\xfe\r\n"), b"\xff\xfe");
    }

    #[test]
    fn test_extract_subject_from_bytes_with_non_utf8_body() {
        let mut data = Vec::new();
        data.extend_from_slice(b"From: a@example.test\r\n");
        data.extend_from_slice(b"Subject: 8BITMIME body below\r\n");
        data.extend_from_slice(b"\r\n");
        data.extend_from_slice(&[0xff, 0xfe, 0xfd]); // garbage bytes
        assert_eq!(extract_subject(&data), "8BITMIME body below");
    }

    #[tokio::test]
    async fn test_smtp_server_new() {
        let config = SmtpConfig::default();
        let registry = Arc::new(SmtpSpecRegistry::new());
        let server = SmtpServer::new(config, registry);
        assert!(server.is_ok());
    }

    #[tokio::test]
    async fn test_smtp_server_with_middleware() {
        let config = SmtpConfig::default();
        let registry = Arc::new(SmtpSpecRegistry::new());
        let middleware = Arc::new(MiddlewareChain::new());
        let server = SmtpServer::with_middleware(config, registry, middleware);
        assert!(server.is_ok());
    }
}
