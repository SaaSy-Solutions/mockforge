//! Config screen — two-pane layout with categories on left and details on right.
//! Supports inline editing of boolean, numeric, and string fields.

use std::collections::HashMap;
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
use crate::api::models::{ConfigState, FaultConfig, LatencyConfig, ProxyConfig};
use crate::event::Event;
use crate::screens::Screen;
use crate::theme::Theme;

const CATEGORIES: &[&str] = &[
    "Latency",
    "Faults",
    "Proxy",
    "Traffic Shaping",
    "Validation",
];

/// Describes a single editable field within a config category.
#[derive(Clone, Copy)]
enum FieldKind {
    Bool,
    Uint,
    Float,
    Str,
    ReadOnly,
}

struct FieldDef {
    name: &'static str,
    json_key: &'static str,
    kind: FieldKind,
}

/// Returns the field definitions for each config category.
fn fields_for_category(cat: usize) -> &'static [FieldDef] {
    match cat {
        0 => &[
            FieldDef {
                name: "Enabled",
                json_key: "enabled",
                kind: FieldKind::Bool,
            },
            FieldDef {
                name: "Base Latency (ms)",
                json_key: "base_ms",
                kind: FieldKind::Uint,
            },
            FieldDef {
                name: "Jitter (ms)",
                json_key: "jitter_ms",
                kind: FieldKind::Uint,
            },
        ],
        1 => &[
            FieldDef {
                name: "Enabled",
                json_key: "enabled",
                kind: FieldKind::Bool,
            },
            FieldDef {
                name: "Failure Rate",
                json_key: "failure_rate",
                kind: FieldKind::Float,
            },
            FieldDef {
                name: "Status Codes",
                json_key: "status_codes",
                kind: FieldKind::ReadOnly,
            },
        ],
        2 => &[
            FieldDef {
                name: "Enabled",
                json_key: "enabled",
                kind: FieldKind::Bool,
            },
            FieldDef {
                name: "Upstream URL",
                json_key: "upstream_url",
                kind: FieldKind::Str,
            },
            FieldDef {
                name: "Timeout (s)",
                json_key: "timeout_seconds",
                kind: FieldKind::Uint,
            },
        ],
        3 => &[
            FieldDef {
                name: "Enabled",
                json_key: "enabled",
                kind: FieldKind::ReadOnly,
            },
            FieldDef {
                name: "Bandwidth",
                json_key: "bandwidth",
                kind: FieldKind::ReadOnly,
            },
            FieldDef {
                name: "Burst Loss",
                json_key: "burst_loss",
                kind: FieldKind::ReadOnly,
            },
        ],
        4 => &[
            FieldDef {
                name: "Mode",
                json_key: "mode",
                kind: FieldKind::ReadOnly,
            },
            FieldDef {
                name: "Aggregate Errors",
                json_key: "aggregate_errors",
                kind: FieldKind::ReadOnly,
            },
            FieldDef {
                name: "Validate Responses",
                json_key: "validate_responses",
                kind: FieldKind::ReadOnly,
            },
        ],
        _ => &[],
    }
}

enum PendingMutation {
    Latency(LatencyConfig),
    Faults(FaultConfig),
    Proxy(ProxyConfig),
}

pub struct ConfigScreen {
    data: Option<serde_json::Value>,
    error: Option<String>,
    last_fetch: Option<Instant>,
    selected_category: usize,
    selected_field: usize,
    editing: bool,
    /// When Some, an inline text editor is active for the current field.
    input_buf: Option<String>,
    input_cursor: usize,
    pending_mutation: Option<PendingMutation>,
}

impl ConfigScreen {
    pub fn new() -> Self {
        Self {
            data: None,
            error: None,
            last_fetch: None,
            selected_category: 0,
            selected_field: 0,
            editing: false,
            input_buf: None,
            input_cursor: 0,
            pending_mutation: None,
        }
    }

