//! Builders for audit-log entries that concern a person.
//!
//! `audit_logs` is append-only and hash-chained per org (#872), so a row can
//! never be scrubbed after the fact: every field is inside `canonical_entry`.
//! Whatever lands in `description` or `metadata` stays for the full audit
//! retention window, including after the person's account is erased (#1087).
//!
//! So these builders identify people only by stable ids (`user_id`,
//! `target_user_id`, invitation nonce) and never by email address, username or
//! IdP subject. The one place a raw identifier is the only signal available
//! (a failed login for an address that has no account) stores a keyed
//! pseudonym from [`audit_email_pseudonym`] instead of the address.
//!
//! Every handler that records a person-related audit event goes through here,
//! and `tests/audit_pii_guard.rs` fails the build if a handler passes an email
//! or username to `record_audit_event` directly.

use mockforge_registry_core::models::audit_log::audit_email_pseudonym;
use mockforge_registry_core::models::User;
use serde_json::{json, Value};

/// The free-text `description` and structured `metadata` of one audit row.
#[derive(Debug, Clone, PartialEq)]
pub struct AuditEntry {
    pub description: String,
    pub metadata: Option<Value>,
}

/// `member_added`. The acting user is the row's `user_id`.
pub fn member_added(target: &User, role: &str) -> AuditEntry {
    AuditEntry {
        description: format!("Added member {} with role {}", target.id, role),
        metadata: Some(json!({ "target_user_id": target.id, "role": role })),
    }
}

/// `member_removed`.
pub fn member_removed(target: &User) -> AuditEntry {
    AuditEntry {
        description: format!("Removed member {}", target.id),
        metadata: Some(json!({ "target_user_id": target.id })),
    }
}

/// `member_role_changed`.
pub fn member_role_changed(target: &User, from_role: &str, to_role: &str) -> AuditEntry {
    AuditEntry {
        description: format!(
            "Changed role of member {} from {} to {}",
            target.id, from_role, to_role
        ),
        metadata: Some(json!({
            "target_user_id": target.id,
            "from_role": from_role,
            "to_role": to_role,
        })),
    }
}

/// `invitation_created`. The nonce is the invitation's id; the invitee's
/// address is not recorded (it may never become an account).
pub fn invitation_created(nonce: &str, role: &str) -> AuditEntry {
    AuditEntry {
        description: format!("Created invitation {} ({})", nonce, role),
        metadata: Some(json!({ "nonce": nonce, "role": role })),
    }
}

/// `invitation_revoked`.
pub fn invitation_revoked(nonce: &str, role: &str) -> AuditEntry {
    AuditEntry {
        description: format!("Revoked invitation {} ({})", nonce, role),
        metadata: Some(json!({ "nonce": nonce, "role": role })),
    }
}

/// `invitation_accepted`. The accepting user is also the row's `user_id`.
pub fn invitation_accepted(nonce: &str, user: &User, role: &str) -> AuditEntry {
    AuditEntry {
        description: format!("Invitation {} accepted by user {}", nonce, user.id),
        metadata: Some(json!({ "nonce": nonce, "user_id": user.id, "role": role })),
    }
}

/// `login_failed` for an email with no account. There is no user id to
/// record, so the attempted address is stored as a keyed pseudonym: repeated
/// attempts against the same address correlate, the address itself is not kept.
pub fn login_failed_unknown_user(secret: &str, attempted_email: &str) -> AuditEntry {
    AuditEntry {
        description: "Login failed: unknown email".to_string(),
        metadata: Some(json!({
            "attempted_email_hash": audit_email_pseudonym(attempted_email, secret),
            "reason": "unknown_user",
        })),
    }
}

/// `login_failed` with a wrong password. The user is the row's `user_id`; the
/// pseudonym is kept too so both failure kinds correlate on one key.
pub fn login_failed_bad_password(secret: &str, attempted_email: &str) -> AuditEntry {
    AuditEntry {
        description: "Login failed: incorrect password".to_string(),
        metadata: Some(json!({
            "attempted_email_hash": audit_email_pseudonym(attempted_email, secret),
            "reason": "bad_password",
        })),
    }
}

/// `login_succeeded` via SAML. The IdP `NameID` is not recorded: it is often
/// the email address, and `user_id` already identifies the account.
pub fn saml_login_succeeded(user: &User, jit_created: bool) -> AuditEntry {
    sso_login_succeeded("saml", "SAML", user, jit_created)
}

/// `login_succeeded` via OIDC.
pub fn oidc_login_succeeded(user: &User, jit_created: bool) -> AuditEntry {
    sso_login_succeeded("oidc", "OIDC", user, jit_created)
}

fn sso_login_succeeded(method: &str, label: &str, user: &User, jit_created: bool) -> AuditEntry {
    AuditEntry {
        description: format!("SSO login via {} for user {}", label, user.id),
        metadata: Some(json!({
            "method": method,
            "jit_created": jit_created,
            "user_id": user.id,
        })),
    }
}

