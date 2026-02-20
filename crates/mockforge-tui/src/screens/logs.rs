//! Live log stream with filtering and follow mode.

use std::collections::VecDeque;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tokio::sync::mpsc;

use crate::api::client::MockForgeClient;
use crate::api::sse;
use crate::event::Event;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::widgets::confirm::ConfirmDialog;
use crate::widgets::filter::FilterInput;

const MAX_LOG_LINES: usize = 5000;

pub struct LogsScreen {
    lines: VecDeque<String>,
    filtered_indices: Vec<usize>,
    filter: FilterInput,
    follow: bool,
    scroll_offset: usize,
    sse_started: bool,
    error: Option<String>,
    last_fetch: Option<Instant>,
    confirm: ConfirmDialog,
}

impl LogsScreen {
    pub fn new() -> Self {
        Self {
            lines: VecDeque::new(),
            filtered_indices: Vec::new(),
            filter: FilterInput::new(),
            follow: true,
            scroll_offset: 0,
            sse_started: false,
            error: None,
            last_fetch: None,
            confirm: ConfirmDialog::new(),
        }
    }

    fn rebuild_filtered(&mut self) {
        self.filtered_indices = self
            .lines
            .iter()
            .enumerate()
            .filter(|(_, line)| self.filter.matches(line))
            .map(|(i, _)| i)
            .collect();
    }

    fn visible_count(&self) -> usize {
        if self.filter.is_empty() {
            self.lines.len()
        } else {
            self.filtered_indices.len()
        }
    }

    fn get_line(&self, index: usize) -> Option<&str> {
        if self.filter.is_empty() {
            self.lines.get(index).map(String::as_str)
        } else {
            self.filtered_indices
                .get(index)
                .and_then(|&i| self.lines.get(i))
                .map(String::as_str)
        }
    }
}

impl Screen for LogsScreen {
    fn title(&self) -> &str {
        "Logs"
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Confirm dialog takes priority when visible.
        if self.confirm.visible {
            if let Some(confirmed) = self.confirm.handle_key(key) {
                if confirmed {
                    self.lines.clear();
                    self.filtered_indices.clear();
                    self.scroll_offset = 0;
                }
                return true;
            }
            return true;
        }

        // If filter is active, let it handle input first.
        if self.filter.active {
            let consumed = self.filter.handle_key(key);
            if consumed {
                self.rebuild_filtered();
            }
            return consumed;
        }

        match key.code {
            KeyCode::Char('f') => {
                self.follow = !self.follow;
                if self.follow {
                    let count = self.visible_count();
                    self.scroll_offset = count.saturating_sub(1);
                }
                true
            }
            KeyCode::Char('/') => {
                self.filter.activate();
                true
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.follow = false;
                let count = self.visible_count();
                if self.scroll_offset < count.saturating_sub(1) {
                    self.scroll_offset += 1;
                }
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.follow = false;
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                true
            }
            KeyCode::Char('g') => {
                self.follow = false;
                self.scroll_offset = 0;
                true
            }
            KeyCode::Char('G') => {
                self.follow = true;
                let count = self.visible_count();
                self.scroll_offset = count.saturating_sub(1);
                true
            }
            KeyCode::Char('c') => {
                if self.lines.is_empty() {
                    return true;
                }
                self.confirm
                    .show("Clear Logs", format!("Clear all {} log lines?", self.lines.len()));
                true
            }
            _ => false,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(2), // filter bar
            Constraint::Min(0),    // log content
        ])
        .split(area);

        // Filter bar with follow indicator
        let follow_text = if self.follow {
            " [f]ollow ON "
        } else {
            " [f]ollow OFF"
        };
        let follow_style = if self.follow {
            Theme::success()
        } else {
            Theme::dim()
        };

