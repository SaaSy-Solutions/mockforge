//! Metrics screen — percentile charts and request rates.

use std::time::Instant;

use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tokio::sync::mpsc;

use crate::api::client::MockForgeClient;
use crate::event::Event;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::widgets::bar_chart;

const FETCH_INTERVAL: u64 = 5;

pub struct MetricsScreen {
    data: Option<serde_json::Value>,
    error: Option<String>,
    last_fetch: Option<Instant>,
}

impl MetricsScreen {
    pub fn new() -> Self {
        Self {
            data: None,
            error: None,
            last_fetch: None,
        }
    }
}

impl Screen for MetricsScreen {
    fn title(&self) -> &str {
        "Metrics"
    }

    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let Some(ref data) = self.data else {
            let loading = Paragraph::new("Loading metrics...").style(Theme::dim()).block(
                Block::default()
                    .title(" Metrics ")
                    .borders(Borders::ALL)
                    .border_style(Theme::dim()),
            );
            frame.render_widget(loading, area);
            return;
        };

        let rows = Layout::vertical([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(area);

        // Response time percentiles
        self.render_percentiles(frame, rows[0], data);
        // Requests by endpoint
        self.render_requests_by_endpoint(frame, rows[1], data);
        // Error rate by endpoint
        self.render_error_rates(frame, rows[2], data);
    }

    fn tick(&mut self, client: &MockForgeClient, tx: &mpsc::UnboundedSender<Event>) {
        let should_fetch =
            self.last_fetch.map_or(true, |t| t.elapsed().as_secs() >= FETCH_INTERVAL);
        if !should_fetch {
            return;
        }
        self.last_fetch = Some(Instant::now());

        let client = client.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            match client.get_metrics().await {
                Ok(data) => {
                    // Convert to Value for generic storage.
                    let json = serde_json::json!({
                        "response_time_percentiles": data.response_time_percentiles,
                        "requests_by_endpoint": data.requests_by_endpoint,
                        "error_rate_by_endpoint": data.error_rate_by_endpoint,
                    });
                    let payload = serde_json::to_string(&json).unwrap_or_default();
                    let _ = tx.send(Event::Data {
                        screen: "metrics",
                        payload,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Event::ApiError {
                        screen: "metrics",
                        message: e.to_string(),
                    });
                }
            }
        });
    }

    fn on_data(&mut self, payload: &str) {
        match serde_json::from_str::<serde_json::Value>(payload) {
            Ok(data) => {
                self.data = Some(data);
                self.error = None;
            }
            Err(e) => {
                self.error = Some(format!("Parse error: {e}"));
            }
        }
    }

    fn on_error(&mut self, message: &str) {
        self.error = Some(message.to_string());
    }

    fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    fn force_refresh(&mut self) {
        self.last_fetch = None;
    }

    fn status_hint(&self) -> &str {
        "r:refresh"
    }
}

impl MetricsScreen {
    fn render_percentiles(&self, frame: &mut Frame, area: Rect, data: &serde_json::Value) {
        let block = Block::default()
            .title(" Response Time Percentiles ")
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());

        let mut lines = Vec::new();
        if let Some(percentiles) = data.get("response_time_percentiles").and_then(|v| v.as_object())
        {
            for (key, value) in percentiles {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {key:<8}"), Theme::dim()),
                    Span::styled(
                        format!("{}ms", value.as_u64().unwrap_or(0)),
                        Style::default().fg(Theme::FG),
                    ),
                ]));
            }
        }
        if lines.is_empty() {
            lines.push(Line::from(Span::styled(" No percentile data", Theme::dim())));
        }

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    fn render_requests_by_endpoint(&self, frame: &mut Frame, area: Rect, data: &serde_json::Value) {
        if let Some(endpoints) = data.get("requests_by_endpoint").and_then(|v| v.as_object()) {
            let entries: Vec<(&str, u64)> =
                endpoints.iter().map(|(k, v)| (k.as_str(), v.as_u64().unwrap_or(0))).collect();
            bar_chart::render(frame, area, "Requests by Endpoint", &entries, None);
        } else {
            let block = Block::default()
                .title(" Requests by Endpoint ")
                .title_style(Theme::title())
                .borders(Borders::ALL)
                .border_style(Theme::dim())
                .style(Theme::surface());
            let paragraph =
                Paragraph::new(Span::styled(" No endpoint data", Theme::dim())).block(block);
            frame.render_widget(paragraph, area);
        }
    }

    fn render_error_rates(&self, frame: &mut Frame, area: Rect, data: &serde_json::Value) {
        let block = Block::default()
            .title(" Error Rate by Endpoint ")
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());

        let mut lines = Vec::new();
        if let Some(errors) = data.get("error_rate_by_endpoint").and_then(|v| v.as_object()) {
            for (key, value) in errors {
                let rate = value.as_f64().unwrap_or(0.0);
                let style = if rate > 0.05 {
                    Theme::error()
                } else {
                    Theme::success()
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("  {key:<30}"), Style::default().fg(Theme::FG)),
                    Span::styled(format!("{:.1}%", rate * 100.0), style),
                ]));
            }
        }
        if lines.is_empty() {
            lines.push(Line::from(Span::styled(" No error rate data", Theme::dim())));
        }

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }
}
