//! Network-layer fault-injecting HTTP client for chaos campaigns
//! against external (non-hosted_mock) targets — issue #349.
//!
//! ## Why this exists
//!
//! For hosted-mock deployments the chaos executor toggles in-process
//! middleware on the deployment itself (`/__mockforge/chaos/toggle`).
//! That works because we own the listener.
//!
//! For external targets the executor only has a URL. There's nothing
//! to toggle. The previous behaviour was to emit synthetic
//! `fault_injected` / `fault_recovered` events on a sleep loop without
//! touching the network — useful for wiring tests, useless for actual
//! resilience verification.
//!
//! This crate provides [`ChaosClient`], a thin wrapper around
//! `reqwest::Client` that:
//!
//! 1. Validates the target URL through the [`mockforge_bench::ssrf`]
//!    guard before each request (defense in depth — the registry's
//!    trigger-time check (#370) is the primary line of defense).
//! 2. Applies latency / error / drop chaos using inline sync helpers
//!    rather than delegating to `mockforge-chaos`'s `LatencyInjector`
//!    / `FaultInjector` — those use a thread-local `rand::ThreadRng`
//!    that's `!Send`, so holding one across `.await` would make the
//!    surrounding `Future` `!Send`, which the runner's `Executor`
//!    trait won't accept (it spawns futures on a multi-threaded
//!    tokio runtime). The dice semantics here are deliberately
//!    simpler than the in-process middleware: latency at probability
//!    1.0 when set, plus independent error / drop rates.
//! 3. Returns a [`ChaosOutcome`] per request so the executor can emit
//!    real-data events (status code, observed latency, fault kind)
//!    instead of synthetic ones.
//!
//! ## What's not in v1
//!
//! - **Standalone server.** Issue #349's design proposes deploying
//!   the proxy as a separate Fly app so the egress trust zone is
//!   isolated. This crate is structured so a future PR can wrap
//!   [`ChaosClient`] in an axum router (it's already `Send + Sync +
//!   Clone`), but for v1 the executor calls it in-process to keep
//!   the deployment story simple.
//! - **DNS-TXT / `/.well-known/mockforge-chaos-authorized` proof.**
//!   The issue defers customer authorization to a follow-up. The
//!   executor enforces a minimal `external_target_authorized: true`
//!   foot-gun guard in the meantime so chaos can't be pointed at an
//!   arbitrary URL by accident.
//!
//! ## Usage
//!
//! ```no_run
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use mockforge_chaos_proxy::{ChaosClient, ChaosDirective};
//!
//! let directive = ChaosDirective::default()
//!     .with_latency_ms(500)
//!     .with_error_rate(0.25);
//!
//! let client = ChaosClient::new(directive)?;
//! let outcome = client
//!     .probe("GET", "https://api.example.com/health", None)
//!     .await?;
//!
//! println!("status={:?} latency_ms={} fault={:?}",
//!     outcome.status_code,
//!     outcome.latency_ms,
//!     outcome.fault_kind,
//! );
//! # Ok(())
//! # }
//! ```

use std::time::{Duration, Instant};

mod error;
pub use error::{ChaosProxyError, Result};

/// What the executor wants to do to outbound requests for one
/// campaign run. Plain values — no shared state — so the directive
/// can be cloned per request and tweaked between iterations.
#[derive(Debug, Clone)]
pub struct ChaosDirective {
    /// Optional injected latency, in milliseconds, applied before the
    /// outbound HTTP request fires. `None` means "no latency
    /// injection." Always applied (probability 1.0) when set —
    /// callers that want probabilistic latency can switch to
    /// [`Self::with_latency_config`] directly.
    pub latency_ms: Option<u64>,
    /// Optional probability of synthesising an HTTP 5xx instead of
    /// forwarding to the target. `None` means "no fault injection."
    /// Roll happens per request.
    pub error_rate: Option<f64>,
    /// Optional probability of dropping the request entirely (no
    /// outbound traffic, no response — the outcome carries
    /// `succeeded=false` with a synthetic `Drop` fault).
    pub drop_rate: Option<f64>,
    /// Per-request egress timeout. Defaults to 10s; a chaos run
    /// against a slow target shouldn't pin a runner for minutes.
    pub timeout: Duration,
}