        let filter_area = chunks[0];
        self.filter.render(
            frame,
            Rect {
                width: filter_area.width.saturating_sub(14),
                ..filter_area
            },
        );
        let follow_area = Rect {
            x: filter_area.x + filter_area.width.saturating_sub(14),
            width: 14,
            ..filter_area
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(follow_text, follow_style))),
            follow_area,
        );

        // Log content
        let block = Block::default()
            .title(format!(" Logs ({}/{}) ", self.visible_count(), self.lines.len()))
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());

        let inner_height = block.inner(chunks[1]).height as usize;
        let count = self.visible_count();

        let start = if self.follow {
            count.saturating_sub(inner_height)
        } else {
            self.scroll_offset.min(count.saturating_sub(inner_height))
        };

        let lines: Vec<Line> = (start..count.min(start + inner_height))
            .filter_map(|i| self.get_line(i))
            .map(|line| colorize_log_line(line))
            .collect();

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, chunks[1]);

        self.confirm.render(frame);
    }

    fn tick(&mut self, client: &MockForgeClient, tx: &mpsc::UnboundedSender<Event>) {
        // Start SSE listener on first tick.
        if !self.sse_started {
            self.sse_started = true;
            sse::spawn_sse_listener(client.base_url().to_string(), None, tx.clone());
        }

        // Also fetch initial batch of logs via REST.
        let should_fetch = self.last_fetch.map_or(true, |t| t.elapsed().as_secs() >= 30);
        if should_fetch && self.lines.is_empty() {
            self.last_fetch = Some(Instant::now());
            let client = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                match client.get_logs(Some(100)).await {
                    Ok(logs) => {
                        let json = serde_json::to_string(&logs).unwrap_or_default();
                        let _ = tx.send(Event::Data {
                            screen: "logs",
                            payload: json,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(Event::ApiError {
                            screen: "logs",
                            message: e.to_string(),
                        });
                    }
                }
            });
        }
    }

    fn on_data(&mut self, payload: &str) {
        // Bulk load from REST endpoint.
        if let Ok(logs) = serde_json::from_str::<Vec<crate::api::models::RequestLog>>(payload) {
            for log in logs {
                let line = format!(
                    "{} {:>6} {:<30} {} {:>5}ms",
                    log.timestamp.format("%H:%M:%S"),
                    log.method,
                    truncate_path(&log.path, 30),
                    log.status_code,
                    log.response_time_ms,
                );
                self.lines.push_back(line);
            }
            while self.lines.len() > MAX_LOG_LINES {
                self.lines.pop_front();
            }
            self.rebuild_filtered();
            self.error = None;
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
        "f:follow  c:clear  /:filter (method:X status:Nxx path:X)  j/k:scroll"
    }

    fn push_log_line(&mut self, line: String) {
        self.lines.push_back(line);
        while self.lines.len() > MAX_LOG_LINES {
            self.lines.pop_front();
        }

        // Update filtered indices if filter is active.
        let idx = self.lines.len() - 1;
        if self.filter.is_empty() || self.filter.matches(self.lines.back().unwrap()) {
            self.filtered_indices.push(idx);
        }

        // Auto-scroll if following.
        if self.follow {
            self.scroll_offset = self.visible_count().saturating_sub(1);
        }
    }
}

fn truncate_path(s: &str, max: usize) -> String {
    if s.len() <= max {
        format!("{s:<max$}")
    } else {
        format!("{}…", &s[..max - 1])
    }
}

