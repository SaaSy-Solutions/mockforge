//! Bottom status bar: connection indicator, screen-specific hints, error count.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::theme::Theme;

/// Render the status bar at the bottom of the terminal.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    connected: bool,
    screen_hint: &str,
    error_count: usize,
    admin_url: &str,
) {
    let chunks = Layout::horizontal([
        Constraint::Length(30),
        Constraint::Min(0),
        Constraint::Length(20),
    ])
    .split(area);

    // Left: connection status
    let status_icon = if connected { "●" } else { "○" };
    let status_color = if connected {
        Theme::STATUS_UP
    } else {
        Theme::STATUS_DOWN
    };
    let conn = Line::from(vec![
        Span::styled(format!(" {status_icon} "), ratatui::style::Style::default().fg(status_color)),
        Span::styled(admin_url, Theme::status_bar()),
    ]);
    frame.render_widget(Paragraph::new(conn).style(Theme::status_bar()), chunks[0]);

    // Center: keybinding hints
    let hints = Line::from(vec![
        Span::styled("q", Theme::key_hint()),
        Span::styled(":quit ", Theme::status_bar()),
        Span::styled("r", Theme::key_hint()),
        Span::styled(":refresh ", Theme::status_bar()),
        Span::styled("/", Theme::key_hint()),
        Span::styled(":filter ", Theme::status_bar()),
        Span::styled("?", Theme::key_hint()),
        Span::styled(":help ", Theme::status_bar()),
        Span::styled("Tab", Theme::key_hint()),
        Span::styled(":next ", Theme::status_bar()),
        if screen_hint.is_empty() {
            Span::raw("")
        } else {
            Span::styled(screen_hint, Theme::dim())
        },
    ]);
    frame.render_widget(Paragraph::new(hints).style(Theme::status_bar()), chunks[1]);

    // Right: error count
    let err_style = if error_count > 0 {
        Theme::error()
    } else {
        Theme::dim()
    };
    let err_text = Line::from(Span::styled(format!("{error_count} errs "), err_style));
    frame.render_widget(
        Paragraph::new(err_text)
            .style(Theme::status_bar())
            .alignment(ratatui::layout::Alignment::Right),
        chunks[2],
    );
}
