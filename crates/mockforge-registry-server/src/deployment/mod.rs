//! Deployment orchestrator for hosted mocks
//!
//! Handles actual deployment of mock services to cloud platforms (Fly.io, Render, Railway)

pub mod flyio;
pub mod health_check;
pub mod metrics;
pub mod orchestrator;
pub mod router;

pub use health_check::HealthCheckWorker;
pub use metrics::MetricsCollector;
pub use orchestrator::DeploymentOrchestrator;
pub use router::MultitenantRouter;
