//! End-to-end regression: a real `lettre` SMTP client delivers a message
//! through the mock server and we verify it landed in the in-memory
//! mailbox with the expected sender / recipient / subject / body.
//!
//! The pre-existing SMTP tests cover fixture loading, mailbox stats, and
//! spec-registry search at the Rust level, but none of them binds the
//! TCP listener and drives a real SMTP client. A regression in the
//! HELO/EHLO, MAIL FROM, RCPT TO, DATA, or message-capture path would
//! ship silently. This locks in the end-to-end deliver-and-inspect
//! contract.

use lettre::message::{header::ContentType, Mailbox, Message};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use mockforge_smtp::{SmtpConfig, SmtpServer, SmtpSpecRegistry};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

async fn free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

async fn wait_for_port(port: u16, max: Duration) {
    let deadline = tokio::time::Instant::now() + max;
    loop {
        if tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("smtp server never started listening on 127.0.0.1:{port}");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn smtp_real_client_delivers_and_message_is_captured() {
    let port = free_port().await;
    let config = SmtpConfig {
        port,
        host: "127.0.0.1".into(),
        // Disable TLS / fixtures so the test drives only the MAIL/RCPT/DATA
        // path — that's what we're pinning down.
        enable_starttls: false,
        fixtures_dir: None,
        ..SmtpConfig::default()
    };
    let spec_registry = Arc::new(SmtpSpecRegistry::new());
    let server = SmtpServer::new(config, spec_registry.clone()).expect("server builds cleanly");
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });
    wait_for_port(port, Duration::from_secs(5)).await;

    // Use lettre in plaintext mode — the mock doesn't require auth or TLS.
    let transport: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("127.0.0.1")
            .port(port)
            .build();

    let email = Message::builder()
        .from("sender@example.test".parse::<Mailbox>().unwrap())
        .to("receiver@example.test".parse::<Mailbox>().unwrap())
        .subject("MockForge SMTP E2E")
        .header(ContentType::TEXT_PLAIN)
        .body("Hello from the e2e test".to_string())
        .unwrap();

    transport.send(email).await.expect("lettre delivers via mock SMTP");

    // Mailbox stores emails synchronously once DATA completes on the server
    // side, but `send` returns as soon as the 250 arrives back — so give
    // the handler task a brief chance to store the message.
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let captured = loop {
        let emails = spec_registry.get_emails().expect("mailbox read");
        if !emails.is_empty() {
            break emails;
        }
        if std::time::Instant::now() >= deadline {
            panic!("message never made it into the mock mailbox");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };

    assert_eq!(captured.len(), 1, "expected exactly one delivered message");
    let mail = &captured[0];
    assert!(
        mail.from.contains("sender@example.test"),
        "expected sender address preserved, got {:?}",
        mail.from
    );
    assert!(
        mail.to.iter().any(|r| r.contains("receiver@example.test")),
        "expected recipient preserved, got {:?}",
        mail.to
    );
    assert_eq!(mail.subject, "MockForge SMTP E2E");
    assert!(
        mail.body.contains("Hello from the e2e test"),
        "expected body preserved, got {:?}",
        mail.body
    );

    server_handle.abort();
}

/// Multi-recipient delivery: a single SMTP transaction with `MAIL FROM`
/// followed by several `RCPT TO` lines before `DATA` must capture every
/// recipient in `StoredEmail.to`. The existing single-recipient test
/// only exercises one `RCPT TO`, so a regression that drops all-but-one
/// recipient would ship silently.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn smtp_real_client_multi_recipient_delivery_captures_all() {
    let port = free_port().await;
    let config = SmtpConfig {
        port,
        host: "127.0.0.1".into(),
        enable_starttls: false,
        fixtures_dir: None,
        ..SmtpConfig::default()
    };
    let spec_registry = Arc::new(SmtpSpecRegistry::new());
    let server = SmtpServer::new(config, spec_registry.clone()).expect("server builds cleanly");
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });
    wait_for_port(port, Duration::from_secs(5)).await;

    let transport: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("127.0.0.1")
            .port(port)
            .build();

    // lettre expands `.to()` into one RCPT TO per recipient in the same
    // SMTP transaction, which is exactly what the issue asks us to
    // cover.
    let email = Message::builder()
        .from("sender@example.test".parse::<Mailbox>().unwrap())
        .to("one@example.test".parse::<Mailbox>().unwrap())
        .to("two@example.test".parse::<Mailbox>().unwrap())
        .to("three@example.test".parse::<Mailbox>().unwrap())
        .subject("MockForge SMTP multi-RCPT")
        .header(ContentType::TEXT_PLAIN)
        .body("Hello to three addresses".to_string())
        .unwrap();

    transport.send(email).await.expect("lettre delivers via mock SMTP");

    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let captured = loop {
        let emails = spec_registry.get_emails().expect("mailbox read");
        if !emails.is_empty() {
            break emails;
        }
        if std::time::Instant::now() >= deadline {
            panic!("message never made it into the mock mailbox");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };

    assert_eq!(captured.len(), 1, "multi-RCPT must still produce exactly one captured message");
    let mail = &captured[0];
    for expected in ["one@example.test", "two@example.test", "three@example.test"] {
        assert!(
            mail.to.iter().any(|r| r.contains(expected)),
            "recipient `{expected}` missing from captured mail.to = {:?}",
            mail.to
        );
    }

    server_handle.abort();
}

