//! End-to-end regression: a real `suppaftp` client lists and downloads
//! files from the mock FTP server.
//!
//! The existing integration tests cover fixture loading and VFS
//! construction, but nothing binds the libunftp listener and drives a
//! real FTP client. A regression in anonymous login, LIST, or RETR
//! would ship silently. This locks in the on-the-wire contract.

use mockforge_core::config::FtpConfig;
use mockforge_ftp::{FtpFixture, FtpServer};
use std::io::Cursor;
use std::path::PathBuf;
use std::time::Duration;

fn free_port() -> u16 {
    // libunftp listens synchronously on a string address; bind a probe
    // socket first to grab an ephemeral port, then drop it so libunftp
    // can bind the same port. There's a tiny race here but it's reliable
    // for serial-test runs.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
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
            panic!("ftp server never started listening on 127.0.0.1:{port}");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ftp_readme_fixture_file_lists_and_retrieves_over_the_wire() {
    let port = free_port();
    let config = FtpConfig {
        port,
        host: "127.0.0.1".into(),
        allow_anonymous: true,
        virtual_root: PathBuf::from("/"),
        ..FtpConfig::default()
    };

    // Exercise the README's actual YAML and public file-loading constructor.
    let readme = include_str!("../README.md");
    let yaml = readme.split("```yaml\n").nth(1).unwrap().split("```").next().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ftp-fixture.yaml");
    std::fs::write(&path, yaml).unwrap();
    let fixture = FtpFixture::from_yaml_file(path).unwrap();
    let server = FtpServer::new_with_fixtures(config, vec![fixture]).unwrap();

    let server_handle = tokio::spawn(async move {
        server.start().await.unwrap();
    });
    wait_for_port(port, Duration::from_secs(5)).await;

    // Drive the FTP conversation on a blocking thread — suppaftp's
    // default client is sync.
    let port_c = port;
    let retrieved = tokio::task::spawn_blocking(move || {
        use suppaftp::FtpStream;

        let mut ftp =
            FtpStream::connect(format!("127.0.0.1:{port_c}")).expect("connect to mock ftp");
        ftp.login("anonymous", "anon@example.test").expect("anonymous login");

        // LIST the root — we should see our pre-seeded file.
        let entries = ftp.list(None).expect("LIST root");
        let joined = entries.join("\n");
        assert!(joined.contains("welcome.txt"), "LIST must show fixture file, got:\n{joined}");

        // RETR the file and read its full contents.
        let mut reader = ftp.retr_as_buffer("welcome.txt").expect("RETR welcome.txt");
        let mut buf = Vec::new();
        std::io::copy(&mut reader, &mut Cursor::new(&mut buf)).expect("read RETR payload");
        ftp.quit().ok();
        buf
    })
    .await
    .expect("blocking FTP task did not panic");

    assert_eq!(retrieved, b"Welcome to MockForge FTP!");

    server_handle.abort();
    let _ = server_handle.await;
}
