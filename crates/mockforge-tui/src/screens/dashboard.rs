//! Dashboard screen — server status, system metrics, sparklines, recent logs.

use std::time::Instant;

use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Sparkline},
    Frame,
};
use tokio::sync::mpsc;

use crate::api::client::MockForgeClient;
use crate::api::models::DashboardData;
use crate::event::Event;
use crate::screens::Screen;
use crate::theme::Theme;

pub struct DashboardScreen {
    data: Option<DashboardData>,
    error: Option<String>,
    last_fetch: Option<Instant>,
    request_rate_history: Vec<u64>,
}

impl DashboardScreen {
    pub fn new() -> Self {
        Self {
            data: None,
            error: None,
            last_fetch: None,
            request_rate_history: Vec::new(),
        }
    }
}

impl Screen for DashboardScreen {
    fn title(&self) -> &str {
        "Dashboard"
    }

    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let Some(ref data) = self.data else {
            let loading = Paragraph::new("Loading dashboard…").style(Theme::dim()).block(
                Block::default()
                    .title(" Dashboard ")
                    .borders(Borders::ALL)
                    .border_style(Theme::dim()),
            );
            frame.render_widget(loading, area);
            return;
        };

        // Split into 2x2 quadrants.
        let rows =
            Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);
        let top = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[0]);
        let bottom = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);

        self.render_server_status(frame, top[0], data);
        self.render_system_metrics(frame, top[1], data);
        self.render_request_stats(frame, bottom[0], data);
        self.render_recent_logs(frame, bottom[1], data);
    }

    fn tick(&mut self, client: &MockForgeClient, tx: &mpsc::UnboundedSender<Event>) {
        let should_fetch = self.last_fetch.map_or(true, |t| t.elapsed().as_secs() >= 2);
        if !should_fetch {
            return;
        }
        self.last_fetch = Some(Instant::now());

        let client = client.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            match client.get_dashboard().await {
                Ok(data) => {
                    let json = serde_json::to_string(&data).unwrap_or_default();
                    let _ = tx.send(Event::Data {
                        screen: "dashboard",
                        payload: json,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Event::ApiError {
                        screen: "dashboard",
                        message: e.to_string(),
                    });
                }
            }
        });
    }

    fn on_data(&mut self, payload: &str) {
        match serde_json::from_str::<DashboardData>(payload) {
            Ok(data) => {
                // Track request rate for sparkline.
                self.request_rate_history.push(data.metrics.total_requests);
                if self.request_rate_history.len() > 60 {
                    self.request_rate_history.remove(0);
                }
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

impl DashboardScreen {
    fn render_server_status(&self, frame: &mut Frame, area: Rect, data: &DashboardData) {
        let block = Block::default()
            .title(" Server Status ")
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());

        let mut lines = Vec::new();
        for server in &data.servers {
            let status_icon = if server.running { "●" } else { "○" };
            let status_color = if server.running {
                Theme::STATUS_UP
            } else {
                Theme::STATUS_DOWN
            };
            let addr = server.address.as_deref().unwrap_or("—");
            lines.push(Line::from(vec![
                Span::styled(format!(" {status_icon} "), Style::default().fg(status_color)),
                Span::styled(
                    format!("{:<6}", server.server_type),
                    Style::default().fg(Theme::FG).add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(" {addr}"), Theme::dim()),
            ]));
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(" No servers detected", Theme::dim())));
        }

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    fn render_system_metrics(&self, frame: &mut Frame, area: Rect, data: &DashboardData) {
        let block = Block::default()
            .title(" System Metrics ")
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

        // CPU gauge
        let cpu = data.system.cpu_usage_percent;
        let cpu_color = if cpu > 80.0 {
            Theme::RED
        } else if cpu > 50.0 {
            Theme::YELLOW
        } else {
            Theme::GREEN
        };
        let cpu_label = format!("CPU: {cpu:.0}%");
        let cpu_gauge = Gauge::default()
            .label(cpu_label)
            .ratio(cpu / 100.0)
            .gauge_style(Style::default().fg(cpu_color).bg(Theme::OVERLAY));
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(" CPU", Theme::dim()))),
            chunks[0],
        );
        let gauge_area = Rect {
            y: chunks[0].y + 1,
            height: 1,
            ..chunks[0]
        };
        frame.render_widget(cpu_gauge, gauge_area);

        // Memory gauge
        let mem_mb = data.system.memory_usage_mb;
        let mem_label = format!("Mem: {mem_mb} MB");
        let mem_gauge = Gauge::default()
            .label(mem_label)
            .ratio((mem_mb as f64 / 1024.0).min(1.0))
            .gauge_style(Style::default().fg(Theme::TEAL).bg(Theme::OVERLAY));
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(" Memory", Theme::dim()))),
            chunks[1],
        );
        let gauge_area2 = Rect {
            y: chunks[1].y + 1,
            height: 1,
            ..chunks[1]
        };
        frame.render_widget(mem_gauge, gauge_area2);

        // Threads + other info
        let info = Line::from(vec![
            Span::styled(" Threads: ", Theme::dim()),
            Span::styled(data.system.active_threads.to_string(), Style::default().fg(Theme::FG)),
            Span::styled("  Routes: ", Theme::dim()),
            Span::styled(data.system.total_routes.to_string(), Style::default().fg(Theme::FG)),
            Span::styled("  Fixtures: ", Theme::dim()),
            Span::styled(data.system.total_fixtures.to_string(), Style::default().fg(Theme::FG)),
        ]);
        frame.render_widget(Paragraph::new(info), chunks[2]);
    }

    fn render_request_stats(&self, frame: &mut Frame, area: Rect, data: &DashboardData) {
        let block = Block::default()
            .title(" Request Stats ")
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(2)]).split(inner);

        // Stats summary
        let total = data.metrics.total_requests;
        let avg_rt = data.metrics.average_response_time;
        let err_rate = data.metrics.error_rate;

        let stats = vec![
            Line::from(vec![
                Span::styled(" Total: ", Theme::dim()),
                Span::styled(
                    format_number(total),
                    Style::default().fg(Theme::FG).add_modifier(Modifier::BOLD),
                ),
                Span::styled("  Err Rate: ", Theme::dim()),
                Span::styled(
                    format!("{:.1}%", err_rate * 100.0),
                    if err_rate > 0.05 {
                        Theme::error()
                    } else {
                        Theme::success()
                    },
                ),
            ]),
            Line::from(vec![
                Span::styled(" Avg RT: ", Theme::dim()),
                Span::styled(format!("{avg_rt:.0}ms"), Style::default().fg(Theme::FG)),
            ]),
        ];
        frame.render_widget(Paragraph::new(stats), chunks[0]);

        // Sparkline for request rate
        if !self.request_rate_history.is_empty() {
            let sparkline = Sparkline::default()
                .data(&self.request_rate_history)
                .style(Style::default().fg(Theme::BLUE));
            frame.render_widget(sparkline, chunks[1]);
        }
    }

    fn render_recent_logs(&self, frame: &mut Frame, area: Rect, data: &DashboardData) {
        let block = Block::default()
            .title(" Recent Logs ")
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());

        let lines: Vec<Line> = data
            .recent_logs
            .iter()
            .rev()
            .take(area.height.saturating_sub(2) as usize)
            .map(|log| {
                Line::from(vec![
                    Span::styled(format!("{} ", log.timestamp.format("%H:%M:%S")), Theme::dim()),
                    Span::styled(format!("{:>6} ", log.method), Theme::http_method(&log.method)),
                    Span::styled(truncate_str(&log.path, 20), Style::default().fg(Theme::FG)),
                    Span::raw(" "),
                    Span::styled(
                        format!("{}", log.status_code),
                        Theme::status_code(log.status_code),
                    ),
                    Span::styled(format!(" {:>4}ms", log.response_time_ms), Theme::dim()),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }
}

fn format_number(n: u64) -> String {
    if n < 1_000 {
        n.to_string()
    } else if n < 1_000_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        format!("{s:<max$}")
    } else {
        format!("{}…", &s[..max - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    /// Build a valid `DashboardData` JSON string for testing.
    fn sample_dashboard_json() -> String {
        let payload = serde_json::json!({
            "server_info": {
                "version": "0.3.31"
            },
            "system_info": {},
            "servers": [
                {
                    "server_type": "HTTP",
                    "running": true,
                    "address": "127.0.0.1:3000"
                }
            ],
            "metrics": {
                "total_requests": 1234,
                "average_response_time": 45.0,
                "error_rate": 0.02
            },
            "system": {
                "cpu_usage_percent": 78.5,
                "memory_usage_mb": 512,
                "active_threads": 8,
                "total_routes": 25,
                "total_fixtures": 10
            },
            "recent_logs": [
                {
                    "id": "req-001",
                    "method": "GET",
                    "path": "/api/users",
                    "status_code": 200,
                    "response_time_ms": 12,
                    "timestamp": "2025-01-01T14:23:01Z"
                }
            ]
        });
        serde_json::to_string(&payload).unwrap()
    }

    #[test]
    fn new_creates_screen_with_expected_defaults() {
        let screen = DashboardScreen::new();
        assert!(screen.data.is_none(), "data should be None on new screen");
        assert!(screen.error.is_none(), "error should be None on new screen");
        assert!(screen.last_fetch.is_none(), "last_fetch should be None on new screen");
        assert!(
            screen.request_rate_history.is_empty(),
            "request_rate_history should be empty on new screen"
        );
    }

    #[test]
    fn on_data_parses_dashboard_data_json() {
        let mut screen = DashboardScreen::new();
        let json = sample_dashboard_json();

        screen.on_data(&json);

        assert!(screen.data.is_some(), "data should be populated after on_data");
        assert!(screen.error.is_none(), "error should be None after valid on_data");

        let data = screen.data.as_ref().unwrap();
        assert_eq!(data.servers.len(), 1);
        assert_eq!(data.servers[0].server_type, "HTTP");
        assert!(data.servers[0].running);
        assert_eq!(data.metrics.total_requests, 1234);
        assert!((data.metrics.average_response_time - 45.0).abs() < f64::EPSILON);
        assert!((data.metrics.error_rate - 0.02).abs() < f64::EPSILON);
        assert_eq!(data.system.active_threads, 8);
        assert_eq!(data.system.total_routes, 25);
        assert_eq!(data.system.total_fixtures, 10);
        assert_eq!(data.recent_logs.len(), 1);
        assert_eq!(data.recent_logs[0].method, "GET");
        assert_eq!(data.recent_logs[0].path, "/api/users");
        assert_eq!(data.recent_logs[0].status_code, 200);
    }

    #[test]
    fn on_data_tracks_request_rate_history() {
        let mut screen = DashboardScreen::new();
        let json = sample_dashboard_json();

        screen.on_data(&json);
        assert_eq!(screen.request_rate_history.len(), 1);
        assert_eq!(screen.request_rate_history[0], 1234);

        // Feed another data point to confirm accumulation.
        screen.on_data(&json);
        assert_eq!(screen.request_rate_history.len(), 2);
        assert_eq!(screen.request_rate_history[1], 1234);
    }

    #[test]
    fn request_rate_history_caps_at_60_entries() {
        let mut screen = DashboardScreen::new();
        let json = sample_dashboard_json();

        for _ in 0..70 {
            screen.on_data(&json);
        }

        assert_eq!(screen.request_rate_history.len(), 60, "history should be capped at 60 entries");
    }

    #[test]
    fn on_data_with_invalid_json_sets_error() {
        let mut screen = DashboardScreen::new();

        screen.on_data("not valid json {{{");

        assert!(screen.data.is_none(), "data should remain None after invalid JSON");
        assert!(screen.error.is_some(), "error should be set after invalid JSON");
        let err = screen.error.as_ref().unwrap();
        assert!(
            err.starts_with("Parse error:"),
            "error message should start with 'Parse error:', got: {err}"
        );
    }

    #[test]
    fn handle_key_returns_false() {
        let mut screen = DashboardScreen::new();
        assert!(
            !screen.handle_key(key(KeyCode::Char('r'))),
            "dashboard handle_key should return false"
        );
        assert!(
            !screen.handle_key(key(KeyCode::Enter)),
            "dashboard handle_key should return false for Enter"
        );
        assert!(
            !screen.handle_key(key(KeyCode::Esc)),
            "dashboard handle_key should return false for Esc"
        );
    }

    #[test]
    fn force_refresh_clears_last_fetch() {
        let mut screen = DashboardScreen::new();
        screen.last_fetch = Some(Instant::now());
        assert!(screen.last_fetch.is_some());

        screen.force_refresh();

        assert!(screen.last_fetch.is_none(), "force_refresh should clear last_fetch");
    }

    #[test]
    fn status_hint_returns_expected_text() {
        let screen = DashboardScreen::new();
        assert_eq!(screen.status_hint(), "r:refresh");
    }

    #[test]
    fn format_number_small() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(42), "42");
        assert_eq!(format_number(999), "999");
    }

    #[test]
    fn format_number_thousands() {
        assert_eq!(format_number(1_000), "1.0K");
        assert_eq!(format_number(1_500), "1.5K");
        assert_eq!(format_number(999_999), "1000.0K");
    }

    #[test]
    fn format_number_millions() {
        assert_eq!(format_number(1_000_000), "1.0M");
        assert_eq!(format_number(2_500_000), "2.5M");
        assert_eq!(format_number(10_000_000), "10.0M");
    }

    #[test]
    fn truncate_str_short_string() {
        let result = truncate_str("hi", 10);
        assert_eq!(result.len(), 10, "short string should be left-padded to max width");
        assert_eq!(result, "hi        ");
    }

    #[test]
    fn truncate_str_exact_length() {
        let result = truncate_str("hello", 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn truncate_str_long_string() {
        let result = truncate_str("/api/very/long/path/here", 10);
        // Should truncate to 9 chars + ellipsis character
        assert_eq!(result, "/api/very…");
    }
}
