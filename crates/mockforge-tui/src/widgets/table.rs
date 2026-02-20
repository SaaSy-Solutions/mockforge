//! Enhanced table widget with sorting, scrolling, and row selection.

use crossterm::event::{KeyCode, KeyEvent};

/// Table state that tracks selection, scroll position, and sort column.
#[derive(Debug)]
pub struct TableState {
    /// Currently selected row index.
    pub selected: usize,
    /// Scroll offset (first visible row).
    pub offset: usize,
    /// Visible height (set on each render).
    pub visible_height: usize,
    /// Total number of rows.
    pub total_rows: usize,
    /// Current sort column index.
    pub sort_column: usize,
    /// Sort ascending.
    pub sort_ascending: bool,
}

impl Default for TableState {
    fn default() -> Self {
        Self {
            selected: 0,
            offset: 0,
            visible_height: 20,
            total_rows: 0,
            sort_column: 0,
            sort_ascending: true,
        }
    }
}

impl TableState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the total row count (call before render).
    pub fn set_total(&mut self, total: usize) {
        self.total_rows = total;
        if self.selected >= total && total > 0 {
            self.selected = total - 1;
        }
    }

    /// Scroll down by one row.
    pub fn scroll_down(&mut self) {
        if self.total_rows == 0 {
            return;
        }
        if self.selected < self.total_rows - 1 {
            self.selected += 1;
        }
        // Keep selected visible.
        if self.selected >= self.offset + self.visible_height {
            self.offset = self.selected - self.visible_height + 1;
        }
    }

    /// Scroll up by one row.
    pub fn scroll_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
        if self.selected < self.offset {
            self.offset = self.selected;
        }
    }

    /// Jump to first row.
    pub fn scroll_top(&mut self) {
        self.selected = 0;
        self.offset = 0;
    }

    /// Jump to last row.
    pub fn scroll_bottom(&mut self) {
        if self.total_rows > 0 {
            self.selected = self.total_rows - 1;
            self.offset = self.total_rows.saturating_sub(self.visible_height);
        }
    }

    /// Page down.
    pub fn page_down(&mut self) {
        let jump = self.visible_height.saturating_sub(1).max(1);
        self.selected = (self.selected + jump).min(self.total_rows.saturating_sub(1));
        self.offset = self.selected.saturating_sub(self.visible_height.saturating_sub(1));
    }

    /// Page up.
    pub fn page_up(&mut self) {
        let jump = self.visible_height.saturating_sub(1).max(1);
        self.selected = self.selected.saturating_sub(jump);
        if self.selected < self.offset {
            self.offset = self.selected;
        }
    }

    /// Cycle sort column.
    pub fn next_sort(&mut self, num_columns: usize) {
        if num_columns == 0 {
            return;
        }
        if self.sort_column + 1 < num_columns {
            self.sort_column += 1;
        } else {
            self.sort_column = 0;
            self.sort_ascending = !self.sort_ascending;
        }
    }

    /// Handle common navigation keys. Returns `true` if consumed.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_down();
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_up();
                true
            }
            KeyCode::Char('g') => {
                self.scroll_top();
                true
            }
            KeyCode::Char('G') => {
                self.scroll_bottom();
                true
            }
            KeyCode::PageDown => {
                self.page_down();
                true
            }
            KeyCode::PageUp => {
                self.page_up();
                true
            }
            _ => false,
        }
    }

    /// Visible row range for slicing data.
    pub fn visible_range(&self) -> std::ops::Range<usize> {
        let end = (self.offset + self.visible_height).min(self.total_rows);
        self.offset..end
    }

    /// Convert ratatui table state from our state.
    pub fn to_ratatui_state(&self) -> ratatui::widgets::TableState {
        let mut state = ratatui::widgets::TableState::default();
        if self.total_rows > 0 {
            state.select(Some(self.selected - self.offset));
        }
        state
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
    fn default_state() {
        let ts = TableState::new();
        assert_eq!(ts.selected, 0);
        assert_eq!(ts.offset, 0);
        assert_eq!(ts.visible_height, 20);
        assert_eq!(ts.total_rows, 0);
        assert_eq!(ts.sort_column, 0);
        assert!(ts.sort_ascending);
    }

    #[test]
    fn set_total_clamps_selected() {
        let mut ts = TableState::new();
        ts.total_rows = 10;
        ts.selected = 8;

        // Shrink total below selected
        ts.set_total(5);
        assert_eq!(ts.total_rows, 5);
        assert_eq!(ts.selected, 4); // Clamped to total - 1
    }

    #[test]
    fn set_total_does_not_clamp_when_within_range() {
        let mut ts = TableState::new();
        ts.selected = 3;
        ts.set_total(10);
        assert_eq!(ts.selected, 3);
    }

    #[test]
    fn scroll_down_increments_selected() {
        let mut ts = TableState::new();
        ts.set_total(10);
        ts.visible_height = 5;

        ts.scroll_down();
        assert_eq!(ts.selected, 1);
        assert_eq!(ts.offset, 0);
    }

    #[test]
    fn scroll_down_stops_at_last_row() {
        let mut ts = TableState::new();
        ts.set_total(3);
        ts.selected = 2;

        ts.scroll_down();
        assert_eq!(ts.selected, 2); // No change — already at end
    }

    #[test]
    fn scroll_down_adjusts_offset_when_past_visible() {
        let mut ts = TableState::new();
        ts.set_total(10);
        ts.visible_height = 3;
        ts.selected = 2; // At bottom of visible area
        ts.offset = 0;

        ts.scroll_down();
        assert_eq!(ts.selected, 3);
        assert_eq!(ts.offset, 1); // offset adjusts to keep selected visible
    }

    #[test]
    fn scroll_down_with_zero_rows_does_nothing() {
        let mut ts = TableState::new();
        ts.set_total(0);
        ts.scroll_down();
        assert_eq!(ts.selected, 0);
    }

    #[test]
    fn scroll_up_decrements_selected() {
        let mut ts = TableState::new();
        ts.set_total(10);
        ts.selected = 5;

        ts.scroll_up();
        assert_eq!(ts.selected, 4);
    }

    #[test]
    fn scroll_up_stops_at_zero() {
        let mut ts = TableState::new();
        ts.set_total(10);
        ts.selected = 0;

        ts.scroll_up();
        assert_eq!(ts.selected, 0);
    }

    #[test]
    fn scroll_up_adjusts_offset() {
        let mut ts = TableState::new();
        ts.set_total(10);
        ts.visible_height = 3;
        ts.selected = 3;
        ts.offset = 3;

        ts.scroll_up();
        assert_eq!(ts.selected, 2);
        assert_eq!(ts.offset, 2); // Adjusts to keep selected visible
    }

    #[test]
    fn scroll_top_resets_to_zero() {
        let mut ts = TableState::new();
        ts.set_total(10);
        ts.selected = 7;
        ts.offset = 5;

        ts.scroll_top();
        assert_eq!(ts.selected, 0);
        assert_eq!(ts.offset, 0);
    }

    #[test]
    fn scroll_bottom_jumps_to_last_row() {
        let mut ts = TableState::new();
        ts.set_total(10);
        ts.visible_height = 3;

        ts.scroll_bottom();
        assert_eq!(ts.selected, 9);
        assert_eq!(ts.offset, 7); // 10 - 3
    }

    #[test]
    fn scroll_bottom_with_zero_rows_does_nothing() {
        let mut ts = TableState::new();
        ts.set_total(0);
        ts.scroll_bottom();
        assert_eq!(ts.selected, 0);
        assert_eq!(ts.offset, 0);
    }

    #[test]
    fn page_down_jumps_visible_height() {
        let mut ts = TableState::new();
        ts.set_total(50);
        ts.visible_height = 10;
        ts.selected = 0;

        ts.page_down();
        assert_eq!(ts.selected, 9); // visible_height - 1
    }

    #[test]
    fn page_down_clamps_to_last_row() {
        let mut ts = TableState::new();
        ts.set_total(5);
        ts.visible_height = 10;
        ts.selected = 3;

        ts.page_down();
        assert_eq!(ts.selected, 4); // Last row
    }

    #[test]
    fn page_up_jumps_visible_height() {
        let mut ts = TableState::new();
        ts.set_total(50);
        ts.visible_height = 10;
        ts.selected = 20;
        ts.offset = 15;

        ts.page_up();
        assert_eq!(ts.selected, 11); // 20 - 9
    }

    #[test]
    fn page_up_clamps_to_zero() {
        let mut ts = TableState::new();
        ts.set_total(50);
        ts.visible_height = 10;
        ts.selected = 3;
        ts.offset = 0;

        ts.page_up();
        assert_eq!(ts.selected, 0);
    }

    #[test]
    fn next_sort_cycles_columns() {
        let mut ts = TableState::new();
        assert_eq!(ts.sort_column, 0);
        assert!(ts.sort_ascending);

        ts.next_sort(3);
        assert_eq!(ts.sort_column, 1);
        assert!(ts.sort_ascending);

        ts.next_sort(3);
        assert_eq!(ts.sort_column, 2);
        assert!(ts.sort_ascending);

        // Wraps around and flips sort direction
        ts.next_sort(3);
        assert_eq!(ts.sort_column, 0);
        assert!(!ts.sort_ascending);
    }

    #[test]
    fn next_sort_zero_columns_does_nothing() {
        let mut ts = TableState::new();
        ts.next_sort(0);
        assert_eq!(ts.sort_column, 0);
    }

    #[test]
    fn handle_key_j_scrolls_down() {
        let mut ts = TableState::new();
        ts.set_total(10);
        assert!(ts.handle_key(key(KeyCode::Char('j'))));
        assert_eq!(ts.selected, 1);
    }

    #[test]
    fn handle_key_k_scrolls_up() {
        let mut ts = TableState::new();
        ts.set_total(10);
        ts.selected = 5;
        assert!(ts.handle_key(key(KeyCode::Char('k'))));
        assert_eq!(ts.selected, 4);
    }

    #[test]
    fn handle_key_g_scrolls_top() {
        let mut ts = TableState::new();
        ts.set_total(10);
        ts.selected = 5;
        assert!(ts.handle_key(key(KeyCode::Char('g'))));
        assert_eq!(ts.selected, 0);
    }

    #[test]
    fn handle_key_shift_g_scrolls_bottom() {
        let mut ts = TableState::new();
        ts.set_total(10);
        ts.visible_height = 5;
        assert!(ts.handle_key(key(KeyCode::Char('G'))));
        assert_eq!(ts.selected, 9);
    }

    #[test]
    fn handle_key_arrow_keys() {
        let mut ts = TableState::new();
        ts.set_total(10);
        assert!(ts.handle_key(key(KeyCode::Down)));
        assert_eq!(ts.selected, 1);
        assert!(ts.handle_key(key(KeyCode::Up)));
        assert_eq!(ts.selected, 0);
    }

    #[test]
    fn handle_key_page_up_down() {
        let mut ts = TableState::new();
        ts.set_total(50);
        ts.visible_height = 10;
        assert!(ts.handle_key(key(KeyCode::PageDown)));
        assert_eq!(ts.selected, 9);
        assert!(ts.handle_key(key(KeyCode::PageUp)));
        assert_eq!(ts.selected, 0);
    }

    #[test]
    fn handle_key_unrecognized_returns_false() {
        let mut ts = TableState::new();
        ts.set_total(10);
        assert!(!ts.handle_key(key(KeyCode::Char('x'))));
    }

    #[test]
    fn visible_range_basic() {
        let mut ts = TableState::new();
        ts.set_total(50);
        ts.visible_height = 10;
        ts.offset = 5;

        let range = ts.visible_range();
        assert_eq!(range, 5..15);
    }

    #[test]
    fn visible_range_clamps_to_total() {
        let mut ts = TableState::new();
        ts.set_total(3);
        ts.visible_height = 10;
        ts.offset = 0;

        let range = ts.visible_range();
        assert_eq!(range, 0..3);
    }

    #[test]
    fn visible_range_empty() {
        let ts = TableState::new();
        let range = ts.visible_range();
        assert_eq!(range, 0..0);
    }
}
