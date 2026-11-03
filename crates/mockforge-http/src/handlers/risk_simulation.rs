//! Compatibility state and engine adapter for internal risk simulation routes.
//!
//! Route behavior lives in intelligence. Public state fields and auth types stay
//! available at their original HTTP paths. No admin UI consumes these endpoints.

use crate::auth::risk_engine::RiskEngine;
use std::sync::Arc;

pub use mockforge_intelligence::handlers::risk_simulation::*;

/// Risk simulation state.
#[derive(Clone)]
pub struct RiskSimulationState {
    /// Existing risk engine shared with the application's authentication code.
    pub risk_engine: Arc<RiskEngine>,
}

impl RiskSimulationStateProvider for RiskSimulationState {
    type Engine = RiskEngine;

    fn risk_engine(&self) -> &Self::Engine {
        &self.risk_engine
    }
}
