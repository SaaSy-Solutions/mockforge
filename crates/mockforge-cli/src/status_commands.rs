//! `mockforge status` — a project's MockForge state, read without side effects.
//!
//! Tools that drive MockForge from outside (editors, agent hosts, CI) need to
//! know, for one directory: which configuration file `serve` would use, the
//! HTTP port and spec it declares, whether a server answers on that port, and
//! the last recorded `verify-mocks --record` result. `--format json` prints
//! that as one versioned document ([`STATUS_SCHEMA_VERSION`]); nothing is
//! written and nothing is started.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The version of the JSON document `status --format json` prints.
pub(crate) const STATUS_SCHEMA_VERSION: u32 = 1;
/// The version of the report `verify-mocks --record` writes.
pub(crate) const VERIFICATION_SCHEMA_VERSION: u32 = 1;
/// Where `verify-mocks --record` keeps the last result, under the project root.
pub(crate) const VERIFICATION_FILE: &str = ".mockforge/reports/verify-mocks.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum StatusFormat {
    Text,
    Json,
}

/// Show a project's MockForge state without changing anything.
#[derive(clap::Args, Debug)]
pub(crate) struct StatusArgs {
    /// The project directory (default: the current directory). The
    /// configuration file is discovered from here exactly as `serve` does.
    #[arg(short, long)]
    pub dir: Option<PathBuf>,

    /// Output format.
    #[arg(long, value_enum, default_value = "text")]
    pub format: StatusFormat,

