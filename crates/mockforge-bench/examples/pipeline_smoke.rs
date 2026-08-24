//! Manual smoke runner for the pipelining bench (#937).
//!
//! Usage:
//!   cargo run -p mockforge-bench --example pipeline_smoke -- \
//!     --target http://127.0.0.1:39999/api --content-type json \
//!     --body-size 4KB --pipeline-depth 8 --connections 2 --duration 5s
//!
//! Mirrors the `mockforge bench-pipeline` CLI so the transport can be
//! exercised without building the full CLI graph.

use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut target = "http://127.0.0.1:39999/".to_string();
    let mut content_type = "json".to_string();
    let mut body_size = "4KB".to_string();
    let mut depth = 8usize;
    let mut connections = 2usize;
    let mut duration_secs = 5u64;
    let mut method = "POST".to_string();

    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let mut take = || args.next().expect("missing value for flag");
        match flag.as_str() {
            "--target" => target = take(),
            "--content-type" => content_type = take(),
            "--body-size" => body_size = take(),
            "--pipeline-depth" => depth = take().parse().unwrap(),
            "--connections" => connections = take().parse().unwrap(),
            "--duration" => duration_secs = take().trim_end_matches('s').parse().unwrap(),
            "--method" => method = take(),
            other => anyhow::bail!("unknown flag {other}"),
        }
    }

    let cfg = mockforge_bench::pipeline_bench::PipelineBenchConfig {
        target_url: target,
        method,
        body_kind: mockforge_bench::pipeline_bench::parse_body_kind(&content_type)
            .map_err(anyhow::Error::msg)?,
        body_size: mockforge_bench::pipeline_bench::parse_body_size(&body_size)
            .map_err(anyhow::Error::msg)?,
        pipeline_depth: depth,
        connections,
        duration: Duration::from_secs(duration_secs),
    };
    let result = mockforge_bench::pipeline_bench::run(cfg).await?;
    print!("{}", mockforge_bench::pipeline_bench::render_report(&result));
    Ok(())
}
