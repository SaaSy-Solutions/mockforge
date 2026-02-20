//! Chaos control panel — toggle chaos engineering, select presets, view settings.

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
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

const PRESETS: &[(&str, &str)] = &[
    ("network_degradation", "Simulate degraded network conditions"),
    ("service_instability", "Intermittent service failures"),
    ("cascading_failure", "Chain reaction of failures"),
    ("peak_traffic", "High traffic load simulation"),
    ("slow_backend", "Backend latency injection"),
];

enum PendingAction {
    Toggle,
    StartScenario(String),
    StopScenario(String),
}

pub struct ChaosScreen {
    data: Option<serde_json::Value>,
    error: Option<String>,
    last_fetch: Option<Instant>,
    pending_action: Option<PendingAction>,
    confirm: ConfirmDialog,
    /// Whether the preset picker overlay is showing.
    preset_picker: bool,
    selected_preset: usize,
    /// Whether the settings detail pane is focused.
    detail_focus: bool,
    detail_scroll: usize,
}

impl ChaosScreen {
    pub fn new() -> Self {
        Self {
            data: None,
            error: None,
            last_fetch: None,
            pending_action: None,
            confirm: ConfirmDialog::new(),
            preset_picker: false,
            selected_preset: 0,
            detail_focus: false,
            detail_scroll: 0,
        }
    }

