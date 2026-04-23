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
