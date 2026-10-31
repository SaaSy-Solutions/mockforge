//! Machine configuration shared by every hosted-mock backend: the runtime
//! environment a mock starts with (Fly machines and Cloudflare Containers) and
//! the Fly services that expose it.

use anyhow::Result;
use std::collections::HashMap;

use crate::models::HostedMock;

/// Environment for a hosted mock's runtime container, shared by every backend
/// (Fly machines and Cloudflare Containers) and by first deploys and redeploys
/// alike. `kafka_advertised_host` is the public name Kafka clients should use;
/// backends that cannot expose Kafka pass `None`.
pub fn hosted_mock_runtime_env(
    deployment: &HostedMock,
    kafka_advertised_host: Option<String>,
) -> Result<HashMap<String, String>> {
    let mut env = HashMap::new();
    env.insert("MOCKFORGE_DEPLOYMENT_ID".to_string(), deployment.id.to_string());
    env.insert("MOCKFORGE_ORG_ID".to_string(), deployment.org_id.to_string());
    env.insert("MOCKFORGE_CONFIG".to_string(), serde_json::to_string(&deployment.config_json)?);
    env.insert("PORT".to_string(), "3000".to_string());
    // Admin server hosts `/api/resilience/*` (#468 Phase 2) and other
    // internal endpoints the registry proxies over Fly 6PN. It binds on
    // 0.0.0.0:9080 inside the container (Docker default for AdminConfig)
    // and isn't published as a public Fly service, so it's reachable
    // only on `{app}.internal:9080` from sibling apps in the same org.
    env.insert("MOCKFORGE_ADMIN_ENABLED".to_string(), "true".to_string());
    env.extend(hosted_mock_cors_env());
    env.extend(crate::handlers::management_token::env_entry(deployment.id));
    env.extend(crate::handlers::hosted_mock_overrides::overrides_env(deployment));

    if let Some(ref spec_url) = deployment.openapi_spec_url {
        env.insert("MOCKFORGE_OPENAPI_SPEC_URL".to_string(), spec_url.clone());
    }

    // Inject env vars + extra Fly services for any protocols the
    // deployment opted into beyond HTTP. HTTP/WS/GraphQL all share port
    // 3000 (router merge happens in mockforge-cli/src/serve.rs) so they
    // never produce a Fly service entry; gRPC and the broker family do.
    let enabled_protocols = deployment.enabled_protocols();
    for protocol in &enabled_protocols {
        if let Some((key, value)) = protocol.enable_env() {
            env.insert(key.to_string(), value);
        }
    }

    // Kafka clients respect the broker's advertised listener over the
    // bootstrap address — without this env var they'd loop on the
    // internal Fly address and fail. The mockforge-kafka broker must
    // honor `config.kafka.advertised_host` for this to be functional
    // end-to-end (#231 broker-side todo).
    if enabled_protocols.contains(&crate::models::Protocol::Kafka) {
        if let Some(host) = kafka_advertised_host {
            env.insert("MOCKFORGE_KAFKA_ADVERTISED_HOST".to_string(), host);
            env.insert("MOCKFORGE_KAFKA_ADVERTISED_PORT".to_string(), "9092".to_string());
        }
    }

    // Wire the in-container request log shipper (#232) and the recorder
    // capture cloud-sync (#234 part 2) to forward to MockForge Cloud.
    // We only set the env vars when:
    //   * a JWT secret is available (production deploys always have one)
    //   * a public ingest base URL is configured via MOCKFORGE_LOG_INGEST_BASE_URL
    // Both shippers reuse the same deployment-scoped JWT — they're
    // ingest-only, so a single token with the deployment subject
    // suffices for both. When the base URL is absent the shippers
    // auto-disable and the deployment runs without cloud forwarding.
    if let Ok(jwt_secret) = std::env::var("JWT_SECRET") {
        if let Ok(ingest_base) = std::env::var("MOCKFORGE_LOG_INGEST_BASE_URL") {
            let trimmed_base = ingest_base.trim_end_matches('/');
            let token = mockforge_registry_core::auth::create_deployment_ingest_token(
                deployment.id,
                &jwt_secret,
                30, // 30 days; rotated on every redeploy
            )
            .ok();
            if let Some(token) = token {
                env.insert(
                    "MOCKFORGE_LOG_INGEST_URL".to_string(),
                    format!("{}/api/v1/hosted-mocks/{}/log-ingest", trimmed_base, deployment.id),
                );
                env.insert("MOCKFORGE_LOG_INGEST_TOKEN".to_string(), token.clone());
                env.insert(
                    "MOCKFORGE_CAPTURE_INGEST_URL".to_string(),
                    format!(
                        "{}/api/v1/hosted-mocks/{}/captures/ingest",
                        trimmed_base, deployment.id
                    ),
                );
                env.insert("MOCKFORGE_CAPTURE_INGEST_TOKEN".to_string(), token);
            }
        }
    }

    // OTLP tracing export (#233). When the registry server has a
    // collector reachable at `MOCKFORGE_OTLP_INGEST_ENDPOINT` we tell
    // the deployment to ship spans there. Identifying labels (deployment
    // id, org id) are already set above as MOCKFORGE_DEPLOYMENT_ID /
    // _ORG_ID; the receiver uses those for multi-tenant routing.
    if let Ok(otlp_endpoint) = std::env::var("MOCKFORGE_OTLP_INGEST_ENDPOINT") {
        if !otlp_endpoint.trim().is_empty() {
            env.insert("MOCKFORGE_OTLP_ENDPOINT".to_string(), otlp_endpoint);
            // Deployment-scoped service name keeps spans separable when
            // multiple hosted mocks share a backend.
            env.insert(
                "MOCKFORGE_OTLP_SERVICE_NAME".to_string(),
                format!("hosted-mock/{}", deployment.slug),
            );
        }
    }

    // Sentry error capture (#713). Forwarded after OTLP so both
    // observability sinks share the same injection point.
    apply_sentry_env(&mut env);

    // Reality-driven proxy upstream (#222). When set, the deployed
    // mockforge-cli's `reality_proxy` middleware forwards a per-request
    // probabilistic share of traffic to this URL based on the
    // workspace's reality_continuum_ratio. Unset = always-mock,
    // current behaviour preserved.
    if let Some(upstream) = deployment.upstream_url() {
        env.insert("MOCKFORGE_PROXY_UPSTREAM".to_string(), upstream);
    }

    Ok(env)
}

