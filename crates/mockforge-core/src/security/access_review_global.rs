//! Global access review service manager
//!
//! Similar to the SIEM emitter, this provides a global singleton for the access review service
//! that can be accessed throughout the application.

use crate::security::access_review_service::AccessReviewService;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

static GLOBAL_ACCESS_REVIEW_SERVICE: Lazy<Arc<RwLock<Option<Arc<RwLock<AccessReviewService>>>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Initialize the global access review service
///
/// This should be called once during application startup.
/// Takes an Arc<RwLock<AccessReviewService>> to share the same instance with the scheduler.
pub async fn init_global_access_review_service(service: Arc<RwLock<AccessReviewService>>) -> Result<(), crate::Error> {
    let mut global = GLOBAL_ACCESS_REVIEW_SERVICE.write().await;
    if global.is_some() {
        return Err(crate::Error::Generic("Global access review service already initialized".to_string()));
    }
    *global = Some(service);
    debug!("Global access review service initialized");
    Ok(())
}

/// Get the global access review service
///
/// Returns None if the service has not been initialized.
pub async fn get_global_access_review_service() -> Option<Arc<RwLock<AccessReviewService>>> {
    GLOBAL_ACCESS_REVIEW_SERVICE.read().await.clone()
}

/// Check if the global access review service is initialized
pub async fn is_access_review_service_initialized() -> bool {
    GLOBAL_ACCESS_REVIEW_SERVICE.read().await.is_some()
}
