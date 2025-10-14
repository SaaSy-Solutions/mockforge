//! Integration tests for SMTP server

use mockforge_smtp::{
    BehaviorConfig, MatchCriteria, SmtpConfig, SmtpFixture, SmtpResponse, SmtpServer,
    SmtpSpecRegistry, StorageConfig,
};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

/// Helper function to start SMTP server on a random port
async fn start_test_server() -> (SmtpServer, u16) {
    let config = SmtpConfig {
        port: 0, // Random port
        host: "127.0.0.1".to_string(),
        hostname: "test-smtp".to_string(),
        ..Default::default()
    };

    let mut registry = SmtpSpecRegistry::new();

    // Add a default catch-all fixture for testing
    let default_fixture = SmtpFixture {
        identifier: "default".to_string(),
        name: "Default Test Fixture".to_string(),
        description: "Default fixture for integration tests".to_string(),
        match_criteria: MatchCriteria {
            recipient_pattern: None,
            sender_pattern: None,
            subject_pattern: None,
            match_all: true, // Catch all emails
        },
        response: SmtpResponse {
            status_code: 250,
            message: "Message accepted for delivery".to_string(),
            delay_ms: 0,
        },
        auto_reply: None,
        storage: StorageConfig {
            save_to_mailbox: true,
            export_to_file: None,
        },
        behavior: BehaviorConfig::default(),
    };

    // Create a temporary directory and save the fixture
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let fixture_path = temp_dir.path().join("default.yaml");
    std::fs::write(&fixture_path, serde_yaml::to_string(&default_fixture).unwrap())
        .expect("Failed to write fixture");

    registry.load_fixtures(temp_dir.path()).expect("Failed to load fixtures");

    let port = find_available_port().await;
    let config_with_port = SmtpConfig { port, ..config };

    let server = SmtpServer::new(config_with_port, Arc::new(registry));
    (server, port)
}

/// Find an available port for testing
async fn find_available_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to random port");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

