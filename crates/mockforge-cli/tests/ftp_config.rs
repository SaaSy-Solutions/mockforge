//! Exercise FTP configuration through the actual CLI and TCP listener.
#![cfg(feature = "ftp")]

use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    process::Command,
};

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mockforge"));
    command
        .args(["ftp", "serve"])
        .env_remove("SENTRY_DSN")
        .env_remove("MOCKFORGE_ANALYTICS_DB")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    command
}

async fn reply(reader: &mut BufReader<TcpStream>, expected: &str) -> String {
    let mut line = String::new();
    tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut line))
        .await
        .expect("FTP reply deadline")
        .unwrap();
    assert!(line.starts_with(expected), "expected {expected}, received {line:?}");
    line
}

async fn send(reader: &mut BufReader<TcpStream>, request: &str, expected: &str) -> String {
    reader.get_mut().write_all(request.as_bytes()).await.unwrap();
    reply(reader, expected).await
}

#[tokio::test]
async fn configured_listener_relative_fixtures_passive_ports_and_flag_overrides() {
    for (nested, override_port) in [(false, false), (true, false), (true, true)] {
        let directory = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("fixtures")).unwrap();
        std::fs::write(
            directory.path().join("fixtures/welcome.yaml"),
            "identifier: welcome\nname: welcome\ndescription: null\nvirtual_files:\n  - path: /welcome.txt\n    content:\n      type: static\n      content: configured fixture\n    permissions: rw-r--r--\n    owner: mock\n    group: mock\nupload_rules: []\n",
        ).unwrap();
        let passive = free_port();
        let configured_port = free_port();
        let port = if override_port {
            free_port()
        } else {
            configured_port
        };
        let settings = format!(
            "host: 127.0.0.1\nport: {configured_port}\nvirtual_root: /\nfixtures_dir: fixtures\npassive_ports: [{passive}, {passive}]\ntimeout_secs: 1\n"
        );
        let yaml = if nested {
            format!(
                "ftp:\n{}",
                settings.lines().map(|line| format!("  {line}\n")).collect::<String>()
            )
        } else {
            settings
        };
        let path = directory.path().join("ftp.yaml");
        std::fs::write(&path, yaml).unwrap();
        let mut cli = command();
        cli.arg("--config").arg(&path).current_dir(elsewhere.path());
        if override_port {
            cli.arg("--port").arg(port.to_string());
        }
        let mut child = cli.spawn().unwrap();
        let stream = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if let Ok(stream) = TcpStream::connect(("127.0.0.1", port)).await {
                    break stream;
                }
                assert!(child.try_wait().unwrap().is_none(), "FTP CLI exited before listening");
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("configured FTP port never listened");
        let mut control = BufReader::new(stream);
        reply(&mut control, "220").await;
        send(&mut control, "USER anonymous\r\n", "331").await;
        send(&mut control, "PASS fixture@example.test\r\n", "230").await;
        send(&mut control, "TYPE I\r\n", "200").await;
        let pasv = send(&mut control, "PASV\r\n", "227").await;
        let tuple = pasv.split('(').nth(1).unwrap().split(')').next().unwrap();
        let numbers: Vec<u16> = tuple.split(',').map(|part| part.parse().unwrap()).collect();
        assert_eq!(numbers.len(), 6);
        let data_port = numbers[4] * 256 + numbers[5];
        assert_eq!(data_port, passive, "configured passive range must be honored");
        let mut data = TcpStream::connect(("127.0.0.1", data_port)).await.unwrap();
        send(&mut control, "RETR /welcome.txt\r\n", "150").await;
        let mut bytes = Vec::new();
        tokio::time::timeout(Duration::from_secs(5), data.read_to_end(&mut bytes))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(bytes, b"configured fixture");
        reply(&mut control, "226").await;
        // This deadline is shorter than libunftp's default timeout, so it also
        // verifies that the configured idle timeout reaches the listener.
        reply(&mut control, "221").await;
        child.kill().await.unwrap();
        child.wait().await.unwrap();
    }
}

#[tokio::test]
async fn invalid_configuration_fails_before_binding() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ftp.yaml");
    for (contents, expected) in [
        (None, "Could not read FTP configuration"),
        (Some("ftp: [invalid"), "Invalid FTP configuration"),
        (Some("passive_ports: [51000, 50000]"), "FTP passive port range"),
        (Some("fixtures_dir: missing-fixtures"), "Could not read FTP fixtures directory"),
    ] {
        if let Some(contents) = contents {
            std::fs::write(&path, contents).unwrap();
        }
        let output = tokio::time::timeout(
            Duration::from_secs(10),
            command().arg("--config").arg(&path).output(),
        )
        .await
        .expect("invalid configuration must fail immediately")
        .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "expected {expected}"
        );
    }
}
