//! Confirmation dialog widget.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::theme::Theme;

/// A simple yes/no confirmation dialog.
#[derive(Debug)]
pub struct ConfirmDialog {
    pub visible: bool,
    pub title: String,
    pub message: String,
    pub selected_yes: bool,
}

impl Default for ConfirmDialog {
    fn default() -> Self {
        Self {
            visible: false,
            title: String::new(),
            message: String::new(),
            selected_yes: false,
        }
    }
}

impl ConfirmDialog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the dialog with a custom title and message.
    pub fn show(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.title = title.into();
        self.message = message.into();
        self.selected_yes = false;
        self.visible = true;
    }

    /// Hide the dialog.
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Handle a key event. Returns `Some(true)` for yes, `Some(false)` for no,
    /// `None` if still undecided.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<bool> {
        if !self.visible {
            return None;
        }
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                self.selected_yes = true;
                None
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.selected_yes = false;
                None
            }
            KeyCode::Char('y') => {
                self.hide();
                Some(true)
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.hide();
                Some(false)
            }
            KeyCode::Enter => {
                let result = self.selected_yes;
                self.hide();
                Some(result)
            }
            _ => None,
        }
    }

    /// Render the dialog centred on screen.
    pub fn render(&self, frame: &mut Frame) {
        if !self.visible {
            return;
        }

        let area = centered_rect(40, 20, frame.area());
        frame.render_widget(Clear, area);

        let yes_style = if self.selected_yes {
            Theme::highlight()
        } else {
            Theme::dim()
        };
        let no_style = if self.selected_yes {
            Theme::dim()
        } else {
            Theme::highlight()
        };

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(&self.message, Theme::base())),
            Line::from(""),
            Line::from(vec![
                Span::styled("  [ Yes ]  ", yes_style),
                Span::raw("  "),
                Span::styled("  [ No ]  ", no_style),
            ]),
        ];

        let block = Block::default()
            .title(format!(" {} ", self.title))
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());

        let paragraph =
            Paragraph::new(lines).block(block).alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(paragraph, area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
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
    fn new_creates_hidden_dialog() {
        let d = ConfirmDialog::new();
        assert!(!d.visible);
        assert!(d.title.is_empty());
        assert!(d.message.is_empty());
        assert!(!d.selected_yes);
    }

    #[test]
    fn show_makes_dialog_visible() {
        let mut d = ConfirmDialog::new();
        d.show("Delete?", "Are you sure you want to delete this?");
        assert!(d.visible);
        assert_eq!(d.title, "Delete?");
        assert_eq!(d.message, "Are you sure you want to delete this?");
        // show() defaults to No selected
        assert!(!d.selected_yes);
    }

    #[test]
    fn hide_makes_dialog_invisible() {
        let mut d = ConfirmDialog::new();
        d.show("Title", "Message");
        d.hide();
        assert!(!d.visible);
    }

    #[test]
    fn hidden_dialog_does_not_consume_keys() {
        let mut d = ConfirmDialog::new();
        let result = d.handle_key(key(KeyCode::Char('y')));
        assert_eq!(result, None);
    }

    #[test]
    fn y_key_confirms_and_hides() {
        let mut d = ConfirmDialog::new();
        d.show("Title", "Msg");

        let result = d.handle_key(key(KeyCode::Char('y')));
        assert_eq!(result, Some(true));
        assert!(!d.visible);
    }

    #[test]
    fn n_key_rejects_and_hides() {
        let mut d = ConfirmDialog::new();
        d.show("Title", "Msg");

        let result = d.handle_key(key(KeyCode::Char('n')));
        assert_eq!(result, Some(false));
        assert!(!d.visible);
    }

    #[test]
    fn esc_key_rejects_and_hides() {
        let mut d = ConfirmDialog::new();
        d.show("Title", "Msg");

        let result = d.handle_key(key(KeyCode::Esc));
        assert_eq!(result, Some(false));
        assert!(!d.visible);
    }

    #[test]
    fn left_right_toggle_selection() {
        let mut d = ConfirmDialog::new();
        d.show("Title", "Msg");
        // Initially selected_yes is false (No selected)
        assert!(!d.selected_yes);

        // Left selects Yes
        let result = d.handle_key(key(KeyCode::Left));
        assert_eq!(result, None); // Still undecided
        assert!(d.selected_yes);

        // Right selects No
        let result = d.handle_key(key(KeyCode::Right));
        assert_eq!(result, None);
        assert!(!d.selected_yes);
    }

    #[test]
    fn h_l_toggle_selection() {
        let mut d = ConfirmDialog::new();
        d.show("Title", "Msg");

        // 'h' selects Yes
        let result = d.handle_key(key(KeyCode::Char('h')));
        assert_eq!(result, None);
        assert!(d.selected_yes);

        // 'l' selects No
        let result = d.handle_key(key(KeyCode::Char('l')));
        assert_eq!(result, None);
        assert!(!d.selected_yes);
    }

    #[test]
    fn enter_confirms_current_selection_yes() {
        let mut d = ConfirmDialog::new();
        d.show("Title", "Msg");

        // Select yes first
        d.handle_key(key(KeyCode::Left));
        assert!(d.selected_yes);

        let result = d.handle_key(key(KeyCode::Enter));
        assert_eq!(result, Some(true));
        assert!(!d.visible);
    }

    #[test]
    fn enter_confirms_current_selection_no() {
        let mut d = ConfirmDialog::new();
        d.show("Title", "Msg");
        // Default is No
        assert!(!d.selected_yes);

        let result = d.handle_key(key(KeyCode::Enter));
        assert_eq!(result, Some(false));
        assert!(!d.visible);
    }

    #[test]
    fn unrecognized_key_returns_none() {
        let mut d = ConfirmDialog::new();
        d.show("Title", "Msg");

        let result = d.handle_key(key(KeyCode::Char('x')));
        assert_eq!(result, None);
        // Dialog remains visible
        assert!(d.visible);
    }

    #[test]
    fn show_resets_selection_to_no() {
        let mut d = ConfirmDialog::new();
        d.show("First", "First message");
        // Select yes
        d.handle_key(key(KeyCode::Left));
        assert!(d.selected_yes);

        // Show again — selection should reset to No
        d.show("Second", "Second message");
        assert!(!d.selected_yes);
        assert_eq!(d.title, "Second");
    }
}
