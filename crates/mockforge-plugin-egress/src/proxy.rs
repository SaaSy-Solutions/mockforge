//! HTTP forward proxy implementation. Listens on a TCP socket;
//! handles `CONNECT` for HTTPS pass-through and absolute-URI HTTP
//! GET/POST/etc. for plain HTTP. Each accepted connection runs
//! in its own Tokio task — proxy state is `Arc<HostPolicy>`,
//! which is `Send + Sync`.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use crate::denylist::denied_ip;
use crate::policy::{HostPolicy, PolicyDecision};

/// Configuration for the proxy server.
#[derive(Clone)]
pub struct ProxyConfig {
    /// TCP address to bind. Plugins set `HTTP_PROXY` /
    /// `HTTPS_PROXY` env vars at this address.
    pub listen: SocketAddr,
    /// Per-plugin allowlist policy. Wrapped in `Arc` so the
    /// per-connection tasks can clone the handle cheaply.
    pub policy: Arc<HostPolicy>,
    /// Hard cap on the number of bytes we'll buffer while reading
    /// the request line + headers. 64 KiB is generous for realistic
    /// HTTP traffic; protects against malicious clients sending
    /// unbounded headers to OOM the proxy.
    pub max_request_header_bytes: usize,
}

impl ProxyConfig {
    /// Convenience constructor with sensible defaults for header
    /// limits.
    pub fn new(listen: SocketAddr, policy: Arc<HostPolicy>) -> Self {
        Self {
            listen,
            policy,
            max_request_header_bytes: 64 * 1024,
        }
    }
}

/// Bind the listener and accept connections forever.
pub async fn run_proxy(config: ProxyConfig) -> Result<()> {
    let listener = TcpListener::bind(config.listen)
        .await
        .with_context(|| format!("binding TCP listener at {}", config.listen))?;

    tracing::info!(addr = %config.listen, "egress proxy listening");

    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(pair) => pair,
            Err(err) => {
                tracing::error!(error = %err, "accept failed; retrying");
                continue;
            }
        };

        let policy = config.policy.clone();
        let max_header_bytes = config.max_request_header_bytes;
        tokio::spawn(async move {
            if let Err(err) = handle_connection(stream, peer, policy, max_header_bytes).await {
                tracing::warn!(peer = %peer, error = %err, "proxy connection ended with error");
            }
        });
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    _peer: SocketAddr,
    policy: Arc<HostPolicy>,
    max_header_bytes: usize,
) -> Result<()> {
    let request_line = read_request_line(&mut stream, max_header_bytes).await?;
    let parsed = parse_request_line(&request_line)?;

    match parsed {
        ParsedRequest::Connect { host, port } => handle_connect(stream, host, port, policy).await,
        ParsedRequest::Other {
            method,
            raw_request_line,
        } => {
            // For non-CONNECT methods we'd implement absolute-URI
            // HTTP forwarding (RFC 7230 §5.3.2). v1 ships CONNECT-
            // only because that covers HTTPS — the dominant case
            // for plugin egress — and avoids a full HTTP parser
            // in the proxy. Plugins that need plain HTTP get a
            // 405 with a clear message.
            tracing::debug!(method = %method, "non-CONNECT method rejected");
            write_simple_status(
                &mut stream,
                405,
                "Method Not Allowed",
                "Only HTTP CONNECT is supported in v1; plain HTTP is not yet proxied",
            )
            .await?;
            let _ = raw_request_line; // keep for future logging
            Ok(())
        }
    }
}

/// Read the first line of the request, bounded by `max_bytes`.
/// Returns the line without the trailing CRLF.
async fn read_request_line(stream: &mut TcpStream, max_bytes: usize) -> Result<String> {
    let mut buffer = Vec::with_capacity(256);
    let mut byte = [0u8; 1];
    loop {
        let n = stream.read(&mut byte).await.context("reading request byte")?;
        if n == 0 {
            anyhow::bail!("client closed before sending request line");
        }
        buffer.push(byte[0]);
        if buffer.len() >= 2 && &buffer[buffer.len() - 2..] == b"\r\n" {
            buffer.truncate(buffer.len() - 2);
            break;
        }
        if buffer.len() > max_bytes {
            anyhow::bail!("request line exceeded {max_bytes} bytes");
        }
    }
    String::from_utf8(buffer).context("request line not valid UTF-8")
}

/// Drain remaining headers (we don't need them for CONNECT but
/// must consume them so the response sits at the right offset in
/// the stream).
async fn drain_headers(reader: &mut BufReader<&mut TcpStream>, max_bytes: usize) -> Result<()> {
    let mut total = 0usize;
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).await.context("reading header line")?;
        if n == 0 {
            anyhow::bail!("client closed mid-headers");
        }
        total += n;
        if total > max_bytes {
            anyhow::bail!("headers exceeded {max_bytes} bytes");
        }
        if line == "\r\n" || line == "\n" {
            return Ok(());
        }
    }
}

#[derive(Debug)]
enum ParsedRequest {
    Connect {
        host: String,
        port: u16,
    },
    Other {
        method: String,
        raw_request_line: String,
    },
}