    fn category_key(&self) -> &'static str {
        match self.selected_category {
            0 => "latency",
            1 => "faults",
            2 => "proxy",
            3 => "traffic_shaping",
            4 => "validation",
            _ => "latency",
        }
    }

    fn field_count(&self) -> usize {
        fields_for_category(self.selected_category).len()
    }

    fn current_field(&self) -> Option<&'static FieldDef> {
        fields_for_category(self.selected_category).get(self.selected_field)
    }

    /// Read the current value of the selected field from local data.
    fn read_field_value(&self, json_key: &str) -> Option<serde_json::Value> {
        self.data
            .as_ref()
            .and_then(|d| d.get(self.category_key()))
            .and_then(|s| s.get(json_key))
            .cloned()
    }

    /// Start inline editing for the current field.
    fn start_input(&mut self) {
        let Some(field) = self.current_field() else {
            return;
        };
        let current = self.read_field_value(field.json_key);
        let text = match current {
            Some(serde_json::Value::String(s)) => s,
            Some(serde_json::Value::Number(n)) => n.to_string(),
            Some(serde_json::Value::Null) => String::new(),
            Some(v) => v.to_string(),
            None => String::new(),
        };
        self.input_cursor = text.len();
        self.input_buf = Some(text);
    }

    /// Commit the inline edit and build a mutation.
    fn commit_input(&mut self) {
        let Some(buf) = self.input_buf.take() else {
            return;
        };
        let Some(field) = self.current_field() else {
            return;
        };
        let key = self.category_key();

        // Parse the input into the appropriate JSON value.
        let new_value = match field.kind {
            FieldKind::Uint => {
                let Ok(n) = buf.trim().parse::<u64>() else {
                    return;
                };
                serde_json::Value::Number(n.into())
            }
            FieldKind::Float => {
                let Ok(f) = buf.trim().parse::<f64>() else {
                    return;
                };
                let Some(n) = serde_json::Number::from_f64(f) else {
                    return;
                };
                serde_json::Value::Number(n)
            }
            FieldKind::Str => serde_json::Value::String(buf),
            _ => return,
        };

        // Optimistically update local state.
        if let Some(ref mut data) = self.data {
            if let Some(section) = data.get_mut(key) {
                section[field.json_key] = new_value;
            }
        }

        self.build_mutation();
    }

    /// Toggle a boolean field.
    fn toggle_bool(&mut self) {
        let Some(field) = self.current_field() else {
            return;
        };
        let key = self.category_key();
        let current =
            self.read_field_value(field.json_key).and_then(|v| v.as_bool()).unwrap_or(false);
        let new_val = !current;

        // Optimistically update local state.
        if let Some(ref mut data) = self.data {
            if let Some(section) = data.get_mut(key) {
                section[field.json_key] = serde_json::Value::Bool(new_val);
            }
        }

        self.build_mutation();
    }

    /// Build a pending mutation from the current local data for the selected category.
    fn build_mutation(&mut self) {
        let Some(ref data) = self.data else { return };
        let key = self.category_key();
        let Some(section) = data.get(key) else { return };

        match self.selected_category {
            0 => {
                self.pending_mutation = Some(PendingMutation::Latency(LatencyConfig {
                    enabled: section.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
                    base_ms: section.get("base_ms").and_then(|v| v.as_u64()).unwrap_or(0),
                    jitter_ms: section.get("jitter_ms").and_then(|v| v.as_u64()).unwrap_or(0),
                    tag_overrides: HashMap::default(),
                }));
            }
            1 => {
                self.pending_mutation = Some(PendingMutation::Faults(FaultConfig {
                    enabled: section.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
                    failure_rate: section
                        .get("failure_rate")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                    status_codes: section
                        .get("status_codes")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_u64().and_then(|n| u16::try_from(n).ok()))
                                .collect()
                        })
                        .unwrap_or_default(),
                }));
            }
            2 => {
                self.pending_mutation = Some(PendingMutation::Proxy(ProxyConfig {
                    enabled: section.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
                    upstream_url: section
                        .get("upstream_url")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    timeout_seconds: section
                        .get("timeout_seconds")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(30),
                }));
            }
            _ => {}
        }
    }

    /// Handle key events while the inline text editor is active.
    fn handle_input_key(&mut self, key: KeyEvent) -> bool {
        let Some(ref mut buf) = self.input_buf else {
            return false;
        };
        match key.code {
            KeyCode::Char(c) => {
                buf.insert(self.input_cursor, c);
                self.input_cursor += 1;
            }
            KeyCode::Backspace => {
                if self.input_cursor > 0 {
                    self.input_cursor -= 1;
                    buf.remove(self.input_cursor);
                }
            }
            KeyCode::Delete => {
                if self.input_cursor < buf.len() {
                    buf.remove(self.input_cursor);
                }
            }
            KeyCode::Left => {
                self.input_cursor = self.input_cursor.saturating_sub(1);
            }
            KeyCode::Right => {
                self.input_cursor = (self.input_cursor + 1).min(buf.len());
            }
            KeyCode::Home => {
                self.input_cursor = 0;
            }
            KeyCode::End => {
                self.input_cursor = buf.len();
            }
            KeyCode::Enter => {
                self.commit_input();
            }
            KeyCode::Esc => {
                self.input_buf = None;
            }
            _ => return false,
        }
        true
    }
}

