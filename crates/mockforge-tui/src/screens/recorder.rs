//! Recorder status screen with start/stop controls.

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
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
use crate::widgets::confirm::ConfirmDialog;

const FETCH_INTERVAL: u64 = 5;

pub struct RecorderScreen {
    data: Option<serde_json::Value>,
    error: Option<String>,
    last_fetch: Option<Instant>,
    confirm: ConfirmDialog,
    pending_toggle: Option<bool>,
    status_message: Option<(bool, String)>,
}

impl RecorderScreen {
    pub fn new() -> Self {
        Self {
            data: None,
            error: None,
            last_fetch: None,
            confirm: ConfirmDialog::new(),
            pending_toggle: None,
            status_message: None,
        }
    }

    fn is_recording(&self) -> bool {
        self.data
            .as_ref()
            .and_then(|d| d.get("recording"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    fn recorded_count(&self) -> u64 {
        self.data
            .as_ref()
            .and_then(|d| {
                d.get("recorded_count")
                    .and_then(|v| v.as_u64())
                    .or_else(|| d.get("count").and_then(|v| v.as_u64()))
            })
            .unwrap_or(0)
    }
}

impl Screen for RecorderScreen {
    fn title(&self) -> &str {
        "Recorder"
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Confirm dialog takes priority.
        if self.confirm.visible {
            if let Some(confirmed) = self.confirm.handle_key(key) {
                if confirmed {
                    let currently_recording = self.is_recording();
                    self.pending_toggle = Some(!currently_recording);
                }
                return true;
            }
            return true;
        }

        match key.code {
            KeyCode::Char('s') => {
                let recording = self.is_recording();
                let action = if recording { "Stop" } else { "Start" };
                self.confirm.show(format!("{action} Recording"), format!("{action} recording?"));
                true
            }
            KeyCode::Char('c') => {
                // Clear status message.
                self.status_message = None;
                true
            }
            _ => false,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let Some(ref data) = self.data else {
            let loading = Paragraph::new("Loading recorder status...").style(Theme::dim()).block(
                Block::default()
                    .title(" Recorder ")
                    .borders(Borders::ALL)
                    .border_style(Theme::dim()),
            );
            frame.render_widget(loading, area);
            self.confirm.render(frame);
            return;
        };

        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).split(area);

        let block = Block::default()
            .title(" Recorder ")
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());

        let recording = self.is_recording();
        let count = self.recorded_count();

        let status_text = if recording { "RECORDING" } else { "STOPPED" };
        let status_color = if recording {
            Theme::STATUS_UP
        } else {
            Theme::STATUS_DOWN
        };

        let toggle_hint = if recording {
            "Press 's' to stop recording"
        } else {
            "Press 's' to start recording"
        };

        let mut lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Status:   ", Theme::dim()),
                Span::styled(status_text, Style::default().fg(status_color)),
            ]),
            Line::from(vec![
                Span::styled("  Recorded: ", Theme::dim()),
                Span::styled(count.to_string(), Style::default().fg(Theme::FG)),
            ]),
            Line::from(""),
            Line::from(Span::styled(format!("  {toggle_hint}"), Theme::dim())),
        ];