/// CORS origins for hosted mocks (#1017). The mock server now denies
/// cross-origin requests unless origins are configured. Hosted mocks are public,
/// credential-free APIs that tenants call from their own front-ends, so they keep
/// the wildcard explicitly; the server never combines it with credentials.
pub(crate) fn hosted_mock_cors_env() -> [(String, String); 1] {
    [("MOCKFORGE_CORS_ALLOWED_ORIGINS".to_string(), "*".to_string())]
}

/// Forward Sentry config (#643/#664) into a hosted-mock's Fly machine env so
/// tenant machines report errors to the same Sentry project as the registry
/// server. `SENTRY_DSN` is required for capture (empty/unset disables it);
/// environment and trace sampling are passed through when the operator sets
/// them, so per-tenant events stay separable without extra config. Release is
/// intentionally NOT forwarded: tenant machines run the mockforge-cli image,
/// whose own crate version is the accurate release identifier.
pub(crate) fn apply_sentry_env(env: &mut HashMap<String, String>) {
    if let Ok(dsn) = std::env::var("SENTRY_DSN") {
        if !dsn.trim().is_empty() {
            env.insert("SENTRY_DSN".to_string(), dsn);
            if let Ok(environment) = std::env::var("SENTRY_ENVIRONMENT") {
                if !environment.trim().is_empty() {
                    env.insert("SENTRY_ENVIRONMENT".to_string(), environment);
                }
            }
            if let Ok(sample_rate) = std::env::var("SENTRY_TRACES_SAMPLE_RATE") {
                if !sample_rate.trim().is_empty() {
                    env.insert("SENTRY_TRACES_SAMPLE_RATE".to_string(), sample_rate);
                }
            }
        }
    }
}
