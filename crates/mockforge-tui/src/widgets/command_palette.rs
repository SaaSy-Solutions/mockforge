//! Command palette — fuzzy-filtered list of actions triggered by `:`.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::screens::ScreenId;
use crate::theme::Theme;

/// A command that can be executed from the palette.
#[derive(Debug, Clone)]
struct Command {
    label: &'static str,
    action: PaletteAction,
}

/// What happens when a command is selected.
#[derive(Debug, Clone, Copy)]
pub enum PaletteAction {
    /// Navigate to a specific screen tab.
    GoToScreen(usize),
    /// Refresh the current screen.
    Refresh,
    /// Toggle help overlay.
    ToggleHelp,
    /// Quit the application.
    Quit,
}

/// State for the command palette overlay.
pub struct CommandPalette {
    pub visible: bool,
    input: String,
    cursor: usize,
    commands: Vec<Command>,
    filtered: Vec<usize>,
    selected: usize,
}

impl CommandPalette {
    pub fn new() -> Self {
        let mut commands = Vec::new();

        // Add screen navigation commands.
        for (i, sid) in ScreenId::ALL.iter().enumerate() {
            commands.push(Command {
                label: sid.label(),
                action: PaletteAction::GoToScreen(i),
            });
        }

        // Add utility commands.
        commands.push(Command {
            label: "Refresh",
            action: PaletteAction::Refresh,
        });
        commands.push(Command {
            label: "Help",
            action: PaletteAction::ToggleHelp,
        });
        commands.push(Command {
            label: "Quit",
            action: PaletteAction::Quit,
        });

        let filtered: Vec<usize> = (0..commands.len()).collect();

        Self {
            visible: false,
            input: String::new(),
            cursor: 0,
            commands,
            filtered,
            selected: 0,
        }
    }

    /// Open the command palette.
    pub fn open(&mut self) {
        self.visible = true;
        self.input.clear();
        self.cursor = 0;
        self.selected = 0;
        self.rebuild_filtered();
    }

    /// Close the palette without executing.
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// Handle a key event. Returns `Some(action)` if a command was selected.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<PaletteAction> {
        if !self.visible {
            return None;
        }

        match key.code {
            KeyCode::Char(c) => {
                self.input.insert(self.cursor, c);
                self.cursor += 1;
                self.rebuild_filtered();
                self.selected = 0;
                None
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.input.remove(self.cursor);
                    self.rebuild_filtered();
                    self.selected = 0;
                }
                None
            }
            KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                None
            }
            KeyCode::Down => {
                if !self.filtered.is_empty() {
                    self.selected = (self.selected + 1).min(self.filtered.len() - 1);
                }
                None
            }
            KeyCode::Enter => {
                let action = self.filtered.get(self.selected).map(|&idx| self.commands[idx].action);
                self.close();
                action
            }
            KeyCode::Esc => {
                self.close();
                None
            }
            _ => None,
        }
    }

    fn rebuild_filtered(&mut self) {
        if self.input.is_empty() {
            self.filtered = (0..self.commands.len()).collect();
        } else {
            let query = self.input.to_lowercase();
            self.filtered = self
                .commands
                .iter()
                .enumerate()
                .filter(|(_, cmd)| cmd.label.to_lowercase().contains(&query))
                .map(|(i, _)| i)
                .collect();
        }
    }

    /// Render the palette overlay.
    pub fn render(&self, frame: &mut Frame) {
        if !self.visible {
            return;
        }

        let area = centered_rect(50, 60, frame.area());
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(" Command Palette ")
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Split inner into input line + results list.
        let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(inner);

        // Input line.
        let input_line = Line::from(vec![
            Span::styled(": ", Theme::key_hint()),
            Span::styled(&self.input, Theme::base()),
            Span::styled("▏", Theme::key_hint()),
        ]);
        frame.render_widget(Paragraph::new(input_line), chunks[0]);

        // Results list (max visible items).
        let max_visible = chunks[1].height as usize;
        let mut lines = Vec::new();
        for (display_idx, &cmd_idx) in self.filtered.iter().enumerate().take(max_visible) {
            let cmd = &self.commands[cmd_idx];
            let style = if display_idx == self.selected {
                Theme::highlight()
            } else {
                Theme::base()
            };
            lines.push(Line::from(Span::styled(format!("  {}", cmd.label), style)));
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled("  No matching commands", Theme::dim())));
        }

        frame.render_widget(Paragraph::new(lines), chunks[1]);
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
    fn open_and_close() {
        let mut palette = CommandPalette::new();
        assert!(!palette.visible);
        palette.open();
        assert!(palette.visible);
        palette.close();
        assert!(!palette.visible);
    }

    #[test]
    fn filter_narrows_results() {
        let mut palette = CommandPalette::new();
        palette.open();
        let all_count = palette.filtered.len();

        // Type "log" to filter
        palette.handle_key(key(KeyCode::Char('l')));
        palette.handle_key(key(KeyCode::Char('o')));
        palette.handle_key(key(KeyCode::Char('g')));

        assert!(palette.filtered.len() < all_count);
        assert!(!palette.filtered.is_empty());
    }

    #[test]
    fn enter_selects_command() {
        let mut palette = CommandPalette::new();
        palette.open();
        // First item should be Dashboard (GoToScreen(0))
        let action = palette.handle_key(key(KeyCode::Enter));
        assert!(action.is_some());
        assert!(!palette.visible);
    }

    #[test]
    fn esc_closes_without_action() {
        let mut palette = CommandPalette::new();
        palette.open();
        let action = palette.handle_key(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(!palette.visible);
    }

    #[test]
    fn hidden_palette_returns_none() {
        let mut palette = CommandPalette::new();
        let action = palette.handle_key(key(KeyCode::Enter));
        assert!(action.is_none());
    }
}
