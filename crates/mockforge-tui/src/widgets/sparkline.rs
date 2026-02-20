//! Sparkline wrapper for inline trend visualization.

use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Sparkline as RatatuiSparkline},
    Frame,
};

use crate::theme::Theme;

/// Render a sparkline chart with title inside a bordered block.
pub fn render(frame: &mut Frame, area: Rect, title: &str, data: &[u64]) {
    let block = Block::default()
        .title(format!(" {title} "))
        .title_style(Theme::title())
        .borders(Borders::ALL)
        .border_style(Theme::dim())
        .style(Theme::surface());

    let sparkline = RatatuiSparkline::default()
        .block(block)
        .data(data)
        .style(ratatui::style::Style::default().fg(Theme::BLUE));

    frame.render_widget(sparkline, area);
}

/// Render a compact sparkline with a label and current value inline.
pub fn render_inline(frame: &mut Frame, area: Rect, label: &str, data: &[u64], suffix: &str) {
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;

    let current = data.last().copied().unwrap_or(0);

    // Use unicode braille characters for a compact inline sparkline.
    let spark_chars = data_to_spark_string(
        data,
        area.width
            .saturating_sub(u16::try_from(label.len() + suffix.len() + 10).unwrap_or(20))
            as usize,
    );

    let line = Line::from(vec![
        Span::styled(format!("  {label}: "), Theme::dim()),
        Span::styled(spark_chars, ratatui::style::Style::default().fg(Theme::BLUE)),
        Span::styled(format!(" {current}{suffix}"), ratatui::style::Style::default().fg(Theme::FG)),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

/// Convert data points to a string of spark characters (▁▂▃▄▅▆▇█).
fn data_to_spark_string(data: &[u64], max_width: usize) -> String {
    const SPARKS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    if data.is_empty() {
        return String::new();
    }

    let max = *data.iter().max().unwrap_or(&1);
    let max = max.max(1);

    // Take the last `max_width` data points.
    let start = data.len().saturating_sub(max_width);
    data[start..]
        .iter()
        .map(|&v| {
            // Proportional index — precision loss acceptable for visual rendering.
            let ratio = v as f64 / max as f64;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let idx = (ratio * (SPARKS.len() - 1) as f64) as usize;
            SPARKS[idx.min(SPARKS.len() - 1)]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_data_returns_empty_string() {
        assert_eq!(data_to_spark_string(&[], 10), "");
    }

    #[test]
    fn single_value_returns_one_char() {
        let result = data_to_spark_string(&[5], 10);
        assert_eq!(result.chars().count(), 1);
    }

    #[test]
    fn all_same_values() {
        let result = data_to_spark_string(&[5, 5, 5, 5], 10);
        assert_eq!(result.chars().count(), 4);
        // All chars should be the same (highest spark since max == value).
        let chars: Vec<char> = result.chars().collect();
        assert!(chars.iter().all(|c| *c == chars[0]));
    }

    #[test]
    fn ascending_values() {
        let result = data_to_spark_string(&[0, 1, 2, 3, 4, 5, 6, 7], 10);
        let chars: Vec<char> = result.chars().collect();
        assert_eq!(chars.len(), 8);
        // First should be lowest, last should be highest.
        assert_eq!(chars[0], '▁');
        assert_eq!(chars[7], '█');
    }

    #[test]
    fn truncates_to_max_width() {
        let data: Vec<u64> = (0..100).collect();
        let result = data_to_spark_string(&data, 10);
        assert_eq!(result.chars().count(), 10);
    }

    #[test]
    fn zero_data_produces_lowest_spark() {
        let result = data_to_spark_string(&[0, 0, 0], 10);
        let chars: Vec<char> = result.chars().collect();
        // All zeros maps to 0/1 = 0.0, which is the lowest spark character.
        assert!(chars.iter().all(|c| *c == '▁'));
    }
}
