//! Compatibility state and engine adapter for internal risk simulation routes.
//!
//! Route behavior lives in intelligence. Public state fields and auth types stay
//! available at their original HTTP paths. No admin UI consumes these endpoints.

use crate::auth::risk_engine::{RiskAssessment, RiskEngine};
use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};

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

#[async_trait]
impl RiskSimulationEngine for RiskEngine {
    async fn set_simulated_risk(&self, user_id: String, score: Option<f64>) {
        RiskEngine::set_simulated_risk(self, user_id, score).await;
    }

    async fn set_simulated_factors(&self, user_id: String, factors: HashMap<String, f64>) {
        RiskEngine::set_simulated_factors(self, user_id, factors).await;
    }

    async fn clear_simulated_risk(&self, user_id: &str) {
        RiskEngine::clear_simulated_risk(self, user_id).await;
    }

    async fn assess_risk(&self, user_id: &str, factors: &HashMap<String, f64>) -> RiskAssessment {
        RiskEngine::assess_risk(self, user_id, factors).await
    }
}
