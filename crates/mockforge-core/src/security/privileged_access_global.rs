//! Global privileged access manager
//!
//! Similar to the SIEM emitter and access review service, this provides a global singleton
//! for the privileged access manager that can be accessed throughout the application.

use crate::security::privileged_access::PrivilegedAccessManager;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

static GLOBAL_PRIVILEGED_ACCESS_MANAGER: Lazy<
    Arc<RwLock<Option<Arc<RwLock<PrivilegedAccessManager>>>>>,
> = Lazy::new(|| Arc::new(RwLock::new(None)));

/// Initialize the global privileged access manager
///
/// This should be called once during application startup.
/// Takes an Arc<RwLock<PrivilegedAccessManager>> to share the same instance.
pub async fn init_global_privileged_access_manager(
    manager: Arc<RwLock<PrivilegedAccessManager>>,
) -> Result<(), crate::Error> {
    let mut global = GLOBAL_PRIVILEGED_ACCESS_MANAGER.write().await;
    if global.is_some() {
        return Err(crate::Error::Generic(
            "Global privileged access manager already initialized".to_string(),
        ));
    }
    *global = Some(manager);
    debug!("Global privileged access manager initialized");
    Ok(())
}

/// Get the global privileged access manager
///
/// Returns None if the manager has not been initialized.
pub async fn get_global_privileged_access_manager() -> Option<Arc<RwLock<PrivilegedAccessManager>>>
{
    GLOBAL_PRIVILEGED_ACCESS_MANAGER.read().await.clone()
}

/// Check if the global privileged access manager is initialized
pub async fn is_privileged_access_manager_initialized() -> bool {
    GLOBAL_PRIVILEGED_ACCESS_MANAGER.read().await.is_some()
}
