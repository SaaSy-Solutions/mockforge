//! Help overlay popup showing all keybindings.

use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::theme::Theme;

const HELP_TEXT: &[(&str, &str)] = &[
    ("Global", ""),
    ("  q / Ctrl+C", "Quit"),
    ("  Tab / Shift+Tab", "Next / Previous tab"),
    ("  1-0", "Jump to tab 1-10"),
    ("  r", "Refresh current screen"),
    ("  /", "Open filter input"),
    ("  ?", "Toggle this help"),
    ("  :", "Command palette"),
    ("", ""),
    ("Navigation", ""),
    ("  j / ↓", "Scroll down"),
    ("  k / ↑", "Scroll up"),
    ("  g / G", "Jump to top / bottom"),
    ("  PgUp / PgDn", "Page up / down"),
    ("  Enter", "Select / expand"),
    ("  Esc", "Close popup / cancel"),
    ("", ""),
    ("Screen-specific", ""),
    ("  f", "Toggle follow mode (Logs)"),
    ("  e", "Edit selected item (Config)"),
    ("  t", "Toggle (Chaos, Time Travel)"),
    ("  s", "Sort column (Routes, Fixtures)"),
    ("  d", "Delete selected item"),
];

/// Render the help overlay centred on screen.
pub fn render(frame: &mut Frame) {
    let area = centered_rect(60, 70, frame.area());

    // Clear the background behind the popup.
    frame.render_widget(Clear, area);

    let lines: Vec<Line> = HELP_TEXT
        .iter()
        .map(|(key, desc)| {
            if desc.is_empty() {
                // Section header or blank line
                Line::from(Span::styled(*key, Theme::title()))
            } else {
                Line::from(vec![
                    Span::styled(format!("{key:<22}"), Theme::key_hint()),
                    Span::styled(*desc, Theme::base()),
                ])
            }
        })
        .collect();

    let block = Block::default()
        .title(" Help — press ? or Esc to close ")
        .title_style(Theme::title())
        .borders(Borders::ALL)
        .border_style(Theme::dim())
        .style(Theme::surface());

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

/// Return a centred `Rect` that takes `percent_x`% width and `percent_y`% height.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}
