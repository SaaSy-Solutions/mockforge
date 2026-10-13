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