/// `password_changed` through the reset-by-email flow.
pub fn password_reset_completed(user: &User) -> AuditEntry {
    AuditEntry {
        description: format!("Password reset completed for user {}", user.id),
        metadata: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    const EMAIL: &str = "Alice.Smith@Example.COM";
    const USERNAME: &str = "alice_smith_1987";
    const SECRET: &str = "test-jwt-secret";

    fn user() -> User {
        User {
            id: Uuid::new_v4(),
            username: USERNAME.to_string(),
            email: EMAIL.to_string(),
            password_hash: "x".to_string(),
            api_token: None,
            is_verified: true,
            is_admin: false,
            two_factor_enabled: false,
            two_factor_secret: None,
            two_factor_backup_codes: None,
            two_factor_verified_at: None,
            email_notifications: true,
            security_alerts: true,
            preferences: json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// The row text the hash chain commits to: description plus compact
    /// metadata, exactly what `canonical_entry` would see.
    fn row_text(entry: &AuditEntry) -> String {
        format!(
            "{}|{}",
            entry.description,
            entry.metadata.as_ref().map(Value::to_string).unwrap_or_default()
        )
    }

    fn assert_no_pii(event: &str, entry: &AuditEntry) {
        let text = row_text(entry);
        let lower = text.to_lowercase();
        assert!(!text.contains('@'), "{event}: audit row contains an email address: {text}");
        assert!(
            !lower.contains(&EMAIL.to_lowercase()),
            "{event}: audit row contains the email: {text}"
        );
        assert!(!lower.contains("alice"), "{event}: audit row contains the username: {text}");
        for key in ["email", "username", "name_id", "attempted_email"] {
            let has_key =
                entry.metadata.as_ref().and_then(|m| m.get(key)).is_some_and(|v| !v.is_null());
            assert!(!has_key, "{event}: metadata carries `{key}`: {text}");
        }
    }

    fn metadata_str<'a>(entry: &'a AuditEntry, key: &str) -> &'a str {
        entry
            .metadata
            .as_ref()
            .and_then(|m| m.get(key))
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("metadata.{key} missing: {}", row_text(entry)))
    }

    #[test]
    fn member_events_record_target_user_id_not_identity() {
        let target = user();
        let id = target.id.to_string();
        for (event, entry) in [
            ("member_added", member_added(&target, "admin")),
            ("member_removed", member_removed(&target)),
            ("member_role_changed", member_role_changed(&target, "member", "admin")),
        ] {
            assert_no_pii(event, &entry);
            assert_eq!(metadata_str(&entry, "target_user_id"), id, "{event}");
        }
        let changed = member_role_changed(&target, "member", "admin");
        assert_eq!(metadata_str(&changed, "from_role"), "member");
        assert_eq!(metadata_str(&changed, "to_role"), "admin");
    }

    #[test]
    fn invitation_events_record_nonce_not_invitee_email() {
        let accepter = user();
        for (event, entry) in [
            ("invitation_created", invitation_created("nonce-1", "member")),
            ("invitation_revoked", invitation_revoked("nonce-1", "member")),
            ("invitation_accepted", invitation_accepted("nonce-1", &accepter, "member")),
        ] {
            assert_no_pii(event, &entry);
            assert_eq!(metadata_str(&entry, "nonce"), "nonce-1", "{event}");
            assert_eq!(metadata_str(&entry, "role"), "member", "{event}");
        }
    }

    #[test]
    fn login_failed_stores_a_keyed_pseudonym_not_the_address() {
        let unknown = login_failed_unknown_user(SECRET, EMAIL);
        let bad_pw = login_failed_bad_password(SECRET, EMAIL);
        assert_no_pii("login_failed/unknown_user", &unknown);
        assert_no_pii("login_failed/bad_password", &bad_pw);

        // Both failure kinds, and case/whitespace variants of the address,
        // correlate on one pseudonym so brute-force forensics still works.
        let hash = metadata_str(&unknown, "attempted_email_hash");
        assert_eq!(hash, metadata_str(&bad_pw, "attempted_email_hash"));
        let variant = login_failed_unknown_user(SECRET, "  alice.smith@example.com ");
        assert_eq!(hash, metadata_str(&variant, "attempted_email_hash"));

        // Keyed: a different server secret yields an unrelated value, so the
        // pseudonym is not a plain hash anyone can recompute from the address.
        let other = login_failed_unknown_user("another-secret", EMAIL);
        assert_ne!(hash, metadata_str(&other, "attempted_email_hash"));
    }

    #[test]
    fn sso_logins_drop_email_and_name_id() {
        let u = user();
        let saml = saml_login_succeeded(&u, true);
        let oidc = oidc_login_succeeded(&u, false);
        assert_no_pii("login_succeeded/saml", &saml);
        assert_no_pii("login_succeeded/oidc", &oidc);
        assert_eq!(metadata_str(&saml, "method"), "saml");
        assert_eq!(metadata_str(&oidc, "method"), "oidc");
        assert_eq!(metadata_str(&saml, "user_id"), u.id.to_string());
        assert_eq!(metadata_str(&oidc, "user_id"), u.id.to_string());
    }

    #[test]
    fn password_reset_identifies_user_by_id() {
        let u = user();
        let entry = password_reset_completed(&u);
        assert_no_pii("password_changed/reset", &entry);
        assert!(entry.description.contains(&u.id.to_string()), "{}", entry.description);
    }
}
