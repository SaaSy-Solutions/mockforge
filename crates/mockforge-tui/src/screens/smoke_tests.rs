//! Smoke test results table screen.

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Rect},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use tokio::sync::mpsc;

use crate::api::client::MockForgeClient;
use crate::api::models::SmokeTestResult;
use crate::event::Event;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::widgets::table::TableState;

const FETCH_INTERVAL: u64 = 30;

pub struct SmokeTestsScreen {
    results: Vec<SmokeTestResult>,
    table: TableState,
    error: Option<String>,
    last_fetch: Option<Instant>,
    pending_run: bool,
    running: bool,
}

impl SmokeTestsScreen {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            table: TableState::new(),
            error: None,
            last_fetch: None,
            pending_run: false,
            running: false,
        }
    }
}

impl Screen for SmokeTestsScreen {
    fn title(&self) -> &str {
        "Smoke Tests"
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('r') => {
                self.pending_run = true;
                true
            }
            _ => self.table.handle_key(key),
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if self.results.is_empty() && self.last_fetch.is_none() {
            let loading = Paragraph::new("Loading smoke tests...").style(Theme::dim()).block(
                Block::default()
                    .title(" Smoke Tests ")
                    .borders(Borders::ALL)
                    .border_style(Theme::dim()),
            );
            frame.render_widget(loading, area);
            return;
        }

        let header = Row::new(vec![
            Cell::from("Name").style(Theme::dim()),
            Cell::from("Status").style(Theme::dim()),
            Cell::from("Response Time").style(Theme::dim()),
        ])
        .height(1);

        let rows: Vec<Row> = self
            .results
            .iter()
            .skip(self.table.offset)
            .take(self.table.visible_height)
            .map(|test| {
                let status_style = match test.status.as_str() {
                    "passed" | "pass" | "ok" => Theme::success(),
                    "failed" | "fail" | "error" => Theme::error(),
                    _ => Theme::dim(),
                };
                Row::new(vec![
                    Cell::from(test.name.clone()),
                    Cell::from(test.status.clone()).style(status_style),
                    Cell::from(
                        test.response_time_ms.map_or("--".to_string(), |ms| format!("{ms}ms")),
                    ),
                ])
            })
            .collect();

        let widths = [
            Constraint::Min(20),
            Constraint::Length(10),
            Constraint::Length(15),
        ];

        let running_indicator = if self.running { " (running...)" } else { "" };

        let table = Table::new(rows, widths)
            .header(header)
            .row_highlight_style(Theme::highlight())
            .block(
                Block::default()
                    .title(format!(" Smoke Tests ({}) {running_indicator}", self.results.len()))
                    .title_style(Theme::title())
                    .borders(Borders::ALL)
                    .border_style(Theme::dim())
                    .style(Theme::surface()),
            );

        let mut table_state = self.table.to_ratatui_state();
        frame.render_stateful_widget(table, area, &mut table_state);
    }

