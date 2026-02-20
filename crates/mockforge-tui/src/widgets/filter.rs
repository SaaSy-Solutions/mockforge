//! Text filter input widget with live typing support.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::theme::Theme;

/// Manages a filter text input.
#[derive(Debug, Default)]
pub struct FilterInput {
    /// Whether the filter input is active (accepting keystrokes).
    pub active: bool,
    /// Current filter text.
    pub text: String,
    /// Cursor position within `text`.
    cursor: usize,
}

impl FilterInput {
    pub fn new() -> Self {
        Self::default()
    }

    /// Activate the filter input.
    pub fn activate(&mut self) {
        self.active = true;
    }

    /// Deactivate and optionally clear.
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Clear the filter text.
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    /// Whether the filter has any text.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Handle a key event while the filter is active. Returns `true` if consumed.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if !self.active {
            return false;
        }

        match key.code {
            KeyCode::Char(c) => {
                self.text.insert(self.cursor, c);
                self.cursor += 1;
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.text.remove(self.cursor);
                }
            }
            KeyCode::Delete => {
                if self.cursor < self.text.len() {
                    self.text.remove(self.cursor);
                }
            }
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
            }
            KeyCode::Right => {
                self.cursor = (self.cursor + 1).min(self.text.len());
            }
            KeyCode::Home => {
                self.cursor = 0;
            }
            KeyCode::End => {
                self.cursor = self.text.len();
            }
            KeyCode::Esc => {
                self.deactivate();
            }
            KeyCode::Enter => {
                self.deactivate();
            }
            _ => return false,
        }
        true
    }

    /// Render the filter bar into the given area.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let style = if self.active {
            Theme::surface()
        } else {
            Theme::dim()
        };

        let prefix = if self.active { "/ " } else { "Filter: " };
        let line = Line::from(vec![
            Span::styled(prefix, Theme::key_hint()),
            Span::styled(&self.text, style),
            if self.active {
                Span::styled("▏", Theme::key_hint())
            } else {
                Span::raw("")
            },
        ]);

        let block = Block::default().borders(Borders::BOTTOM).border_style(Theme::dim());
        let paragraph = Paragraph::new(line).block(block).style(style);
        frame.render_widget(paragraph, area);
    }

    /// Check if a string matches the current filter.
    ///
    /// Supports structured syntax: `method:GET status:2xx path:/api`.
    /// Multiple terms are ANDed together. Plain text without `:` does
    /// a case-insensitive substring match on the whole line.
    pub fn matches(&self, text: &str) -> bool {
        if self.text.is_empty() {
            return true;
        }

        let lower_text = text.to_lowercase();

        // Split filter into whitespace-separated terms.
        for term in self.text.split_whitespace() {
            if let Some((key, value)) = term.split_once(':') {
                // Structured filter: match against specific fields in the log line.
                let lower_value = value.to_lowercase();
                let matched = match key.to_lowercase().as_str() {
                    "method" => {
                        // Log line format: "HH:MM:SS METHOD PATH STATUS TIME"
                        // Method is the second whitespace-delimited token.
                        text.split_whitespace()
                            .nth(1)
                            .is_some_and(|m| m.eq_ignore_ascii_case(value))
                    }
                    "status" => {
                        // Status might be "2xx", "4xx", "5xx" or exact like "200".
                        text.split_whitespace().nth(3).is_some_and(|s| {
                            if lower_value.ends_with("xx") {
                                // Match status class: "2xx" matches 200-299.
                                s.starts_with(&lower_value[..1])
                            } else {
                                s == value
                            }
                        })
                    }
                    "path" => {
                        // Match path (third token) as substring.
                        text.split_whitespace()
                            .nth(2)
                            .is_some_and(|p| p.to_lowercase().contains(&lower_value))
                    }
                    _ => {
                        // Unknown key — fall back to plain substring match.
                        lower_text.contains(&term.to_lowercase())
                    }
                };
                if !matched {
                    return false;
                }
            } else {
                // Plain text: case-insensitive substring.
                if !lower_text.contains(&term.to_lowercase()) {
                    return false;
                }
            }
        }
        true
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
    fn new_creates_empty_inactive_filter() {
        let f = FilterInput::new();
        assert!(!f.active);
        assert!(f.text.is_empty());
        assert!(f.is_empty());
    }

    #[test]
    fn activate_and_deactivate() {
        let mut f = FilterInput::new();
        assert!(!f.active);

        f.activate();
        assert!(f.active);

        f.deactivate();
        assert!(!f.active);
    }

    #[test]
    fn char_input_appends_text() {
        let mut f = FilterInput::new();
        f.activate();

        f.handle_key(key(KeyCode::Char('h')));
        assert_eq!(f.text, "h");

        f.handle_key(key(KeyCode::Char('i')));
        assert_eq!(f.text, "hi");
    }

    #[test]
    fn backspace_removes_last_char() {
        let mut f = FilterInput::new();
        f.activate();

        f.handle_key(key(KeyCode::Char('a')));
        f.handle_key(key(KeyCode::Char('b')));
        f.handle_key(key(KeyCode::Char('c')));
        assert_eq!(f.text, "abc");

        f.handle_key(key(KeyCode::Backspace));
        assert_eq!(f.text, "ab");
    }

    #[test]
    fn backspace_at_start_does_nothing() {
        let mut f = FilterInput::new();
        f.activate();

        let consumed = f.handle_key(key(KeyCode::Backspace));
        assert!(consumed);
        assert!(f.text.is_empty());
    }

    #[test]
    fn delete_removes_char_at_cursor() {
        let mut f = FilterInput::new();
        f.activate();

        f.handle_key(key(KeyCode::Char('a')));
        f.handle_key(key(KeyCode::Char('b')));
        f.handle_key(key(KeyCode::Char('c')));
        // Cursor is at position 3, move left to position 2
        f.handle_key(key(KeyCode::Left));
        // Now delete 'c' at position 2
        f.handle_key(key(KeyCode::Delete));
        assert_eq!(f.text, "ab");
    }

    #[test]
    fn delete_at_end_does_nothing() {
        let mut f = FilterInput::new();
        f.activate();

        f.handle_key(key(KeyCode::Char('a')));
        f.handle_key(key(KeyCode::Delete));
        assert_eq!(f.text, "a");
    }

    #[test]
    fn left_right_movement() {
        let mut f = FilterInput::new();
        f.activate();

        f.handle_key(key(KeyCode::Char('a')));
        f.handle_key(key(KeyCode::Char('b')));
        f.handle_key(key(KeyCode::Char('c')));
        // Cursor at 3. Move left twice to position 1.
        f.handle_key(key(KeyCode::Left));
        f.handle_key(key(KeyCode::Left));
        // Insert 'X' at position 1
        f.handle_key(key(KeyCode::Char('X')));
        assert_eq!(f.text, "aXbc");
    }

    #[test]
    fn left_at_start_does_not_underflow() {
        let mut f = FilterInput::new();
        f.activate();

        // Move left multiple times with no text
        f.handle_key(key(KeyCode::Left));
        f.handle_key(key(KeyCode::Left));
        // Now type a char — should be at position 0
        f.handle_key(key(KeyCode::Char('a')));
        assert_eq!(f.text, "a");
    }

    #[test]
    fn right_at_end_does_not_overflow() {
        let mut f = FilterInput::new();
        f.activate();

        f.handle_key(key(KeyCode::Char('a')));
        // Move right past end
        f.handle_key(key(KeyCode::Right));
        f.handle_key(key(KeyCode::Right));
        f.handle_key(key(KeyCode::Char('b')));
        assert_eq!(f.text, "ab");
    }

    #[test]
    fn home_and_end_keys() {
        let mut f = FilterInput::new();
        f.activate();

        f.handle_key(key(KeyCode::Char('a')));
        f.handle_key(key(KeyCode::Char('b')));
        f.handle_key(key(KeyCode::Char('c')));

        // Home moves cursor to 0
        f.handle_key(key(KeyCode::Home));
        f.handle_key(key(KeyCode::Char('X')));
        assert_eq!(f.text, "Xabc");

        // End moves cursor to end
        f.handle_key(key(KeyCode::End));
        f.handle_key(key(KeyCode::Char('Y')));
        assert_eq!(f.text, "XabcY");
    }

    #[test]
    fn esc_deactivates_filter() {
        let mut f = FilterInput::new();
        f.activate();
        f.handle_key(key(KeyCode::Char('a')));

        let consumed = f.handle_key(key(KeyCode::Esc));
        assert!(consumed);
        assert!(!f.active);
        // Text is preserved on deactivate
        assert_eq!(f.text, "a");
    }

    #[test]
    fn enter_deactivates_filter() {
        let mut f = FilterInput::new();
        f.activate();
        f.handle_key(key(KeyCode::Char('x')));

        let consumed = f.handle_key(key(KeyCode::Enter));
        assert!(consumed);
        assert!(!f.active);
        assert_eq!(f.text, "x");
    }

    #[test]
    fn inactive_filter_does_not_consume_keys() {
        let mut f = FilterInput::new();
        // Not activated — should not consume
        let consumed = f.handle_key(key(KeyCode::Char('a')));
        assert!(!consumed);
        assert!(f.text.is_empty());
    }

    #[test]
    fn matches_empty_filter_matches_everything() {
        let f = FilterInput::new();
        assert!(f.matches("anything"));
        assert!(f.matches(""));
        assert!(f.matches("Hello World"));
    }

    #[test]
    fn matches_case_insensitive() {
        let mut f = FilterInput::new();
        f.activate();
        f.handle_key(key(KeyCode::Char('h')));
        f.handle_key(key(KeyCode::Char('e')));
        f.handle_key(key(KeyCode::Char('l')));
        f.handle_key(key(KeyCode::Char('l')));
        f.handle_key(key(KeyCode::Char('o')));

        assert!(f.matches("Hello World"));
        assert!(f.matches("HELLO"));
        assert!(f.matches("say hello"));
        assert!(!f.matches("world"));
    }

    #[test]
    fn matches_partial_substring() {
        let mut f = FilterInput::new();
        f.text = "api".to_string();

        assert!(f.matches("/api/v1/users"));
        assert!(f.matches("API_KEY"));
        assert!(!f.matches("application"));
    }

    #[test]
    fn structured_filter_method() {
        let mut f = FilterInput::new();
        f.text = "method:GET".to_string();

        let log_line = "14:23:01    GET /api/users                 200   12ms";
        assert!(f.matches(log_line));

        let post_line = "14:23:02   POST /api/items                 201   34ms";
        assert!(!f.matches(post_line));
    }

    #[test]
    fn structured_filter_status_class() {
        let mut f = FilterInput::new();
        f.text = "status:4xx".to_string();

        let ok = "14:23:01    GET /api/users                 200   12ms";
        assert!(!f.matches(ok));

        let not_found = "14:23:03    GET /api/users/5               404    8ms";
        assert!(f.matches(not_found));
    }

    #[test]
    fn structured_filter_path() {
        let mut f = FilterInput::new();
        f.text = "path:/api".to_string();

        let api_line = "14:23:01    GET /api/users                 200   12ms";
        assert!(f.matches(api_line));

        let health_line = "14:23:03    GET /health                    200    2ms";
        assert!(!f.matches(health_line));
    }

    #[test]
    fn structured_filter_multiple_terms() {
        let mut f = FilterInput::new();
        f.text = "method:GET status:2xx".to_string();

        let get200 = "14:23:01    GET /api/users                 200   12ms";
        assert!(f.matches(get200));

        let post201 = "14:23:02   POST /api/items                 201   34ms";
        assert!(!f.matches(post201));
    }

    #[test]
    fn clear_resets_text_and_cursor() {
        let mut f = FilterInput::new();
        f.activate();
        f.handle_key(key(KeyCode::Char('t')));
        f.handle_key(key(KeyCode::Char('e')));
        f.handle_key(key(KeyCode::Char('s')));
        f.handle_key(key(KeyCode::Char('t')));

        f.clear();
        assert!(f.text.is_empty());
        assert!(f.is_empty());
        // After clear, typing should insert at position 0
        f.handle_key(key(KeyCode::Char('a')));
        assert_eq!(f.text, "a");
    }
}