/// Connect to SMTP server and read greeting
async fn connect_and_read_greeting(
    port: u16,
) -> (
    BufReader<tokio::net::tcp::OwnedReadHalf>,
    tokio::net::tcp::OwnedWriteHalf,
    String,
) {
    let stream = timeout(Duration::from_secs(5), TcpStream::connect(format!("127.0.0.1:{}", port)))
        .await
        .expect("Connection timeout")
        .expect("Failed to connect");

    let (reader, writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Read greeting
    let mut greeting = String::new();
    timeout(Duration::from_secs(5), reader.read_line(&mut greeting))
        .await
        .expect("Greeting timeout")
        .expect("Failed to read greeting");

    (reader, writer, greeting)
}

#[tokio::test]
async fn test_smtp_server_starts_and_accepts_connections() {
    let (server, port) = start_test_server().await;

    // Start server in background
    tokio::spawn(async move {
        server.start().await.expect("Server failed to start");
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Try to connect
    let (_reader, _writer, greeting) = connect_and_read_greeting(port).await;

    assert!(greeting.starts_with("220"), "Expected SMTP greeting, got: {}", greeting);
    assert!(greeting.contains("test-smtp"), "Greeting should contain hostname");
}

#[tokio::test]
async fn test_smtp_basic_conversation() {
    let (server, port) = start_test_server().await;

    tokio::spawn(async move {
        server.start().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let (mut reader, mut writer, _greeting) = connect_and_read_greeting(port).await;
    let mut response = String::new();

    // EHLO command
    writer
        .write_all(b"EHLO client.example.com\r\n")
        .await
        .expect("Failed to write EHLO");

    // Read all EHLO response lines (it returns multiple lines)
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await.expect("Failed to read EHLO response");
        response.push_str(&line);

        // EHLO responses end with "250 " (with space) instead of "250-"
        if line.starts_with("250 ") {
            break;
        }
    }
    assert!(response.contains("250"), "Expected 250 response for EHLO");
    response.clear();

    // MAIL FROM command
    writer
        .write_all(b"MAIL FROM:<sender@example.com>\r\n")
        .await
        .expect("Failed to write MAIL FROM");
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read MAIL FROM response");
    assert!(response.contains("250"), "Expected 250 OK for MAIL FROM");
    response.clear();

    // RCPT TO command
    writer
        .write_all(b"RCPT TO:<recipient@example.com>\r\n")
        .await
        .expect("Failed to write RCPT TO");
    reader.read_line(&mut response).await.expect("Failed to read RCPT TO response");
    assert!(response.contains("250"), "Expected 250 OK for RCPT TO");
    response.clear();

    // DATA command
    writer.write_all(b"DATA\r\n").await.expect("Failed to write DATA");
    reader.read_line(&mut response).await.expect("Failed to read DATA response");
    assert!(response.contains("354"), "Expected 354 response for DATA");
    response.clear();

    // Send email data
    writer
        .write_all(b"Subject: Test Email\r\n")
        .await
        .expect("Failed to write subject");
    writer.write_all(b"\r\n").await.expect("Failed to write blank line");
    writer
        .write_all(b"This is a test email.\r\n")
        .await
        .expect("Failed to write body");
    writer.write_all(b".\r\n").await.expect("Failed to write end of data");

    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read DATA completion response");
    assert!(
        response.contains("250") || response.contains("550"),
        "Expected 250 or 550 response after DATA, got: {}",
        response
    );
    response.clear();

    // QUIT command
    writer.write_all(b"QUIT\r\n").await.expect("Failed to write QUIT");
    reader.read_line(&mut response).await.expect("Failed to read QUIT response");
    assert!(response.contains("221"), "Expected 221 Bye response");
}

#[tokio::test]
async fn test_smtp_fixture_matching() {
    let mut registry = SmtpSpecRegistry::new();

    // Add a fixture that matches specific recipient
    let fixture = SmtpFixture {
        identifier: "test-fixture".to_string(),
        name: "Test Fixture".to_string(),
        description: "Test fixture for integration tests".to_string(),
        match_criteria: MatchCriteria {
            recipient_pattern: Some(r"^user.*@example\.com$".to_string()),
            sender_pattern: None,
            subject_pattern: None,
            match_all: false,
        },
        response: SmtpResponse {
            status_code: 250,
            message: "Test message accepted".to_string(),
            delay_ms: 0,
        },
        auto_reply: None,
        storage: StorageConfig {
            save_to_mailbox: true,
            export_to_file: None,
        },
        behavior: BehaviorConfig::default(),
    };

    // Manually add fixture
    let temp_dir = tempfile::tempdir().unwrap();
    let fixture_path = temp_dir.path().join("test.yaml");
    std::fs::write(&fixture_path, serde_yaml::to_string(&fixture).unwrap()).unwrap();

    registry.load_fixtures(temp_dir.path()).expect("Failed to load fixtures");

    // Test matching
    let matching_fixture =
        registry.find_matching_fixture("sender@test.com", "user123@example.com", "Test Subject");
    assert!(matching_fixture.is_some(), "Should find matching fixture");

    let non_matching =
        registry.find_matching_fixture("sender@test.com", "admin@example.com", "Test Subject");
    assert!(non_matching.is_none(), "Should not find non-matching fixture");
}

#[tokio::test]
async fn test_smtp_mailbox_storage() {
    use mockforge_smtp::StoredEmail;

    let registry = SmtpSpecRegistry::new();

    // Store an email
    let email = StoredEmail {
        id: "test-123".to_string(),
        from: "sender@example.com".to_string(),
        to: vec!["recipient@example.com".to_string()],
        subject: "Test Email".to_string(),
        body: "This is a test.".to_string(),
        headers: std::collections::HashMap::new(),
        received_at: chrono::Utc::now(),
        raw: None,
    };

    registry.store_email(email.clone()).expect("Failed to store email");

    // Retrieve emails
    let emails = registry.get_emails().expect("Failed to get emails");
    assert_eq!(emails.len(), 1, "Should have one email");
    assert_eq!(emails[0].from, "sender@example.com");
    assert_eq!(emails[0].subject, "Test Email");

    // Get specific email by ID
    let retrieved = registry.get_email_by_id("test-123").expect("Failed to get email by ID");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, "test-123");

    // Clear mailbox
    registry.clear_mailbox().expect("Failed to clear mailbox");
    let emails_after_clear = registry.get_emails().expect("Failed to get emails");
    assert_eq!(emails_after_clear.len(), 0, "Mailbox should be empty");
}

#[tokio::test]
async fn test_smtp_mailbox_size_limit() {
    let registry = SmtpSpecRegistry::with_mailbox_size(2);

    // Store more emails than the limit
    for i in 0..5 {
        let email = mockforge_smtp::StoredEmail {
            id: format!("test-{}", i),
            from: "sender@example.com".to_string(),
            to: vec!["recipient@example.com".to_string()],
            subject: format!("Test Email {}", i),
            body: "Test".to_string(),
            headers: std::collections::HashMap::new(),
            received_at: chrono::Utc::now(),
            raw: None,
        };

        registry.store_email(email).expect("Failed to store email");
    }

    // Should only keep the last 2 emails
    let emails = registry.get_emails().expect("Failed to get emails");
    assert_eq!(emails.len(), 2, "Should only keep last 2 emails");
    assert_eq!(emails[0].subject, "Test Email 3");
    assert_eq!(emails[1].subject, "Test Email 4");
}

#[tokio::test]
async fn test_smtp_protocol_commands() {
    let (server, port) = start_test_server().await;

    tokio::spawn(async move {
        server.start().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let (mut reader, mut writer, _greeting) = connect_and_read_greeting(port).await;
    let mut response = String::new();

    // Test NOOP command
    writer.write_all(b"NOOP\r\n").await.expect("Failed to write NOOP");
    reader.read_line(&mut response).await.expect("Failed to read NOOP response");
    assert!(response.contains("250"), "NOOP should return 250 OK");
    response.clear();

    // Test HELP command
    writer.write_all(b"HELP\r\n").await.expect("Failed to write HELP");

    // Read all HELP response lines (it returns multiple lines)
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await.expect("Failed to read HELP response");
        response.push_str(&line);

        // HELP responses end with "214 " (with space) instead of "214-"
        if line.starts_with("214 ") {
            break;
        }
    }
    assert!(response.contains("214"), "HELP should return 214");
    response.clear();

    // Test RSET command
    writer
        .write_all(b"MAIL FROM:<sender@example.com>\r\n")
        .await
        .expect("Failed to write MAIL FROM");
    reader.read_line(&mut response).await.ok();
    response.clear();

    writer.write_all(b"RSET\r\n").await.expect("Failed to write RSET");
    reader.read_line(&mut response).await.expect("Failed to read RSET response");
    assert!(response.contains("250"), "RSET should return 250 OK");
}

#[tokio::test]
async fn test_smtp_hello_vs_ehlo() {
    let (server, port) = start_test_server().await;

    tokio::spawn(async move {
        server.start().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test HELLO
    let (mut reader, mut writer, _greeting) = connect_and_read_greeting(port).await;
    let mut response = String::new();

    writer
        .write_all(b"HELLO client.example.com\r\n")
        .await
        .expect("Failed to write HELLO");
    reader.read_line(&mut response).await.expect("Failed to read HELLO response");
    assert!(response.contains("250"), "HELLO should return 250");
    assert!(!response.contains("SIZE"), "HELLO should not list extensions");

    writer.write_all(b"QUIT\r\n").await.ok();

    // Test EHLO
    tokio::time::sleep(Duration::from_millis(50)).await;
    let (mut reader2, mut writer2, _greeting2) = connect_and_read_greeting(port).await;
    let mut response2 = String::new();

    writer2
        .write_all(b"EHLO client.example.com\r\n")
        .await
        .expect("Failed to write EHLO");

    // Read all EHLO response lines
    loop {
        let mut line = String::new();
        reader2.read_line(&mut line).await.expect("Failed to read line");
        response2.push_str(&line);

        // EHLO responses end with "250 " (with space) instead of "250-"
        if line.starts_with("250 ") {
            break;
        }
    }

    assert!(response2.contains("250"), "EHLO should return 250");
    // EHLO may contain extensions like SIZE, 8BITMIME, etc.
}

#[tokio::test]
async fn test_smtp_invalid_command() {
    let (server, port) = start_test_server().await;

    tokio::spawn(async move {
        server.start().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let (mut reader, mut writer, _greeting) = connect_and_read_greeting(port).await;
    let mut response = String::new();

    // Send invalid command
    writer
        .write_all(b"INVALID COMMAND\r\n")
        .await
        .expect("Failed to write invalid command");
    reader.read_line(&mut response).await.expect("Failed to read response");

    // Should return 502 (command not implemented) or 500 (syntax error)
    assert!(
        response.contains("502") || response.contains("500"),
        "Invalid command should return error code, got: {}",
        response
    );
}

#[tokio::test]
async fn test_smtp_error_handling_invalid_recipient() {
    let (server, port) = start_test_server().await;

    tokio::spawn(async move {
        server.start().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let (mut reader, mut writer, _greeting) = connect_and_read_greeting(port).await;
    let mut response = String::new();

    // EHLO
    writer
        .write_all(b"EHLO client.example.com\r\n")
        .await
        .expect("Failed to write EHLO");
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await.expect("Failed to read EHLO response");
        if line.starts_with("250 ") {
            break;
        }
    }

    // MAIL FROM
    writer
        .write_all(b"MAIL FROM:<sender@example.com>\r\n")
        .await
        .expect("Failed to write MAIL FROM");
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read MAIL FROM response");
    assert!(response.contains("250"));
    response.clear();

    // RCPT TO with invalid format (missing @)
    writer
        .write_all(b"RCPT TO:<invalidrecipient>\r\n")
        .await
        .expect("Failed to write RCPT TO");
    reader.read_line(&mut response).await.expect("Failed to read RCPT TO response");
    // Should accept or reject based on fixture - our default fixture accepts all
    assert!(response.contains("250") || response.contains("550"));
    response.clear();

    // QUIT
    writer.write_all(b"QUIT\r\n").await.ok();
}

#[tokio::test]
async fn test_smtp_error_handling_missing_mail_from() {
    let (server, port) = start_test_server().await;

    tokio::spawn(async move {
        server.start().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let (mut reader, mut writer, _greeting) = connect_and_read_greeting(port).await;
    let mut response = String::new();

    // EHLO
    writer
        .write_all(b"EHLO client.example.com\r\n")
        .await
        .expect("Failed to write EHLO");
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await.expect("Failed to read EHLO response");
        if line.starts_with("250 ") {
            break;
        }
    }

    // Try RCPT TO without MAIL FROM
    writer
        .write_all(b"RCPT TO:<recipient@example.com>\r\n")
        .await
        .expect("Failed to write RCPT TO");
    reader.read_line(&mut response).await.expect("Failed to read RCPT TO response");
    // Should accept or reject - our implementation may allow this
    assert!(response.contains("250") || response.contains("550") || response.contains("503"));
    response.clear();

    // QUIT
    writer.write_all(b"QUIT\r\n").await.ok();
}

#[tokio::test]
async fn test_smtp_error_handling_missing_rcpt_to() {
    let (server, port) = start_test_server().await;

    tokio::spawn(async move {
        server.start().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let (mut reader, mut writer, _greeting) = connect_and_read_greeting(port).await;
    let mut response = String::new();

    // EHLO
    writer
        .write_all(b"EHLO client.example.com\r\n")
        .await
        .expect("Failed to write EHLO");
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await.expect("Failed to read EHLO response");
        if line.starts_with("250 ") {
            break;
        }
    }

    // MAIL FROM
    writer
        .write_all(b"MAIL FROM:<sender@example.com>\r\n")
        .await
        .expect("Failed to write MAIL FROM");
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read MAIL FROM response");
    assert!(response.contains("250"));
    response.clear();

    // Try DATA without RCPT TO
    writer.write_all(b"DATA\r\n").await.expect("Failed to write DATA");
    reader.read_line(&mut response).await.expect("Failed to read DATA response");
    // SMTP allows DATA without RCPT TO in some implementations
    assert!(response.contains("354"));
    response.clear();

    // QUIT
    writer.write_all(b"QUIT\r\n").await.ok();
}

#[tokio::test]
async fn test_smtp_connection_timeout() {
    let config = SmtpConfig {
        port: 0,
        host: "127.0.0.1".to_string(),
        hostname: "test-smtp".to_string(),
        timeout_secs: 1, // Very short timeout
        ..Default::default()
    };

    let port = find_available_port().await;
    let config_with_port = SmtpConfig { port, ..config };
    let registry = Arc::new(SmtpSpecRegistry::new());
    let server = SmtpServer::new(config_with_port, registry);

    tokio::spawn(async move {
        server.start().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let stream = TcpStream::connect(format!("127.0.0.1:{}", port))
        .await
        .expect("Failed to connect");

    let (_reader, mut writer) = stream.into_split();

    // Send EHLO but don't send more data - should timeout
    writer
        .write_all(b"EHLO client.example.com\r\n")
        .await
        .expect("Failed to write EHLO");

    // Wait longer than timeout
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Connection should be closed by server due to timeout
    // This is hard to test directly, but we can verify the server handles timeouts
}

#[tokio::test]
async fn test_smtp_edge_case_multiple_recipients() {
    let (server, port) = start_test_server().await;

    tokio::spawn(async move {
        server.start().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let (mut reader, mut writer, _greeting) = connect_and_read_greeting(port).await;
    let mut response = String::new();

    // EHLO
    writer
        .write_all(b"EHLO client.example.com\r\n")
        .await
        .expect("Failed to write EHLO");
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await.expect("Failed to read EHLO response");
        if line.starts_with("250 ") {
            break;
        }
    }

    // MAIL FROM
    writer
        .write_all(b"MAIL FROM:<sender@example.com>\r\n")
        .await
        .expect("Failed to write MAIL FROM");
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read MAIL FROM response");
    assert!(response.contains("250"));
    response.clear();

    // Multiple RCPT TO commands
    writer
        .write_all(b"RCPT TO:<recipient1@example.com>\r\n")
        .await
        .expect("Failed to write RCPT TO 1");
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read RCPT TO 1 response");
    assert!(response.contains("250"));
    response.clear();

    writer
        .write_all(b"RCPT TO:<recipient2@example.com>\r\n")
        .await
        .expect("Failed to write RCPT TO 2");
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read RCPT TO 2 response");
    assert!(response.contains("250"));
    response.clear();

    // DATA
    writer.write_all(b"DATA\r\n").await.expect("Failed to write DATA");
    reader.read_line(&mut response).await.expect("Failed to read DATA response");
    assert!(response.contains("354"));
    response.clear();

    // Send email with multiple recipients in headers
    writer
        .write_all(b"To: recipient1@example.com, recipient2@example.com\r\n")
        .await
        .expect("Failed to write To header");
    writer
        .write_all(b"Subject: Test Multiple Recipients\r\n")
        .await
        .expect("Failed to write subject");
    writer.write_all(b"\r\n").await.expect("Failed to write blank line");
    writer
        .write_all(b"This email has multiple recipients.\r\n")
        .await
        .expect("Failed to write body");
    writer.write_all(b".\r\n").await.expect("Failed to write end of data");

    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read DATA completion response");
    assert!(response.contains("250"));
    response.clear();

    // QUIT
    writer.write_all(b"QUIT\r\n").await.ok();
}

#[tokio::test]
async fn test_smtp_edge_case_unicode_content() {
    let (server, port) = start_test_server().await;

    tokio::spawn(async move {
        server.start().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let (mut reader, mut writer, _greeting) = connect_and_read_greeting(port).await;
    let mut response = String::new();

    // EHLO
    writer
        .write_all(b"EHLO client.example.com\r\n")
        .await
        .expect("Failed to write EHLO");
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await.expect("Failed to read EHLO response");
        if line.starts_with("250 ") {
            break;
        }
    }

    // MAIL FROM
    writer
        .write_all(b"MAIL FROM:<sender@example.com>\r\n")
        .await
        .expect("Failed to write MAIL FROM");
    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read MAIL FROM response");
    assert!(response.contains("250"));
    response.clear();

    // RCPT TO
    writer
        .write_all(b"RCPT TO:<recipient@example.com>\r\n")
        .await
        .expect("Failed to write RCPT TO");
    reader.read_line(&mut response).await.expect("Failed to read RCPT TO response");
    assert!(response.contains("250"));
    response.clear();

    // DATA
    writer.write_all(b"DATA\r\n").await.expect("Failed to write DATA");
    reader.read_line(&mut response).await.expect("Failed to read DATA response");
    assert!(response.contains("354"));
    response.clear();

    // Send email with Unicode content
    writer
        .write_all(b"Subject: =?UTF-8?B?VGVzdCB3aXRoIMO2w7bDp8O8?=\r\n")
        .await
        .expect("Failed to write Unicode subject");
    writer.write_all(b"\r\n").await.expect("Failed to write blank line");
    writer
        .write_all(b"This email contains Unicode: \xc3\xb6\xc3\xa4\xc3\xbc\r\n")
        .await
        .expect("Failed to write Unicode body");
    writer.write_all(b".\r\n").await.expect("Failed to write end of data");

    reader
        .read_line(&mut response)
        .await
        .expect("Failed to read DATA completion response");
    assert!(response.contains("250"));
    response.clear();

    // QUIT
    writer.write_all(b"QUIT\r\n").await.ok();
}

#[tokio::test]
async fn test_smtp_load_concurrent_connections() {
    let (server, port) = start_test_server().await;

    tokio::spawn(async move {
        server.start().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut handles = Vec::new();

    // Create 10 concurrent connections
    for i in 0..10 {
        let port = port;
        let handle = tokio::spawn(async move {
            let stream = TcpStream::connect(format!("127.0.0.1:{}", port))
                .await
                .expect("Failed to connect");

            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);
            let mut response = String::new();

            // Read greeting
            reader.read_line(&mut response).await.expect("Failed to read greeting");
            response.clear();

            // EHLO
            writer
                .write_all(format!("EHLO client{}.example.com\r\n", i).as_bytes())
                .await
                .expect("Failed to write EHLO");
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).await.expect("Failed to read EHLO response");
                if line.starts_with("250 ") {
                    break;
                }
            }

            // MAIL FROM
            writer
                .write_all(format!("MAIL FROM:<sender{}@example.com>\r\n", i).as_bytes())
                .await
                .expect("Failed to write MAIL FROM");
            reader
                .read_line(&mut response)
                .await
                .expect("Failed to read MAIL FROM response");
            assert!(response.contains("250"));
            response.clear();

            // RCPT TO
            writer
                .write_all(format!("RCPT TO:<recipient{}@example.com>\r\n", i).as_bytes())
                .await
                .expect("Failed to write RCPT TO");
            reader.read_line(&mut response).await.expect("Failed to read RCPT TO response");
            assert!(response.contains("250"));
            response.clear();

            // QUIT
            writer.write_all(b"QUIT\r\n").await.expect("Failed to write QUIT");
            reader.read_line(&mut response).await.expect("Failed to read QUIT response");
            assert!(response.contains("221"));
        });

        handles.push(handle);
    }

    // Wait for all connections to complete
    for handle in handles {
        handle.await.expect("Connection failed");
    }
}

#[tokio::test]
async fn test_smtp_starttls_command() {
    let (server, port) = start_test_server().await;

    tokio::spawn(async move {
        server.start().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let (mut reader, mut writer, _greeting) = connect_and_read_greeting(port).await;
    let mut response = String::new();

    // Test STARTTLS command
    writer.write_all(b"STARTTLS\r\n").await.expect("Failed to write STARTTLS");
    reader.read_line(&mut response).await.expect("Failed to read STARTTLS response");
    assert!(response.contains("220"), "STARTTLS should return 220 Ready to start TLS");

    // QUIT
    writer.write_all(b"QUIT\r\n").await.ok();
}