impl Default for ChaosDirective {
    fn default() -> Self {
        Self {
            latency_ms: None,
            error_rate: None,
            drop_rate: None,
            timeout: Duration::from_secs(10),
        }
    }
}

impl ChaosDirective {
    /// Set a fixed latency to inject before each outbound request.
    pub fn with_latency_ms(mut self, ms: u64) -> Self {
        self.latency_ms = Some(ms);
        self
    }

    /// Set the probability (0.0..=1.0) of synthesising an HTTP error
    /// instead of forwarding.
    pub fn with_error_rate(mut self, rate: f64) -> Self {
        self.error_rate = Some(rate.clamp(0.0, 1.0));
        self
    }

    /// Set the probability (0.0..=1.0) of dropping the request
    /// entirely.
    pub fn with_drop_rate(mut self, rate: f64) -> Self {
        self.drop_rate = Some(rate.clamp(0.0, 1.0));
        self
    }

    /// Override the per-request egress timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Effective latency in milliseconds, or `0` when no latency
    /// was requested. Always applied at probability 1.0 in v1.
    fn effective_latency_ms(&self) -> u64 {
        self.latency_ms.unwrap_or(0)
    }

    /// True iff this directive has any chaos at all (helpful for the
    /// executor's "no chaos at all" early-return path).
    pub fn is_active(&self) -> bool {
        self.latency_ms.is_some_and(|ms| ms > 0)
            || self.error_rate.is_some_and(|r| r > 0.0)
            || self.drop_rate.is_some_and(|r| r > 0.0)
    }
}

/// Why a request didn't reach the target (or didn't return a
/// success). Maps onto the `fault_kind` field in the executor's
/// `fault_injected` event so the UI's chaos timeline shows real data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultKind {
    /// No fault — the request was forwarded normally.
    None,
    /// Latency injected before forwarding.
    Latency,
    /// Synthesised HTTP error instead of forwarding.
    SynthesizedError,
    /// Dropped — no outbound request was made.
    Dropped,
}

impl FaultKind {
    /// Wire string used in event payloads.
    pub fn as_str(&self) -> &'static str {
        match self {
            FaultKind::None => "none",
            FaultKind::Latency => "latency",
            FaultKind::SynthesizedError => "synthesized_error",
            FaultKind::Dropped => "dropped",
        }
    }
}

/// One probe's worth of outcome data.
#[derive(Debug, Clone)]
pub struct ChaosOutcome {
    /// HTTP status code observed at the target, or the synthesized
    /// status when [`FaultKind::SynthesizedError`] fires. `None` when
    /// the request was dropped or the network failed.
    pub status_code: Option<u16>,
    /// Total wall-clock time from probe start to outcome, including
    /// injected latency. `0` when dropped before any work happened.
    pub latency_ms: u64,
    /// Whether the probe surfaced a 2xx/3xx response.
    pub succeeded: bool,
    /// Which fault (if any) was applied to this probe.
    pub fault_kind: FaultKind,
    /// Free-text error when `succeeded=false`. Empty otherwise.
    pub error_message: String,
}

/// Fault-injecting HTTP client. `Clone`-able and cheap; build one per
/// campaign and reuse across probe iterations.
#[derive(Clone)]
pub struct ChaosClient {
    http: reqwest::Client,
    directive: ChaosDirective,
    ssrf_policy: mockforge_bench::ssrf::Policy,
}

impl ChaosClient {
    /// Build a new client with the strict (production) SSRF policy
    /// — RFC1918 / loopback / link-local are all rejected. Use
    /// [`Self::with_policy`] to override (e.g. for tests against a
    /// loopback target).
    ///
    /// Errors if the underlying `reqwest::Client` can't be
    /// constructed — should be unreachable on supported platforms.
    pub fn new(directive: ChaosDirective) -> Result<Self> {
        Self::with_policy(directive, mockforge_bench::ssrf::Policy::strict())
    }