/// Drive the SMTP server directly over a raw `TcpStream` so we can
/// include non-UTF-8 bytes in the body. lettre / `Message` insist on
/// valid UTF-8 internally, so a lettre-based test can't exercise the
/// 8BITMIME byte-preservation path end to end.
async fn read_line_until_code(reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>) -> String {
    let mut buf = String::new();
    reader.read_line(&mut buf).await.expect("read SMTP reply line");
    buf
}

/// 8BITMIME body preservation. The server advertises 8BITMIME in its
/// EHLO response and must therefore preserve non-ASCII bytes in the
/// DATA payload verbatim — specifically, `String::push_str`-based
/// accumulation would drop or corrupt invalid UTF-8. This test drives
/// a raw SMTP session and puts 0xFF/0xFE/0x80 in the body, then
/// inspects `StoredEmail.raw` for a byte-exact match.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn smtp_raw_client_8bitmime_body_preserved_byte_for_byte() {
    let port = free_port().await;
    let config = SmtpConfig {
        port,
        host: "127.0.0.1".into(),
        enable_starttls: false,
        fixtures_dir: None,
        ..SmtpConfig::default()
    };
    let spec_registry = Arc::new(SmtpSpecRegistry::new());
    let server = SmtpServer::new(config, spec_registry.clone()).expect("server builds cleanly");
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });
    wait_for_port(port, Duration::from_secs(5)).await;

    let stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    // Greeting
    let line = read_line_until_code(&mut reader).await;
    assert!(line.starts_with("220"), "unexpected greeting: {line}");

    // EHLO → 250 multiline response; drain until we see a code line
    // without a '-' continuation.
    write_half.write_all(b"EHLO test-client\r\n").await.unwrap();
    loop {
        let line = read_line_until_code(&mut reader).await;
        if line.len() >= 4 && &line[3..4] == " " {
            break;
        }
    }

    // Use BODY=8BITMIME on MAIL FROM to match what a real 8BITMIME
    // client would advertise. The server doesn't need to do anything
    // special with it — it just shouldn't reject the verb form.
    write_half
        .write_all(b"MAIL FROM:<sender@example.test> BODY=8BITMIME\r\n")
        .await
        .unwrap();
    let line = read_line_until_code(&mut reader).await;
    assert!(line.starts_with("250"), "MAIL FROM reply: {line}");

    write_half.write_all(b"RCPT TO:<receiver@example.test>\r\n").await.unwrap();
    let line = read_line_until_code(&mut reader).await;
    assert!(line.starts_with("250"), "RCPT TO reply: {line}");

    write_half.write_all(b"DATA\r\n").await.unwrap();
    let line = read_line_until_code(&mut reader).await;
    assert!(line.starts_with("354"), "DATA reply: {line}");

    // Headers (ASCII) + blank line + body (deliberately non-UTF-8).
    let body_bytes: &[u8] = &[0xff, 0xfe, 0x80, b'h', b'i'];
    let mut payload = Vec::new();
    payload.extend_from_slice(b"Subject: 8BITMIME test\r\n");
    payload.extend_from_slice(b"From: sender@example.test\r\n");
    payload.extend_from_slice(b"To: receiver@example.test\r\n");
    payload.extend_from_slice(b"\r\n");
    payload.extend_from_slice(body_bytes);
    payload.extend_from_slice(b"\r\n.\r\n");
    write_half.write_all(&payload).await.unwrap();

    let line = read_line_until_code(&mut reader).await;
    assert!(line.starts_with("250"), "end-of-DATA reply: {line}");

    write_half.write_all(b"QUIT\r\n").await.unwrap();
    // Drain anything else the server sends.
    let mut tail = Vec::new();
    let _ = reader.read_to_end(&mut tail).await;

    // Wait for the mailbox to register the delivery, then assert.
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let captured = loop {
        let emails = spec_registry.get_emails().expect("mailbox read");
        if !emails.is_empty() {
            break emails;
        }
        if std::time::Instant::now() >= deadline {
            panic!("8BITMIME message never made it into the mock mailbox");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };

    assert_eq!(captured.len(), 1);
    let mail = &captured[0];
    let raw = mail.raw.as_ref().expect("raw bytes must be captured");
    // The body bytes must survive. We don't assert on the exact header
    // formatting — different SMTP servers canonicalize headers — but
    // the non-ASCII body bytes must be present intact.
    assert!(
        raw.windows(body_bytes.len()).any(|w| w == body_bytes),
        "non-ASCII body bytes (0xFF 0xFE 0x80 'h' 'i') were lost from raw: {raw:?}"
    );
    assert_eq!(mail.subject, "8BITMIME test");

    server_handle.abort();
}