    /// How long to wait for the server's health endpoint, in milliseconds.
    #[arg(long, default_value = "1000")]
    pub timeout_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct ProjectStatus {
    pub schema_version: u32,
    /// The version of this `mockforge` binary.
    pub version: String,
    /// The directory the status was read for, absolute.
    pub dir: String,
    /// The configuration file `serve` would load, or null when none exists.
    pub config: Option<ConfigStatus>,
    /// The HTTP server the configuration declares; null without a loadable config.
    pub http: Option<HttpStatus>,
    /// The admin UI the configuration declares; null without a loadable config.
    pub admin: Option<AdminStatus>,
    /// What answered on the declared HTTP port; null when HTTP is disabled
    /// or there is no loadable config.
    pub server: Option<ServerProbe>,
    /// The last `verify-mocks --record` result for this project, if any.
    pub verification: Option<VerificationRecord>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct ConfigStatus {
    pub path: String,
    /// Why the file could not be loaded; null when it loaded.
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct HttpStatus {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub openapi_spec: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct AdminStatus {
    pub enabled: bool,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct ServerProbe {
    /// The health URL that was requested.
    pub url: String,
    /// Whether a MockForge health response came back.
    pub reachable: bool,
    /// The `status` field of that response (`healthy`, `not_ready`, ...).
    pub status: Option<String>,
    pub version: Option<String>,
    pub uptime_seconds: Option<u64>,
    /// Why it was not reachable.
    pub error: Option<String>,
}

/// One recorded `verify-mocks` run, as `--record` writes it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct VerificationRecord {
    pub schema_version: u32,
    /// RFC 3339 time the run finished.
    pub recorded_at: String,
    /// The `mockforge` version that ran it.
    pub mockforge_version: String,
    pub spec: String,
    pub capture_db: String,
    /// `faithful`, `drift`, or `no_traffic` (nothing was recorded to check).
    pub outcome: VerificationOutcome,
    pub exchanges_checked: usize,
    pub endpoints_with_drift: usize,
    pub unmatched_real_paths: usize,
    pub findings: Vec<VerificationFinding>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum VerificationOutcome {
    Faithful,
    Drift,
    NoTraffic,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct VerificationFinding {
    pub endpoint: String,
    pub kind: String,
    pub detail: String,
}

/// The directory that owns a project's MockForge state: the discovered
/// configuration file's directory, else `dir` itself.
pub(crate) async fn project_root(dir: &Path) -> PathBuf {
    match mockforge_core::config::discover_config_file_from(dir).await {
        Ok(path) => path.parent().map(Path::to_path_buf).unwrap_or_else(|| dir.to_path_buf()),
        Err(_) => dir.to_path_buf(),
    }
}

/// The last recorded verification under `root`, if a readable one exists.
fn read_verification(root: &Path) -> Option<VerificationRecord> {
    let text = std::fs::read_to_string(root.join(VERIFICATION_FILE)).ok()?;
    serde_json::from_str::<VerificationRecord>(&text)
        .ok()
        .filter(|record| record.schema_version == VERIFICATION_SCHEMA_VERSION)
}

/// The host a local client reaches a server bound to `host` on.
fn probe_host(host: &str) -> &str {
    match host {
        "" | "0.0.0.0" | "::" | "[::]" => "127.0.0.1",
        other => other,
    }
}

/// The management API's health answer, which a user's spec cannot shadow
/// the way it can `/health`.
#[derive(Deserialize)]
struct ManagementHealth {
    service: Option<String>,
    status: Option<String>,
}
#[derive(Deserialize)]
struct ManagementStats {
    uptime_seconds: Option<u64>,
}
#[derive(Deserialize)]
struct ManagementConfig {
    version: Option<String>,
}
/// The `service` MockForge's management API names itself.
const MANAGEMENT_SERVICE: &str = "mockforge-management";

async fn read_json<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
) -> Option<T> {
    client.get(url).send().await.ok()?.json::<T>().await.ok()
}

async fn probe(host: &str, port: u16, timeout: Duration) -> ServerProbe {
    let host = probe_host(host);
    let authority = if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    let base = format!("http://{authority}/__mockforge/api");
    let url = format!("{base}/health");
    let unreachable = |error: String| ServerProbe {
        url: url.clone(),
        reachable: false,
        status: None,
        version: None,
        uptime_seconds: None,
        error: Some(error),
    };
    let client = match reqwest::Client::builder()
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
    {
        Ok(client) => client,
        Err(error) => return unreachable(error.to_string()),
    };
    let response = match client.get(&url).send().await {
        Ok(response) => response,
        Err(error) if error.is_timeout() => {
            return unreachable(format!("no answer within {} ms", timeout.as_millis()))
        }
        Err(error) if error.is_connect() => {
            return unreachable(format!("could not connect: {error}"))
        }
        Err(error) => return unreachable(error.to_string()),
    };
    match response.json::<ManagementHealth>().await {
        Ok(body) if body.service.as_deref() == Some(MANAGEMENT_SERVICE) => {
            let stats = read_json::<ManagementStats>(&client, &format!("{base}/stats")).await;
            let config = read_json::<ManagementConfig>(&client, &format!("{base}/config")).await;
            ServerProbe {
                url: url.clone(),
                reachable: true,
                status: body.status,
                version: config.and_then(|c| c.version),
                uptime_seconds: stats.and_then(|s| s.uptime_seconds),
                error: None,
            }
        }
        _ => unreachable("the port answered, but not as a MockForge server".into()),
    }
}

/// Read a project's state. Writes nothing.
pub(crate) async fn read_status(dir: &Path, timeout: Duration) -> ProjectStatus {
    let config_path = mockforge_core::config::discover_config_file_from(dir).await.ok();
    let root = config_path
        .as_deref()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| dir.to_path_buf());
    let loaded = match &config_path {
        Some(path) => Some(mockforge_core::config::load_config_auto(path).await),
        None => None,
    };
    let config = config_path.as_ref().map(|path| ConfigStatus {
        path: path.display().to_string(),
        error: match &loaded {
            Some(Err(error)) => Some(error.to_string()),
            _ => None,
        },
    });
    let (http, admin, server) = match loaded {
        Some(Ok(config)) => {
            let server = if config.http.enabled {
                Some(probe(&config.http.host, config.http.port, timeout).await)
            } else {
                None
            };
            (
                Some(HttpStatus {
                    enabled: config.http.enabled,
                    host: config.http.host.clone(),
                    port: config.http.port,
                    openapi_spec: config.http.openapi_spec.clone(),
                }),
                Some(AdminStatus {
                    enabled: config.admin.enabled,
                    port: config.admin.port,
                }),
                server,
            )
        }
        _ => (None, None, None),
    };
    ProjectStatus {
        schema_version: STATUS_SCHEMA_VERSION,
        version: env!("CARGO_PKG_VERSION").to_string(),
        dir: dir.display().to_string(),
        config,
        http,
        admin,
        server,
        verification: read_verification(&root),
    }
}

fn print_text(status: &ProjectStatus) {
    println!("MockForge {} · {}", status.version, status.dir);
    match &status.config {
        None => println!("config        : none found (run `mockforge init`)"),
        Some(ConfigStatus { path, error: None }) => println!("config        : {path}"),
        Some(ConfigStatus {
            path,
            error: Some(error),
        }) => println!("config        : {path} (cannot load: {error})"),
    }
    if let Some(http) = &status.http {
        let spec = http.openapi_spec.as_deref().unwrap_or("no OpenAPI spec");
        println!("http          : {}:{} · {spec}", http.host, http.port);
    }
    match &status.server {
        Some(probe) if probe.reachable => println!(
            "server        : running at {} ({})",
            probe.url,
            probe.status.as_deref().unwrap_or("unknown")
        ),
        Some(probe) => println!(
            "server        : not reachable at {} ({})",
            probe.url,
            probe.error.as_deref().unwrap_or("unknown")
        ),
        None => {}
    }
    match &status.verification {
        Some(record) => println!(
            "verify-mocks  : {:?} at {} ({} exchanges, {} endpoints with drift)",
            record.outcome,
            record.recorded_at,
            record.exchanges_checked,
            record.endpoints_with_drift
        ),
        None => println!("verify-mocks  : no recorded run (use `verify-mocks --record`)"),
    }
}

pub(crate) async fn handle_status_command(args: StatusArgs) -> mockforge_core::Result<()> {
    let dir = match args.dir {
        Some(dir) => dir,
        None => std::env::current_dir().map_err(|e| {
            mockforge_core::Error::config(format!("cannot read the current directory: {e}"))
        })?,
    };
    let dir = std::fs::canonicalize(&dir).map_err(|e| {
        mockforge_core::Error::config(format!("cannot read {}: {e}", dir.display()))
    })?;
    let status = read_status(&dir, Duration::from_millis(args.timeout_ms)).await;
    match args.format {
        StatusFormat::Json => {
            let json = serde_json::to_string_pretty(&status)
                .map_err(|e| mockforge_core::Error::config(e.to_string()))?;
            println!("{json}");
        }
        StatusFormat::Text => print_text(&status),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(outcome: VerificationOutcome) -> VerificationRecord {
        VerificationRecord {
            schema_version: VERIFICATION_SCHEMA_VERSION,
            recorded_at: "2026-10-26T08:00:00Z".into(),
            mockforge_version: "0.3.236".into(),
            spec: "api.yaml".into(),
            capture_db: "captures.db".into(),
            outcome,
            exchanges_checked: 3,
            endpoints_with_drift: 1,
            unmatched_real_paths: 0,
            findings: vec![VerificationFinding {
                endpoint: "GET /users".into(),
                kind: "response-shape".into(),
                detail: "missing field id".into(),
            }],
        }
    }

    /// A directory whose parents discovery reaches all lie inside a fresh
    /// temporary directory, so a config another test leaves in the shared
    /// temporary directory cannot be found.
    fn isolated() -> (tempfile::TempDir, PathBuf) {
        let base = tempfile::tempdir().unwrap();
        let dir = base.path().join("a/b/c/d/e/project");
        std::fs::create_dir_all(&dir).unwrap();
        (base, dir)
    }

    #[tokio::test]
    async fn a_directory_without_config_reports_none_and_writes_nothing() {
        let (_base, dir) = isolated();
        let status = read_status(&dir, Duration::from_millis(200)).await;
        assert_eq!(status.schema_version, 1);
        assert!(status.config.is_none());
        assert!(status.http.is_none());
        assert!(status.server.is_none());
        assert!(status.verification.is_none());
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn a_configured_project_reports_its_port_and_an_unreachable_server() {
        let dir = tempfile::tempdir().unwrap();
        let port = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap().port()
        };
        std::fs::write(
            dir.path().join("mockforge.yaml"),
            format!("http:\n  port: {port}\n  host: 127.0.0.1\n"),
        )
        .unwrap();
        let status = read_status(dir.path(), Duration::from_millis(500)).await;
        let config = status.config.unwrap();
        assert!(config.path.ends_with("mockforge.yaml"));
        assert_eq!(config.error, None);
        assert_eq!(status.http.unwrap().port, port);
        let server = status.server.unwrap();
        assert_eq!(server.url, format!("http://127.0.0.1:{port}/__mockforge/api/health"));
        assert!(!server.reachable);
    }

    #[tokio::test]
    async fn a_running_server_is_reachable_with_its_health_fields() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let json = |value: serde_json::Value| move || async move { axum::Json(value) };
        let app = axum::Router::new()
            .route(
                "/__mockforge/api/health",
                axum::routing::get(json(serde_json::json!({
                    "service": "mockforge-management",
                    "status": "healthy",
                    "timestamp": "2026-10-26T08:00:00Z"
                }))),
            )
            .route(
                "/__mockforge/api/stats",
                axum::routing::get(json(serde_json::json!({
                    "uptime_seconds": 42, "total_requests": 0, "active_mocks": 0,
                    "enabled_mocks": 0, "registered_routes": 0
                }))),
            )
            .route(
                "/__mockforge/api/config",
                axum::routing::get(json(serde_json::json!({
                    "version": "0.3.236", "port": 3000, "has_openapi_spec": false
                }))),
            );
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("mockforge.yaml"), format!("http:\n  port: {port}\n"))
            .unwrap();
        let server = read_status(dir.path(), Duration::from_secs(2)).await.server.unwrap();
        assert!(server.reachable, "{server:?}");
        assert_eq!(server.status.as_deref(), Some("healthy"));
        assert_eq!(server.version.as_deref(), Some("0.3.236"));
        assert_eq!(server.uptime_seconds, Some(42));
    }

    #[tokio::test]
    async fn another_server_on_the_port_is_not_a_mockforge_server() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        // A user's spec can define `/health`; only the management API counts.
        let app = axum::Router::new()
            .route(
                "/health",
                axum::routing::get(|| async {
                    axum::Json(serde_json::json!({"status": "healthy"}))
                }),
            )
            .route(
                "/__mockforge/api/health",
                axum::routing::get(|| async {
                    axum::Json(serde_json::json!({"status": "healthy"}))
                }),
            );
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("mockforge.yaml"), format!("http:\n  port: {port}\n"))
            .unwrap();
        let server = read_status(dir.path(), Duration::from_secs(2)).await.server.unwrap();
        assert!(!server.reachable, "{server:?}");
        assert!(server.error.is_some());
    }

    #[tokio::test]
    async fn an_unloadable_config_is_named_with_its_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("mockforge.yaml"), "http: [not, a, map]\n").unwrap();
        let status = read_status(dir.path(), Duration::from_millis(200)).await;
        assert!(status.config.unwrap().error.is_some());
        assert!(status.http.is_none());
        assert!(status.server.is_none());
    }

    #[tokio::test]
    async fn the_recorded_verification_is_read_from_the_config_directory() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("services/api");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(dir.path().join("mockforge.yaml"), "http:\n  enabled: false\n").unwrap();
        std::fs::create_dir_all(dir.path().join(".mockforge/reports")).unwrap();
        std::fs::write(
            dir.path().join(VERIFICATION_FILE),
            serde_json::to_string(&record(VerificationOutcome::Drift)).unwrap(),
        )
        .unwrap();
        let status = read_status(&nested, Duration::from_millis(200)).await;
        assert_eq!(status.verification, Some(record(VerificationOutcome::Drift)));
        assert!(status.server.is_none(), "HTTP is disabled, so nothing is probed");
    }

    #[tokio::test]
    async fn a_verification_of_another_schema_version_is_not_read() {
        let (_base, dir) = isolated();
        std::fs::create_dir_all(dir.join(".mockforge/reports")).unwrap();
        let mut record = record(VerificationOutcome::Faithful);
        let write = |value: &VerificationRecord| {
            std::fs::write(dir.join(VERIFICATION_FILE), serde_json::to_string(value).unwrap())
                .unwrap()
        };
        write(&record);
        assert!(read_status(&dir, Duration::from_millis(200)).await.verification.is_some());
        record.schema_version = 2;
        write(&record);
        assert!(read_status(&dir, Duration::from_millis(200)).await.verification.is_none());
    }
}
