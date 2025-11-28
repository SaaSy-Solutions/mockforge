//! Forecasting service for API change predictions
//!
//! This service orchestrates forecasting operations, including querying
//! historical incidents, generating forecasts, and managing forecast cache.

use chrono::{DateTime, Duration, Utc};
use mockforge_core::contract_drift::forecasting::{
    ChangeForecast, Forecaster, ForecastingConfig,
};
use mockforge_core::incidents::types::DriftIncident;
use std::sync::Arc;

/// Forecasting service
#[derive(Clone)]
pub struct ForecastingService {
    /// Forecaster engine
    forecaster: Arc<Forecaster>,
    /// Configuration
    config: ForecastingConfig,
}

impl ForecastingService {
    /// Create a new forecasting service
    pub fn new(config: ForecastingConfig) -> Self {
        let forecaster = Arc::new(Forecaster::new(config.clone()));
        Self { forecaster, config }
    }

    /// Generate forecast for a service or endpoint
    ///
    /// This method queries historical incidents and generates a forecast.
    /// In a full implementation, this would query the database for incidents.
    pub async fn generate_forecast(
        &self,
        incidents: &[DriftIncident],
        workspace_id: Option<String>,
        service_id: Option<String>,
        service_name: Option<String>,
        endpoint: String,
        method: String,
        forecast_window_days: u32,
    ) -> Option<ChangeForecast> {
        self.forecaster.generate_forecast(
            incidents,
            workspace_id,
            service_id,
            service_name,
            endpoint,
            method,
            forecast_window_days,
        )
    }

    /// Check if a forecast is stale and needs refresh
    pub fn is_forecast_stale(&self, forecast: &ChangeForecast) -> bool {
        Utc::now() >= forecast.expires_at
    }

    /// Get default expiration time for forecasts
    pub fn default_expiration(&self) -> DateTime<Utc> {
        Utc::now() + Duration::hours(self.config.default_expiration_hours as i64)
    }
}

impl Default for ForecastingService {
    fn default() -> Self {
        Self::new(ForecastingConfig::default())
    }
}
