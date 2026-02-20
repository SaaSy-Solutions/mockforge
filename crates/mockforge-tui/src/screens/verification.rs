//! Verification screen — interactive query interface for verifying recorded requests.

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
use crate::api::models::VerificationResult;
use crate::event::Event;
use crate::screens::Screen;
use crate::theme::Theme;

/// Which input field is focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Method,
    Path,
    MinCount,
}

impl Field {
    fn next(self) -> Self {
        match self {
            Self::Method => Self::Path,
            Self::Path => Self::MinCount,
            Self::MinCount => Self::Method,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Method => Self::MinCount,
            Self::Path => Self::Method,
            Self::MinCount => Self::Path,
        }
    }
}

pub struct VerificationScreen {
    method: String,
    path: String,
    min_count: String,
    focused: Field,
    editing: bool,
    input_buf: String,
    input_cursor: usize,
    last_result: Option<VerificationResult>,
    pending_query: Option<serde_json::Value>,
    error: Option<String>,
    status_message: Option<(bool, String)>,
}

impl VerificationScreen {
    pub fn new() -> Self {
        Self {
            method: "GET".into(),
            path: String::new(),
            min_count: "1".into(),
            focused: Field::Method,
            editing: false,
            input_buf: String::new(),
            input_cursor: 0,
            last_result: None,
            pending_query: None,
            error: None,
            status_message: None,
        }
    }

    fn start_edit(&mut self) {
        self.editing = true;
        self.input_buf = match self.focused {
            Field::Method => self.method.clone(),
            Field::Path => self.path.clone(),
            Field::MinCount => self.min_count.clone(),
        };
        self.input_cursor = self.input_buf.len();
    }

    fn commit_edit(&mut self) {
        match self.focused {
            Field::Method => self.method = self.input_buf.to_uppercase(),
            Field::Path => self.path = self.input_buf.clone(),
            Field::MinCount => {
                // Validate: must be a non-negative integer.
                if self.input_buf.parse::<u64>().is_ok() {
                    self.min_count = self.input_buf.clone();
                }
            }
        }
        self.editing = false;
        self.input_buf.clear();
    }

    fn cancel_edit(&mut self) {
        self.editing = false;
        self.input_buf.clear();
    }

    fn render_form(&self, frame: &mut Frame, area: Rect) {
        let form_block = Block::default()
            .title(" Verification Query ")
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());

        let fields = [
            ("Method", &self.method, Field::Method),
            ("Path", &self.path, Field::Path),
            ("Min Count", &self.min_count, Field::MinCount),
        ];