impl Screen for ConfigScreen {
    fn title(&self) -> &str {
        "Config"
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Inline text editor takes priority.
        if self.input_buf.is_some() {
            return self.handle_input_key(key);
        }

        if self.editing {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if self.selected_field + 1 < self.field_count() {
                        self.selected_field += 1;
                    }
                    return true;
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.selected_field = self.selected_field.saturating_sub(1);
                    return true;
                }
                KeyCode::Enter => {
                    if let Some(field) = self.current_field() {
                        match field.kind {
                            FieldKind::Bool => self.toggle_bool(),
                            FieldKind::Uint | FieldKind::Float | FieldKind::Str => {
                                self.start_input();
                            }
                            FieldKind::ReadOnly => {}
                        }
                    }
                    return true;
                }
                KeyCode::Esc => {
                    self.editing = false;
                    return true;
                }
                KeyCode::Char('r') => {
                    self.last_fetch = None;
                    return true;
                }
                _ => return false,
            }
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if self.selected_category + 1 < CATEGORIES.len() {
                    self.selected_category += 1;
                    self.selected_field = 0;
                }
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected_category > 0 {
                    self.selected_category -= 1;
                    self.selected_field = 0;
                }
                true
            }
            KeyCode::Char('e') | KeyCode::Enter => {
                self.editing = true;
                self.selected_field = 0;
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
            let loading = Paragraph::new("Loading config...").style(Theme::dim()).block(
                Block::default()
                    .title(" Config ")
                    .borders(Borders::ALL)
                    .border_style(Theme::dim()),
            );
            frame.render_widget(loading, area);
            return;
        };

        let cols = Layout::horizontal([Constraint::Length(22), Constraint::Min(30)]).split(area);

        // Left pane: categories
        self.render_categories(frame, cols[0]);

        // Right pane: details for selected category
        self.render_details(frame, cols[1], data);
    }

    fn tick(&mut self, client: &MockForgeClient, tx: &mpsc::UnboundedSender<Event>) {
        // Handle pending mutation.
        if let Some(mutation) = self.pending_mutation.take() {
            let client = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let result = match mutation {
                    PendingMutation::Latency(config) => {
                        client.update_latency(&config).await.map(|_| ())
                    }
                    PendingMutation::Faults(config) => {
                        client.update_faults(&config).await.map(|_| ())
                    }
                    PendingMutation::Proxy(config) => {
                        client.update_proxy(&config).await.map(|_| ())
                    }
                };
                match result {
                    Ok(()) => send_config_data(&client, &tx).await,
                    Err(e) => {
                        let _ = tx.send(Event::ApiError {
                            screen: "config",
                            message: format!("Update failed: {e}"),
                        });
                    }
                }
            });
            return;
        }

        // On-demand fetch only (first load + manual refresh).
        if self.last_fetch.is_some() {
            return;
        }
        self.last_fetch = Some(Instant::now());

        let client = client.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            send_config_data(&client, &tx).await;
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
        if self.input_buf.is_some() {
            "Enter:save  Esc:cancel  ←→:cursor"
        } else if self.editing {
            "Enter:edit field  Esc:stop  j/k:fields  r:refresh"
        } else {
            "e:edit  j/k:categories  r:refresh"
        }
    }
}

