//! Behavioral cloning (VBR) status screen — read-only viewer.

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

const FETCH_INTERVAL: u64 = 30;

pub struct BehavioralCloningScreen {
    data: Option<serde_json::Value>,
    error: Option<String>,
    last_fetch: Option<Instant>,
}

impl BehavioralCloningScreen {
    pub fn new() -> Self {
        Self {
            data: None,
            error: None,
            last_fetch: None,
        }
    }
}

impl Screen for BehavioralCloningScreen {
    fn title(&self) -> &str {
        "VBR"
    }

    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let Some(ref data) = self.data else {
            let loading = Paragraph::new("Loading VBR status...").style(Theme::dim()).block(
                Block::default()
                    .title(" Behavioral Cloning (VBR) ")
                    .borders(Borders::ALL)
                    .border_style(Theme::dim()),
            );
            frame.render_widget(loading, area);
            return;
        };

        let block = Block::default()
            .title(" Behavioral Cloning (VBR) ")
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());

        let enabled = data.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
        let model_count = data.get("model_count").and_then(|v| v.as_u64()).unwrap_or(0);
        let training_status =
            data.get("training_status").and_then(|v| v.as_str()).unwrap_or("unknown");
        let accuracy = data
            .get("accuracy")
            .and_then(|v| v.as_f64())
            .map(|a| format!("{a:.1}%"))
            .unwrap_or_else(|| "--".to_string());

        let status_text = if enabled { "ENABLED" } else { "DISABLED" };
        let status_color = if enabled {
            Theme::STATUS_UP
        } else {
            Theme::STATUS_DOWN
        };

        let lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Status:          ", Theme::dim()),
                Span::styled(status_text, Style::default().fg(status_color)),
            ]),
            Line::from(vec![
                Span::styled("  Models:          ", Theme::dim()),
                Span::styled(model_count.to_string(), Style::default().fg(Theme::FG)),
            ]),
            Line::from(vec![
                Span::styled("  Training:        ", Theme::dim()),
                Span::styled(training_status.to_string(), Style::default().fg(Theme::FG)),
            ]),
            Line::from(vec![
                Span::styled("  Accuracy:        ", Theme::dim()),
                Span::styled(accuracy, Style::default().fg(Theme::FG)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  Virtual Branch Resilience learns response patterns",
                Theme::dim(),
            )),
            Line::from(Span::styled(
                "  from recorded traffic to generate realistic mocks.",
                Theme::dim(),
            )),
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
            match client.get_vbr_status().await {
                Ok(data) => {
                    let json = serde_json::to_string(&data).unwrap_or_default();
                    let _ = tx.send(Event::Data {
                        screen: "behavioral_cloning",
                        payload: json,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Event::ApiError {
                        screen: "behavioral_cloning",
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
