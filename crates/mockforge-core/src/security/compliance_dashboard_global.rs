//! Global compliance dashboard engine
//!
//! Similar to other global services, this provides a global singleton
//! for the compliance dashboard engine that can be accessed throughout the application.

use crate::security::compliance_dashboard::ComplianceDashboardEngine;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

#[allow(clippy::type_complexity)]
static GLOBAL_COMPLIANCE_DASHBOARD_ENGINE: Lazy<
    Arc<RwLock<Option<Arc<RwLock<ComplianceDashboardEngine>>>>>,
> = Lazy::new(|| Arc::new(RwLock::new(None)));

/// Initialize the global compliance dashboard engine
///
/// This should be called once during application startup.
/// Takes an Arc<RwLock<ComplianceDashboardEngine>> to share the same instance.
pub async fn init_global_compliance_dashboard_engine(
    engine: Arc<RwLock<ComplianceDashboardEngine>>,
) -> Result<(), crate::Error> {
    let mut global = GLOBAL_COMPLIANCE_DASHBOARD_ENGINE.write().await;
    if global.is_some() {
        return Err(crate::Error::Generic(
            "Global compliance dashboard engine already initialized".to_string(),
        ));
    }
    *global = Some(engine);
    debug!("Global compliance dashboard engine initialized");
    Ok(())
}

/// Get the global compliance dashboard engine
///
/// Returns None if the engine has not been initialized.
pub async fn get_global_compliance_dashboard_engine(
) -> Option<Arc<RwLock<ComplianceDashboardEngine>>> {
    GLOBAL_COMPLIANCE_DASHBOARD_ENGINE.read().await.clone()
}

/// Check if the global compliance dashboard engine is initialized
pub async fn is_compliance_dashboard_engine_initialized() -> bool {
    GLOBAL_COMPLIANCE_DASHBOARD_ENGINE.read().await.is_some()
}