    /// Build a client with an explicit SSRF policy. Allows test code
    /// to opt into loopback without mutating the global
    /// `MOCKFORGE_SSRF_ALLOW_LOOPBACK` env var (which doesn't play
    /// well with parallel tests).
    pub fn with_policy(
        directive: ChaosDirective,
        ssrf_policy: mockforge_bench::ssrf::Policy,
    ) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(directive.timeout)
            .user_agent("mockforge-chaos-proxy/1.0")
            .build()
            .map_err(ChaosProxyError::ClientBuild)?;
        Ok(Self {
            http,
            directive,
            ssrf_policy,
        })
    }

    /// Get the directive this client was built with. Useful for
    /// logging in the executor.
    pub fn directive(&self) -> &ChaosDirective {
        &self.directive
    }

    /// Send one chaos-injected probe to `target_url` with `method`
    /// and an optional JSON body. SSRF-validates `target_url` before
    /// any other work.
    ///
    /// Returns a [`ChaosOutcome`] regardless of whether the request
    /// succeeded — chaos is a measurement tool, so a network failure
    /// is just another data point, not an `Err`. `Err` is reserved
    /// for setup failures (URL is malformed, SSRF rejected, builder
    /// failed) where the executor should abort the campaign.
    pub async fn probe(
        &self,
        method: &str,
        target_url: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<ChaosOutcome> {
        // 1. SSRF guard. Policy was decided at client construction so
        //    we don't read env vars on every probe.
        mockforge_bench::ssrf::validate_target_url(target_url, self.ssrf_policy)
            .await
            .map_err(|e| ChaosProxyError::SsrfRejected(e.to_string()))?;

        let started = Instant::now();

        // 2. Drop dice. Roll synchronously so the rng's !Send
        //    `ThreadRng` doesn't end up captured in this async fn's
        //    state machine. The drop fault has highest priority —
        //    if it fires we don't even build the request, since
        //    "drop" semantically means "no traffic at all."
        if roll_dice(self.directive.drop_rate) {
            return Ok(ChaosOutcome {
                status_code: None,
                latency_ms: 0,
                succeeded: false,
                fault_kind: FaultKind::Dropped,
                error_message: "request dropped by chaos directive".to_string(),
            });
        }

        // 3. Fault dice. Roll BEFORE injecting latency so a
        //    synthesised error doesn't waste the campaign's
        //    wall-clock budget on a sleep we're going to throw
        //    away. (The in-process middleware applies latency
        //    first, but it controls the listener — we don't, so
        //    keeping the synth-error path fast matters more.)
        let synthesize_error = roll_dice(self.directive.error_rate);

        // 4. Latency. Always applied at probability 1.0 when set —
        //    callers wanting probabilistic latency can roll the dice
        //    themselves and pass `latency_ms = None` when they want
        //    no delay this iteration.
        let mut applied_fault = FaultKind::None;
        let latency_ms = self.directive.effective_latency_ms();
        if latency_ms > 0 {
            tokio::time::sleep(Duration::from_millis(latency_ms)).await;
            applied_fault = FaultKind::Latency;
        }

        // 5. If we rolled an error earlier, synthesise it now.
        if synthesize_error {
            let elapsed_ms = started.elapsed().as_millis() as u64;
            return Ok(ChaosOutcome {
                status_code: Some(503),
                latency_ms: elapsed_ms,
                succeeded: false,
                fault_kind: FaultKind::SynthesizedError,
                error_message: "synthesized HTTP 503 by chaos directive".to_string(),
            });
        }

        // 5. Forward to the real target.
        let method = reqwest::Method::from_bytes(method.as_bytes())
            .map_err(|_| ChaosProxyError::BadMethod)?;
        let mut req = self.http.request(method, target_url);
        if let Some(b) = body {
            req = req.json(b);
        }

        match req.send().await {
            Ok(resp) => {
                let status = resp.status();
                let elapsed_ms = started.elapsed().as_millis() as u64;
                Ok(ChaosOutcome {
                    status_code: Some(status.as_u16()),
                    latency_ms: elapsed_ms,
                    succeeded: status.is_success() || status.is_redirection(),
                    fault_kind: applied_fault,
                    error_message: if status.is_success() || status.is_redirection() {
                        String::new()
                    } else {
                        format!("target returned HTTP {}", status.as_u16())
                    },
                })
            }
            Err(e) => {
                let elapsed_ms = started.elapsed().as_millis() as u64;
                Ok(ChaosOutcome {
                    status_code: None,
                    latency_ms: elapsed_ms,
                    succeeded: false,
                    fault_kind: applied_fault,
                    error_message: format!("network error: {e}"),
                })
            }
        }
    }
}