fn colorize_log_line(line: &str) -> Line<'static> {
    // Simple heuristic: find status code and method for coloring.
    let parts: Vec<&str> = line.splitn(5, ' ').collect();
    if parts.len() >= 4 {
        let time = parts[0];
        let method = parts[1].trim();
        let path = parts[2];
        let rest = parts[3..].join(" ");

        // Try to parse status code from rest.
        let status_code: u16 =
            rest.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);

        Line::from(vec![
            Span::styled(format!("{time} "), Theme::dim()),
            Span::styled(format!("{method:>6} "), Theme::http_method(method)),
            Span::styled(format!("{path} "), ratatui::style::Style::default().fg(Theme::FG)),
            Span::styled(rest, Theme::status_code(status_code)),
        ])
    } else {
        Line::from(Span::styled(line.to_string(), ratatui::style::Style::default().fg(Theme::FG)))
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

    #[test]
    fn new_creates_screen_with_defaults() {
        let screen = LogsScreen::new();
        assert!(screen.lines.is_empty());
        assert!(screen.follow);
        assert!(!screen.sse_started);
    }

    #[test]
    fn push_log_line_appends_and_auto_scrolls() {
        let mut screen = LogsScreen::new();
        assert!(screen.follow);

        screen.push_log_line("first line".to_string());
        assert_eq!(screen.lines.len(), 1);
        assert_eq!(screen.scroll_offset, 0);

        screen.push_log_line("second line".to_string());
        assert_eq!(screen.lines.len(), 2);
        assert_eq!(screen.scroll_offset, 1);

        screen.push_log_line("third line".to_string());
        assert_eq!(screen.lines.len(), 3);
        assert_eq!(screen.scroll_offset, 2);
    }

    #[test]
    fn push_log_line_respects_max_log_lines() {
        let mut screen = LogsScreen::new();
        for i in 0..MAX_LOG_LINES + 100 {
            screen.push_log_line(format!("line {i}"));
        }
        assert_eq!(screen.lines.len(), MAX_LOG_LINES);
        // Oldest lines should have been dropped; first remaining is line 100.
        assert_eq!(screen.lines.front().unwrap(), "line 100");
    }

    #[test]
    fn handle_key_f_toggles_follow() {
        let mut screen = LogsScreen::new();
        assert!(screen.follow);

        let consumed = screen.handle_key(key(KeyCode::Char('f')));
        assert!(consumed);
        assert!(!screen.follow);

        let consumed = screen.handle_key(key(KeyCode::Char('f')));
        assert!(consumed);
        assert!(screen.follow);
    }

    #[test]
    fn handle_key_j_k_scrolls_and_disables_follow() {
        let mut screen = LogsScreen::new();
        screen.push_log_line("line 0".to_string());
        screen.push_log_line("line 1".to_string());
        screen.push_log_line("line 2".to_string());
        assert!(screen.follow);

        // 'k' scrolls up and disables follow
        let consumed = screen.handle_key(key(KeyCode::Char('k')));
        assert!(consumed);
        assert!(!screen.follow);

        // Reset follow via 'f'
        screen.handle_key(key(KeyCode::Char('f')));
        assert!(screen.follow);

        // 'j' scrolls down and disables follow
        screen.scroll_offset = 0;
        let consumed = screen.handle_key(key(KeyCode::Char('j')));
        assert!(consumed);
        assert!(!screen.follow);
    }

    #[test]
    fn handle_key_g_jumps_to_top() {
        let mut screen = LogsScreen::new();
        screen.push_log_line("line 0".to_string());
        screen.push_log_line("line 1".to_string());
        screen.push_log_line("line 2".to_string());

        let consumed = screen.handle_key(key(KeyCode::Char('g')));
        assert!(consumed);
        assert_eq!(screen.scroll_offset, 0);
        assert!(!screen.follow);
    }

    #[test]
    fn handle_key_shift_g_jumps_to_bottom_and_enables_follow() {
        let mut screen = LogsScreen::new();
        screen.push_log_line("line 0".to_string());
        screen.push_log_line("line 1".to_string());
        screen.push_log_line("line 2".to_string());
        screen.follow = false;
        screen.scroll_offset = 0;

        let consumed = screen.handle_key(key(KeyCode::Char('G')));
        assert!(consumed);
        assert!(screen.follow);
        assert_eq!(screen.scroll_offset, 2);
    }

    #[test]
    fn handle_key_slash_activates_filter() {
        let mut screen = LogsScreen::new();
        assert!(!screen.filter.active);

        let consumed = screen.handle_key(key(KeyCode::Char('/')));
        assert!(consumed);
        assert!(screen.filter.active);
    }

    #[test]
    fn handle_key_c_on_empty_logs_does_not_show_confirm() {
        let mut screen = LogsScreen::new();
        assert!(screen.lines.is_empty());

        let consumed = screen.handle_key(key(KeyCode::Char('c')));
        assert!(consumed);
        assert!(!screen.confirm.visible);
    }

    #[test]
    fn handle_key_c_with_logs_shows_confirm() {
        let mut screen = LogsScreen::new();
        screen.push_log_line("some log line".to_string());

        let consumed = screen.handle_key(key(KeyCode::Char('c')));
        assert!(consumed);
        assert!(screen.confirm.visible);
    }

    #[test]
    fn confirm_yes_clears_all_lines() {
        let mut screen = LogsScreen::new();
        screen.push_log_line("line 0".to_string());
        screen.push_log_line("line 1".to_string());
        screen.push_log_line("line 2".to_string());
        assert_eq!(screen.lines.len(), 3);

        // Press 'c' to show confirm dialog
        screen.handle_key(key(KeyCode::Char('c')));
        assert!(screen.confirm.visible);

        // Press 'y' to confirm
        screen.handle_key(key(KeyCode::Char('y')));
        assert!(screen.lines.is_empty());
        assert!(!screen.confirm.visible);
    }

    #[test]
    fn on_data_parses_request_log_json_array() {
        let mut screen = LogsScreen::new();
        let payload = serde_json::json!([{
            "id": "req-001",
            "method": "GET",
            "path": "/api/users",
            "status_code": 200,
            "response_time_ms": 12,
            "timestamp": "2025-01-01T14:23:01Z"
        }]);
        let json = payload.to_string();

        screen.on_data(&json);
        assert_eq!(screen.lines.len(), 1);

        let line = screen.lines.front().unwrap();
        assert!(line.contains("GET"), "line should contain method: {line}");
        assert!(line.contains("/api/users"), "line should contain path: {line}");
        assert!(line.contains("200"), "line should contain status code: {line}");
        assert!(line.contains("12"), "line should contain response time: {line}");
    }

    #[test]
    fn force_refresh_clears_last_fetch() {
        let mut screen = LogsScreen::new();
        screen.last_fetch = Some(Instant::now());
        assert!(screen.last_fetch.is_some());

        screen.force_refresh();
        assert!(screen.last_fetch.is_none());
    }

    #[test]
    fn status_hint_contains_expected_keywords() {
        let screen = LogsScreen::new();
        let hint = screen.status_hint();
        assert!(hint.contains("follow"), "hint should mention follow: {hint}");
        assert!(hint.contains("clear"), "hint should mention clear: {hint}");
        assert!(hint.contains("filter"), "hint should mention filter: {hint}");
        assert!(hint.contains("scroll"), "hint should mention scroll: {hint}");
    }
}
