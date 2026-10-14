//! Validate requested capabilities before creating listeners or runtime state.
use super::{ServeArgs, ServerConfig};

fn require(
    name: &str,
    compiled: bool,
    requested: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if requested && !compiled {
        return Err(
            format!("Requested startup capability requires the {name} Cargo feature").into()
        );
    }
    Ok(())
}

pub(super) fn validate(
    args: &ServeArgs,
    config: &ServerConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let has_server = [
        cfg!(feature = "http"),
        cfg!(feature = "ws"),
        cfg!(feature = "grpc"),
        cfg!(feature = "smtp"),
        cfg!(feature = "mqtt"),
        cfg!(feature = "amqp"),
        cfg!(feature = "kafka"),
        cfg!(feature = "tcp"),
    ]
    .into_iter()
    .any(std::convert::identity);
    if !args.dry_run && !has_server && !config.observability.prometheus.enabled {
        return Err("This build has no serve listener; enable a server feature or use a protocol subcommand".into());
    }
    let defaults = ServeArgs::default();
    let http_requested = [
        args.http_port.is_some(),
        args.https_port.is_some(),
        !args.spec.is_empty(),
        args.spec_dir.is_some(),
        args.base_path.is_some(),
        // Match the public Clap default; the internal test builder uses overwrite.
        args.merge_conflicts != "error",
        args.api_versioning != defaults.api_versioning,
        args.tls_enabled,
        args.tls_cert.is_some(),
        args.tls_key.is_some(),
        args.tls_ca.is_some(),
        args.tls_min_version != defaults.tls_min_version,
        args.mtls != defaults.mtls,
        args.llm_mock,
        args.llm_mock_mode != defaults.llm_mock_mode,
        args.llm_mock_upstream.is_some(),
        args.llm_mock_api_key.is_some(),
        args.llm_mock_cassette.is_some(),
        args.mcp_mock,
        config.http.openapi_spec.is_some(),
        !config.routes.is_empty(),
        config.multi_tenant.enabled,
    ]
    .into_iter()
    .any(std::convert::identity);
    require("http", cfg!(feature = "http"), http_requested)?;
    // HTTP, WebSocket and gRPC are enabled in the generic config defaults;
    // defaults alone must not request a protocol omitted from this build.
    // Preserve existing startup scripts that zero unused optional ports. A
    // compiled listener may bind an ephemeral port; an omitted listener does
    // not become a requested operation merely because its port is zero.
    for (name, compiled, requested) in [
        (
            "ws",
            cfg!(feature = "ws"),
            args.ws_port.is_some_and(|port| port != 0)
                || args.ws_replay_file.is_some()
                || config.websocket.replay_file.is_some(),
        ),
        (
            "grpc",
            cfg!(feature = "grpc"),
            args.grpc_port.is_some_and(|port| port != 0)
                || config.grpc.proto_dir.is_some()
                || config.grpc.tls.is_some()
                || !config.grpc.overrides.is_empty(),
        ),
        (
            "tcp",
            cfg!(feature = "tcp"),
            args.tcp_port.is_some_and(|port| port != 0) || config.tcp.enabled,
        ),
        (
            "graphql",
            cfg!(feature = "graphql"),
            args.graphql.is_some()
                || args.graphql_port.is_some_and(|port| port != 0)
                || args.graphql_upstream.is_some()
                || config.graphql.schema_path.is_some()
                || config.graphql.upstream_url.is_some(),
        ),
        ("smtp", cfg!(feature = "smtp"), config.smtp.enabled),
        ("mqtt", cfg!(feature = "mqtt"), config.mqtt.enabled),
        ("amqp", cfg!(feature = "amqp"), config.amqp.enabled),
        ("kafka", cfg!(feature = "kafka"), config.kafka.enabled),
    ] {
        require(name, compiled, requested)?;
    }
    let chaos_requested = [
        args.chaos,
        args.circuit_breaker,
        args.bulkhead,
        args.circuit_breaker_failure_threshold != defaults.circuit_breaker_failure_threshold,
        args.circuit_breaker_success_threshold != defaults.circuit_breaker_success_threshold,
        args.circuit_breaker_timeout_ms != defaults.circuit_breaker_timeout_ms,
        args.circuit_breaker_failure_rate != defaults.circuit_breaker_failure_rate,
        args.bulkhead_max_concurrent != defaults.bulkhead_max_concurrent,
        args.bulkhead_max_queue != defaults.bulkhead_max_queue,
        args.bulkhead_queue_timeout_ms != defaults.bulkhead_queue_timeout_ms,
        config.observability.chaos.as_ref().is_some_and(|config| config.enabled),
    ]
    .into_iter()
    .any(std::convert::identity);
    require("chaos", cfg!(feature = "chaos"), chaos_requested)?;
    require("admin", cfg!(feature = "admin"), config.admin.enabled)?;
    require(
        "recorder",
        cfg!(feature = "recorder"),
        config.observability.recorder.as_ref().is_some_and(|config| config.enabled),
    )?;
    require(
        "tunnel",
        cfg!(feature = "tunnel"),
        config.deceptive_deploy.enabled && config.deceptive_deploy.auto_tunnel,
    )?;
    Ok(())
}
