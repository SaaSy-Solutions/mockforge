//! Performance benchmarks for SMTP server

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mockforge_smtp::{SmtpConfig, SmtpServer, SmtpSpecRegistry};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;

/// Helper to start SMTP server on a random port
async fn start_test_server() -> (SmtpServer, u16) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("Failed to bind");
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let config = SmtpConfig {
        port,
        host: "127.0.0.1".to_string(),
        hostname: "bench-smtp".to_string(),
        ..Default::default()
    };

    let registry = Arc::new(SmtpSpecRegistry::new());
    let server = SmtpServer::new(config, registry).expect("Failed to create SMTP server");
    (server, port)
}

/// Benchmark SMTP server startup
fn bench_server_startup(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("smtp_server_startup", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (server, _port) = start_test_server().await.unwrap();
                black_box(server);
            })
        });
    });
}

/// Benchmark single SMTP connection and greeting
fn bench_connection_greeting(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (server, port) = rt.block_on(start_test_server());

    // Start server in background
    rt.spawn(async move {
        let _ = server.start().await;
    });

    // Wait for server to start
    std::thread::sleep(std::time::Duration::from_millis(100));

    c.bench_function("smtp_connection_greeting", |b| {
        b.iter(|| {
            rt.block_on(async {
                let stream = TcpStream::connect(format!("127.0.0.1:{}", port))
                    .await
                    .expect("Failed to connect");

                let (reader, _writer) = stream.into_split();
                let mut reader = BufReader::new(reader);
                let mut greeting = String::new();
                reader.read_line(&mut greeting).await.expect("Failed to read greeting");

                black_box(greeting);
            })
        });
    });
}

/// Benchmark full SMTP conversation (EHLO → MAIL → RCPT → DATA → QUIT)
fn bench_full_conversation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (server, port) = rt.block_on(start_test_server());

    rt.spawn(async move {
        let _ = server.start().await;
    });

    std::thread::sleep(std::time::Duration::from_millis(100));

    c.bench_function("smtp_full_conversation", |b| {
        b.iter(|| {
            rt.block_on(async {
                let stream = TcpStream::connect(format!("127.0.0.1:{}", port))
                    .await
                    .expect("Failed to connect");

                let (reader, mut writer) = stream.into_split();
                let mut reader = BufReader::new(reader);
                let mut response = String::new();

                // Read greeting
                reader.read_line(&mut response).await.unwrap();
                response.clear();

                // EHLO
                writer.write_all(b"EHLO client.example.com\r\n").await.unwrap();
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).await.unwrap();
                    if line.starts_with("250 ") {
                        break;
                    }
                }

                // MAIL FROM
                writer.write_all(b"MAIL FROM:<sender@example.com>\r\n").await.unwrap();
                response.clear();
                reader.read_line(&mut response).await.unwrap();

                // RCPT TO
                writer.write_all(b"RCPT TO:<recipient@example.com>\r\n").await.unwrap();
                response.clear();
                reader.read_line(&mut response).await.unwrap();

                // DATA
                writer.write_all(b"DATA\r\n").await.unwrap();
                response.clear();
                reader.read_line(&mut response).await.unwrap();

                // Send email
                writer
                    .write_all(b"Subject: Benchmark\r\n\r\nBenchmark email\r\n.\r\n")
                    .await
                    .unwrap();
                response.clear();
                reader.read_line(&mut response).await.unwrap();

                // QUIT
                writer.write_all(b"QUIT\r\n").await.unwrap();
                response.clear();
                reader.read_line(&mut response).await.unwrap();

                black_box(response);
            })
        });
    });
}

/// Benchmark SMTP command processing
fn bench_command_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (server, port) = rt.block_on(start_test_server());

    rt.spawn(async move {
        let _ = server.start().await;
    });

    std::thread::sleep(std::time::Duration::from_millis(100));

    let mut group = c.benchmark_group("smtp_commands");

    for command in &["NOOP", "HELP", "RSET"] {
        group.bench_with_input(BenchmarkId::from_parameter(command), command, |b, &cmd| {
            b.iter(|| {
                rt.block_on(async {
                    let stream = TcpStream::connect(format!("127.0.0.1:{}", port))
                        .await
                        .expect("Failed to connect");

                    let (reader, mut writer) = stream.into_split();
                    let mut reader = BufReader::new(reader);
                    let mut response = String::new();

                    // Read greeting
                    reader.read_line(&mut response).await.unwrap();
                    response.clear();

                    // Send command
                    writer.write_all(format!("{}\r\n", cmd).as_bytes()).await.unwrap();
                    reader.read_line(&mut response).await.unwrap();

                    black_box(response);
                })
            });
        });
    }

    group.finish();
}

