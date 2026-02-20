//! Collapsible JSON tree viewer widget.

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::theme::Theme;

/// Render a JSON value as a flat list of indented lines.
pub fn render(frame: &mut Frame, area: Rect, title: &str, value: &serde_json::Value) {
    render_scrollable(frame, area, title, value, 0);
}

/// Render a JSON value with vertical scroll offset.
pub fn render_scrollable(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    value: &serde_json::Value,
    scroll_offset: u16,
) {
    let lines = json_to_lines(value, 0);

    let block = Block::default()
        .title(format!(" {title} "))
        .title_style(Theme::title())
        .borders(Borders::ALL)
        .border_style(Theme::dim())
        .style(Theme::surface());

    let paragraph = Paragraph::new(lines).block(block).scroll((scroll_offset, 0));
    frame.render_widget(paragraph, area);
}

fn json_to_lines(value: &serde_json::Value, depth: usize) -> Vec<Line<'static>> {
    let indent = "  ".repeat(depth);
    match value {
        serde_json::Value::Object(map) => {
            let mut lines = Vec::new();
            for (key, val) in map {
                match val {
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        lines.push(Line::from(vec![
                            Span::raw(indent.clone()),
                            Span::styled(format!("{key}: "), Theme::key_hint()),
                        ]));
                        lines.extend(json_to_lines(val, depth + 1));
                    }
                    _ => {
                        lines.push(Line::from(vec![
                            Span::raw(indent.clone()),
                            Span::styled(format!("{key}: "), Theme::key_hint()),
                            value_span(val),
                        ]));
                    }
                }
            }
            lines
        }
        serde_json::Value::Array(arr) => {
            let mut lines = Vec::new();
            for (i, val) in arr.iter().enumerate() {
                match val {
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        lines.push(Line::from(vec![
                            Span::raw(indent.clone()),
                            Span::styled(format!("[{i}]:"), Theme::dim()),
                        ]));
                        lines.extend(json_to_lines(val, depth + 1));
                    }
                    _ => {
                        lines.push(Line::from(vec![
                            Span::raw(indent.clone()),
                            Span::styled(format!("[{i}]: "), Theme::dim()),
                            value_span(val),
                        ]));
                    }
                }
            }
            lines
        }
        _ => {
            vec![Line::from(vec![Span::raw(indent), value_span(value)])]
        }
    }
}

fn value_span(value: &serde_json::Value) -> Span<'static> {
    match value {
        serde_json::Value::String(s) => {
            Span::styled(format!("\"{s}\""), ratatui::style::Style::default().fg(Theme::GREEN))
        }
        serde_json::Value::Number(n) => {
            Span::styled(n.to_string(), ratatui::style::Style::default().fg(Theme::PEACH))
        }
        serde_json::Value::Bool(b) => {
            let color = if *b { Theme::GREEN } else { Theme::RED };
            Span::styled(b.to_string(), ratatui::style::Style::default().fg(color))
        }
        serde_json::Value::Null => Span::styled("null", Theme::dim()),
        _ => Span::raw(value.to_string()),
    }
}
