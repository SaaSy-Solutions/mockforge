//! Shared socket fixture configuration and grouped regressions.

use super::*;

mod framing;
mod generation;
mod sockets;

fn socket_config(port: u16, kind: BodyKind, size: u64, duration: Duration) -> PipelineBenchConfig {
    PipelineBenchConfig {
        target_url: format!("http://127.0.0.1:{port}/owned-test"),
        method: "POST".into(),
        body_kind: kind,
        body_size: size,
        pipeline_depth: 2,
        connections: 1,
        duration,
    }
}
