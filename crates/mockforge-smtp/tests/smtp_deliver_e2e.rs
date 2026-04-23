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
