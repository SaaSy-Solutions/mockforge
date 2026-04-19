//! Database models

pub mod api_token;
pub mod attestation;
pub mod audit_log;
pub mod cloud_fixture;
pub mod cloud_service;
pub mod cloud_workspace;
pub mod feature_usage;
pub mod federation;
pub mod hosted_mock;
pub mod login_attempt;
pub mod mock_environment;
pub mod org_template;
pub mod organization;
pub mod osv;
pub mod plugin;
pub mod project;
pub mod review;
pub mod saml_assertion;
pub mod scenario;
pub mod scenario_promotion;
pub mod scenario_review;
pub mod settings;
pub mod sso;
pub mod subscription;
pub mod suspicious_activity;
pub mod template;
pub mod template_review;
pub mod template_star;
pub mod user;
pub mod verification_token;
pub mod waitlist;

pub use verification_token::VerificationToken;

pub use api_token::{ApiToken, TokenScope};
pub use attestation::{
    verify_sbom_attestation, SbomAttestationInput, SbomVerifyOutcome, UserPublicKey,
};
#[cfg(feature = "postgres")]
pub use audit_log::record_audit_event;
pub use audit_log::AuditEventType;
pub use cloud_workspace::Workspace as CloudWorkspace;
pub use federation::Federation;
pub use hosted_mock::{DeploymentStatus, HealthStatus, HostedMock};
pub use org_template::OrgTemplate;
pub use organization::{OrgMember, OrgRole, Organization, Plan};
pub use osv::{OsvAdvisory, OsvAffected, OsvImportRecord, OsvMatch, OsvPackage, OsvSeverity};
pub use plugin::{PendingScanJob, Plugin, PluginSecurityScan, PluginVersion};
pub use review::Review;
pub use saml_assertion::SAMLAssertionId;
pub use scenario::Scenario;
pub use scenario_promotion::{PromotionStatus, ScenarioEnvironmentVersion, ScenarioPromotion};
pub use settings::{BYOKConfig, OrgAiSettings, OrgSetting};
pub use sso::{SSOConfiguration, SSOSession};
pub use subscription::{Subscription, SubscriptionStatus, UsageCounter};
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
pub use settings::UserSetting;
pub use template::{Template, TemplateCategory, TemplateVersion};
pub use template_review::TemplateReview;
#[cfg(feature = "postgres")]
pub use template_star::TemplateStar;
