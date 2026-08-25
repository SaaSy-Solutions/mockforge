//! Contract tests: privileged governance handlers MUST reject unauthenticated
//! requests.
//!
//! Regression coverage for the removal of `extract_user_id_with_fallback`:
//! previously these endpoints silently attributed actions to the synthetic
//! user `00000000-0000-0000-0000-000000000001` when no claims were present,
//! letting unauthenticated callers approve changes, revoke access, and alter
//! permissions. Each test drives a representative mutating handler directly
//! with no claims and asserts `StatusCode::UNAUTHORIZED`.

#![cfg(test)]

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use super::access_review::{approve_access, AccessReviewState, ApproveAccessRequest};
use super::change_management::{approve_change, ApproveChangeRequest, ChangeManagementState};
use super::privileged_access::{approve_manager, PrivilegedAccessState};
use super::risk_assessment::{review_risk, RiskAssessmentState};

use mockforge_core::security::access_review::{
    AccessReviewConfig, AccessReviewEngine, ApiTokenInfo, PrivilegedAccessInfo, UserAccessInfo,
};
use mockforge_core::security::access_review_service::{AccessReviewService, UserDataProvider};
use mockforge_core::security::change_management::{ChangeManagementConfig, ChangeManagementEngine};
use mockforge_core::security::privileged_access::{
    PrivilegedAccessConfig, PrivilegedAccessManager,
};
use mockforge_core::security::risk_assessment::{RiskAssessmentConfig, RiskAssessmentEngine};
use mockforge_core::Error;

use std::sync::Arc;
use tokio::sync::RwLock;

/// Empty provider — the auth check fires before any data access.
struct NoUsers;

#[async_trait::async_trait]
impl UserDataProvider for NoUsers {
    async fn get_all_users(&self) -> Result<Vec<UserAccessInfo>, Error> {
        Ok(vec![])
    }
    async fn get_privileged_users(&self) -> Result<Vec<PrivilegedAccessInfo>, Error> {
        Ok(vec![])
    }
    async fn get_api_tokens(&self) -> Result<Vec<ApiTokenInfo>, Error> {
        Ok(vec![])
    }
    async fn get_user(&self, _user_id: Uuid) -> Result<Option<UserAccessInfo>, Error> {
        Ok(None)
    }
    async fn get_last_login(
        &self,
        _user_id: Uuid,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>, Error> {
        Ok(None)
    }
    async fn revoke_user_access(&self, _user_id: Uuid, _reason: String) -> Result<(), Error> {
        Err(Error::feature_disabled("test stub"))
    }
    async fn update_user_permissions(
        &self,
        _user_id: Uuid,
        _roles: Vec<String>,
        _permissions: Vec<String>,
    ) -> Result<(), Error> {
        Err(Error::feature_disabled("test stub"))
    }
}

#[tokio::test]
async fn access_review_approve_requires_auth() {
    let service = AccessReviewService::new(
        AccessReviewEngine::new(AccessReviewConfig::default()),
        Box::new(NoUsers),
    );
    let state = AccessReviewState {
        service: Arc::new(RwLock::new(service)),
    };
    let request = ApproveAccessRequest {
        user_id: Uuid::new_v4(),
        approved: true,
        justification: None,
    };

    let result = approve_access(State(state), Path("rev-1".to_string()), None, Json(request)).await;

    assert_eq!(result.unwrap_err(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn change_management_approve_requires_auth() {
    let engine = ChangeManagementEngine::new(ChangeManagementConfig::default());
    let state = ChangeManagementState {
        engine: Arc::new(RwLock::new(engine)),
    };
    let request = ApproveChangeRequest {
        approved: true,
        comments: None,
        conditions: None,
        reason: None,
    };

    let result = approve_change(State(state), Path("chg-1".to_string()), None, Json(request)).await;

    assert_eq!(result.unwrap_err(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn privileged_access_approve_requires_auth() {
    let manager = PrivilegedAccessManager::new(PrivilegedAccessConfig::default(), None, None);
    let state = PrivilegedAccessState {
        manager: Arc::new(RwLock::new(manager)),
    };

    let result = approve_manager(State(state), Path(Uuid::new_v4()), None).await;

    assert_eq!(result.unwrap_err(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn risk_assessment_review_requires_auth() {
    let engine = RiskAssessmentEngine::new(RiskAssessmentConfig::default());
    let state = RiskAssessmentState {
        engine: Arc::new(RwLock::new(engine)),
    };

    let result = review_risk(State(state), Path("risk-1".to_string()), None).await;

    assert_eq!(result.unwrap_err(), StatusCode::UNAUTHORIZED);
}
