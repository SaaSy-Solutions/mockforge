//! Screen trait and registry — each screen owns its own state and renders
//! into the main content area.

pub mod analytics;
pub mod audit;
pub mod behavioral_cloning;
pub mod chains;
pub mod chaos;
pub mod config;
pub mod contract_diff;
pub mod dashboard;
pub mod federation;
pub mod fixtures;
pub mod health;
pub mod import;
pub mod logs;
pub mod metrics;
pub mod plugins;
pub mod recorder;
pub mod routes;
pub mod smoke_tests;
pub mod time_travel;
pub mod verification;
pub mod workspaces;
pub mod world_state;

use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::api::client::MockForgeClient;
use crate::event::Event;
use tokio::sync::mpsc;

/// Every screen implements this trait.
pub trait Screen: Send {
    /// Display name shown in the tab bar.
    fn title(&self) -> &str;

    /// Handle a key event. Return `true` if the event was consumed.
    fn handle_key(&mut self, key: KeyEvent) -> bool;

    /// Render into the given area.
    fn render(&self, frame: &mut Frame, area: Rect);

    /// Called on tick to refresh data if needed. The screen can spawn
    /// background fetches via the event sender.
    fn tick(&mut self, client: &MockForgeClient, tx: &mpsc::UnboundedSender<Event>);

    /// Ingest an event payload pushed by a background data fetcher.
    fn on_data(&mut self, payload: &str);

    /// Ingest an API error for this screen.
    fn on_error(&mut self, message: &str);

    /// Hint text for the status bar (screen-specific key hints).
    fn status_hint(&self) -> &str {
        ""
    }

    /// Return the current error message, if any. Used by the app to render
    /// a persistent error banner while still showing stale data.
    fn error(&self) -> Option<&str> {
        None
    }

    /// Reset internal fetch timer so data is re-fetched on the next tick.
    fn force_refresh(&mut self) {}

    /// Push a single log line (only meaningful for the Logs screen).
    fn push_log_line(&mut self, _line: String) {}
}

/// Screen identifiers used for tab ordering and data routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenId {
    Dashboard,
    Logs,
    Routes,
    Metrics,
    Config,
    Chaos,
    Workspaces,
    Plugins,
    Fixtures,
    Health,
    SmokeTests,
    TimeTravel,
    Chains,
    Verification,
    Analytics,
    Recorder,
    Import,
    Audit,
    WorldState,
    ContractDiff,
    Federation,
    BehavioralCloning,
}

