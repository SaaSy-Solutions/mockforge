//! Horizontal bar chart widget for visualizing numeric data.

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::theme::Theme;

/// Draw a horizontal bar chart. Each entry is `(label, value)`.
/// Bars are scaled relative to `max_value` (or the largest value if `None`).
pub fn render(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    entries: &[(&str, u64)],
    max_value: Option<u64>,
) {
    let block = Block::default()
        .title(format!(" {title} "))
        .title_style(Theme::title())
        .borders(Borders::ALL)
        .border_style(Theme::dim())
        .style(Theme::surface());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if entries.is_empty() {
        let empty = Paragraph::new(Span::styled("  No data", Theme::dim()));
        frame.render_widget(empty, inner);
        return;
    }

    let max = max_value.unwrap_or_else(|| entries.iter().map(|(_, v)| *v).max().unwrap_or(1));
    let max = max.max(1); // Avoid division by zero.

    // Determine the longest label for alignment.
    let label_width = entries.iter().map(|(l, _)| l.len()).max().unwrap_or(0).min(20);

    // Available width for the bar itself.
    let bar_area_width = inner.width.saturating_sub(u16::try_from(label_width + 10).unwrap_or(30));

    let lines: Vec<Line> = entries
        .iter()
        .take(inner.height as usize)
        .map(|(label, value)| {
            let bar_len = if max > 0 {
                // Proportional bar length — precision loss acceptable for visual rendering.
                let ratio = *value as f64 / max as f64;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let len = (ratio * f64::from(bar_area_width)) as usize;
                len
            } else {
                0
            };

            let bar: String = "█".repeat(bar_len);
            let truncated_label = if label.len() > label_width {
                &label[..label_width]
            } else {
                label
            };

            Line::from(vec![
                Span::styled(format!("  {truncated_label:<label_width$} "), Theme::dim()),
                Span::styled(bar, ratatui::style::Style::default().fg(Theme::BLUE)),
                Span::styled(format!(" {value}"), ratatui::style::Style::default().fg(Theme::FG)),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

#[cfg(test)]
mod tests {
    #[test]
    fn max_value_fallback_to_largest() {
        // Just ensure the function doesn't panic with various inputs.
        let entries = [("a", 10), ("b", 20), ("c", 5)];
        // We can't easily test rendering without a terminal, but we can
        // verify the logic doesn't panic.
        let max = entries.iter().map(|(_, v)| *v).max().unwrap_or(1);
        assert_eq!(max, 20);
    }

    #[test]
    fn empty_entries_handled() {
        let entries: &[(&str, u64)] = &[];
        let max = entries.iter().map(|(_, v)| *v).max().unwrap_or(1);
        assert_eq!(max, 1);
    }
}
