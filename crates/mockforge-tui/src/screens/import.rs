//! Import history viewer screen with clear action.

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tokio::sync::mpsc;

use crate::api::client::MockForgeClient;
use crate::event::Event;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::widgets::confirm::ConfirmDialog;

pub struct ImportScreen {
    data: Option<serde_json::Value>,
    error: Option<String>,
    last_fetch: Option<Instant>,
    scroll_offset: usize,
    confirm: ConfirmDialog,
    pending_clear: bool,
    status_message: Option<(bool, String)>,
}

impl ImportScreen {
    pub fn new() -> Self {
        Self {
            data: None,
            error: None,
            last_fetch: None,
            scroll_offset: 0,
            confirm: ConfirmDialog::new(),
            pending_clear: false,
            status_message: None,
        }
    }

    fn entry_count(&self) -> usize {
        self.data.as_ref().and_then(|d| d.as_array()).map_or(0, |a| a.len())
    }
}

impl Screen for ImportScreen {
    fn title(&self) -> &str {
        "Import"
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Confirm dialog takes priority.
        if self.confirm.visible {
            if let Some(confirmed) = self.confirm.handle_key(key) {
                if confirmed {
                    self.pending_clear = true;
                }
                return true;
            }
            return true;
        }

        match key.code {
            KeyCode::Char('r') => {
                self.last_fetch = None;
                true
            }
            KeyCode::Char('c') => {
                if self.entry_count() > 0 {
                    self.confirm.show(
                        "Clear History",
                        format!("Clear all {} import entries?", self.entry_count()),
                    );
                }
                true
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_offset = self.scroll_offset.saturating_add(1);
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                true
            }
            KeyCode::Char('g') => {
                self.scroll_offset = 0;
                true
            }
            KeyCode::Char('G') => {
                self.scroll_offset = self.entry_count().saturating_sub(1);
                true
            }
            _ => false,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let Some(ref data) = self.data else {
            let loading = Paragraph::new("Loading import history...").style(Theme::dim()).block(
                Block::default()
                    .title(" Import ")
                    .borders(Borders::ALL)
                    .border_style(Theme::dim()),
            );
            frame.render_widget(loading, area);
            self.confirm.render(frame);
            return;
        };

        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).split(area);

        let block = Block::default()
            .title(format!(" Import History ({}) ", self.entry_count()))
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());

        let mut lines = Vec::new();

