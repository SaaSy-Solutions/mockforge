//! Side-by-side or inline diff viewer widget.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::theme::Theme;

/// A single line in a diff — added, removed, or unchanged.
#[derive(Debug, Clone)]
pub enum DiffLine {
    Added(String),
    Removed(String),
    Unchanged(String),
}

/// Render a unified diff view with color-coded add/remove lines.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    diff_lines: &[DiffLine],
    scroll_offset: u16,
) {
    let block = Block::default()
        .title(format!(" {title} "))
        .title_style(Theme::title())
        .borders(Borders::ALL)
        .border_style(Theme::dim())
        .style(Theme::surface());

    let lines: Vec<Line> = diff_lines
        .iter()
        .map(|dl| match dl {
            DiffLine::Added(text) => {
                Line::from(Span::styled(format!("+ {text}"), Style::default().fg(Theme::GREEN)))
            }
            DiffLine::Removed(text) => {
                Line::from(Span::styled(format!("- {text}"), Style::default().fg(Theme::RED)))
            }
            DiffLine::Unchanged(text) => {
                Line::from(Span::styled(format!("  {text}"), Style::default().fg(Theme::FG)))
            }
        })
        .collect();

    if lines.is_empty() {
        let empty = Paragraph::new(Span::styled("  No differences", Theme::dim()))
            .block(block)
            .scroll((scroll_offset, 0));
        frame.render_widget(empty, area);
    } else {
        let paragraph = Paragraph::new(lines).block(block).scroll((scroll_offset, 0));
        frame.render_widget(paragraph, area);
    }
}

/// Parse a unified diff string into `DiffLine` entries.
pub fn parse_unified_diff(text: &str) -> Vec<DiffLine> {
    text.lines()
        .map(|line| {
            if let Some(rest) = line.strip_prefix('+') {
                DiffLine::Added(rest.to_string())
            } else if let Some(rest) = line.strip_prefix('-') {
                DiffLine::Removed(rest.to_string())
            } else {
                DiffLine::Unchanged(line.strip_prefix(' ').unwrap_or(line).to_string())
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_diff() {
        let result = parse_unified_diff("");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_additions_and_removals() {
        let diff = "+added line\n-removed line\n unchanged line";
        let result = parse_unified_diff(diff);
        assert_eq!(result.len(), 3);
        assert!(matches!(&result[0], DiffLine::Added(s) if s == "added line"));
        assert!(matches!(&result[1], DiffLine::Removed(s) if s == "removed line"));
        assert!(matches!(&result[2], DiffLine::Unchanged(s) if s == "unchanged line"));
    }

    #[test]
    fn parse_mixed_diff() {
        let diff = " context\n+new\n-old\n context2";
        let result = parse_unified_diff(diff);
        assert_eq!(result.len(), 4);
        assert!(matches!(&result[0], DiffLine::Unchanged(s) if s == "context"));
        assert!(matches!(&result[1], DiffLine::Added(s) if s == "new"));
        assert!(matches!(&result[2], DiffLine::Removed(s) if s == "old"));
        assert!(matches!(&result[3], DiffLine::Unchanged(s) if s == "context2"));
    }
}
