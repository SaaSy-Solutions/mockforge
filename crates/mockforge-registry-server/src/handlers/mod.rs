//! API handlers

pub mod admin;
pub mod analytics;
pub mod auth;
pub mod billing;
pub mod faq;
pub mod health;
pub mod hosted_mocks;
pub mod legal;
pub mod oauth;
pub mod org_templates;
pub mod organization_settings;
pub mod organizations;
pub mod password_reset;
pub mod pillar_analytics;
pub mod plugins;
pub mod reviews;
pub mod scenario_promotions;
pub mod sso;
pub mod stats;
pub mod status;
pub mod support;
pub mod tokens;
pub mod two_factor;
pub mod usage;
pub mod verification;

// Marketplace, settings, security, GDPR, and audit handlers
pub mod audit;
pub mod gdpr;
pub mod scenario_reviews;
pub mod scenarios;
pub mod security;
pub mod settings;
pub mod template_reviews;
pub mod templates;
pub mod token_rotation;
