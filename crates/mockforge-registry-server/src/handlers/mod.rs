//! API handlers

pub mod admin;
pub mod analytics;
pub mod auth;
pub mod billing;
pub mod cloud_dashboard;
pub mod cloud_fixtures;
pub mod cloud_services;
pub mod cloud_workspaces;
pub mod faq;
pub mod federations;
pub mod health;
pub mod hosted_mocks;
pub mod legal;
pub mod oauth;
pub mod org_templates;
pub mod organization_settings;
pub mod organizations;
pub mod otlp;
pub mod password_reset;
pub mod pillar_analytics;
pub mod plugins;
pub mod projects;
pub mod public_keys;
pub mod reviews;
pub mod scenario_promotions;
pub mod sso;
pub mod stats;
pub mod status;
pub mod support;
pub mod tokens;
pub mod two_factor;
pub mod usage;
pub mod users_me;
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
pub mod waitlist;

// Workspace content (environments, variables, folders, requests, import, activate/reorder)
pub mod workspace_encryption;
pub mod workspace_environments;
pub mod workspace_folders;
pub mod workspace_import;
pub mod workspace_ordering;
pub mod workspace_request_execute;