/// Fetch config from the API and send it as a data event.
async fn send_config_data(client: &MockForgeClient, tx: &mpsc::UnboundedSender<Event>) {
    match client.get_config().await {
        Ok(data) => {
            let payload = config_state_to_json(&data);
            let _ = tx.send(Event::Data {
                screen: "config",
                payload,
            });
        }
        Err(e) => {
            let _ = tx.send(Event::ApiError {
                screen: "config",
                message: e.to_string(),
            });
        }
    }
}

fn config_state_to_json(data: &ConfigState) -> String {
    let json = serde_json::json!({
        "latency": {
            "enabled": data.latency.enabled,
            "base_ms": data.latency.base_ms,
            "jitter_ms": data.latency.jitter_ms,
        },
        "faults": {
            "enabled": data.faults.enabled,
            "failure_rate": data.faults.failure_rate,
            "status_codes": data.faults.status_codes,
        },
        "proxy": {
            "enabled": data.proxy.enabled,
            "upstream_url": data.proxy.upstream_url,
            "timeout_seconds": data.proxy.timeout_seconds,
        },
        "traffic_shaping": {
            "enabled": data.traffic_shaping.enabled,
            "bandwidth": data.traffic_shaping.bandwidth,
            "burst_loss": data.traffic_shaping.burst_loss,
        },
        "validation": {
            "mode": data.validation.mode,
            "aggregate_errors": data.validation.aggregate_errors,
            "validate_responses": data.validation.validate_responses,
        },
    });
    serde_json::to_string(&json).unwrap_or_default()
}

impl ConfigScreen {
    fn render_categories(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Categories ")
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(if self.editing {
                Theme::dim()
            } else {
                Style::default().fg(Theme::BLUE)
            })
            .style(Theme::surface());

        let lines: Vec<Line> = CATEGORIES
            .iter()
            .enumerate()
            .map(|(i, &name)| {
                let style = if i == self.selected_category {
                    if self.editing {
                        Style::default().fg(Theme::BLUE).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Theme::BG).bg(Theme::BLUE).add_modifier(Modifier::BOLD)
                    }
                } else {
                    Style::default().fg(Theme::FG)
                };
                Line::from(Span::styled(format!(" {name}"), style))
            })
            .collect();

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    fn render_details(&self, frame: &mut Frame, area: Rect, data: &serde_json::Value) {
        let cat_key = self.category_key();
        let fields = fields_for_category(self.selected_category);
        let editing_indicator = if self.editing { " [EDITING]" } else { "" };

        let border_style = if self.editing {
            Style::default().fg(Theme::BLUE)
        } else {
            Theme::dim()
        };

        let block = Block::default()
            .title(format!(" {} {editing_indicator}", CATEGORIES[self.selected_category]))
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(Theme::surface());

        let section = data.get(cat_key);
        let mut lines = Vec::new();

        for (i, field) in fields.iter().enumerate() {
            let value = section.and_then(|s| s.get(field.json_key));
            let is_selected = self.editing && i == self.selected_field;
            let is_readonly = matches!(field.kind, FieldKind::ReadOnly);

            // Field label
            let label_style = if is_selected {
                Style::default().fg(Theme::BLUE).add_modifier(Modifier::BOLD)
            } else {
                Theme::dim()
            };

            let kind_hint = match field.kind {
                FieldKind::Bool => "",
                FieldKind::ReadOnly => " (ro)",
                _ => "",
            };

            let selector = if is_selected { "▸ " } else { "  " };

            // Value rendering
            let value_span = if is_selected && self.input_buf.is_some() {
                // Inline text editor is active.
                let buf = self.input_buf.as_deref().unwrap_or("");
                Span::styled(format!("{buf}▏"), Style::default().fg(Theme::FG).bg(Theme::OVERLAY))
            } else {
                let display = format_field_value(value, field.kind);
                let style = if is_readonly {
                    Theme::dim()
                } else if is_selected {
                    Style::default().fg(Theme::BLUE).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::FG)
                };
                Span::styled(display, style)
            };

            lines.push(Line::from(vec![
                Span::styled(selector.to_string(), label_style),
                Span::styled(format!("{:<20}{kind_hint}", field.name), label_style),
                value_span,
            ]));
        }