    fn tick(&mut self, client: &MockForgeClient, tx: &mpsc::UnboundedSender<Event>) {
        // Handle pending run action — actually execute smoke tests via API.
        if self.pending_run {
            self.pending_run = false;
            self.running = true;
            let client = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                match client.run_smoke_tests().await {
                    Ok(results) => {
                        let json = serde_json::json!(results
                            .iter()
                            .map(|t| serde_json::json!({
                                "id": t.id,
                                "name": t.name,
                                "method": t.method,
                                "path": t.path,
                                "status": t.status,
                                "response_time_ms": t.response_time_ms,
                                "error_message": t.error_message,
                            }))
                            .collect::<Vec<_>>());
                        let payload = serde_json::to_string(&json).unwrap_or_default();
                        let _ = tx.send(Event::Data {
                            screen: "smoke_tests",
                            payload,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(Event::ApiError {
                            screen: "smoke_tests",
                            message: format!("Run failed: {e}"),
                        });
                    }
                }
            });
            return;
        }

        let should_fetch =
            self.last_fetch.map_or(true, |t| t.elapsed().as_secs() >= FETCH_INTERVAL);
        if !should_fetch {
            return;
        }
        self.last_fetch = Some(Instant::now());

        let client = client.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            match client.get_smoke_tests().await {
                Ok(data) => {
                    let json = serde_json::json!(data
                        .iter()
                        .map(|t| serde_json::json!({
                            "id": t.id,
                            "name": t.name,
                            "method": t.method,
                            "path": t.path,
                            "status": t.status,
                            "response_time_ms": t.response_time_ms,
                            "error_message": t.error_message,
                        }))
                        .collect::<Vec<_>>());
                    let payload = serde_json::to_string(&json).unwrap_or_default();
                    let _ = tx.send(Event::Data {
                        screen: "smoke_tests",
                        payload,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Event::ApiError {
                        screen: "smoke_tests",
                        message: e.to_string(),
                    });
                }
            }
        });
    }

    fn on_data(&mut self, payload: &str) {
        self.running = false;
        match serde_json::from_str::<Vec<SmokeTestResult>>(payload) {
            Ok(results) => {
                self.table.set_total(results.len());
                self.results = results;
                self.error = None;
            }
            Err(e) => {
                self.error = Some(format!("Parse error: {e}"));
            }
        }
    }

    fn on_error(&mut self, message: &str) {
        self.running = false;
        self.error = Some(message.to_string());
    }

    fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    fn force_refresh(&mut self) {
        self.last_fetch = None;
    }

    fn status_hint(&self) -> &str {
        "r:run tests  j/k:navigate  g/G:top/bottom"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn sample_payload() -> String {
        let payload = serde_json::json!([
            {
                "id": "test-1",
                "name": "GET /health",
                "method": "GET",
                "path": "/health",
                "status": "passed",
                "response_time_ms": 5,
                "error_message": null
            },
            {
                "id": "test-2",
                "name": "POST /api/items",
                "method": "POST",
                "path": "/api/items",
                "status": "failed",
                "response_time_ms": 120,
                "error_message": "Expected 201, got 500"
            }
        ]);
        serde_json::to_string(&payload).unwrap()
    }

    #[test]
    fn new_creates_expected_defaults() {
        let s = SmokeTestsScreen::new();
        assert!(s.results.is_empty());
        assert!(!s.pending_run);
        assert!(!s.running);
        assert!(s.error.is_none());
        assert!(s.last_fetch.is_none());
    }

    #[test]
    fn on_data_parses_smoke_test_results() {
        let mut s = SmokeTestsScreen::new();
        s.on_data(&sample_payload());

        assert_eq!(s.results.len(), 2);
        assert_eq!(s.results[0].id, "test-1");
        assert_eq!(s.results[0].name, "GET /health");
        assert_eq!(s.results[0].method, "GET");
        assert_eq!(s.results[0].path, "/health");
        assert_eq!(s.results[0].status, "passed");
        assert_eq!(s.results[0].response_time_ms, Some(5));
        assert!(s.results[0].error_message.is_none());

        assert_eq!(s.results[1].id, "test-2");
        assert_eq!(s.results[1].name, "POST /api/items");
        assert_eq!(s.results[1].status, "failed");
        assert_eq!(s.results[1].response_time_ms, Some(120));
        assert_eq!(s.results[1].error_message.as_deref(), Some("Expected 201, got 500"));
        assert!(s.error.is_none());
    }

    #[test]
    fn on_data_clears_running_state() {
        let mut s = SmokeTestsScreen::new();
        s.running = true;
        s.on_data(&sample_payload());

        assert!(!s.running);
    }

    #[test]
    fn on_data_invalid_json_sets_error() {
        let mut s = SmokeTestsScreen::new();
        s.on_data("not valid json!!!");

        assert!(s.error.is_some());
        let err = s.error.as_ref().unwrap();
        assert!(
            err.contains("Parse error"),
            "expected error to contain 'Parse error', got: {err}"
        );
    }

    #[test]
    fn handle_key_r_sets_pending_run() {
        let mut s = SmokeTestsScreen::new();
        let consumed = s.handle_key(key(KeyCode::Char('r')));

        assert!(consumed);
        assert!(s.pending_run);
    }

    #[test]
    fn handle_key_j_k_navigates_table() {
        let mut s = SmokeTestsScreen::new();
        // Populate with data so table has rows to navigate
        s.on_data(&sample_payload());
        assert_eq!(s.table.selected, 0);

        let consumed = s.handle_key(key(KeyCode::Char('j')));
        assert!(consumed);
        assert_eq!(s.table.selected, 1);

        let consumed = s.handle_key(key(KeyCode::Char('k')));
        assert!(consumed);
        assert_eq!(s.table.selected, 0);
    }

    #[test]
    fn on_error_sets_error_and_clears_running() {
        let mut s = SmokeTestsScreen::new();
        s.running = true;
        s.on_error("something went wrong");

        assert!(!s.running);
        assert_eq!(s.error.as_deref(), Some("something went wrong"));
    }

    #[test]
    fn force_refresh_clears_last_fetch() {
        let mut s = SmokeTestsScreen::new();
        s.last_fetch = Some(Instant::now());
        assert!(s.last_fetch.is_some());

        s.force_refresh();
        assert!(s.last_fetch.is_none());
    }

    #[test]
    fn status_hint_contains_expected_keywords() {
        let s = SmokeTestsScreen::new();
        let hint = s.status_hint();
        assert!(hint.contains("run"), "hint should mention 'run'");
        assert!(hint.contains("j/k"), "hint should mention 'j/k' navigation");
        assert!(hint.contains("navigate"), "hint should mention 'navigate'");
    }
}
