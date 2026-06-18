//! `MockForge` TUI binary entry point.

use anyhow::Result;
use clap::Parser;
use mockforge_tui::config::TuiConfig;
use mockforge_tui::App;
use tracing_appender::non_blocking::WorkerGuard;

#[derive(Parser)]
#[command(name = "mockforge-tui")]
#[command(about = "Terminal UI dashboard for MockForge")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// Admin server URL (overrides config file). The first occurrence
    /// becomes the active server; additional occurrences (round 37 #876
    /// follow-up / Srikanth on 0.3.181: "Can I connect Mockforge tui to
    /// multiple Server IPs and see Logs at the same time by switching
    /// Tabs based on Server IPs") are added to the rotation. Inside the
    /// TUI, `Ctrl-]` / `Ctrl-[` cycle between them.
    ///
    /// Example: `mockforge-tui --admin-url http://h1:9080 --admin-url http://h2:9080`
    #[arg(long, num_args = 0..)]
    admin_url: Vec<String>,

    /// Authentication token
    #[arg(long)]
    token: Option<String>,

    /// Dashboard refresh interval in seconds (overrides config file)
    #[arg(long)]
    refresh_interval: Option<u64>,

    /// Color theme: "dark" or "light" (overrides config file)
    #[arg(long)]
    theme: Option<String>,

    /// Log file path (TUI logs cannot go to stdout)
    #[arg(long)]
    log_file: Option<String>,
}

fn init_logging(log_file: Option<&str>) -> Option<WorkerGuard> {
    use tracing_subscriber::fmt;

    log_file.map(|path| {
        let file_appender = tracing_appender::rolling::never(".", path);
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(non_blocking)
            .with_ansi(false)
            .init();
        guard
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load config file, then overlay CLI args.
    let mut cfg = TuiConfig::load();

    if let Some((first, rest)) = cli.admin_url.split_first() {
        cfg.admin_url = first.clone();
        // CLI takes precedence over config: the rotation is exactly
        // what was passed, not a merge with `~/.config/mockforge/tui.toml`.
        cfg.extra_servers = rest.to_vec();
    }
    if let Some(interval) = cli.refresh_interval {
        cfg.refresh_interval = interval;
    }
    if let Some(theme) = cli.theme {
        cfg.theme = theme;
    }
    if cli.log_file.is_some() {
        cfg.log_file = cli.log_file;
    }

    let _guard = init_logging(cfg.log_file.as_deref());

    let app = App::new(cfg, cli.token);
    app.run().await
}