/// Roll a synchronous probability dice. Returns `true` with the
/// given probability. Inlined as a free function (rather than going
/// through `mockforge-chaos`'s `LatencyInjector` / `FaultInjector`)
/// because those types use `rand::rng()` internally, which returns
/// a `ThreadRng` that's `!Send`. Holding one across `.await` makes
/// the surrounding `Future` `!Send`, which the runner's `Executor`
/// trait won't accept (it requires `Send` futures so the dispatcher
/// can spawn them on the multi-threaded tokio runtime). Doing the
/// roll in a sync helper keeps the rng strictly out of the async
/// state machine.
fn roll_dice(rate: Option<f64>) -> bool {
    let Some(rate) = rate else {
        return false;
    };
    if rate <= 0.0 {
        return false;
    }
    if rate >= 1.0 {
        return true;
    }
    use rand::Rng;
    rand::rng().random::<f64>() < rate
}

/// Aggregated counters for a chaos campaign run. The executor
/// updates this in place per probe and serialises it as the
/// `chaos_summary` metric event so the UI can render run totals
/// without parsing every individual `fault_injected` event.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct CampaignCounters {
    pub probes_sent: u32,
    pub successes: u32,
    pub failures: u32,
    pub dropped: u32,
    pub synthesized_errors: u32,
    pub latency_injections: u32,
    pub total_observed_latency_ms: u64,
}