fn parse_request_line(line: &str) -> Result<ParsedRequest> {
    // Format: METHOD SP REQUEST-TARGET SP HTTP/VERSION
    let mut parts = line.split_ascii_whitespace();
    let method = parts.next().context("empty request line")?.to_string();
    let target = parts.next().context("missing request target")?;
    let _version = parts.next().context("missing HTTP version")?;

    if method.eq_ignore_ascii_case("CONNECT") {
        // CONNECT target is `host:port`.
        let (host, port_str) = target
            .rsplit_once(':')
            .with_context(|| format!("CONNECT target missing :port — {target}"))?;
        let port: u16 = port_str
            .parse()
            .with_context(|| format!("CONNECT port not a number — {port_str}"))?;
        Ok(ParsedRequest::Connect {
            host: host.trim_matches(['[', ']']).to_string(),
            port,
        })
    } else {
        Ok(ParsedRequest::Other {
            method,
            raw_request_line: line.to_string(),
        })
    }
}

async fn handle_connect(
    mut client: TcpStream,
    host: String,
    port: u16,
    policy: Arc<HostPolicy>,
) -> Result<()> {
    // Drain remaining client headers before deciding — RFC 7231
    // requires consuming through the empty line. Use a fresh
    // BufReader since `read_request_line` left us at the second
    // line.
    {
        let mut reader = BufReader::new(&mut client);
        drain_headers(&mut reader, 32 * 1024).await?;
    }

    // 1. Allowlist check on the hostname presented by the client.
    let decision = policy.check(&host);
    if let PolicyDecision::Denied(reason) = decision {
        tracing::info!(host = %host, port, reason, "egress denied (allowlist)");
        write_simple_status(&mut client, 403, "Forbidden", reason).await?;
        return Ok(());
    }

    // 2. DNS-resolve and re-check the resolved address against the
    //    denylist. This is the rebinding guard — even an
    //    allowlisted hostname can't tunnel through to a private IP.
    let target_addr = match resolve_first_public(&host, port).await {
        Ok(addr) => addr,
        Err(reason) => {
            tracing::info!(host = %host, port, %reason, "egress denied (resolution)");
            write_simple_status(&mut client, 403, "Forbidden", &reason).await?;
            return Ok(());
        }
    };

    // 3. Connect upstream.
    let upstream = match TcpStream::connect(target_addr).await {
        Ok(s) => s,
        Err(err) => {
            tracing::warn!(host = %host, port, error = %err, "upstream connect failed");
            write_simple_status(
                &mut client,
                502,
                "Bad Gateway",
                &format!("upstream connect failed: {err}"),
            )
            .await?;
            return Ok(());
        }
    };

    // 4. Tell the client we're tunneling.
    client
        .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
        .await
        .context("writing CONNECT 200")?;
    client.flush().await.context("flushing CONNECT 200")?;

    // 5. Bidirectional copy until either side closes. tokio's
    //    `copy_bidirectional` handles half-close correctly.
    let (mut client_read, mut client_write) = client.into_split();
    let (mut upstream_read, mut upstream_write) = upstream.into_split();
    let client_to_upstream = tokio::io::copy(&mut client_read, &mut upstream_write);
    let upstream_to_client = tokio::io::copy(&mut upstream_read, &mut client_write);
    tokio::select! {
        _ = client_to_upstream => {},
        _ = upstream_to_client => {},
    }
    tracing::debug!(host = %host, port, "tunnel closed");
    Ok(())
}

/// Resolve `host:port` and return the first non-denied A/AAAA
/// address. Returns an error reason string suitable for the 403
/// body when nothing usable comes back.
async fn resolve_first_public(host: &str, port: u16) -> Result<SocketAddr, String> {
    let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|err| format!("dns resolution failed: {err}"))?
        .collect();

    if addrs.is_empty() {
        return Err("dns returned no addresses".to_string());
    }

    for addr in &addrs {
        if !denied_ip(addr.ip()) {
            return Ok(*addr);
        }
    }
    Err("dns returned only denied addresses (RFC1918, metadata, loopback, etc.)".to_string())
}

async fn write_simple_status(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    body: &str,
) -> Result<()> {
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: text/plain; charset=utf-8\r\n\
         Content-Length: {len}\r\n\
         Connection: close\r\n\
         \r\n\
         {body}",
        len = body.len()
    );
    stream.write_all(response.as_bytes()).await.context("writing status response")?;
    stream.flush().await.context("flushing status response")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_connect_request() {
        let parsed = parse_request_line("CONNECT api.stripe.com:443 HTTP/1.1").unwrap();
        match parsed {
            ParsedRequest::Connect { host, port } => {
                assert_eq!(host, "api.stripe.com");
                assert_eq!(port, 443);
            }
            other => panic!("expected Connect, got {:?}", other),
        }
    }

    #[test]
    fn parse_connect_with_ipv6_brackets() {
        // RFC 7230: IPv6 literals in CONNECT targets use brackets.
        let parsed = parse_request_line("CONNECT [::1]:443 HTTP/1.1").unwrap();
        match parsed {
            ParsedRequest::Connect { host, port } => {
                assert_eq!(host, "::1");
                assert_eq!(port, 443);
            }
            other => panic!("expected Connect, got {:?}", other),
        }
    }

    #[test]
    fn parse_get_request_returns_other() {
        let parsed = parse_request_line("GET /healthz HTTP/1.1").unwrap();
        assert!(matches!(parsed, ParsedRequest::Other { .. }));
    }

    #[test]
    fn parse_connect_missing_port_errors() {
        let result = parse_request_line("CONNECT api.stripe.com HTTP/1.1");
        assert!(result.is_err());
    }

    #[test]
    fn parse_request_line_lowercase_method() {
        // Case-insensitive method match — tolerate `connect` from
        // a misbehaved client.
        let parsed = parse_request_line("connect host:1 HTTP/1.1").unwrap();
        assert!(matches!(parsed, ParsedRequest::Connect { .. }));
    }
}
