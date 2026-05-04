//! Database models

pub mod api_token;
pub mod attestation;
pub mod audit_log;
pub mod capture;
pub mod chaos;
pub mod cloud_fixture;
pub mod cloud_proxy;
pub mod cloud_service;
pub mod cloud_workspace;
pub mod contract_verification;
pub mod feature_usage;
pub mod federation;
pub mod federation_scenario_activation;
pub mod flow;
pub mod hosted_mock;
pub mod incident;
pub mod learning;
pub mod login_attempt;
pub mod mock_environment;
pub mod notification_channel;
pub mod observability_query;
pub mod org_template;
pub mod organization;
pub mod osv;
pub mod plugin;
pub mod project;
pub mod review;
pub mod routing_rule;
pub mod saml_assertion;
pub mod scenario;
pub mod scenario_promotion;
pub mod scenario_review;
pub mod scenario_star;
pub mod settings;
pub mod showcase;
pub mod snapshot;
pub mod sso;
pub mod subscription;
pub mod suspicious_activity;
pub mod template;
pub mod template_review;
pub mod template_star;
pub mod test_execution;
pub mod test_run;
pub mod tunnel;
pub mod user;
pub mod verification_token;
pub mod waitlist;
pub mod workspace_environment;
pub mod workspace_folder;
pub mod workspace_request;

pub use verification_token::VerificationToken;

pub use api_token::{ApiToken, TokenScope};
pub use attestation::{
    verify_sbom_attestation, SbomAttestationInput, SbomVerifyOutcome, UserPublicKey,
    UserPublicKeyWithUsage,
};
#[cfg(feature = "postgres")]
pub use audit_log::record_audit_event;
pub use audit_log::AuditEventType;
pub use capture::{CaptureSession, CloneModel};
pub use chaos::{ChaosCampaign, ChaosCampaignReport, ResiliencePattern};
pub use cloud_proxy::{
    generate_session_token, CloudProxyCapture, CloudProxySession, DEFAULT_SESSION_TTL_HOURS,
    MAX_SESSION_TTL_HOURS, PROXY_BODY_MAX_BYTES,
};
pub use cloud_workspace::Workspace as CloudWorkspace;
pub use contract_verification::{
    ContractDiffFinding, ContractDiffRun, FitnessFunction, MonitoredService, VerificationSuite,
};
pub use federation::Federation;
pub use federation_scenario_activation::{
    FederationScenarioActivation, FederationScenarioActivationStatus, PerServiceActivationState,
};
pub use flow::{Flow, FlowVersion};
pub use hosted_mock::{
    protocols_allowed_on_plan, DeploymentStatus, HealthStatus, HostedMock, Protocol,
};
pub use incident::{Incident, IncidentEvent};
pub use learning::{LearningLesson, LearningProgress, LearningRecipe, LearningTrack};
pub use notification_channel::NotificationChannel;
pub use observability_query::{ObservabilityDashboard, ObservabilitySavedQuery};
pub use org_template::OrgTemplate;
pub use organization::{OrgMember, OrgRole, Organization, Plan};
pub use osv::{OsvAdvisory, OsvAffected, OsvImportRecord, OsvMatch, OsvPackage, OsvSeverity};
pub use plugin::{PendingScanJob, Plugin, PluginSecurityScan, PluginVersion};
pub use review::Review;
pub use routing_rule::RoutingRule;
pub use saml_assertion::SAMLAssertionId;
pub use scenario::Scenario;
pub use scenario_promotion::{PromotionStatus, ScenarioEnvironmentVersion, ScenarioPromotion};
pub use settings::{BYOKConfig, OrgAiSettings, OrgSetting};
pub use showcase::ShowcaseEntry;
pub use snapshot::Snapshot;
pub use sso::{SSOConfiguration, SSOSession};
pub use subscription::{Subscription, SubscriptionStatus, UsageAlert, UsageCounter};
pub use user::User;

// Re-export deployment-related models for convenience
pub use feature_usage::{FeatureType, FeatureUsage};
pub use hosted_mock::{DeploymentLog, DeploymentMetrics};
#[cfg(feature = "postgres")]
pub use suspicious_activity::record_suspicious_activity;
pub use suspicious_activity::{SuspiciousActivity, SuspiciousActivityType};

// Re-export types needed by handler modules
pub use audit_log::AuditLog;
pub use project::Project;
pub use scenario::ScenarioVersion;
pub use scenario_review::ScenarioReview;
#[cfg(feature = "postgres")]
pub use scenario_star::ScenarioStar;
pub use settings::UserSetting;
pub use template::{Template, TemplateCategory, TemplateVersion};
pub use template_review::TemplateReview;
#[cfg(feature = "postgres")]
pub use template_star::TemplateStar;
pub use test_execution::{TestSchedule, TestSuite};
pub use test_run::TestRun;
pub use tunnel::TunnelReservation;
