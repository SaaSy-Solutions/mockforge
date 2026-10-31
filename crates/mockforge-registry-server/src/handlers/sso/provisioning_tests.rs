use super::*;
use crate::{
    models::OrgRole,
    sso_domain::DomainOwnershipVerifier,
    store::{RegistryStore, SqliteRegistryStore},
};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Ownership {
    org_id: uuid::Uuid,
    email: String,
    allowed: bool,
    calls: AtomicUsize,
}

#[async_trait::async_trait]
impl DomainOwnershipVerifier for Ownership {
    async fn email_domain_verified(&self, org_id: uuid::Uuid, email: &str) -> bool {
        assert_eq!(org_id, self.org_id);
        assert_eq!(email, self.email);
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.allowed
    }
}

async fn fixture() -> (SqliteRegistryStore, Organization, User) {
    let store = SqliteRegistryStore::connect("sqlite::memory:").await.unwrap();
    let owner = store.create_user("owner", "owner@tenant.example", "fixture").await.unwrap();
    let org = store
        .create_organization("Tenant", "tenant", owner.id, Plan::Team)
        .await
        .unwrap();
    let member = store.create_user("victim", "victim@foreign.example", "fixture").await.unwrap();
    store.create_org_member(org.id, member.id, OrgRole::Member).await.unwrap();
    (store, org, member)
}

fn ownership(org: &Organization, email: &str, allowed: bool) -> Ownership {
    Ownership {
        org_id: org.id,
        email: email.into(),
        allowed,
        calls: AtomicUsize::new(0),
    }
}

#[tokio::test]
async fn existing_foreign_domain_member_cannot_bypass_shared_sso_gate() {
    let (store, org, member) = fixture().await;
    let verifier = ownership(&org, &member.email, false);
    let result = provision_verified_sso_user(&store, &verifier, &org, &member.email, None).await;
    assert!(
        matches!(result, Err(ApiError::InvalidRequest(message)) if message.contains("has not verified ownership"))
    );
    assert_eq!(verifier.calls.load(Ordering::SeqCst), 1);
    assert_eq!(store.find_user_by_email(&member.email).await.unwrap().unwrap().id, member.id);
    assert_eq!(store.find_org_member(org.id, member.id).await.unwrap().unwrap().role, "member");
}

#[tokio::test]
async fn rejected_domain_is_checked_before_even_an_unavailable_user_store() {
    let (_, org, member) = fixture().await;
    let pool = sqlx::sqlite::SqlitePoolOptions::new().connect("sqlite::memory:").await.unwrap();
    pool.close().await;
    let store = SqliteRegistryStore::new(pool);
    let verifier = ownership(&org, &member.email, false);
    let result = provision_verified_sso_user(&store, &verifier, &org, &member.email, None).await;
    // A lookup or mutation against the closed pool would return a storage error.
    assert!(
        matches!(result, Err(ApiError::InvalidRequest(message)) if message.contains("has not verified ownership"))
    );
    assert_eq!(verifier.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn verified_existing_member_can_login_without_duplicate_membership() {
    let (store, org, member) = fixture().await;
    let verifier = ownership(&org, &member.email, true);
    let before = store.find_org_member(org.id, member.id).await.unwrap().unwrap();
    let result = provision_verified_sso_user(&store, &verifier, &org, &member.email, None)
        .await
        .unwrap();
    assert_eq!(result.user.id, member.id);
    assert!(!result.jit_created);
    assert_eq!(verifier.calls.load(Ordering::SeqCst), 1);
    assert_eq!(store.find_org_member(org.id, member.id).await.unwrap().unwrap().id, before.id);
}

#[tokio::test]
async fn verified_domain_can_attach_or_create_users() {
    let (store, org, _) = fixture().await;
    let existing = store.create_user("attach", "attach@tenant.example", "fixture").await.unwrap();
    for (email, created) in [
        (existing.email.as_str(), false),
        ("new@tenant.example", true),
    ] {
        let verifier = ownership(&org, email, true);
        let result =
            provision_verified_sso_user(&store, &verifier, &org, email, None).await.unwrap();
        assert_eq!(result.jit_created, created);
        assert_eq!(result.user.email, email);
        assert!(store.find_org_member(org.id, result.user.id).await.unwrap().is_some());
        assert_eq!(verifier.calls.load(Ordering::SeqCst), 1);
    }
}
