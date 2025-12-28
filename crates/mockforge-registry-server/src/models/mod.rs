//! Database models

pub mod api_token;
pub mod audit_log;
pub mod feature_usage;
pub mod hosted_mock;
pub mod login_attempt;
pub mod mock_environment;
pub mod org_template;
pub mod organization;
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
pub mod user;
pub mod verification_token;

pub use verification_token::VerificationToken;

pub use api_token::{ApiToken, TokenScope};
pub use audit_log::{record_audit_event, AuditEventType, AuditLog};
pub use feature_usage::{FeatureType, FeatureUsage};
pub use hosted_mock::{DeploymentStatus, HealthStatus, HostedMock};
pub use mock_environment::{MockEnvironment, MockEnvironmentName};
pub use org_template::OrgTemplate;
pub use organization::{OrgMember, OrgRole, Organization, Plan};
pub use plugin::{Plugin, PluginVersion, PluginWithVersions};
pub use project::Project;
pub use review::Review;
pub use saml_assertion::SAMLAssertionId;
pub use scenario::{Scenario, ScenarioVersion};
pub use scenario_promotion::{PromotionStatus, ScenarioEnvironmentVersion, ScenarioPromotion};
pub use scenario_review::ScenarioReview;
pub use settings::{AiFeatureFlags, BYOKConfig, OrgAiSettings, OrgSetting, UserSetting};
pub use sso::{SSOConfiguration, SSOSession};
pub use subscription::{Subscription, SubscriptionStatus, UsageCounter};
pub use suspicious_activity::{
    record_suspicious_activity, SuspiciousActivity, SuspiciousActivityType,
};
pub use template::{Template, TemplateCategory, TemplateVersion};
pub use template_review::TemplateReview;
pub use user::User;

// Re-export deployment-related models for convenience
pub use hosted_mock::{DeploymentLog, DeploymentMetrics};
