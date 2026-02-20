//! Time travel screen — shows enabled/disabled, current time, time scale.

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
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
use crate::widgets::confirm::ConfirmDialog;

const FETCH_INTERVAL: u64 = 5;

pub struct TimeTravelScreen {
    data: Option<serde_json::Value>,
    error: Option<String>,
    last_fetch: Option<Instant>,
    pending_toggle: bool,
    confirm: ConfirmDialog,
}

impl TimeTravelScreen {
    pub fn new() -> Self {
        Self {
            data: None,
            error: None,
            last_fetch: None,
            pending_toggle: false,
            confirm: ConfirmDialog::new(),
        }
    }
}

impl Screen for TimeTravelScreen {
    fn title(&self) -> &str {
        "Time Travel"
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Confirm dialog takes priority when visible.
        if self.confirm.visible {
            if let Some(confirmed) = self.confirm.handle_key(key) {
                if confirmed {
                    self.pending_toggle = true;
                }
                return true;
            }
            return true;
        }

        match key.code {
            KeyCode::Char('t') => {
                let enabled = self
                    .data
                    .as_ref()
                    .and_then(|d| d.get("enabled"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let action = if enabled { "disable" } else { "enable" };
                self.confirm.show(
                    "Toggle Time Travel",
                    format!("Are you sure you want to {action} time travel?"),
                );
                true
            }
            KeyCode::Char('r') => {
                self.last_fetch = None;
                true
            }
            _ => false,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let Some(ref data) = self.data else {
            let loading =
                Paragraph::new("Loading time travel status...").style(Theme::dim()).block(
                    Block::default()
                        .title(" Time Travel ")
                        .borders(Borders::ALL)
                        .border_style(Theme::dim()),
                );
            frame.render_widget(loading, area);
            return;
        };

        let block = Block::default()
            .title(" Time Travel ")
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());

        let enabled = data.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
        let current_time = data.get("current_time").and_then(|v| v.as_str()).unwrap_or("--");
        let time_scale = data
            .get("time_scale")
            .and_then(|v| v.as_f64())
            .map(|s| format!("{s:.1}x"))
            .unwrap_or_else(|| "--".to_string());

        let status_text = if enabled { "ENABLED" } else { "DISABLED" };
        let status_color = if enabled {
            Theme::STATUS_UP
        } else {
            Theme::STATUS_DOWN
        };

        let lines = vec![
            Line::from(vec![
                Span::styled("  Status:       ", Theme::dim()),
                Span::styled(status_text, Style::default().fg(status_color)),
            ]),
            Line::from(vec![
                Span::styled("  Current Time: ", Theme::dim()),
                Span::styled(current_time.to_string(), Style::default().fg(Theme::FG)),
            ]),
            Line::from(vec![
                Span::styled("  Time Scale:   ", Theme::dim()),
                Span::styled(time_scale, Style::default().fg(Theme::FG)),
            ]),
        ];

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);

        self.confirm.render(frame);
    }

    fn tick(&mut self, client: &MockForgeClient, tx: &mpsc::UnboundedSender<Event>) {
        // Handle pending toggle action.
        if self.pending_toggle {
            self.pending_toggle = false;
            let current_enabled = self
                .data
                .as_ref()
                .and_then(|d| d.get("enabled"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let client = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let result = if current_enabled {
                    client.disable_time_travel().await
                } else {
                    client.enable_time_travel().await
                };
                match result {
                    Ok(_) => {
                        // Refetch to get updated state from server.
                        match client.get_time_travel_status().await {
                            Ok(data) => {
                                let json = serde_json::json!({
                                    "enabled": data.enabled,
                                    "current_time": data.current_time,
                                    "time_scale": data.time_scale,
                                    "scheduled_responses": data.scheduled_responses,
                                });
                                let payload = serde_json::to_string(&json).unwrap_or_default();
                                let _ = tx.send(Event::Data {
                                    screen: "time_travel",
                                    payload,
                                });
                            }
                            Err(e) => {
                                let _ = tx.send(Event::ApiError {
                                    screen: "time_travel",
                                    message: e.to_string(),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Event::ApiError {
                            screen: "time_travel",
                            message: format!("Toggle failed: {e}"),
                        });
                    }
                }
            });
            return;
        }

        let should_fetch =
            self.last_fetch.map_or(true, |t| t.elapsed().as_secs() >= FETCH_INTERVAL);
        if !should_fetch {
            return;
        }
        self.last_fetch = Some(Instant::now());

        let client = client.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            match client.get_time_travel_status().await {
                Ok(data) => {
                    let json = serde_json::json!({
                        "enabled": data.enabled,
                        "current_time": data.current_time,
                        "time_scale": data.time_scale,
                        "scheduled_responses": data.scheduled_responses,
                    });
                    let payload = serde_json::to_string(&json).unwrap_or_default();
                    let _ = tx.send(Event::Data {
                        screen: "time_travel",
                        payload,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Event::ApiError {
                        screen: "time_travel",
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
        "t:toggle  r:refresh"
    }
}
