//! Global risk assessment engine
//!
//! Similar to other global services, this provides a global singleton
//! for the risk assessment engine that can be accessed throughout the application.

use crate::security::risk_assessment::RiskAssessmentEngine;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

#[allow(clippy::type_complexity)]
static GLOBAL_RISK_ASSESSMENT_ENGINE: Lazy<Arc<RwLock<Option<Arc<RwLock<RiskAssessmentEngine>>>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Initialize the global risk assessment engine
///
/// This should be called once during application startup.
/// Takes an Arc<RwLock<RiskAssessmentEngine>> to share the same instance.
pub async fn init_global_risk_assessment_engine(
    engine: Arc<RwLock<RiskAssessmentEngine>>,
) -> Result<(), crate::Error> {
    let mut global = GLOBAL_RISK_ASSESSMENT_ENGINE.write().await;
    if global.is_some() {
        return Err(crate::Error::Generic(
            "Global risk assessment engine already initialized".to_string(),
        ));
    }
    *global = Some(engine);
    debug!("Global risk assessment engine initialized");
    Ok(())
}

/// Get the global risk assessment engine
///
/// Returns None if the engine has not been initialized.
pub async fn get_global_risk_assessment_engine() -> Option<Arc<RwLock<RiskAssessmentEngine>>> {
    GLOBAL_RISK_ASSESSMENT_ENGINE.read().await.clone()
}

/// Check if the global risk assessment engine is initialized
pub async fn is_risk_assessment_engine_initialized() -> bool {
    GLOBAL_RISK_ASSESSMENT_ENGINE.read().await.is_some()
}
