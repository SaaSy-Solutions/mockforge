//! Analytics summary screen — total requests, unique endpoints, error rate.

use std::time::Instant;

use crossterm::event::KeyEvent;
use ratatui::{
    layout::Rect,
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

const FETCH_INTERVAL: u64 = 10;

pub struct AnalyticsScreen {
    data: Option<serde_json::Value>,
    error: Option<String>,
    last_fetch: Option<Instant>,
}

impl AnalyticsScreen {
    pub fn new() -> Self {
        Self {
            data: None,
            error: None,
            last_fetch: None,
        }
    }
}

impl Screen for AnalyticsScreen {
    fn title(&self) -> &str {
        "Analytics"
    }

    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let Some(ref data) = self.data else {
            let loading = Paragraph::new("Loading analytics...").style(Theme::dim()).block(
                Block::default()
                    .title(" Analytics ")
                    .borders(Borders::ALL)
                    .border_style(Theme::dim()),
            );
            frame.render_widget(loading, area);
            return;
        };

        let block = Block::default()
            .title(" Analytics Summary ")
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());

        let total_requests = data.get("total_requests").and_then(|v| v.as_u64()).unwrap_or(0);
        let unique_endpoints = data.get("unique_endpoints").and_then(|v| v.as_u64()).unwrap_or(0);
        let error_rate = data.get("error_rate").and_then(|v| v.as_f64()).unwrap_or(0.0);

        let error_style = if error_rate > 0.05 {
            Theme::error()
        } else {
            Theme::success()
        };

        let lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Total Requests:    ", Theme::dim()),
                Span::styled(total_requests.to_string(), Style::default().fg(Theme::FG)),
            ]),
            Line::from(vec![
                Span::styled("  Unique Endpoints:  ", Theme::dim()),
                Span::styled(unique_endpoints.to_string(), Style::default().fg(Theme::FG)),
            ]),
            Line::from(vec![
                Span::styled("  Error Rate:        ", Theme::dim()),
                Span::styled(format!("{:.1}%", error_rate * 100.0), error_style),
            ]),
        ];

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
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
            match client.get_analytics_summary().await {
                Ok(data) => {
                    let json = serde_json::json!({
                        "total_requests": data.total_requests,
                        "unique_endpoints": data.unique_endpoints,
                        "error_rate": data.error_rate,
                        "avg_response_time": data.avg_response_time,
                    });
                    let payload = serde_json::to_string(&json).unwrap_or_default();
                    let _ = tx.send(Event::Data {
                        screen: "analytics",
                        payload,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Event::ApiError {
                        screen: "analytics",
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