impl CampaignCounters {
    /// Fold one outcome into the running totals.
    pub fn record(&mut self, outcome: &ChaosOutcome) {
        self.probes_sent = self.probes_sent.saturating_add(1);
        self.total_observed_latency_ms =
            self.total_observed_latency_ms.saturating_add(outcome.latency_ms);
        if outcome.succeeded {
            self.successes = self.successes.saturating_add(1);
        } else {
            self.failures = self.failures.saturating_add(1);
        }
        match outcome.fault_kind {
            FaultKind::Latency => {
                self.latency_injections = self.latency_injections.saturating_add(1);
            }
            FaultKind::SynthesizedError => {
                self.synthesized_errors = self.synthesized_errors.saturating_add(1);
            }
            FaultKind::Dropped => {
                self.dropped = self.dropped.saturating_add(1);
            }
            FaultKind::None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directive_default_is_no_chaos() {
        let d = ChaosDirective::default();
        assert!(d.latency_ms.is_none());
        assert!(d.error_rate.is_none());
        assert!(d.drop_rate.is_none());
        assert!(!d.is_active());
        assert_eq!(d.effective_latency_ms(), 0);
    }

    #[test]
    fn directive_clamps_rates() {
        let d = ChaosDirective::default().with_error_rate(2.0).with_drop_rate(-0.5);
        assert_eq!(d.error_rate, Some(1.0));
        assert_eq!(d.drop_rate, Some(0.0));
    }

    #[test]
    fn directive_is_active_only_when_some_chaos() {
        assert!(ChaosDirective::default().with_latency_ms(50).is_active());
        assert!(ChaosDirective::default().with_error_rate(0.5).is_active());
        assert!(ChaosDirective::default().with_drop_rate(0.5).is_active());
        // Latency 0 and rates 0 are degenerate — no chaos.
        assert!(!ChaosDirective::default().with_latency_ms(0).is_active());
        assert!(!ChaosDirective::default().with_error_rate(0.0).is_active());
        assert!(!ChaosDirective::default().with_drop_rate(0.0).is_active());
    }

    #[test]
    fn roll_dice_handles_extremes() {
        // None and 0.0 always false; 1.0 always true; in-between is
        // probabilistic so we don't try to assert.
        assert!(!roll_dice(None));
        assert!(!roll_dice(Some(0.0)));
        assert!(!roll_dice(Some(-0.1)));
        assert!(roll_dice(Some(1.0)));
        assert!(roll_dice(Some(2.0))); // clamped via >=1.0 short-circuit
    }

    #[test]
    fn fault_kind_wire_strings() {
        assert_eq!(FaultKind::None.as_str(), "none");
        assert_eq!(FaultKind::Latency.as_str(), "latency");
        assert_eq!(FaultKind::SynthesizedError.as_str(), "synthesized_error");
        assert_eq!(FaultKind::Dropped.as_str(), "dropped");
    }

    #[test]
    fn campaign_counters_record_succeeded() {
        let mut c = CampaignCounters::default();
        c.record(&ChaosOutcome {
            status_code: Some(200),
            latency_ms: 30,
            succeeded: true,
            fault_kind: FaultKind::None,
            error_message: String::new(),
        });
        assert_eq!(c.probes_sent, 1);
        assert_eq!(c.successes, 1);
        assert_eq!(c.failures, 0);
        assert_eq!(c.total_observed_latency_ms, 30);
    }

    #[test]
    fn campaign_counters_record_each_fault_kind() {
        let mut c = CampaignCounters::default();
        for (status, fk, succ) in [
            (Some(200), FaultKind::Latency, true),
            (Some(503), FaultKind::SynthesizedError, false),
            (None, FaultKind::Dropped, false),
            (Some(500), FaultKind::None, false),
        ] {
            c.record(&ChaosOutcome {
                status_code: status,
                latency_ms: 10,
                succeeded: succ,
                fault_kind: fk,
                error_message: String::new(),
            });
        }
        assert_eq!(c.probes_sent, 4);
        assert_eq!(c.successes, 1);
        assert_eq!(c.failures, 3);
        assert_eq!(c.latency_injections, 1);
        assert_eq!(c.synthesized_errors, 1);
        assert_eq!(c.dropped, 1);
        assert_eq!(c.total_observed_latency_ms, 40);
    }

    fn test_policy() -> mockforge_bench::ssrf::Policy {
        mockforge_bench::ssrf::Policy::for_test()
    }

    #[tokio::test]
    async fn drop_rate_one_always_drops() {
        // probability=1.0 means every probe should be dropped without
        // touching the network. Use an obviously-invalid target so a
        // missed-drop would surface as a network error rather than
        // success.
        let directive = ChaosDirective::default().with_drop_rate(1.0);
        let client = ChaosClient::with_policy(directive, test_policy()).expect("client builds");

        let outcome = client
            .probe("GET", "http://127.0.0.1:1/never-listen", None)
            .await
            .expect("probe returns Ok even when dropped");

        assert_eq!(outcome.fault_kind, FaultKind::Dropped);
        assert!(!outcome.succeeded);
        assert!(outcome.status_code.is_none());
    }

    #[tokio::test]
    async fn error_rate_one_always_synthesizes() {
        let directive = ChaosDirective::default().with_error_rate(1.0);
        let client = ChaosClient::with_policy(directive, test_policy()).expect("client builds");
        let outcome = client
            .probe("GET", "http://127.0.0.1:1/never-listen", None)
            .await
            .expect("probe returns Ok");
        assert_eq!(outcome.fault_kind, FaultKind::SynthesizedError);
        assert_eq!(outcome.status_code, Some(503));
        assert!(!outcome.succeeded);
    }

    #[tokio::test]
    async fn ssrf_blocks_loopback_in_strict_mode() {
        // Default constructor uses strict policy — loopback must be
        // rejected.
        let client = ChaosClient::new(ChaosDirective::default()).expect("client builds");
        let err = client
            .probe("GET", "http://127.0.0.1:1/anything", None)
            .await
            .expect_err("strict policy must reject loopback");
        match err {
            ChaosProxyError::SsrfRejected(_) => {}
            other => panic!("expected SsrfRejected, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ssrf_blocks_rfc1918() {
        let client = ChaosClient::new(ChaosDirective::default()).expect("client builds");
        let err = client
            .probe("GET", "http://10.0.0.1/anything", None)
            .await
            .expect_err("strict policy must reject RFC1918");
        match err {
            ChaosProxyError::SsrfRejected(_) => {}
            other => panic!("expected SsrfRejected, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn forwards_to_local_target_when_no_chaos() {
        // Spin up a tiny axum server on a random port, point the client
        // at it, and verify the request actually went through.
        use axum::{routing::get, Router};
        use tokio::net::TcpListener;

        let app = Router::new().route("/ok", get(|| async { "hello" }));
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });

        let client = ChaosClient::with_policy(ChaosDirective::default(), test_policy())
            .expect("client builds");
        let outcome = client
            .probe("GET", &format!("http://{addr}/ok"), None)
            .await
            .expect("probe returns Ok");

        assert_eq!(outcome.status_code, Some(200));
        assert!(outcome.succeeded);
        assert_eq!(outcome.fault_kind, FaultKind::None);
    }
}
