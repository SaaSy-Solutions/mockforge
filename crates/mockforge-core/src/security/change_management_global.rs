//! Global change management engine
//!
//! Similar to other global services, this provides a global singleton
//! for the change management engine that can be accessed throughout the application.

use crate::security::change_management::ChangeManagementEngine;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

static GLOBAL_CHANGE_MANAGEMENT_ENGINE: Lazy<
    Arc<RwLock<Option<Arc<RwLock<ChangeManagementEngine>>>>>,
> = Lazy::new(|| Arc::new(RwLock::new(None)));

/// Initialize the global change management engine
///
/// This should be called once during application startup.
/// Takes an Arc<RwLock<ChangeManagementEngine>> to share the same instance.
pub async fn init_global_change_management_engine(
    engine: Arc<RwLock<ChangeManagementEngine>>,
) -> Result<(), crate::Error> {
    let mut global = GLOBAL_CHANGE_MANAGEMENT_ENGINE.write().await;
    if global.is_some() {
        return Err(crate::Error::Generic(
            "Global change management engine already initialized".to_string(),
        ));
    }
    *global = Some(engine);
    debug!("Global change management engine initialized");
    Ok(())
}

/// Get the global change management engine
///
/// Returns None if the engine has not been initialized.
pub async fn get_global_change_management_engine() -> Option<Arc<RwLock<ChangeManagementEngine>>> {
    GLOBAL_CHANGE_MANAGEMENT_ENGINE.read().await.clone()
}

/// Check if the global change management engine is initialized
pub async fn is_change_management_engine_initialized() -> bool {
    GLOBAL_CHANGE_MANAGEMENT_ENGINE.read().await.is_some()
}
