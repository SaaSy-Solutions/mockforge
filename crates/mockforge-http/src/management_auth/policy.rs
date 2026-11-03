//! Deployment policy construction and explicit loopback opt-out validation.
use super::{INSECURE_MANAGEMENT_ENV, MANAGEMENT_TOKEN_ENV, MIN_TOKEN_LEN};
use std::{fmt, net::IpAddr, sync::Arc};

/// The admin user's HTTP Basic credentials. The password may be plaintext or
/// a bcrypt hash.
#[derive(Clone)]
pub struct AdminCredential {
    pub(super) username: String,
    pub(super) password: String,
}

impl AdminCredential {
    /// Both values must be non-empty.
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Option<Self> {
        let (username, password) = (username.into(), password.into());
        if username.is_empty() || password.is_empty() {
            return None;
        }
        Some(Self { username, password })
    }
}

impl fmt::Debug for AdminCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AdminCredential").field("username", &self.username).finish()
    }
}

/// Who may use the control plane on the data port
#[derive(Clone)]
pub struct ControlPlanePolicy {
    pub(super) mode: Mode,
}

#[derive(Clone)]
pub(super) enum Mode {
    /// A credential is required. With neither set, the control plane is locked.
    Credentials {
        token: Option<Arc<str>>,
        admin: Option<AdminCredential>,
    },
    /// No authentication. Constructed only for a loopback bind.
    UnauthenticatedLoopback,
}

impl fmt::Debug for ControlPlanePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.mode {
            Mode::Credentials { token, admin } => f
                .debug_struct("Credentials")
                .field("token", &token.as_ref().map(|_| "<redacted>"))
                .field("admin", admin)
                .finish(),
            Mode::UnauthenticatedLoopback => f.write_str("UnauthenticatedLoopback"),
        }
    }
}

/// Why a control-plane policy cannot be built
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ControlPlaneConfigError {
    /// The token is shorter than [`MIN_TOKEN_LEN`]
    #[error("{MANAGEMENT_TOKEN_ENV} must be at least {MIN_TOKEN_LEN} characters")]
    TokenTooShort,
    /// The opt-out was requested on a non-loopback bind
    #[error(
        "Refusing to start: {INSECURE_MANAGEMENT_ENV} turns off control-plane authentication \
         and is allowed only when the HTTP server is bound to a loopback address, but it is \
         bound to {0}. Unset it, or set {MANAGEMENT_TOKEN_ENV} instead."
    )]
    InsecureOnNonLoopback(IpAddr),
}

impl ControlPlanePolicy {
    /// Every control-plane request is refused.
    pub fn locked() -> Self {
        Self {
            mode: Mode::Credentials {
                token: None,
                admin: None,
            },
        }
    }

    /// Require `token` (at least [`MIN_TOKEN_LEN`] characters).
    pub fn with_token(token: impl Into<String>) -> Result<Self, ControlPlaneConfigError> {
        let token = token.into();
        if token.chars().count() < MIN_TOKEN_LEN {
            return Err(ControlPlaneConfigError::TokenTooShort);
        }
        Ok(Self {
            mode: Mode::Credentials {
                token: Some(Arc::from(token)),
                admin: None,
            },
        })
    }

    /// Also accept the admin user's Basic credentials. Has no effect on an
    /// unauthenticated policy.
    pub fn with_admin(mut self, admin: Option<AdminCredential>) -> Self {
        if let Mode::Credentials { admin: slot, .. } = &mut self.mode {
            if admin.is_some() {
                *slot = admin;
            }
        }
        self
    }

    /// No authentication, for a server bound to `bind`. Refused unless
    /// `bind` is a loopback address (`127.0.0.0/8` or `::1`). Wildcards and
    /// IPv4-mapped IPv6 addresses are refused.
    pub fn unauthenticated_loopback(bind: IpAddr) -> Result<Self, ControlPlaneConfigError> {
        if !is_strict_loopback(bind) {
            return Err(ControlPlaneConfigError::InsecureOnNonLoopback(bind));
        }
        Ok(Self {
            mode: Mode::UnauthenticatedLoopback,
        })
    }

    /// The policy for a data port bound to `bind`, from
    /// `MOCKFORGE_MANAGEMENT_TOKEN` and `MOCKFORGE_INSECURE_MANAGEMENT`.
    pub fn from_env(
        bind: IpAddr,
        admin: Option<AdminCredential>,
    ) -> Result<Self, ControlPlaneConfigError> {
        Self::from_values(
            bind,
            std::env::var(MANAGEMENT_TOKEN_ENV).ok().as_deref(),
            std::env::var(INSECURE_MANAGEMENT_ENV).ok().as_deref(),
            admin,
        )
    }

    /// [`Self::from_env`] with the variables passed in.
    ///
    /// A configured credential always wins over the opt-out, and the opt-out
    /// on a non-loopback bind is an error even when a credential is set, so
    /// a misconfigured deployment fails loudly.
    pub fn from_values(
        bind: IpAddr,
        token: Option<&str>,
        insecure: Option<&str>,
        admin: Option<AdminCredential>,
    ) -> Result<Self, ControlPlaneConfigError> {
        let insecure = insecure.is_some_and(is_truthy);
        if insecure && !is_strict_loopback(bind) {
            return Err(ControlPlaneConfigError::InsecureOnNonLoopback(bind));
        }
        let token = token.filter(|t| !t.is_empty());
        let policy = match token {
            Some(token) => Self::with_token(token)?.with_admin(admin),
            None if admin.is_some() => Self::locked().with_admin(admin),
            None if insecure => Self::unauthenticated_loopback(bind)?,
            None => Self::locked(),
        };
        Ok(policy)
    }

    /// The policy [`super::protect`] uses when [`super::install`] was not applied: the
    /// environment's token if it is valid, otherwise locked.
    pub fn fallback_from_env() -> Self {
        std::env::var(MANAGEMENT_TOKEN_ENV)
            .ok()
            .and_then(|token| Self::with_token(token).ok())
            .unwrap_or_else(Self::locked)
    }

    /// Whether no credential can open the control plane
    pub fn is_locked(&self) -> bool {
        matches!(
            self.mode,
            Mode::Credentials {
                token: None,
                admin: None
            }
        )
    }

    /// Whether the control plane is open without a credential
    pub fn is_unauthenticated(&self) -> bool {
        matches!(self.mode, Mode::UnauthenticatedLoopback)
    }
}

fn is_truthy(value: &str) -> bool {
    matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
}

/// `127.0.0.0/8` or `::1`. IPv4-mapped IPv6 (`::ffff:127.0.0.1`) and
/// wildcard addresses are not loopback for this purpose.
pub fn is_strict_loopback(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback(),
        IpAddr::V6(v6) => v6.is_loopback(),
    }
}
