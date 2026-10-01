//! HTTP client for user-supplied targets.
//!
//! Every URL an executor takes from a job payload, a suite config, or a
//! previous response is attacker-controlled. The runner shares a network
//! with Redis, Postgres and the registry, so a bare `reqwest::Client`
//! pointed at one of those is a read-anywhere SSRF. [`TargetClient`] is the
//! only way executors reach such URLs: `request` refuses blocked literal
//! IPs before building a request, and the inner client refuses blocked
//! addresses at DNS time and on every redirect hop.
//!
//! Registry callbacks do not use this. They go to the internal registry
//! address on purpose, through [`crate::callbacks::RegistryCallbacks`].

use std::time::Duration;

use mockforge_bench::ssrf::{self, Policy, SsrfError};

/// The SSRF policy for this process. Strict unless
/// `MOCKFORGE_SSRF_ALLOW_LOOPBACK` is `1` or `true`, which only local tests
/// set. A job payload cannot relax it.
pub fn ssrf_policy() -> Policy {
    match std::env::var("MOCKFORGE_SSRF_ALLOW_LOOPBACK").as_deref() {
        Ok("1") | Ok("true") => Policy::for_test(),
        _ => Policy::strict(),
    }
}

/// Install the process-wide guards in `mockforge-bench`: the SSRF policy
/// for clients it builds internally (native conformance, pre-flight probe),
/// and the k6 egress proxy. Without a proxy, bench refuses to start k6 at
/// all while the guard is installed. Called once from `Dispatcher::new`.
pub fn install_process_guards(k6_egress_proxy: Option<String>) {
    mockforge_bench::ssrf::install_process_guard(ssrf_policy());
    if let Some(proxy) = k6_egress_proxy {
        mockforge_bench::executor::install_k6_egress_proxy(proxy);
    }
}

/// Kinds that run k6 when `use_cloud_api` is set. k6 does its own DNS and
/// follows redirects, so only an egress proxy can contain it.
pub const K6_KINDS: &[&str] = &["bench", "owasp", "security", "wafbench", "crud_flow"];

/// The refusal message for a k6-backed `kind` when no egress proxy is
/// configured, or `None` when the job may run.
pub fn k6_refusal(kind: &str, egress_proxy: Option<&str>) -> Option<String> {
    if K6_KINDS.contains(&kind) && egress_proxy.is_none() {
        Some(format!(
            "{kind} runs use k6, which this runner only starts through an egress proxy; \
             MOCKFORGE_RUNNER_K6_EGRESS_PROXY is not set, so the run was refused"
        ))
    } else {
        None
    }
}

/// A reqwest client that can only reach addresses the SSRF policy allows.
#[derive(Clone)]
pub struct TargetClient {
    http: reqwest::Client,
    policy: Policy,
}

impl TargetClient {
    /// Build a client under [`ssrf_policy`] following at most 10 redirects.
    pub fn new(timeout: Duration, user_agent: &str) -> reqwest::Result<Self> {
        Self::with_policy(ssrf_policy(), timeout, user_agent, 10)
    }

    /// Build a client with an explicit policy and redirect limit.
    pub fn with_policy(
        policy: Policy,
        timeout: Duration,
        user_agent: &str,
        max_redirects: usize,
    ) -> reqwest::Result<Self> {
        let http = ssrf::guarded_client_builder(policy)
            .redirect(ssrf::guarded_redirect_policy(policy, max_redirects))
            .timeout(timeout)
            .user_agent(user_agent)
            .build()?;
        Ok(Self { http, policy })
    }

    /// Refuse `url` if it is malformed, not http(s), or a literal IP in a
    /// blocked range. Hostnames are checked again at connect time.
    pub fn check(&self, url: &str) -> Result<reqwest::Url, SsrfError> {
        let parsed = reqwest::Url::parse(url).map_err(|e| SsrfError::InvalidUrl(e.to_string()))?;
        ssrf::check_url(&parsed, self.policy)?;
        Ok(parsed)
    }

    /// Start a request to `url` once [`Self::check`] accepts it.
    pub fn request(
        &self,
        method: reqwest::Method,
        url: &str,
    ) -> Result<reqwest::RequestBuilder, SsrfError> {
        Ok(self.http.request(method, self.check(url)?))
    }

    /// `request(GET, url)`.
    pub fn get(&self, url: &str) -> Result<reqwest::RequestBuilder, SsrfError> {
        self.request(reqwest::Method::GET, url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strict() -> TargetClient {
        TargetClient::with_policy(Policy::strict(), Duration::from_secs(2), "test", 10).unwrap()
    }

    #[test]
    fn refuses_internal_literal_addresses_before_sending() {
        let client = strict();
        for url in [
            "http://127.0.0.1:8080/",
            "http://172.18.0.5:6379/",
            "http://10.0.0.1/",
            "http://[::1]/",
            "http://169.254.169.254/latest/meta-data/",
        ] {
            assert!(
                matches!(client.get(url), Err(SsrfError::BlockedAddress { .. })),
                "{url} was not refused"
            );
        }
        assert!(matches!(
            client.get("gopher://example.com/"),
            Err(SsrfError::DisallowedScheme(_))
        ));
        assert!(matches!(client.get("not a url"), Err(SsrfError::InvalidUrl(_))));
    }

    #[tokio::test]
    async fn refuses_compose_service_names_that_resolve_privately() {
        // `localhost` stands in for a Compose name like `redis` or
        // `mockforge-registry`: a hostname that resolves to a private address.
        let err = strict().get("http://localhost:1/").unwrap().send().await.unwrap_err();
        assert!(err.is_connect(), "{err:?}");
    }

    #[test]
    fn public_targets_are_allowed_to_build() {
        strict().check("https://demo.mocks.mockforge.dev/health").unwrap();
    }

    #[test]
    fn k6_kinds_are_refused_without_an_egress_proxy() {
        for kind in K6_KINDS {
            let msg = k6_refusal(kind, None).expect("refused");
            assert!(msg.contains("MOCKFORGE_RUNNER_K6_EGRESS_PROXY"), "{msg}");
            assert_eq!(k6_refusal(kind, Some("http://mockforge-egress:4750")), None);
        }
        // Native (non-k6) kinds are unaffected.
        for kind in ["conformance", "integration", "smoke", "chaos_campaign"] {
            assert_eq!(k6_refusal(kind, None), None);
        }
    }
}
