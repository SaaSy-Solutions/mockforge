//! Health check screen — service health status and K8s probe results.

use std::time::Instant;

use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tokio::sync::mpsc;

use crate::api::client::MockForgeClient;
use crate::api::models::{HealthCheck, ServerInfo};
use crate::event::Event;
use crate::screens::Screen;
use crate::theme::Theme;

pub struct HealthScreen {
    health: Option<HealthCheck>,
    server_info: Option<ServerInfo>,
    error: Option<String>,
    last_fetch: Option<Instant>,
}

impl HealthScreen {
    pub fn new() -> Self {
        Self {
            health: None,
            server_info: None,
            error: None,
            last_fetch: None,
        }
    }
}

impl Screen for HealthScreen {
    fn title(&self) -> &str {
        "Health"
    }

    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        self.render_health(frame, chunks[0]);
        self.render_server_info(frame, chunks[1]);
    }

    fn tick(&mut self, client: &MockForgeClient, tx: &mpsc::UnboundedSender<Event>) {
        let should_fetch = self.last_fetch.map_or(true, |t| t.elapsed().as_secs() >= 10);
        if !should_fetch {
            return;
        }
        self.last_fetch = Some(Instant::now());

        let client = client.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            let health = client.get_health().await;
            let server_info = client.get_server_info().await;

            let payload = serde_json::json!({
                "health": health.ok(),
                "server_info": server_info.ok(),
            });
            let _ = tx.send(Event::Data {
                screen: "health",
                payload: payload.to_string(),
            });
        });
    }

    fn on_data(&mut self, payload: &str) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) {
            if let Some(health) = value.get("health") {
                if !health.is_null() {
                    self.health = serde_json::from_value(health.clone()).ok();
                }
            }
            if let Some(info) = value.get("server_info") {
                if !info.is_null() {
                    self.server_info = serde_json::from_value(info.clone()).ok();
                }
            }
            self.error = None;
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

impl HealthScreen {
    fn render_health(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Health Status ")
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());

        let mut lines = Vec::new();

        if let Some(ref err) = self.error {
            lines.push(Line::from(Span::styled(format!(" Error: {err}"), Theme::error())));
        }

        if let Some(ref health) = self.health {
            let status_color = match health.status.as_str() {
                "healthy" | "ok" => Theme::STATUS_UP,
                "degraded" => Theme::STATUS_WARN,
                _ => Theme::STATUS_DOWN,
            };
            lines.push(Line::from(vec![
                Span::styled(" Overall: ", Theme::dim()),
                Span::styled(
                    &health.status,
                    Style::default().fg(status_color).add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(""));

            // Individual services
            for (name, status) in &health.services {
                let color = match status.as_str() {
                    "healthy" | "ok" | "up" => Theme::STATUS_UP,
                    "degraded" => Theme::STATUS_WARN,
                    _ => Theme::STATUS_DOWN,
                };
                let icon = if status == "healthy" || status == "ok" || status == "up" {
                    "●"
                } else {
                    "○"
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("  {icon} "), Style::default().fg(color)),
                    Span::styled(format!("{name:<20}"), Style::default().fg(Theme::FG)),
                    Span::styled(status.as_str(), Style::default().fg(color)),
                ]));
            }

            if !health.issues.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(" Issues:", Theme::error())));
                for issue in &health.issues {
                    lines.push(Line::from(Span::styled(format!("  • {issue}"), Theme::error())));
                }
            }
        } else {
            lines.push(Line::from(Span::styled(" Loading health data…", Theme::dim())));
        }

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    fn render_server_info(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Server Info ")
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());

        let mut lines = Vec::new();

        if let Some(ref info) = self.server_info {
            let kv = |label: &str, value: &str| -> Line<'static> {
                Line::from(vec![
                    Span::styled(format!(" {label:<16}"), Theme::dim()),
                    Span::styled(value.to_string(), Style::default().fg(Theme::FG)),
                ])
            };

            lines.push(kv("Version:", &info.version));
            lines.push(kv("Build:", &info.build_time));
            lines.push(kv("Git SHA:", &info.git_sha));
            lines.push(kv("Admin Port:", &info.admin_port.to_string()));
            lines.push(kv("API Enabled:", &info.api_enabled.to_string()));
            lines.push(Line::from(""));

            if let Some(ref addr) = info.http_server {
                lines.push(kv("HTTP:", addr));
            }
            if let Some(ref addr) = info.ws_server {
                lines.push(kv("WebSocket:", addr));
            }
            if let Some(ref addr) = info.grpc_server {
                lines.push(kv("gRPC:", addr));
            }
            if let Some(ref addr) = info.graphql_server {
                lines.push(kv("GraphQL:", addr));
            }
        } else {
            lines.push(Line::from(Span::styled(" Loading server info…", Theme::dim())));
        }

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }
}