        let mut lines = vec![Line::from("")];
        for (label, value, field) in &fields {
            let is_focused = *field == self.focused;
            let indicator = if is_focused { "▸ " } else { "  " };
            let label_style = if is_focused {
                Theme::title()
            } else {
                Theme::dim()
            };

            if self.editing && is_focused {
                let before = &self.input_buf[..self.input_cursor];
                let after = &self.input_buf[self.input_cursor..];
                lines.push(Line::from(vec![
                    Span::styled(format!("{indicator}{label:<10} "), label_style),
                    Span::styled(before.to_string(), Style::default().fg(Theme::FG)),
                    Span::styled("▏", Style::default().fg(Theme::BLUE)),
                    Span::styled(after.to_string(), Style::default().fg(Theme::FG)),
                ]));
            } else {
                let display = if value.is_empty() {
                    "(empty)".to_string()
                } else {
                    (*value).clone()
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("{indicator}{label:<10} "), label_style),
                    Span::styled(display, Style::default().fg(Theme::FG)),
                ]));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  ", Theme::dim()),
            Span::styled("[v]", Style::default().fg(Theme::BLUE)),
            Span::styled(" Submit query    ", Theme::dim()),
            Span::styled("[c]", Style::default().fg(Theme::BLUE)),
            Span::styled(" Clear results", Theme::dim()),
        ]));

        let form = Paragraph::new(lines).block(form_block);
        frame.render_widget(form, area);
    }

    fn render_results(&self, frame: &mut Frame, area: Rect) {
        let result_block = Block::default()
            .title(" Results ")
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());

        let mut result_lines = vec![Line::from("")];

        if let Some(ref result) = self.last_result {
            let match_style = if result.matched {
                Theme::success()
            } else {
                Theme::error()
            };
            let match_text = if result.matched {
                "MATCHED"
            } else {
                "NOT MATCHED"
            };

            result_lines.push(Line::from(vec![
                Span::styled("  Status: ", Theme::dim()),
                Span::styled(match_text, match_style),
            ]));
            result_lines.push(Line::from(vec![
                Span::styled("  Count:  ", Theme::dim()),
                Span::styled(result.count.to_string(), Style::default().fg(Theme::FG)),
            ]));

            if !result.details.is_null() {
                result_lines.push(Line::from(""));
                result_lines.push(Line::from(Span::styled("  Details:", Theme::dim())));
                let formatted = serde_json::to_string_pretty(&result.details)
                    .unwrap_or_else(|_| result.details.to_string());
                for detail_line in formatted.lines().take(20) {
                    result_lines.push(Line::from(Span::styled(
                        format!("    {detail_line}"),
                        Style::default().fg(Theme::FG),
                    )));
                }
            }
        } else if let Some((success, ref msg)) = self.status_message {
            let style = if success {
                Theme::success()
            } else {
                Theme::error()
            };
            result_lines.push(Line::from(vec![
                Span::styled("  ", Theme::dim()),
                Span::styled(msg.as_str(), style),
            ]));
        } else {
            result_lines.push(Line::from(Span::styled(
                "  Submit a query with 'v' to see results here.",
                Theme::dim(),
            )));
        }

        let results = Paragraph::new(result_lines).block(result_block);
        frame.render_widget(results, area);
    }

    fn build_query(&self) -> serde_json::Value {
        let mut query = serde_json::json!({
            "method": self.method,
        });
        if !self.path.is_empty() {
            query["path"] = serde_json::Value::String(self.path.clone());
        }
        if let Ok(n) = self.min_count.parse::<u64>() {
            if n > 0 {
                query["min_count"] = serde_json::json!(n);
            }
        }
        query
    }
}

impl Screen for VerificationScreen {
    fn title(&self) -> &str {
        "Verification"
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Inline editing mode.
        if self.editing {
            match key.code {
                KeyCode::Enter => {
                    self.commit_edit();
                    return true;
                }
                KeyCode::Esc => {
                    self.cancel_edit();
                    return true;
                }
                KeyCode::Backspace => {
                    if self.input_cursor > 0 {
                        self.input_cursor -= 1;
                        self.input_buf.remove(self.input_cursor);
                    }
                    return true;
                }
                KeyCode::Left => {
                    self.input_cursor = self.input_cursor.saturating_sub(1);
                    return true;
                }
                KeyCode::Right => {
                    if self.input_cursor < self.input_buf.len() {
                        self.input_cursor += 1;
                    }
                    return true;
                }
                KeyCode::Char(c) => {
                    self.input_buf.insert(self.input_cursor, c);
                    self.input_cursor += 1;
                    return true;
                }
                _ => return true,
            }
        }

        // Normal mode.
        match key.code {
            KeyCode::Tab | KeyCode::Char('j') | KeyCode::Down => {
                self.focused = self.focused.next();
                true
            }
            KeyCode::BackTab | KeyCode::Char('k') | KeyCode::Up => {
                self.focused = self.focused.prev();
                true
            }
            KeyCode::Enter | KeyCode::Char('e') => {
                self.start_edit();
                true
            }
            KeyCode::Char('v') => {
                // Submit the verification query.
                let query = self.build_query();
                self.pending_query = Some(query);
                true
            }
            KeyCode::Char('c') => {
                // Clear results.
                self.last_result = None;
                self.status_message = None;
                true
            }
            _ => false,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(10), // Query form
            Constraint::Min(0),     // Results
        ])
        .split(area);

