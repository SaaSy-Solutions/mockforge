//! Centralised keybinding definitions.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Recognised global actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    NextTab,
    PrevTab,
    JumpTab(usize),
    Refresh,
    ToggleHelp,
    StartFilter,
    ScrollUp,
    ScrollDown,
    ScrollTop,
    ScrollBottom,
    PageUp,
    PageDown,
    Select,
    Back,
    ToggleFollow,
    Edit,
    Toggle,
    Delete,
    Sort,
}

/// Map a key event to an [`Action`].
pub fn resolve(key: KeyEvent) -> Option<Action> {
    // Ctrl+C / Ctrl+Q always quit
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c' | 'q') => Some(Action::Quit),
            _ => None,
        };
    }

    match key.code {
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Tab => Some(Action::NextTab),
        KeyCode::BackTab => Some(Action::PrevTab),
        KeyCode::Char('1') => Some(Action::JumpTab(0)),
        KeyCode::Char('2') => Some(Action::JumpTab(1)),
        KeyCode::Char('3') => Some(Action::JumpTab(2)),
        KeyCode::Char('4') => Some(Action::JumpTab(3)),
        KeyCode::Char('5') => Some(Action::JumpTab(4)),
        KeyCode::Char('6') => Some(Action::JumpTab(5)),
        KeyCode::Char('7') => Some(Action::JumpTab(6)),
        KeyCode::Char('8') => Some(Action::JumpTab(7)),
        KeyCode::Char('9') => Some(Action::JumpTab(8)),
        KeyCode::Char('0') => Some(Action::JumpTab(9)),
        KeyCode::Char('r') => Some(Action::Refresh),
        KeyCode::Char('?') => Some(Action::ToggleHelp),
        KeyCode::Char('/') => Some(Action::StartFilter),
        KeyCode::Char('j') | KeyCode::Down => Some(Action::ScrollDown),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::ScrollUp),
        KeyCode::Char('g') => Some(Action::ScrollTop),
        KeyCode::Char('G') => Some(Action::ScrollBottom),
        KeyCode::PageUp => Some(Action::PageUp),
        KeyCode::PageDown => Some(Action::PageDown),
        KeyCode::Enter => Some(Action::Select),
        KeyCode::Esc => Some(Action::Back),
        KeyCode::Char('f') => Some(Action::ToggleFollow),
        KeyCode::Char('e') => Some(Action::Edit),
        KeyCode::Char('t') => Some(Action::Toggle),
        KeyCode::Char('d') => Some(Action::Delete),
        KeyCode::Char('s') => Some(Action::Sort),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    /// Helper to build a plain key event with no modifiers.
    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    /// Helper to build a key event with Ctrl modifier.
    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn q_maps_to_quit() {
        assert_eq!(resolve(key(KeyCode::Char('q'))), Some(Action::Quit));
    }

    #[test]
    fn ctrl_c_maps_to_quit() {
        assert_eq!(resolve(ctrl(KeyCode::Char('c'))), Some(Action::Quit));
    }

    #[test]
    fn ctrl_q_maps_to_quit() {
        assert_eq!(resolve(ctrl(KeyCode::Char('q'))), Some(Action::Quit));
    }

    #[test]
    fn tab_maps_to_next_tab() {
        assert_eq!(resolve(key(KeyCode::Tab)), Some(Action::NextTab));
    }

    #[test]
    fn backtab_maps_to_prev_tab() {
        assert_eq!(resolve(key(KeyCode::BackTab)), Some(Action::PrevTab));
    }

    #[test]
    fn digit_keys_map_to_jump_tab() {
        assert_eq!(resolve(key(KeyCode::Char('1'))), Some(Action::JumpTab(0)));
        assert_eq!(resolve(key(KeyCode::Char('2'))), Some(Action::JumpTab(1)));
        assert_eq!(resolve(key(KeyCode::Char('3'))), Some(Action::JumpTab(2)));
        assert_eq!(resolve(key(KeyCode::Char('4'))), Some(Action::JumpTab(3)));
        assert_eq!(resolve(key(KeyCode::Char('5'))), Some(Action::JumpTab(4)));
        assert_eq!(resolve(key(KeyCode::Char('6'))), Some(Action::JumpTab(5)));
        assert_eq!(resolve(key(KeyCode::Char('7'))), Some(Action::JumpTab(6)));
        assert_eq!(resolve(key(KeyCode::Char('8'))), Some(Action::JumpTab(7)));
        assert_eq!(resolve(key(KeyCode::Char('9'))), Some(Action::JumpTab(8)));
        assert_eq!(resolve(key(KeyCode::Char('0'))), Some(Action::JumpTab(9)));
    }

    #[test]
    fn r_maps_to_refresh() {
        assert_eq!(resolve(key(KeyCode::Char('r'))), Some(Action::Refresh));
    }

    #[test]
    fn question_mark_maps_to_toggle_help() {
        assert_eq!(resolve(key(KeyCode::Char('?'))), Some(Action::ToggleHelp));
    }

    #[test]
    fn slash_maps_to_start_filter() {
        assert_eq!(resolve(key(KeyCode::Char('/'))), Some(Action::StartFilter));
    }

    #[test]
    fn navigation_keys() {
        assert_eq!(resolve(key(KeyCode::Char('j'))), Some(Action::ScrollDown));
        assert_eq!(resolve(key(KeyCode::Down)), Some(Action::ScrollDown));
        assert_eq!(resolve(key(KeyCode::Char('k'))), Some(Action::ScrollUp));
        assert_eq!(resolve(key(KeyCode::Up)), Some(Action::ScrollUp));
        assert_eq!(resolve(key(KeyCode::Char('g'))), Some(Action::ScrollTop));
        assert_eq!(resolve(key(KeyCode::Char('G'))), Some(Action::ScrollBottom));
        assert_eq!(resolve(key(KeyCode::PageUp)), Some(Action::PageUp));
        assert_eq!(resolve(key(KeyCode::PageDown)), Some(Action::PageDown));
    }

    #[test]
    fn action_keys() {
        assert_eq!(resolve(key(KeyCode::Enter)), Some(Action::Select));
        assert_eq!(resolve(key(KeyCode::Esc)), Some(Action::Back));
        assert_eq!(resolve(key(KeyCode::Char('f'))), Some(Action::ToggleFollow));
        assert_eq!(resolve(key(KeyCode::Char('e'))), Some(Action::Edit));
        assert_eq!(resolve(key(KeyCode::Char('t'))), Some(Action::Toggle));
        assert_eq!(resolve(key(KeyCode::Char('d'))), Some(Action::Delete));
        assert_eq!(resolve(key(KeyCode::Char('s'))), Some(Action::Sort));
    }

    #[test]
    fn unrecognized_key_returns_none() {
        assert_eq!(resolve(key(KeyCode::Char('z'))), None);
        assert_eq!(resolve(key(KeyCode::F(1))), None);
        assert_eq!(resolve(key(KeyCode::Insert)), None);
    }

    #[test]
    fn ctrl_with_unrecognized_char_returns_none() {
        assert_eq!(resolve(ctrl(KeyCode::Char('x'))), None);
        assert_eq!(resolve(ctrl(KeyCode::Char('a'))), None);
    }
}