        if fields.is_empty() {
            lines.push(Line::from(Span::styled(" No data for this category", Theme::dim())));
        }

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }
}

fn format_field_value(value: Option<&serde_json::Value>, kind: FieldKind) -> String {
    match value {
        Some(serde_json::Value::Bool(b)) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Some(serde_json::Value::Number(n)) => {
            if let Some(f) = n.as_f64() {
                if matches!(kind, FieldKind::Float) {
                    format!("{f:.2}")
                } else {
                    n.to_string()
                }
            } else {
                n.to_string()
            }
        }
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Null) => "—".to_string(),
        Some(serde_json::Value::Array(arr)) => {
            let items: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
            format!("[{}]", items.join(", "))
        }
        Some(v) => v.to_string(),
        None => "—".to_string(),
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

    #[test]
    fn new_starts_at_first_category() {
        let s = ConfigScreen::new();
        assert_eq!(s.selected_category, 0);
        assert_eq!(s.selected_field, 0);
        assert!(!s.editing);
        assert!(s.input_buf.is_none());
    }

    #[test]
    fn category_navigation() {
        let mut s = ConfigScreen::new();
        s.data = Some(serde_json::json!({}));

        s.handle_key(key(KeyCode::Char('j')));
        assert_eq!(s.selected_category, 1);

        s.handle_key(key(KeyCode::Char('j')));
        assert_eq!(s.selected_category, 2);

        s.handle_key(key(KeyCode::Char('k')));
        assert_eq!(s.selected_category, 1);

        // Can't go below 0
        s.handle_key(key(KeyCode::Char('k')));
        s.handle_key(key(KeyCode::Char('k')));
        assert_eq!(s.selected_category, 0);
    }

    #[test]
    fn enter_edit_mode() {
        let mut s = ConfigScreen::new();

        s.handle_key(key(KeyCode::Char('e')));
        assert!(s.editing);
        assert_eq!(s.selected_field, 0);

        s.handle_key(key(KeyCode::Esc));
        assert!(!s.editing);
    }

    #[test]
    fn field_navigation_in_edit_mode() {
        let mut s = ConfigScreen::new();
        s.data = Some(serde_json::json!({
            "latency": { "enabled": true, "base_ms": 100, "jitter_ms": 50 }
        }));

        s.handle_key(key(KeyCode::Char('e'))); // enter edit mode
        assert_eq!(s.selected_field, 0);

        s.handle_key(key(KeyCode::Char('j')));
        assert_eq!(s.selected_field, 1);

        s.handle_key(key(KeyCode::Char('j')));
        assert_eq!(s.selected_field, 2);

        // Can't go past last field
        s.handle_key(key(KeyCode::Char('j')));
        assert_eq!(s.selected_field, 2);

        s.handle_key(key(KeyCode::Char('k')));
        assert_eq!(s.selected_field, 1);
    }

    #[test]
    fn toggle_bool_field() {
        let mut s = ConfigScreen::new();
        s.data = Some(serde_json::json!({
            "latency": { "enabled": false, "base_ms": 100, "jitter_ms": 50 }
        }));

        s.handle_key(key(KeyCode::Char('e'))); // enter edit mode
                                               // Field 0 is "enabled" (Bool)
        s.handle_key(key(KeyCode::Enter));

        // Should have toggled and created a mutation.
        let enabled = s.data.as_ref().unwrap()["latency"]["enabled"].as_bool().unwrap();
        assert!(enabled);
        assert!(s.pending_mutation.is_some());
    }

    #[test]
    fn inline_edit_numeric_field() {
        let mut s = ConfigScreen::new();
        s.data = Some(serde_json::json!({
            "latency": { "enabled": true, "base_ms": 100, "jitter_ms": 50 }
        }));

        s.handle_key(key(KeyCode::Char('e'))); // enter edit mode
        s.handle_key(key(KeyCode::Char('j'))); // move to base_ms field
        s.handle_key(key(KeyCode::Enter)); // start inline edit

        assert!(s.input_buf.is_some());
        assert_eq!(s.input_buf.as_deref(), Some("100"));

        // Clear and type new value.
        s.handle_input_key(key(KeyCode::Home));
        // Select all by moving to end with delete
        for _ in 0..3 {
            s.handle_input_key(key(KeyCode::Delete));
        }
        s.handle_input_key(key(KeyCode::Char('2')));
        s.handle_input_key(key(KeyCode::Char('5')));
        s.handle_input_key(key(KeyCode::Char('0')));

        assert_eq!(s.input_buf.as_deref(), Some("250"));

        // Commit
        s.handle_input_key(key(KeyCode::Enter));
        assert!(s.input_buf.is_none());

        let base_ms = s.data.as_ref().unwrap()["latency"]["base_ms"].as_u64().unwrap();
        assert_eq!(base_ms, 250);
        assert!(s.pending_mutation.is_some());
    }

    #[test]
    fn inline_edit_cancel_with_esc() {
        let mut s = ConfigScreen::new();
        s.data = Some(serde_json::json!({
            "latency": { "enabled": true, "base_ms": 100, "jitter_ms": 50 }
        }));

        s.handle_key(key(KeyCode::Char('e')));
        s.handle_key(key(KeyCode::Char('j'))); // base_ms
        s.handle_key(key(KeyCode::Enter)); // start edit

        assert!(s.input_buf.is_some());

        // Type something then cancel.
        s.handle_input_key(key(KeyCode::Char('9')));
        s.handle_input_key(key(KeyCode::Esc));

        assert!(s.input_buf.is_none());

        // Original value should be unchanged.
        let base_ms = s.data.as_ref().unwrap()["latency"]["base_ms"].as_u64().unwrap();
        assert_eq!(base_ms, 100);
        assert!(s.pending_mutation.is_none());
    }

    #[test]
    fn readonly_fields_dont_edit() {
        let mut s = ConfigScreen::new();
        s.data = Some(serde_json::json!({
            "traffic_shaping": { "enabled": false, "bandwidth": null, "burst_loss": null }
        }));
        s.selected_category = 3; // Traffic Shaping (all readonly)

        s.handle_key(key(KeyCode::Char('e')));
        s.handle_key(key(KeyCode::Enter));

        // Should not start inline edit or toggle.
        assert!(s.input_buf.is_none());
        assert!(s.pending_mutation.is_none());
    }

    #[test]
    fn category_change_resets_field() {
        let mut s = ConfigScreen::new();
        s.selected_field = 2;

        s.handle_key(key(KeyCode::Char('j'))); // change category
        assert_eq!(s.selected_field, 0);
    }

    #[test]
    fn fields_for_each_category() {
        assert_eq!(fields_for_category(0).len(), 3); // Latency
        assert_eq!(fields_for_category(1).len(), 3); // Faults
        assert_eq!(fields_for_category(2).len(), 3); // Proxy
        assert_eq!(fields_for_category(3).len(), 3); // Traffic Shaping
        assert_eq!(fields_for_category(4).len(), 3); // Validation
    }

    #[test]
    fn format_field_value_formats_correctly() {
        let bool_val = serde_json::json!(true);
        assert_eq!(format_field_value(Some(&bool_val), FieldKind::Bool), "true");

        let int_val = serde_json::json!(42);
        assert_eq!(format_field_value(Some(&int_val), FieldKind::Uint), "42");

        let float_val = serde_json::json!(0.15);
        assert_eq!(format_field_value(Some(&float_val), FieldKind::Float), "0.15");

        let str_val = serde_json::json!("http://example.com");
        assert_eq!(format_field_value(Some(&str_val), FieldKind::Str), "http://example.com");

        assert_eq!(format_field_value(None, FieldKind::Uint), "—");

        let null_val = serde_json::json!(null);
        assert_eq!(format_field_value(Some(&null_val), FieldKind::Str), "—");

        let arr_val = serde_json::json!([500, 503]);
        assert_eq!(format_field_value(Some(&arr_val), FieldKind::ReadOnly), "[500, 503]");
    }

    #[test]
    fn status_hints_change_with_mode() {
        let mut s = ConfigScreen::new();

        assert!(s.status_hint().contains("e:edit"));

        s.editing = true;
        assert!(s.status_hint().contains("Enter:edit field"));

        s.input_buf = Some("123".to_string());
        assert!(s.status_hint().contains("Enter:save"));
    }

    #[test]
    fn edit_float_field_faults() {
        let mut s = ConfigScreen::new();
        s.data = Some(serde_json::json!({
            "faults": { "enabled": true, "failure_rate": 0.1, "status_codes": [500] }
        }));
        s.selected_category = 1; // Faults

        s.handle_key(key(KeyCode::Char('e')));
        s.handle_key(key(KeyCode::Char('j'))); // failure_rate
        s.handle_key(key(KeyCode::Enter)); // start edit

        assert!(s.input_buf.is_some());

        // Clear and type new value.
        s.handle_input_key(key(KeyCode::Home));
        for _ in 0..10 {
            s.handle_input_key(key(KeyCode::Delete));
        }
        s.handle_input_key(key(KeyCode::Char('0')));
        s.handle_input_key(key(KeyCode::Char('.')));
        s.handle_input_key(key(KeyCode::Char('5')));
        s.handle_input_key(key(KeyCode::Enter));

        let rate = s.data.as_ref().unwrap()["faults"]["failure_rate"].as_f64().unwrap();
        assert!((rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn edit_string_field_proxy() {
        let mut s = ConfigScreen::new();
        s.data = Some(serde_json::json!({
            "proxy": { "enabled": false, "upstream_url": "http://old.com", "timeout_seconds": 30 }
        }));
        s.selected_category = 2; // Proxy

        s.handle_key(key(KeyCode::Char('e')));
        s.handle_key(key(KeyCode::Char('j'))); // upstream_url
        s.handle_key(key(KeyCode::Enter)); // start edit

        assert_eq!(s.input_buf.as_deref(), Some("http://old.com"));

        // Clear and type new URL.
        s.handle_input_key(key(KeyCode::Home));
        for _ in 0..20 {
            s.handle_input_key(key(KeyCode::Delete));
        }
        for c in "http://new.com".chars() {
            s.handle_input_key(key(KeyCode::Char(c)));
        }
        s.handle_input_key(key(KeyCode::Enter));

        let url = s.data.as_ref().unwrap()["proxy"]["upstream_url"].as_str().unwrap();
        assert_eq!(url, "http://new.com");
    }
}