        self.render_form(frame, chunks[0]);
        self.render_results(frame, chunks[1]);
    }

    fn tick(&mut self, client: &MockForgeClient, tx: &mpsc::UnboundedSender<Event>) {
        // Handle pending query.
        if let Some(query) = self.pending_query.take() {
            let client = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let result = match client.verify(&query).await {
                    Ok(vr) => serde_json::json!({
                        "type": "verification_result",
                        "matched": vr.matched,
                        "count": vr.count,
                        "details": vr.details,
                    }),
                    Err(e) => serde_json::json!({
                        "type": "verification_error",
                        "message": e.to_string(),
                    }),
                };
                let _ = tx.send(Event::Data {
                    screen: "verification",
                    payload: serde_json::to_string(&result).unwrap_or_default(),
                });
            });
        }
        // On-demand only — no periodic fetch.
    }

    fn on_data(&mut self, payload: &str) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(payload) {
            match val.get("type").and_then(|v| v.as_str()) {
                Some("verification_result") => {
                    self.last_result = Some(VerificationResult {
                        matched: val.get("matched").and_then(|v| v.as_bool()).unwrap_or(false),
                        count: val.get("count").and_then(|v| v.as_u64()).unwrap_or(0),
                        details: val.get("details").cloned().unwrap_or(serde_json::Value::Null),
                    });
                    self.status_message = None;
                    self.error = None;
                }
                Some("verification_error") => {
                    let message = val
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error")
                        .to_string();
                    self.status_message = Some((false, message));
                    self.last_result = None;
                }
                _ => {
                    // Generic data (unlikely for verification).
                    self.error = None;
                }
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
        // No-op: on-demand only.
    }

    fn status_hint(&self) -> &str {
        "Tab/j/k:fields  Enter/e:edit  v:verify  c:clear"
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
    fn new_creates_default_screen() {
        let s = VerificationScreen::new();
        assert_eq!(s.method, "GET");
        assert!(s.path.is_empty());
        assert_eq!(s.min_count, "1");
        assert_eq!(s.focused, Field::Method);
        assert!(!s.editing);
        assert!(s.last_result.is_none());
    }

    #[test]
    fn tab_cycles_fields_forward() {
        let mut s = VerificationScreen::new();
        assert_eq!(s.focused, Field::Method);
        s.handle_key(key(KeyCode::Tab));
        assert_eq!(s.focused, Field::Path);
        s.handle_key(key(KeyCode::Tab));
        assert_eq!(s.focused, Field::MinCount);
        s.handle_key(key(KeyCode::Tab));
        assert_eq!(s.focused, Field::Method);
    }

    #[test]
    fn j_k_navigate_fields() {
        let mut s = VerificationScreen::new();
        s.handle_key(key(KeyCode::Char('j')));
        assert_eq!(s.focused, Field::Path);
        s.handle_key(key(KeyCode::Char('k')));
        assert_eq!(s.focused, Field::Method);
    }

    #[test]
    fn enter_starts_edit_mode() {
        let mut s = VerificationScreen::new();
        s.handle_key(key(KeyCode::Enter));
        assert!(s.editing);
        assert_eq!(s.input_buf, "GET");
    }

    #[test]
    fn edit_and_commit() {
        let mut s = VerificationScreen::new();
        // Focus path field.
        s.handle_key(key(KeyCode::Tab));
        assert_eq!(s.focused, Field::Path);

        // Start editing.
        s.handle_key(key(KeyCode::Enter));
        assert!(s.editing);

        // Type a path.
        s.handle_key(key(KeyCode::Char('/')));
        s.handle_key(key(KeyCode::Char('a')));
        s.handle_key(key(KeyCode::Char('p')));
        s.handle_key(key(KeyCode::Char('i')));

        // Commit.
        s.handle_key(key(KeyCode::Enter));
        assert!(!s.editing);
        assert_eq!(s.path, "/api");
    }

    #[test]
    fn edit_and_cancel() {
        let mut s = VerificationScreen::new();
        s.handle_key(key(KeyCode::Enter)); // Edit method
        s.handle_key(key(KeyCode::Backspace));
        s.handle_key(key(KeyCode::Backspace));
        s.handle_key(key(KeyCode::Backspace));
        s.handle_key(key(KeyCode::Char('P')));
        s.handle_key(key(KeyCode::Esc)); // Cancel
        assert!(!s.editing);
        assert_eq!(s.method, "GET"); // Unchanged
    }

    #[test]
    fn method_uppercased_on_commit() {
        let mut s = VerificationScreen::new();
        s.handle_key(key(KeyCode::Enter));
        // Clear existing.
        s.input_buf.clear();
        s.input_cursor = 0;
        s.handle_key(key(KeyCode::Char('p')));
        s.handle_key(key(KeyCode::Char('o')));
        s.handle_key(key(KeyCode::Char('s')));
        s.handle_key(key(KeyCode::Char('t')));
        s.handle_key(key(KeyCode::Enter));
        assert_eq!(s.method, "POST");
    }

    #[test]
    fn invalid_min_count_rejected() {
        let mut s = VerificationScreen::new();
        // Focus min_count.
        s.handle_key(key(KeyCode::Tab));
        s.handle_key(key(KeyCode::Tab));
        assert_eq!(s.focused, Field::MinCount);

        s.handle_key(key(KeyCode::Enter));
        s.input_buf = "abc".into();
        s.input_cursor = 3;
        s.handle_key(key(KeyCode::Enter));
        assert_eq!(s.min_count, "1"); // Unchanged
    }

    #[test]
    fn v_key_sets_pending_query() {
        let mut s = VerificationScreen::new();
        s.handle_key(key(KeyCode::Char('v')));
        assert!(s.pending_query.is_some());
        let q = s.pending_query.as_ref().unwrap();
        assert_eq!(q["method"], "GET");
    }

    #[test]
    fn build_query_includes_path_when_set() {
        let mut s = VerificationScreen::new();
        s.path = "/api/users".into();
        let q = s.build_query();
        assert_eq!(q["path"], "/api/users");
    }

    #[test]
    fn build_query_omits_empty_path() {
        let s = VerificationScreen::new();
        let q = s.build_query();
        assert!(q.get("path").is_none());
    }

    #[test]
    fn on_data_parses_verification_result() {
        let mut s = VerificationScreen::new();
        let result = serde_json::json!({
            "type": "verification_result",
            "matched": true,
            "count": 5,
            "details": {"methods": ["GET"]},
        });
        s.on_data(&serde_json::to_string(&result).unwrap());
        assert!(s.last_result.is_some());
        let r = s.last_result.as_ref().unwrap();
        assert!(r.matched);
        assert_eq!(r.count, 5);
    }

    #[test]
    fn on_data_parses_verification_error() {
        let mut s = VerificationScreen::new();
        let result = serde_json::json!({
            "type": "verification_error",
            "message": "No recordings found",
        });
        s.on_data(&serde_json::to_string(&result).unwrap());
        assert!(s.last_result.is_none());
        assert!(s.status_message.is_some());
        let (success, msg) = s.status_message.as_ref().unwrap();
        assert!(!success);
        assert_eq!(msg, "No recordings found");
    }

    #[test]
    fn c_key_clears_results() {
        let mut s = VerificationScreen::new();
        s.last_result = Some(VerificationResult {
            matched: true,
            count: 3,
            details: serde_json::Value::Null,
        });
        s.handle_key(key(KeyCode::Char('c')));
        assert!(s.last_result.is_none());
    }

    #[test]
    fn status_hint_shows_verify() {
        let s = VerificationScreen::new();
        assert!(s.status_hint().contains("verify"));
    }
}