/// AUTH PLAIN via lettre's `Credentials` — the common path for apps
/// that use `SmtpTransport::builder_dangerous(...).credentials(...)`.
/// The mock advertises `AUTH PLAIN LOGIN` in EHLO, so lettre picks
/// one and sends the SASL dialog. Accept any credentials, then send
/// a regular message to prove the session is usable after AUTH.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn smtp_real_client_auth_plain_then_deliver() {
    let port = free_port().await;
    let config = SmtpConfig {
        port,
        host: "127.0.0.1".into(),
        enable_starttls: false,
        fixtures_dir: None,
        ..SmtpConfig::default()
    };
    let spec_registry = Arc::new(SmtpSpecRegistry::new());
    let server = SmtpServer::new(config, spec_registry.clone()).expect("server builds cleanly");
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });
    wait_for_port(port, Duration::from_secs(5)).await;

    let creds = Credentials::new("alice".into(), "hunter2".into());
    let transport: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("127.0.0.1")
            .port(port)
            .credentials(creds)
            .build();

    let email = Message::builder()
        .from("alice@example.test".parse::<Mailbox>().unwrap())
        .to("receiver@example.test".parse::<Mailbox>().unwrap())
        .subject("MockForge SMTP AUTH")
        .header(ContentType::TEXT_PLAIN)
        .body("After a successful AUTH the session accepts mail".to_string())
        .unwrap();

    transport.send(email).await.expect("lettre delivers via mock SMTP after AUTH");

    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let captured = loop {
        let emails = spec_registry.get_emails().expect("mailbox read");
        if !emails.is_empty() {
            break emails;
        }
        if std::time::Instant::now() >= deadline {
            panic!("authed message never made it into the mock mailbox");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };

    assert_eq!(captured.len(), 1, "expected exactly one authed message");
    let mail = &captured[0];
    assert!(
        mail.from.contains("alice@example.test"),
        "authed from address should round-trip; got {:?}",
        mail.from
    );
    assert_eq!(mail.subject, "MockForge SMTP AUTH");

    server_handle.abort();
}

