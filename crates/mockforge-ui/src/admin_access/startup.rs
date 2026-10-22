//! Credential configuration and bind-time startup policy.
use super::{
    is_loopback, AdminAccessPolicy, ADMIN_PASSWORD_ENV, MANAGEMENT_TOKEN_ENV,
    MIN_ADMIN_PASSWORD_LEN, MIN_SECRET_LEN,
};
use std::net::SocketAddr;

/// The credentials the process environment provides
#[derive(Clone, Default, PartialEq, Eq)]
pub struct AdminAuthEnv {
    /// A usable `MOCKFORGE_MANAGEMENT_TOKEN`
    pub management_token: Option<String>,
    /// A usable `MOCKFORGE_ADMIN_PASSWORD`
    pub admin_password: bool,
    /// A usable `JWT_SECRET`
    pub jwt_secret: bool,
    /// Header-based dev auth or anonymous dev access is switched on
    pub dev_bypass: bool,
    /// Credentials that were set but rejected, with the reason
    pub rejected: Vec<String>,
}

fn env_truthy(name: &str) -> bool {
    matches!(
        std::env::var(name).ok().as_deref().map(str::to_ascii_lowercase).as_deref(),
        Some("1") | Some("true") | Some("yes") | Some("on")
    )
}

impl AdminAuthEnv {
    /// Read the credentials from the process environment
    pub fn from_process_env() -> Self {
        let mut env = Self::default();
        match std::env::var(MANAGEMENT_TOKEN_ENV) {
            Ok(t) if t.len() >= MIN_SECRET_LEN => env.management_token = Some(t),
            Ok(t) if !t.is_empty() => env.rejected.push(format!(
                "{MANAGEMENT_TOKEN_ENV} is shorter than {MIN_SECRET_LEN} characters"
            )),
            _ => {}
        }
        match std::env::var(ADMIN_PASSWORD_ENV) {
            Ok(p) if p.len() >= MIN_ADMIN_PASSWORD_LEN => env.admin_password = true,
            Ok(p) if !p.is_empty() => env.rejected.push(format!(
                "{ADMIN_PASSWORD_ENV} is shorter than {MIN_ADMIN_PASSWORD_LEN} characters"
            )),
            _ => {}
        }
        match std::env::var("JWT_SECRET") {
            Ok(s) if s.len() >= MIN_SECRET_LEN => env.jwt_secret = true,
            Ok(s) if !s.is_empty() => env
                .rejected
                .push(format!("JWT_SECRET is shorter than {MIN_SECRET_LEN} characters")),
            _ => {}
        }
        let dev_environment = matches!(
            std::env::var("ENVIRONMENT").unwrap_or_default().to_ascii_lowercase().as_str(),
            "development" | "dev" | "local"
        );
        env.dev_bypass = dev_environment
            && (env_truthy("MOCKFORGE_ENABLE_DEV_AUTH")
                || env_truthy("MOCKFORGE_ALLOW_UNAUTHENTICATED")
                || env_truthy("MOCKFORGE_ALLOW_UNAUTHENTICATED_DEV"));
        env
    }

    fn any_credentials(&self) -> bool {
        self.management_token.is_some() || self.admin_password || self.jwt_secret
    }
}

/// Decide the access policy for an admin server bound to `addr`.
///
/// Returns an error, and the server must not start, when `addr` is not
/// loopback and no credentials are configured, or when a dev bypass is on.
pub fn policy_for_bind(addr: SocketAddr, env: &AdminAuthEnv) -> Result<AdminAccessPolicy, String> {
    let loopback = is_loopback(addr.ip());
    if loopback {
        if env.any_credentials() {
            return Ok(AdminAccessPolicy::authenticated(true, env.management_token.clone()));
        }
        return Ok(AdminAccessPolicy::loopback_dev());
    }

    if env.dev_bypass {
        return Err(format!(
            "Refusing to start the admin UI on non-loopback address {addr}: development \
             auth bypass (MOCKFORGE_ENABLE_DEV_AUTH / MOCKFORGE_ALLOW_UNAUTHENTICATED) is \
             enabled. Unset it, or bind the admin UI to 127.0.0.1 with --admin-host."
        ));
    }
    if !env.any_credentials() {
        let rejected = if env.rejected.is_empty() {
            String::new()
        } else {
            format!(" ({})", env.rejected.join("; "))
        };
        return Err(format!(
            "Refusing to start the admin UI on non-loopback address {addr} without \
             credentials{rejected}. Set {ADMIN_PASSWORD_ENV} (at least \
             {MIN_ADMIN_PASSWORD_LEN} characters) to log in to the admin UI, or \
             {MANAGEMENT_TOKEN_ENV} (at least {MIN_SECRET_LEN} characters) for API \
             access, or bind the admin UI to 127.0.0.1 with --admin-host / \
             MOCKFORGE_ADMIN_HOST."
        ));
    }
    // The built-in demo users (admin/admin123) exist only when neither an
    // admin password nor a JWT secret is configured. Their tokens must not
    // open a network-reachable server, so JWTs are refused in that case.
    let accept_jwt = env.admin_password || env.jwt_secret;
    Ok(AdminAccessPolicy::authenticated(accept_jwt, env.management_token.clone()))
}

impl std::fmt::Debug for AdminAuthEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdminAuthEnv")
            .field("management_token", &self.management_token.as_ref().map(|_| "<redacted>"))
            .field("admin_password", &self.admin_password)
            .field("jwt_secret", &self.jwt_secret)
            .field("dev_bypass", &self.dev_bypass)
            .field("rejected", &self.rejected)
            .finish()
    }
}