impl ScreenId {
    /// All screens in tab order.
    pub const ALL: &[Self] = &[
        Self::Dashboard,
        Self::Logs,
        Self::Routes,
        Self::Metrics,
        Self::Config,
        Self::Chaos,
        Self::Workspaces,
        Self::Plugins,
        Self::Fixtures,
        Self::Health,
        Self::SmokeTests,
        Self::TimeTravel,
        Self::Chains,
        Self::Verification,
        Self::Analytics,
        Self::Recorder,
        Self::Import,
        Self::Audit,
        Self::WorldState,
        Self::ContractDiff,
        Self::Federation,
        Self::BehavioralCloning,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Logs => "Logs",
            Self::Routes => "Routes",
            Self::Metrics => "Metrics",
            Self::Config => "Config",
            Self::Chaos => "Chaos",
            Self::Workspaces => "Workspaces",
            Self::Plugins => "Plugins",
            Self::Fixtures => "Fixtures",
            Self::Health => "Health",
            Self::SmokeTests => "Smoke Tests",
            Self::TimeTravel => "Time Travel",
            Self::Chains => "Chains",
            Self::Verification => "Verification",
            Self::Analytics => "Analytics",
            Self::Recorder => "Recorder",
            Self::Import => "Import",
            Self::Audit => "Audit",
            Self::WorldState => "World State",
            Self::ContractDiff => "Contract Diff",
            Self::Federation => "Federation",
            Self::BehavioralCloning => "VBR",
        }
    }

    pub fn data_key(self) -> &'static str {
        match self {
            Self::Dashboard => "dashboard",
            Self::Logs => "logs",
            Self::Routes => "routes",
            Self::Metrics => "metrics",
            Self::Config => "config",
            Self::Chaos => "chaos",
            Self::Workspaces => "workspaces",
            Self::Plugins => "plugins",
            Self::Fixtures => "fixtures",
            Self::Health => "health",
            Self::SmokeTests => "smoke_tests",
            Self::TimeTravel => "time_travel",
            Self::Chains => "chains",
            Self::Verification => "verification",
            Self::Analytics => "analytics",
            Self::Recorder => "recorder",
            Self::Import => "import",
            Self::Audit => "audit",
            Self::WorldState => "world_state",
            Self::ContractDiff => "contract_diff",
            Self::Federation => "federation",
            Self::BehavioralCloning => "behavioral_cloning",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_has_correct_count() {
        // Count the enum variants: 22 total
        assert_eq!(ScreenId::ALL.len(), 22);
    }

    #[test]
    fn all_labels_are_non_empty() {
        for screen_id in ScreenId::ALL {
            let label = screen_id.label();
            assert!(!label.is_empty(), "label() returned empty string for {screen_id:?}");
        }
    }

    #[test]
    fn all_data_keys_are_non_empty() {
        for screen_id in ScreenId::ALL {
            let key = screen_id.data_key();
            assert!(!key.is_empty(), "data_key() returned empty string for {screen_id:?}");
        }
    }

    #[test]
    fn all_data_keys_are_snake_case() {
        for screen_id in ScreenId::ALL {
            let key = screen_id.data_key();
            assert!(
                !key.contains(' ') && !key.contains('-'),
                "data_key() for {screen_id:?} contains spaces or hyphens: {key}"
            );
            assert_eq!(
                key,
                key.to_lowercase(),
                "data_key() for {screen_id:?} is not lowercase: {key}"
            );
        }
    }

    #[test]
    fn all_data_keys_are_unique() {
        let keys: Vec<&str> = ScreenId::ALL.iter().map(|s| s.data_key()).collect();
        let mut deduped = keys.clone();
        deduped.sort_unstable();
        deduped.dedup();
        assert_eq!(keys.len(), deduped.len(), "Duplicate data_key() values found");
    }

    #[test]
    fn all_labels_and_data_keys_consistent_count() {
        let label_count = ScreenId::ALL.iter().filter(|s| !s.label().is_empty()).count();
        let key_count = ScreenId::ALL.iter().filter(|s| !s.data_key().is_empty()).count();
        assert_eq!(label_count, key_count);
        assert_eq!(label_count, ScreenId::ALL.len());
    }

    #[test]
    fn specific_labels() {
        assert_eq!(ScreenId::Dashboard.label(), "Dashboard");
        assert_eq!(ScreenId::Logs.label(), "Logs");
        assert_eq!(ScreenId::SmokeTests.label(), "Smoke Tests");
        assert_eq!(ScreenId::TimeTravel.label(), "Time Travel");
        assert_eq!(ScreenId::WorldState.label(), "World State");
        assert_eq!(ScreenId::ContractDiff.label(), "Contract Diff");
        assert_eq!(ScreenId::BehavioralCloning.label(), "VBR");
    }

    #[test]
    fn specific_data_keys() {
        assert_eq!(ScreenId::Dashboard.data_key(), "dashboard");
        assert_eq!(ScreenId::SmokeTests.data_key(), "smoke_tests");
        assert_eq!(ScreenId::TimeTravel.data_key(), "time_travel");
        assert_eq!(ScreenId::WorldState.data_key(), "world_state");
        assert_eq!(ScreenId::ContractDiff.data_key(), "contract_diff");
        assert_eq!(ScreenId::BehavioralCloning.data_key(), "behavioral_cloning");
    }

    #[test]
    fn screen_id_equality() {
        assert_eq!(ScreenId::Dashboard, ScreenId::Dashboard);
        assert_ne!(ScreenId::Dashboard, ScreenId::Logs);
    }

    #[test]
    fn screen_id_clone() {
        let id = ScreenId::Dashboard;
        let cloned = id;
        assert_eq!(id, cloned);
    }
}