    fn is_enabled(&self) -> bool {
        self.data
            .as_ref()
            .and_then(|d| d.get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    fn active_scenario(&self) -> Option<&str> {
        self.data
            .as_ref()
            .and_then(|d| d.get("active_scenario"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty() && *s != "none")
    }

    fn active_scenarios(&self) -> Vec<String> {
        self.data
            .as_ref()
            .and_then(|d| d.get("active_scenarios"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default()
    }
}

impl Screen for ChaosScreen {
    fn title(&self) -> &str {
        "Chaos"
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Confirm dialog takes priority when visible.
        if self.confirm.visible {
            if let Some(confirmed) = self.confirm.handle_key(key) {
                if confirmed {
                    if let Some(action) = self.pending_action.take() {
                        self.pending_action = Some(action);
                    }
                } else {
                    self.pending_action = None;
                }
                return true;
            }
            return true;
        }

        // Preset picker overlay.
        if self.preset_picker {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if self.selected_preset + 1 < PRESETS.len() {
                        self.selected_preset += 1;
                    }
                    return true;
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.selected_preset = self.selected_preset.saturating_sub(1);
                    return true;
                }
                KeyCode::Enter => {
                    let (name, _) = PRESETS[self.selected_preset];
                    let active = self.active_scenarios();
                    if active.contains(&name.to_string()) {
                        // Already running — stop it.
                        self.pending_action = Some(PendingAction::StopScenario(name.to_string()));
                        self.confirm.show("Stop Scenario", format!("Stop scenario '{name}'?"));
                    } else {
                        self.pending_action = Some(PendingAction::StartScenario(name.to_string()));
                        self.confirm.show("Start Scenario", format!("Start scenario '{name}'?"));
                    }
                    self.preset_picker = false;
                    return true;
                }
                KeyCode::Esc | KeyCode::Char('p') => {
                    self.preset_picker = false;
                    return true;
                }
                _ => return true,
            }
        }

        // Detail focus for scrolling settings.
        if self.detail_focus {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    self.detail_scroll += 1;
                    return true;
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.detail_scroll = self.detail_scroll.saturating_sub(1);
                    return true;
                }
                KeyCode::Esc | KeyCode::Char('e') => {
                    self.detail_focus = false;
                    return true;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char('t') => {
                let action = if self.is_enabled() {
                    "disable"
                } else {
                    "enable"
                };
                self.pending_action = Some(PendingAction::Toggle);
                self.confirm
                    .show("Toggle Chaos", format!("Are you sure you want to {action} chaos?"));
                true
            }
            KeyCode::Char('p') => {
                self.preset_picker = true;
                self.selected_preset = 0;
                true
            }
            KeyCode::Char('e') => {
                self.detail_focus = true;
                self.detail_scroll = 0;
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
            let loading = Paragraph::new("Loading chaos status...").style(Theme::dim()).block(
                Block::default()
                    .title(" Chaos ")
                    .borders(Borders::ALL)
                    .border_style(Theme::dim()),
            );
            frame.render_widget(loading, area);
            return;
        };

        let cols = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        self.render_status(frame, cols[0], data);
        self.render_settings(frame, cols[1], data);

        // Preset picker overlay.
        if self.preset_picker {
            self.render_preset_picker(frame, area);
        }

        self.confirm.render(frame);
    }

    fn tick(&mut self, client: &MockForgeClient, tx: &mpsc::UnboundedSender<Event>) {
        // Handle pending action.
        if let Some(action) = self.pending_action.take() {
            let client = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let result = match action {
                    PendingAction::Toggle => {
                        // Check current state and toggle.
                        let enabled = client
                            .get_chaos_status()
                            .await
                            .ok()
                            .and_then(|d| d.get("enabled").and_then(|v| v.as_bool()))
                            .unwrap_or(false);
                        client.toggle_chaos(!enabled).await.map(|_| ())
                    }
                    PendingAction::StartScenario(name) => {
                        client.start_chaos_scenario(&name).await.map(|_| ())
                    }
                    PendingAction::StopScenario(name) => {
                        client.stop_chaos_scenario(&name).await.map(|_| ())
                    }
                };
                match result {
                    Ok(()) => {
                        // Refetch status.
                        match client.get_chaos_status().await {
                            Ok(data) => {
                                let json = serde_json::to_string(&data).unwrap_or_default();
                                let _ = tx.send(Event::Data {
                                    screen: "chaos",
                                    payload: json,
                                });
                            }
                            Err(e) => {
                                let _ = tx.send(Event::ApiError {
                                    screen: "chaos",
                                    message: e.to_string(),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Event::ApiError {
                            screen: "chaos",
                            message: format!("Action failed: {e}"),
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
            match client.get_chaos_status().await {
                Ok(data) => {
                    let json = serde_json::to_string(&data).unwrap_or_default();
                    let _ = tx.send(Event::Data {
                        screen: "chaos",
                        payload: json,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Event::ApiError {
                        screen: "chaos",
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
        if self.preset_picker {
            "Enter:select  j/k:navigate  Esc:close"
        } else if self.detail_focus {
            "j/k:scroll  Esc:back"
        } else {
            "t:toggle  p:presets  e:details  r:refresh"
        }
    }
}

impl ChaosScreen {
    fn render_status(&self, frame: &mut Frame, area: Rect, _data: &serde_json::Value) {
        let block = Block::default()
            .title(" Chaos Status ")
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());

        let enabled = self.is_enabled();
        let status_icon = if enabled { "ON " } else { "OFF" };
        let status_color = if enabled {
            Theme::STATUS_UP
        } else {
            Theme::STATUS_DOWN
        };

        let scenario = self.active_scenario().unwrap_or("none");
        let active = self.active_scenarios();

        let mut lines = vec![
            Line::from(vec![
                Span::styled("  Status:      ", Theme::dim()),
                Span::styled(
                    status_icon,
                    Style::default().fg(status_color).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Scenario:    ", Theme::dim()),
                Span::styled(scenario.to_string(), Style::default().fg(Theme::FG)),
            ]),
            Line::from(""),
        ];

        // Show active scenarios.
        if !active.is_empty() {
            lines.push(Line::from(Span::styled("  Active Scenarios:", Theme::dim())));
            for name in &active {
                lines.push(Line::from(vec![
                    Span::styled("    ● ", Style::default().fg(Theme::STATUS_UP)),
                    Span::styled(name.clone(), Style::default().fg(Theme::FG)),
                ]));
            }
            lines.push(Line::from(""));
        }

        // Preset quick list.
        lines.push(Line::from(Span::styled("  Available Presets:", Theme::dim())));
        for (name, desc) in PRESETS {
            let is_active = active.iter().any(|a| a == name);
            let icon = if is_active { "●" } else { "○" };
            let color = if is_active {
                Theme::STATUS_UP
            } else {
                Theme::FG
            };
            lines.push(Line::from(vec![
                Span::styled(format!("    {icon} "), Style::default().fg(color)),
                Span::styled(format!("{name:<25}"), Style::default().fg(color)),
                Span::styled((*desc).to_string(), Theme::dim()),
            ]));
        }

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    fn render_settings(&self, frame: &mut Frame, area: Rect, data: &serde_json::Value) {
        let focus_indicator = if self.detail_focus { " [FOCUS]" } else { "" };
        let border_style = if self.detail_focus {
            Style::default().fg(Theme::BLUE)
        } else {
            Theme::dim()
        };

        let block = Block::default()
            .title(format!(" Settings{focus_indicator} "))
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(Theme::surface());

        let mut lines = Vec::new();

        if let Some(settings) = data.get("settings").and_then(|v| v.as_object()) {
            for (key, value) in settings {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {key:<24}"), Theme::dim()),
                    Span::styled(format!("{value}"), Style::default().fg(Theme::FG)),
                ]));
            }
        }

        // Also show config fields if present.
        for section in [
            "latency",
            "fault_injection",
            "rate_limit",
            "traffic_shaping",
        ] {
            if let Some(config) = data.get(section).and_then(|v| v.as_object()) {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!("  {section}:"),
                    Style::default().fg(Theme::BLUE).add_modifier(Modifier::BOLD),
                )));
                for (key, value) in config {
                    lines.push(Line::from(vec![
                        Span::styled(format!("    {key:<22}"), Theme::dim()),
                        Span::styled(format!("{value}"), Style::default().fg(Theme::FG)),
                    ]));
                }
            }
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled("  No settings data available", Theme::dim())));
        }

        // Apply scroll offset.
        let visible_lines: Vec<Line> = lines.into_iter().skip(self.detail_scroll).collect();

        let paragraph = Paragraph::new(visible_lines).block(block);
        frame.render_widget(paragraph, area);
    }

    fn render_preset_picker(&self, frame: &mut Frame, area: Rect) {
        let width = 60u16.min(area.width.saturating_sub(4));
        let height = (u16::try_from(PRESETS.len()).unwrap_or(u16::MAX).saturating_add(2))
            .min(area.height.saturating_sub(4));
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let popup_area = Rect::new(x, y, width, height);

        let active = self.active_scenarios();

        let block = Block::default()
            .title(" Select Preset ")
            .title_style(Style::default().fg(Theme::BLUE).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Theme::BLUE))
            .style(Theme::surface());

        let lines: Vec<Line> = PRESETS
            .iter()
            .enumerate()
            .map(|(i, (name, desc))| {
                let is_active = active.iter().any(|a| a == name);
                let selected = i == self.selected_preset;

                let icon = if is_active { "●" } else { "○" };
                let selector = if selected { "▸ " } else { "  " };

                let style = if selected {
                    Style::default().fg(Theme::BG).bg(Theme::BLUE).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::FG)
                };

                Line::from(Span::styled(format!("{selector}{icon} {name:<25} {desc}"), style))
            })
            .collect();

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, popup_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn sample_chaos_data() -> serde_json::Value {
        serde_json::json!({
            "enabled": true,
            "active_scenario": "network_degradation",
            "active_scenarios": ["network_degradation"],
            "settings": {
                "latency_ms": 200,
                "failure_rate": 0.1,
            }
        })
    }

    #[test]
    fn new_starts_clean() {
        let s = ChaosScreen::new();
        assert!(s.data.is_none());
        assert!(!s.preset_picker);
        assert!(!s.detail_focus);
    }

    #[test]
    fn toggle_shows_confirm() {
        let mut s = ChaosScreen::new();
        s.data = Some(sample_chaos_data());

        s.handle_key(key(KeyCode::Char('t')));
        assert!(s.confirm.visible);
        assert!(s.pending_action.is_some());
    }

    #[test]
    fn preset_picker_opens_and_closes() {
        let mut s = ChaosScreen::new();
        s.data = Some(sample_chaos_data());

        s.handle_key(key(KeyCode::Char('p')));
        assert!(s.preset_picker);
        assert_eq!(s.selected_preset, 0);

        s.handle_key(key(KeyCode::Esc));
        assert!(!s.preset_picker);
    }

    #[test]
    fn preset_picker_navigation() {
        let mut s = ChaosScreen::new();
        s.data = Some(sample_chaos_data());

        s.handle_key(key(KeyCode::Char('p')));

        s.handle_key(key(KeyCode::Char('j')));
        assert_eq!(s.selected_preset, 1);

        s.handle_key(key(KeyCode::Char('j')));
        assert_eq!(s.selected_preset, 2);

        s.handle_key(key(KeyCode::Char('k')));
        assert_eq!(s.selected_preset, 1);

        // Can't go below 0
        s.handle_key(key(KeyCode::Char('k')));
        s.handle_key(key(KeyCode::Char('k')));
        assert_eq!(s.selected_preset, 0);
    }

    #[test]
    fn preset_picker_enter_shows_confirm() {
        let mut s = ChaosScreen::new();
        s.data = Some(serde_json::json!({
            "enabled": true,
            "active_scenario": "none",
            "active_scenarios": [],
        }));

        s.handle_key(key(KeyCode::Char('p')));
        s.handle_key(key(KeyCode::Enter)); // select first preset

        assert!(!s.preset_picker);
        assert!(s.confirm.visible);
    }

    #[test]
    fn detail_focus() {
        let mut s = ChaosScreen::new();
        s.data = Some(sample_chaos_data());

        s.handle_key(key(KeyCode::Char('e')));
        assert!(s.detail_focus);

        s.handle_key(key(KeyCode::Char('j')));
        assert_eq!(s.detail_scroll, 1);

        s.handle_key(key(KeyCode::Esc));
        assert!(!s.detail_focus);
    }

    #[test]
    fn is_enabled_reads_data() {
        let mut s = ChaosScreen::new();
        assert!(!s.is_enabled());

        s.data = Some(serde_json::json!({ "enabled": true }));
        assert!(s.is_enabled());

        s.data = Some(serde_json::json!({ "enabled": false }));
        assert!(!s.is_enabled());
    }

    #[test]
    fn active_scenarios_reads_data() {
        let mut s = ChaosScreen::new();
        assert!(s.active_scenarios().is_empty());

        s.data = Some(serde_json::json!({
            "active_scenarios": ["network_degradation", "slow_backend"]
        }));
        assert_eq!(s.active_scenarios().len(), 2);
    }

    #[test]
    fn status_hints_change_with_mode() {
        let mut s = ChaosScreen::new();
        assert!(s.status_hint().contains("t:toggle"));

        s.preset_picker = true;
        assert!(s.status_hint().contains("Enter:select"));

        s.preset_picker = false;
        s.detail_focus = true;
        assert!(s.status_hint().contains("j/k:scroll"));
    }

    #[test]
    fn preset_picker_select_active_shows_stop() {
        let mut s = ChaosScreen::new();
        s.data = Some(serde_json::json!({
            "enabled": true,
            "active_scenario": "network_degradation",
            "active_scenarios": ["network_degradation"],
        }));

        s.handle_key(key(KeyCode::Char('p')));
        // First preset is "network_degradation" which is active.
        s.handle_key(key(KeyCode::Enter));

        // Should show "Stop" confirmation since it's already running.
        assert!(s.confirm.visible);
    }
}
