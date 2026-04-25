//! End-to-end regression: a real `suppaftp` client lists and downloads
//! files from the mock FTP server.
//!
//! The existing integration tests cover fixture loading and VFS
//! construction, but nothing binds the libunftp listener and drives a
//! real FTP client. A regression in anonymous login, LIST, or RETR
//! would ship silently. This locks in the on-the-wire contract.

use mockforge_core::config::FtpConfig;
use mockforge_ftp::{FileContent, FileMetadata, FtpServer, VirtualFile};
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
async fn ftp_anonymous_list_and_retrieve() {
    let port = free_port();
    let config = FtpConfig {
        port,
        host: "127.0.0.1".into(),
        allow_anonymous: true,
        virtual_root: PathBuf::from("/"),
        ..FtpConfig::default()
    };

    let server = FtpServer::new(config);

    // Pre-seed a virtual file before starting the server.
    let payload = b"mockforge ftp e2e content".to_vec();
    let file = VirtualFile::new(
        PathBuf::from("/hello.txt"),
        FileContent::Static(payload.clone()),
        FileMetadata {
            size: payload.len() as u64,
            ..Default::default()
        },
    );
    server.vfs().add_file_async(PathBuf::from("/hello.txt"), file).await.unwrap();

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
        assert!(joined.contains("hello.txt"), "LIST must show pre-seeded file, got:\n{joined}");

        // RETR the file and read its full contents.
        let mut reader = ftp.retr_as_buffer("hello.txt").expect("RETR hello.txt");
        let mut buf = Vec::new();
        std::io::copy(&mut reader, &mut Cursor::new(&mut buf)).expect("read RETR payload");
        ftp.quit().ok();
        buf
    })
    .await
    .expect("blocking FTP task did not panic");

    assert_eq!(retrieved, b"mockforge ftp e2e content");

    server_handle.abort();
}