        if let Some(entries) = data.as_array() {
            for entry in entries.iter().skip(self.scroll_offset) {
                let summary = entry
                    .as_object()
                    .map(|obj| {
                        let source =
                            obj.get("source").and_then(|v| v.as_str()).unwrap_or("unknown");
                        let status = obj.get("status").and_then(|v| v.as_str()).unwrap_or("--");
                        let timestamp =
                            obj.get("timestamp").and_then(|v| v.as_str()).unwrap_or("--");
                        let status_style = match status {
                            "success" | "ok" => Theme::success(),
                            "failed" | "error" => Theme::error(),
                            _ => Style::default().fg(Theme::FG),
                        };
                        vec![
                            Span::styled(format!("  {timestamp}  "), Theme::dim()),
                            Span::styled(format!("{source:<20}  "), Style::default().fg(Theme::FG)),
                            Span::styled(status.to_string(), status_style),
                        ]
                    })
                    .unwrap_or_else(|| {
                        vec![Span::styled(
                            format!("  {entry}"),
                            Style::default().fg(Theme::FG),
                        )]
                    });
                lines.push(Line::from(summary));
            }
        } else {
            let formatted = serde_json::to_string_pretty(data).unwrap_or_default();
            for line in formatted.lines() {
                lines.push(Line::from(Span::styled(
                    format!("  {line}"),
                    Style::default().fg(Theme::FG),
                )));
            }
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled("  No import history", Theme::dim())));
        }

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, chunks[0]);

        // Status message bar.
        let msg_line = if let Some((success, ref msg)) = self.status_message {
            let style = if success {
                Theme::success()
            } else {
                Theme::error()
            };
            Line::from(vec![
                Span::styled(if success { "  OK: " } else { "  ERR: " }, style),
                Span::styled(msg.as_str(), Theme::base()),
            ])
        } else {
            Line::from(Span::styled("  Ready", Theme::dim()))
        };
        let msg_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());
        frame.render_widget(Paragraph::new(msg_line).block(msg_block), chunks[1]);

        self.confirm.render(frame);
    }

    fn tick(&mut self, client: &MockForgeClient, tx: &mpsc::UnboundedSender<Event>) {
        // Handle pending clear.
        if self.pending_clear {
            self.pending_clear = false;
            let client = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let result = match client.clear_import_history().await {
                    Ok(msg) => serde_json::json!({
                        "type": "clear_result",
                        "success": true,
                        "message": if msg.is_empty() { "History cleared".to_string() } else { msg },
                    }),
                    Err(e) => serde_json::json!({
                        "type": "clear_result",
                        "success": false,
                        "message": e.to_string(),
                    }),
                };
                let _ = tx.send(Event::Data {
                    screen: "import",
                    payload: serde_json::to_string(&result).unwrap_or_default(),
                });
            });
        }

        // On-demand fetch (first load + manual refresh).
        let should_fetch = self.last_fetch.is_none();
        if !should_fetch {
            return;
        }
        self.last_fetch = Some(Instant::now());

        let client = client.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            match client.get_import_history().await {
                Ok(data) => {
                    let json = serde_json::to_string(&data).unwrap_or_default();
                    let _ = tx.send(Event::Data {
                        screen: "import",
                        payload: json,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Event::ApiError {
                        screen: "import",
                        message: e.to_string(),
                    });
                }
            }
        });
    }

    fn on_data(&mut self, payload: &str) {
        // Check for clear result.
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(payload) {
            if val.get("type").and_then(|v| v.as_str()) == Some("clear_result") {
                let success = val.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
                let message =
                    val.get("message").and_then(|v| v.as_str()).unwrap_or("done").to_string();
                self.status_message = Some((success, message));
                // Force refresh.
                self.last_fetch = None;
                return;
            }
        }

        // Normal history data.
        match serde_json::from_str::<serde_json::Value>(payload) {
            Ok(data) => {
                self.data = Some(data);
                self.error = None;
            }
            Err(e) => {
                self.error = Some(format!("Parse error: {e}"));
            }
        }
    }

    fn on_error(&mut self, message: &str) {
        self.error = Some(message.to_string());
    }

    fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    fn force_refresh(&mut self) {
        self.last_fetch = None;
    }

    fn status_hint(&self) -> &str {
        "r:refresh  c:clear  j/k:scroll  g/G:top/bottom"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn new_creates_empty_screen() {
        let s = ImportScreen::new();
        assert!(s.data.is_none());
        assert!(!s.pending_clear);
        assert!(s.status_message.is_none());
        assert_eq!(s.scroll_offset, 0);
    }

    #[test]
    fn on_data_parses_array() {
        let mut s = ImportScreen::new();
        s.on_data(r#"[{"source":"postman","status":"success","timestamp":"2025-01-01"}]"#);
        assert!(s.data.is_some());
        assert_eq!(s.entry_count(), 1);
    }

    #[test]
    fn r_key_forces_refresh() {
        let mut s = ImportScreen::new();
        s.last_fetch = Some(Instant::now());
        s.handle_key(key(KeyCode::Char('r')));
        assert!(s.last_fetch.is_none());
    }

    #[test]
    fn c_key_on_empty_does_not_show_confirm() {
        let mut s = ImportScreen::new();
        s.on_data("[]");
        s.handle_key(key(KeyCode::Char('c')));
        assert!(!s.confirm.visible);
    }

    #[test]
    fn c_key_with_entries_shows_confirm() {
        let mut s = ImportScreen::new();
        s.on_data(r#"[{"source":"postman","status":"success","timestamp":"2025-01-01"}]"#);
        s.handle_key(key(KeyCode::Char('c')));
        assert!(s.confirm.visible);
    }

    #[test]
    fn confirm_yes_sets_pending_clear() {
        let mut s = ImportScreen::new();
        s.on_data(r#"[{"source":"postman","status":"success","timestamp":"2025-01-01"}]"#);
        s.handle_key(key(KeyCode::Char('c')));
        s.handle_key(key(KeyCode::Char('y')));
        assert!(s.pending_clear);
    }

    #[test]
    fn confirm_no_does_not_clear() {
        let mut s = ImportScreen::new();
        s.on_data(r#"[{"source":"postman","status":"success","timestamp":"2025-01-01"}]"#);
        s.handle_key(key(KeyCode::Char('c')));
        s.handle_key(key(KeyCode::Char('n')));
        assert!(!s.pending_clear);
    }

    #[test]
    fn clear_result_sets_status_message() {
        let mut s = ImportScreen::new();
        let result = serde_json::json!({
            "type": "clear_result",
            "success": true,
            "message": "History cleared",
        });
        s.on_data(&serde_json::to_string(&result).unwrap());
        assert!(s.status_message.is_some());
        let (success, msg) = s.status_message.as_ref().unwrap();
        assert!(success);
        assert_eq!(msg, "History cleared");
    }

    #[test]
    fn j_k_scroll() {
        let mut s = ImportScreen::new();
        s.handle_key(key(KeyCode::Char('j')));
        assert_eq!(s.scroll_offset, 1);
        s.handle_key(key(KeyCode::Char('k')));
        assert_eq!(s.scroll_offset, 0);
        // Does not go below 0.
        s.handle_key(key(KeyCode::Char('k')));
        assert_eq!(s.scroll_offset, 0);
    }

    #[test]
    fn status_hint_shows_controls() {
        let s = ImportScreen::new();
        assert!(s.status_hint().contains("clear"));
        assert!(s.status_hint().contains("refresh"));
    }

    #[test]
    fn force_refresh_clears_last_fetch() {
        let mut s = ImportScreen::new();
        s.last_fetch = Some(Instant::now());
        s.force_refresh();
        assert!(s.last_fetch.is_none());
    }
}