        // Show additional data fields if available.
        if let Some(format) = data.get("format").and_then(|v| v.as_str()) {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  Format:   ", Theme::dim()),
                Span::styled(format.to_string(), Style::default().fg(Theme::FG)),
            ]));
        }
        if let Some(output) = data.get("output_path").and_then(|v| v.as_str()) {
            lines.push(Line::from(vec![
                Span::styled("  Output:   ", Theme::dim()),
                Span::styled(output.to_string(), Style::default().fg(Theme::FG)),
            ]));
        }

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, chunks[0]);

        // Status message bar at the bottom.
        let msg_line = if let Some((success, ref msg)) = self.status_message {
            let style = if success {
                Theme::success()
            } else {
                Theme::error()
            };
            Line::from(vec![
                Span::styled(if success { "  OK: " } else { "  ERR: " }, style),
                Span::styled(msg.as_str(), Theme::base()),
            ])
        } else {
            Line::from(Span::styled("  Ready", Theme::dim()))
        };
        let msg_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());
        frame.render_widget(Paragraph::new(msg_line).block(msg_block), chunks[1]);

        self.confirm.render(frame);
    }

    fn tick(&mut self, client: &MockForgeClient, tx: &mpsc::UnboundedSender<Event>) {
        // Handle pending toggle.
        if let Some(enable) = self.pending_toggle.take() {
            let client = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let result = match client.toggle_recorder(enable).await {
                    Ok(msg) => serde_json::json!({
                        "type": "toggle_result",
                        "success": true,
                        "message": if msg.is_empty() {
                            if enable { "Recording started" } else { "Recording stopped" }.to_string()
                        } else {
                            msg
                        },
                    }),
                    Err(e) => serde_json::json!({
                        "type": "toggle_result",
                        "success": false,
                        "message": e.to_string(),
                    }),
                };
                let _ = tx.send(Event::Data {
                    screen: "recorder",
                    payload: serde_json::to_string(&result).unwrap_or_default(),
                });
            });
        }

        // Periodic fetch.
        let should_fetch =
            self.last_fetch.map_or(true, |t| t.elapsed().as_secs() >= FETCH_INTERVAL);
        if !should_fetch {
            return;
        }
        self.last_fetch = Some(Instant::now());

        let client = client.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            match client.get_recorder_status().await {
                Ok(data) => {
                    let json = serde_json::to_string(&data).unwrap_or_default();
                    let _ = tx.send(Event::Data {
                        screen: "recorder",
                        payload: json,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Event::ApiError {
                        screen: "recorder",
                        message: e.to_string(),
                    });
                }
            }
        });
    }

    fn on_data(&mut self, payload: &str) {
        // Check for toggle result message.
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(payload) {
            if val.get("type").and_then(|v| v.as_str()) == Some("toggle_result") {
                let success = val.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
                let message =
                    val.get("message").and_then(|v| v.as_str()).unwrap_or("done").to_string();
                self.status_message = Some((success, message));
                // Force refresh to get updated status.
                self.last_fetch = None;
                return;
            }
        }

        // Normal status data.
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
        "s:start/stop  c:clear-message  r:refresh"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn new_creates_screen_with_defaults() {
        let s = RecorderScreen::new();
        assert!(s.data.is_none());
        assert!(s.pending_toggle.is_none());
        assert!(s.status_message.is_none());
        assert!(!s.is_recording());
    }

    #[test]
    fn on_data_parses_status() {
        let mut s = RecorderScreen::new();
        s.on_data(r#"{"recording": true, "recorded_count": 42}"#);
        assert!(s.is_recording());
        assert_eq!(s.recorded_count(), 42);
    }

    #[test]
    fn s_key_shows_start_confirm_when_stopped() {
        let mut s = RecorderScreen::new();
        s.on_data(r#"{"recording": false, "recorded_count": 0}"#);
        assert!(s.handle_key(key(KeyCode::Char('s'))));
        assert!(s.confirm.visible);
        assert!(s.confirm.title.contains("Start"));
    }

    #[test]
    fn s_key_shows_stop_confirm_when_recording() {
        let mut s = RecorderScreen::new();
        s.on_data(r#"{"recording": true, "recorded_count": 5}"#);
        assert!(s.handle_key(key(KeyCode::Char('s'))));
        assert!(s.confirm.visible);
        assert!(s.confirm.title.contains("Stop"));
    }

    #[test]
    fn confirm_yes_sets_pending_toggle() {
        let mut s = RecorderScreen::new();
        s.on_data(r#"{"recording": false, "recorded_count": 0}"#);
        s.handle_key(key(KeyCode::Char('s')));
        s.handle_key(key(KeyCode::Char('y')));
        assert!(!s.confirm.visible);
        assert_eq!(s.pending_toggle, Some(true)); // Start = !false
    }

    #[test]
    fn confirm_no_does_not_set_pending() {
        let mut s = RecorderScreen::new();
        s.on_data(r#"{"recording": false, "recorded_count": 0}"#);
        s.handle_key(key(KeyCode::Char('s')));
        s.handle_key(key(KeyCode::Char('n')));
        assert!(s.pending_toggle.is_none());
    }

    #[test]
    fn toggle_result_sets_status_message() {
        let mut s = RecorderScreen::new();
        let result = serde_json::json!({
            "type": "toggle_result",
            "success": true,
            "message": "Recording started",
        });
        s.on_data(&serde_json::to_string(&result).unwrap());
        assert!(s.status_message.is_some());
        let (success, msg) = s.status_message.as_ref().unwrap();
        assert!(success);
        assert_eq!(msg, "Recording started");
    }

    #[test]
    fn c_key_clears_status_message() {
        let mut s = RecorderScreen::new();
        s.status_message = Some((true, "Test".into()));
        s.handle_key(key(KeyCode::Char('c')));
        assert!(s.status_message.is_none());
    }

    #[test]
    fn is_recording_with_no_data() {
        let s = RecorderScreen::new();
        assert!(!s.is_recording());
        assert_eq!(s.recorded_count(), 0);
    }

    #[test]
    fn status_hint_shows_controls() {
        let s = RecorderScreen::new();
        assert!(s.status_hint().contains("start/stop"));
    }

    #[test]
    fn force_refresh_clears_last_fetch() {
        let mut s = RecorderScreen::new();
        s.last_fetch = Some(Instant::now());
        s.force_refresh();
        assert!(s.last_fetch.is_none());
    }
}
