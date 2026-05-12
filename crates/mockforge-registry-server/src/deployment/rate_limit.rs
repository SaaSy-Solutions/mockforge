//! Per-deployment rate limiting for the hosted-mocks proxy.
//!
//! Protects MockForge from runaway customer traffic that would otherwise
//! accrue Fly compute / bandwidth charges with no brake. A buggy or hostile
//! customer can DOS their own deployment, and without this limit the proxy
//! would happily forward every request.
//!
//! In-memory token bucket per deployment. Single-instance only — when the
//! registry server scales beyond one Fly machine, swap this for the Redis
//! pool already on `AppState`.

use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::Instant,
};
use uuid::Uuid;

/// Default RPS limit per deployment when the env var isn't set.
const DEFAULT_RPS_LIMIT: u32 = 100;

/// 1-second rolling window. A deployment is allowed `rps_limit` requests
/// per window; the window advances by replacing `window_start` and resetting
/// the count when a new request arrives more than a second after the start.
struct WindowState {
    window_start: Instant,
    count: u32,
}

pub struct DeploymentRateLimiter {
    rps_limit: u32,
    state: Mutex<HashMap<Uuid, WindowState>>,
}

impl DeploymentRateLimiter {
    fn new(rps_limit: u32) -> Self {
        Self {
            rps_limit,
            state: Mutex::new(HashMap::new()),
        }
    }

    /// Returns `Ok(())` if the deployment is under its RPS budget for this
    /// 1-second window, or `Err(retry_after_secs)` if it exceeded.
    pub fn check(&self, deployment_id: Uuid) -> Result<(), u64> {
        let mut state = self.state.lock().expect("rate limit mutex poisoned");
        let now = Instant::now();
        let entry = state.entry(deployment_id).or_insert(WindowState {
            window_start: now,
            count: 0,
        });

        if now.duration_since(entry.window_start).as_secs() >= 1 {
            entry.window_start = now;
            entry.count = 0;
        }

        if entry.count >= self.rps_limit {
            // Round-up: tell client to retry next second at the earliest.
            let elapsed_ms = now.duration_since(entry.window_start).as_millis() as u64;
            let retry_after_secs = (1000_u64.saturating_sub(elapsed_ms) / 1000).max(1);
            return Err(retry_after_secs);
        }

        entry.count += 1;
        Ok(())
    }
}

/// Process-global limiter. Configured once from the `MOCKFORGE_HOSTED_MOCK_RPS_LIMIT`
/// env var at first access, or `DEFAULT_RPS_LIMIT` if unset / unparsable.
pub fn global() -> &'static DeploymentRateLimiter {
    static LIMITER: OnceLock<DeploymentRateLimiter> = OnceLock::new();
    LIMITER.get_or_init(|| {
        let rps_limit = std::env::var("MOCKFORGE_HOSTED_MOCK_RPS_LIMIT")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .filter(|&n| n > 0)
            .unwrap_or(DEFAULT_RPS_LIMIT);
        DeploymentRateLimiter::new(rps_limit)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_limit() {
        let limiter = DeploymentRateLimiter::new(3);
        let id = Uuid::new_v4();
        assert!(limiter.check(id).is_ok());
        assert!(limiter.check(id).is_ok());
        assert!(limiter.check(id).is_ok());
        assert!(limiter.check(id).is_err());
    }

    #[test]
    fn independent_per_deployment() {
        let limiter = DeploymentRateLimiter::new(1);
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        assert!(limiter.check(a).is_ok());
        assert!(limiter.check(b).is_ok());
        assert!(limiter.check(a).is_err());
        assert!(limiter.check(b).is_err());
    }

    #[test]
    fn window_resets_after_second() {
        let limiter = DeploymentRateLimiter::new(1);
        let id = Uuid::new_v4();
        assert!(limiter.check(id).is_ok());
        assert!(limiter.check(id).is_err());
        std::thread::sleep(std::time::Duration::from_millis(1100));
        assert!(limiter.check(id).is_ok());
    }
}
