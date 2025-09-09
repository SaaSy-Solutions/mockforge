use clap::Parser;
use tracing::*;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)] spec: Option<String>,
    #[arg(long, default_value_t=3000)] http_port: u16,
    #[arg(long, default_value_t=3001)] ws_port: u16,
    #[arg(long, default_value_t=50051)] grpc_port: u16,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    info!("MockForge CLI — http:{} ws:{} grpc:{} spec:{:?}", args.http_port, args.ws_port, args.grpc_port, args.spec);
}
