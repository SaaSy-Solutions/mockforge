use clap::Parser;
use tracing::*;

#[derive(Parser, Debug)]
struct Args {
    /// Path to OpenAPI spec (json or yaml)
    #[arg(long)] spec: Option<String>,
    #[arg(long, default_value_t=3000)] http_port: u16,
    #[arg(long, default_value_t=3001)] ws_port: u16,
    #[arg(long, default_value_t=50051)] grpc_port: u16,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    info!("MockForge cli — http:{} ws:{} grpc:{} spec:{:?}", args.http_port, args.ws_port, args.grpc_port, args.spec);

    let http = tokio::spawn(mockforge_http::start(args.http_port, args.spec.clone()));
    let ws = tokio::spawn(mockforge_ws::start(args.ws_port));
    let grpc = tokio::spawn(mockforge_grpc::start(args.grpc_port));

    let _ = tokio::join!(http, ws, grpc);
}