/// AUTH LOGIN via a raw TCP session so we can drive the two-step
/// `334` challenges explicitly. lettre always picks AUTH PLAIN when
/// both are advertised, so this covers the LOGIN path directly.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn smtp_raw_client_auth_login_two_step_challenge() {
    use base64::Engine as _;

    let port = free_port().await;
    let config = SmtpConfig {
        port,
        host: "127.0.0.1".into(),
        enable_starttls: false,
        fixtures_dir: None,
        ..SmtpConfig::default()
    };
    let spec_registry = Arc::new(SmtpSpecRegistry::new());
    let server = SmtpServer::new(config, spec_registry.clone()).expect("server builds cleanly");
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });
    wait_for_port(port, Duration::from_secs(5)).await;

    let stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    // 220 greeting
    let line = read_line_until_code(&mut reader).await;
    assert!(line.starts_with("220"), "greeting: {line}");

    // EHLO + drain multi-line response. The AUTH capability line must
    // appear so clients know LOGIN is supported.
    write_half.write_all(b"EHLO test-client\r\n").await.unwrap();
    let mut saw_auth = false;
    loop {
        let line = read_line_until_code(&mut reader).await;
        if line.to_ascii_uppercase().contains("AUTH ") && line.contains("LOGIN") {
            saw_auth = true;
        }
        if line.len() >= 4 && &line[3..4] == " " {
            break;
        }
    }
    assert!(saw_auth, "EHLO must advertise AUTH ... LOGIN");

    // AUTH LOGIN → 334 "Username:" base64
    write_half.write_all(b"AUTH LOGIN\r\n").await.unwrap();
    let line = read_line_until_code(&mut reader).await;
    assert!(
        line.trim_end() == "334 VXNlcm5hbWU6",
        "expected `334 VXNlcm5hbWU6` (base64 Username:), got: {line:?}"
    );

    // Send base64 username → 334 "Password:" base64
    let user_b64 = base64::engine::general_purpose::STANDARD.encode("alice");
    write_half.write_all(format!("{user_b64}\r\n").as_bytes()).await.unwrap();
    let line = read_line_until_code(&mut reader).await;
    assert!(
        line.trim_end() == "334 UGFzc3dvcmQ6",
        "expected `334 UGFzc3dvcmQ6` (base64 Password:), got: {line:?}"
    );

    // Send base64 password → 235 Authentication successful
    let pass_b64 = base64::engine::general_purpose::STANDARD.encode("hunter2");
    write_half.write_all(format!("{pass_b64}\r\n").as_bytes()).await.unwrap();
    let line = read_line_until_code(&mut reader).await;
    assert!(line.starts_with("235 "), "expected `235 ...`, got: {line:?}");

    write_half.write_all(b"QUIT\r\n").await.unwrap();
    // Let the server write its 221, ignore content.
    let mut tail = Vec::new();
    let _ = reader.read_to_end(&mut tail).await;

    server_handle.abort();
}

/// Generate a short-lived self-signed cert + key on disk. Returns
/// (cert_path, key_path) — the caller is responsible for deleting
/// them (we lean on `tempfile::TempDir` for that via the caller's
/// drop).
fn write_self_signed_cert(dir: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    use rcgen::{generate_simple_self_signed, CertifiedKey};
    let CertifiedKey { cert, key_pair } =
        generate_simple_self_signed(vec!["127.0.0.1".into(), "localhost".into()]).expect("rcgen");
    let cert_path = dir.join("cert.pem");
    let key_path = dir.join("key.pem");
    std::fs::write(&cert_path, cert.pem()).expect("write cert");
    std::fs::write(&key_path, key_pair.serialize_pem()).expect("write key");
    (cert_path, key_path)
}

