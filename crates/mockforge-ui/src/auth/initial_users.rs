//! Startup creation of the operator-configured admin account.
//!
//! Environment policy and the one-time bcrypt worker live here so `UserStore`
//! only assembles its authentication services.

use super::{hash, User, UserRole, DEFAULT_COST};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

/// A user created at startup: username, password, role, email
type SeedUser = (String, String, UserRole, Option<String>);

/// The operator-configured admin login, when `MOCKFORGE_ADMIN_PASSWORD` is
/// long enough (`crate::admin_access`)
fn configured_admin_login(env: impl Fn(&str) -> Option<String>) -> Option<(String, String)> {
    use crate::admin_access::{ADMIN_PASSWORD_ENV, ADMIN_USERNAME_ENV, MIN_ADMIN_PASSWORD_LEN};
    let password = env(ADMIN_PASSWORD_ENV).filter(|p| p.len() >= MIN_ADMIN_PASSWORD_LEN)?;
    let username = env(ADMIN_USERNAME_ENV)
        .map(|u| u.trim().to_string())
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| "admin".to_string());
    Some((username, password))
}

/// The users to create at startup, read from `env`.
///
/// Only an operator-configured admin login is ever created. There are no
/// built-in users with published passwords, whatever `ENVIRONMENT`,
/// `JWT_SECRET` or the dev-auth flags say (#1016). Without a configured
/// login, a loopback admin server allows anonymous reads and any other bind
/// refuses to start (`crate::admin_access::policy_for_bind`).
fn seed_users_from(env: impl Fn(&str) -> Option<String>) -> Vec<SeedUser> {
    configured_admin_login(env)
        .map(|(username, password)| (username, password, UserRole::Admin, None))
        .into_iter()
        .collect()
}

pub(super) fn initialize(users: &Arc<RwLock<HashMap<String, User>>>) {
    let seed_users = seed_users_from(|name| std::env::var(name).ok());
    if !seed_users.is_empty() {
        let users = Arc::clone(users);
        // Hashing the configured password is CPU-bound.
        // Keep it off Tokio's workers: runtime shutdown otherwise waits for
        // an uninterruptible async task even after SIGTERM has been handled.
        // The detached thread exits after this one-time initialization.
        if let Err(error) = std::thread::Builder::new()
            .name("mockforge-admin-auth-seed".to_string())
            .spawn(move || {
                let mut users = users.blocking_write();
                for (username, password, role, email) in seed_users {
                    if let Ok(password_hash) = hash(&password, DEFAULT_COST) {
                        let user = User {
                            id: format!("{}-001", username),
                            username: username.clone(),
                            password_hash,
                            role,
                            email,
                        };
                        users.insert(username, user);
                    } else {
                        tracing::error!("Failed to hash password for user: {}", username);
                    }
                }
            })
        {
            tracing::error!(%error, "Failed to start admin user initialization");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_user_is_seeded_under_any_environment_without_a_configured_login() {
        let environments = [
            None,
            Some("development"),
            Some("dev"),
            Some("local"),
            Some("production"),
        ];
        let jwt_secrets = [None, Some("an-operator-jwt-secret-of-at-least-32-chars")];
        let dev_flags = [None, Some("true")];
        let short_passwords = [None, Some(""), Some("admin123")];
        let mut cases = 0;
        for environment in environments {
            for jwt_secret in jwt_secrets {
                for dev_auth in dev_flags {
                    for password in short_passwords {
                        let env = |name: &str| {
                            match name {
                                "ENVIRONMENT" => environment,
                                "JWT_SECRET" => jwt_secret,
                                "MOCKFORGE_ENABLE_DEV_AUTH" => dev_auth,
                                crate::admin_access::ADMIN_PASSWORD_ENV => password,
                                _ => None,
                            }
                            .map(str::to_string)
                        };
                        assert!(
                            seed_users_from(env).is_empty(),
                            "seeded a user for ENVIRONMENT={environment:?} \
                             JWT_SECRET={jwt_secret:?} dev_auth={dev_auth:?} \
                             password={password:?}"
                        );
                        cases += 1;
                    }
                }
            }
        }
        assert_eq!(cases, 5 * 2 * 2 * 3);
    }

    #[test]
    fn only_the_configured_admin_login_is_seeded() {
        for environment in [None, Some("development"), Some("production")] {
            for jwt_secret in [None, Some("an-operator-jwt-secret-of-at-least-32-chars")] {
                let env = |name: &str| {
                    match name {
                        "ENVIRONMENT" => environment,
                        "JWT_SECRET" => jwt_secret,
                        crate::admin_access::ADMIN_PASSWORD_ENV => Some("operator-chosen-pw"),
                        crate::admin_access::ADMIN_USERNAME_ENV => Some(" ops "),
                        _ => None,
                    }
                    .map(str::to_string)
                };
                let seeded = seed_users_from(env);
                assert_eq!(seeded.len(), 1);
                let (username, password, role, email) = &seeded[0];
                assert_eq!((username.as_str(), password.as_str()), ("ops", "operator-chosen-pw"));
                assert!(matches!(role, UserRole::Admin));
                assert_eq!(email, &None);
            }
        }
    }
}