/// Benchmark fixture matching performance
fn bench_fixture_matching(c: &mut Criterion) {
    use mockforge_smtp::{BehaviorConfig, MatchCriteria, SmtpFixture, SmtpResponse, StorageConfig};

    let _rt = Runtime::new().unwrap();

    // Create registry with multiple fixtures
    let mut registry = SmtpSpecRegistry::new();

    // Create temp directory and fixtures
    let temp_dir = tempfile::tempdir().unwrap();

    for i in 0..100 {
        let fixture = SmtpFixture {
            identifier: format!("fixture-{}", i),
            name: format!("Fixture {}", i),
            description: "Test fixture".to_string(),
            match_criteria: MatchCriteria {
                recipient_pattern: Some(format!(r"^user{}@example\.com$", i)),
                sender_pattern: None,
                subject_pattern: None,
                match_all: false,
            },
            response: SmtpResponse {
                status_code: 250,
                message: "OK".to_string(),
                delay_ms: 0,
            },
            auto_reply: None,
            storage: StorageConfig {
                save_to_mailbox: true,
                export_to_file: None,
            },
            behavior: BehaviorConfig::default(),
        };

        let fixture_path = temp_dir.path().join(format!("fixture-{}.yaml", i));
        std::fs::write(&fixture_path, serde_yaml::to_string(&fixture).unwrap()).unwrap();
    }

    registry.load_fixtures(temp_dir.path()).expect("Failed to load fixtures");

    let mut group = c.benchmark_group("fixture_matching");
    group.throughput(Throughput::Elements(1));

    group.bench_function("match_found", |b| {
        b.iter(|| {
            let result = registry.find_matching_fixture(
                "sender@test.com",
                "user50@example.com",
                "Test Subject",
            );
            black_box(result);
        });
    });

    group.bench_function("match_not_found", |b| {
        b.iter(|| {
            let result = registry.find_matching_fixture(
                "sender@test.com",
                "nobody@example.com",
                "Test Subject",
            );
            black_box(result);
        });
    });

    group.finish();
}

/// Benchmark mailbox operations
fn bench_mailbox_operations(c: &mut Criterion) {
    use mockforge_smtp::StoredEmail;

    let registry = SmtpSpecRegistry::with_mailbox_size(10000);

    let mut group = c.benchmark_group("mailbox_operations");
    group.throughput(Throughput::Elements(1));

    group.bench_function("store_email", |b| {
        b.iter(|| {
            let email = StoredEmail {
                id: uuid::Uuid::new_v4().to_string(),
                from: "sender@example.com".to_string(),
                to: vec!["recipient@example.com".to_string()],
                subject: "Test Email".to_string(),
                body: "This is a test.".to_string(),
                headers: std::collections::HashMap::new(),
                received_at: chrono::Utc::now(),
                raw: None,
            };

            registry.store_email(email).expect("Failed to store");
        });
    });

    // Store some emails first
    for i in 0..100 {
        let email = StoredEmail {
            id: format!("test-{}", i),
            from: "sender@example.com".to_string(),
            to: vec!["recipient@example.com".to_string()],
            subject: format!("Test Email {}", i),
            body: "Test".to_string(),
            headers: std::collections::HashMap::new(),
            received_at: chrono::Utc::now(),
            raw: None,
        };
        registry.store_email(email).unwrap();
    }

    group.bench_function("get_all_emails", |b| {
        b.iter(|| {
            let emails = registry.get_emails().expect("Failed to get emails");
            black_box(emails);
        });
    });

    group.bench_function("get_email_by_id", |b| {
        b.iter(|| {
            let email = registry.get_email_by_id("test-50").expect("Failed to get email");
            black_box(email);
        });
    });

    group.finish();
}

/// Benchmark concurrent connections
fn bench_concurrent_connections(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (server, port) = rt.block_on(start_test_server());

    rt.spawn(async move {
        let _ = server.start().await;
    });

    std::thread::sleep(std::time::Duration::from_millis(100));

    let mut group = c.benchmark_group("concurrent_connections");

    for num_connections in [1, 10, 50].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_connections),
            num_connections,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut handles = Vec::new();

                        for _ in 0..count {
                            let handle = tokio::spawn(async move {
                                let stream = TcpStream::connect(format!("127.0.0.1:{}", port))
                                    .await
                                    .expect("Failed to connect");

                                let (reader, mut writer) = stream.into_split();
                                let mut reader = BufReader::new(reader);
                                let mut response = String::new();

                                // Read greeting
                                reader.read_line(&mut response).await.unwrap();
                                response.clear();

                                // EHLO
                                writer.write_all(b"EHLO client.example.com\r\n").await.unwrap();
                                loop {
                                    let mut line = String::new();
                                    reader.read_line(&mut line).await.unwrap();
                                    if line.starts_with("250 ") {
                                        break;
                                    }
                                }

                                // QUIT
                                writer.write_all(b"QUIT\r\n").await.unwrap();
                                response.clear();
                                reader.read_line(&mut response).await.unwrap();
                            });

                            handles.push(handle);
                        }

                        for handle in handles {
                            handle.await.unwrap();
                        }
                    })
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_server_startup,
    bench_connection_greeting,
    bench_full_conversation,
    bench_command_processing,
    bench_fixture_matching,
    bench_mailbox_operations,
    bench_concurrent_connections
);
criterion_main!(benches);