/// STARTTLS: client opens a plaintext connection, issues EHLO,
/// upgrades the socket via STARTTLS, then repeats EHLO over TLS
/// before sending any mail command. Before this fix the broker
/// replied `220 Ready to start TLS` and kept the socket plaintext —
/// which makes any TLS client's handshake fail with "garbage" bytes.
///
/// The test drives a raw TcpStream through the plaintext + TLS halves
/// of the session using `tokio_rustls` as a dangerous-trust client,
/// since lettre's STARTTLS transport would refuse the self-signed
/// cert without extra plumbing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn smtp_raw_client_starttls_actually_upgrades_socket() {
    use tokio_rustls::rustls::{Certificate, ClientConfig, RootCertStore, ServerName};
    use tokio_rustls::TlsConnector;

    let tmp = tempfile::tempdir().expect("tempdir");
    let (cert_path, key_path) = write_self_signed_cert(tmp.path());

    let port = free_port().await;
    let config = SmtpConfig {
        port,
        host: "127.0.0.1".into(),
        enable_starttls: true,
        tls_cert_path: Some(cert_path.clone()),
        tls_key_path: Some(key_path),
        fixtures_dir: None,
        ..SmtpConfig::default()
    };
    let spec_registry = Arc::new(SmtpSpecRegistry::new());
    let server = SmtpServer::new(config, spec_registry.clone()).expect("server builds");
    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });
    wait_for_port(port, Duration::from_secs(5)).await;

    // --- Plaintext phase: greeting → EHLO → STARTTLS → 220 Ready ----
    let plain = TcpStream::connect(("127.0.0.1", port)).await.expect("tcp connect");
    let (pread, mut pwrite) = plain.into_split();
    let mut preader = BufReader::new(pread);

    let line = {
        let mut s = String::new();
        preader.read_line(&mut s).await.unwrap();
        s
    };
    assert!(line.starts_with("220"), "unexpected greeting: {line:?}");

    pwrite.write_all(b"EHLO test-client\r\n").await.unwrap();
    // Drain EHLO multiline response until we see a " " after the code.
    let mut saw_starttls = false;
    loop {
        let mut s = String::new();
        preader.read_line(&mut s).await.unwrap();
        if s.to_ascii_uppercase().contains("STARTTLS") {
            saw_starttls = true;
        }
        if s.len() >= 4 && &s[3..4] == " " {
            break;
        }
    }
    assert!(saw_starttls, "server must advertise STARTTLS when enable_starttls=true");

    pwrite.write_all(b"STARTTLS\r\n").await.unwrap();
    let mut tls_ready = String::new();
    preader.read_line(&mut tls_ready).await.unwrap();
    assert!(
        tls_ready.starts_with("220"),
        "STARTTLS must be accepted with 220 Ready, got: {tls_ready:?}"
    );

    // --- Reassemble and negotiate TLS -------------------------------
    let tcp = preader.into_inner().reunite(pwrite).expect("reunite");

    // Trust the server's self-signed cert for this test only. In
    // production lettre/others would verify against the system root
    // store; here we add the test cert directly so the handshake
    // succeeds. rustls 0.21 API (via tokio-rustls 0.24) takes raw
    // `Certificate(Vec<u8>)` rather than the newer `CertificateDer`.
    let cert_bytes = std::fs::read(&cert_path).unwrap();
    let der_certs = rustls_pemfile::certs(&mut cert_bytes.as_slice()).expect("parse PEM");
    let mut root_store = RootCertStore::empty();
    for der in der_certs {
        root_store.add(&Certificate(der)).expect("add cert");
    }
    let client_cfg = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(client_cfg));
    let server_name = ServerName::try_from("localhost").unwrap();
    let mut tls = tokio::time::timeout(Duration::from_secs(5), connector.connect(server_name, tcp))
        .await
        .expect("TLS handshake completes within 5s")
        .expect("TLS handshake succeeds");

    // --- Over TLS: full mail transaction ---------------------------
    // Per RFC 3207 the client MUST re-EHLO after a successful upgrade.
    tls.write_all(b"EHLO test-client\r\n").await.unwrap();
    // Drain EHLO again.
    let mut buf = [0u8; 1024];
    loop {
        let n = tls.read(&mut buf).await.unwrap();
        let chunk = std::str::from_utf8(&buf[..n]).unwrap_or("");
        if chunk.contains("250 ") {
            break;
        }
    }

    tls.write_all(b"MAIL FROM:<sender@example.test>\r\n").await.unwrap();
    let mut r = [0u8; 64];
    let n = tls.read(&mut r).await.unwrap();
    assert!(std::str::from_utf8(&r[..n]).unwrap().starts_with("250"));

    tls.write_all(b"RCPT TO:<receiver@example.test>\r\n").await.unwrap();
    let n = tls.read(&mut r).await.unwrap();
    assert!(std::str::from_utf8(&r[..n]).unwrap().starts_with("250"));

    tls.write_all(b"DATA\r\n").await.unwrap();
    let n = tls.read(&mut r).await.unwrap();
    assert!(std::str::from_utf8(&r[..n]).unwrap().starts_with("354"));

    tls.write_all(b"Subject: TLS only\r\n\r\nTLS body\r\n.\r\n").await.unwrap();
    let n = tls.read(&mut r).await.unwrap();
    assert!(std::str::from_utf8(&r[..n]).unwrap().starts_with("250"));

    tls.write_all(b"QUIT\r\n").await.unwrap();

    // Mailbox check — message must have been captured over the TLS
    // leg, proving the upgrade was real.
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let captured = loop {
        let emails = spec_registry.get_emails().expect("mailbox read");
        if !emails.is_empty() {
            break emails;
        }
        if std::time::Instant::now() >= deadline {
            panic!("STARTTLS-delivered message never made it into the mailbox");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].subject, "TLS only");
    assert!(captured[0].body.contains("TLS body"));

    server_handle.abort();
}
