//! Audit log table screen — timestamp, action, user, details.

use std::time::Instant;

use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Rect},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use tokio::sync::mpsc;

use crate::api::client::MockForgeClient;
use crate::api::models::AuditEntry;
use crate::event::Event;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::widgets::table::TableState;

const FETCH_INTERVAL: u64 = 10;

pub struct AuditScreen {
    data: Option<serde_json::Value>,
    entries: Vec<AuditEntry>,
    table: TableState,
    error: Option<String>,
    last_fetch: Option<Instant>,
}

impl AuditScreen {
    pub fn new() -> Self {
        Self {
            data: None,
            entries: Vec::new(),
            table: TableState::new(),
            error: None,
            last_fetch: None,
        }
    }
}

impl Screen for AuditScreen {
    fn title(&self) -> &str {
        "Audit"
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        self.table.handle_key(key)
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if self.data.is_none() {
            let loading = Paragraph::new("Loading audit logs...").style(Theme::dim()).block(
                Block::default()
                    .title(" Audit ")
                    .borders(Borders::ALL)
                    .border_style(Theme::dim()),
            );
            frame.render_widget(loading, area);
            return;
        }

        let header = Row::new(vec![
            Cell::from("Timestamp").style(Theme::dim()),
            Cell::from("Action").style(Theme::dim()),
            Cell::from("User").style(Theme::dim()),
            Cell::from("Details").style(Theme::dim()),
        ])
        .height(1);

        let rows: Vec<Row> = self
            .entries
            .iter()
            .skip(self.table.offset)
            .take(self.table.visible_height)
            .map(|entry| {
                let timestamp = entry
                    .timestamp
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "--".to_string());
                let details = serde_json::to_string(&entry.details).unwrap_or_default();
                Row::new(vec![
                    Cell::from(timestamp),
                    Cell::from(entry.action.clone()),
                    Cell::from(entry.user.clone()),
                    Cell::from(details),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(20),
            Constraint::Length(15),
            Constraint::Length(15),
            Constraint::Min(20),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .row_highlight_style(Theme::highlight())
            .block(
                Block::default()
                    .title(format!(" Audit Logs ({}) ", self.entries.len()))
                    .title_style(Theme::title())
                    .borders(Borders::ALL)
                    .border_style(Theme::dim())
                    .style(Theme::surface()),
            );

        let mut table_state = self.table.to_ratatui_state();
        frame.render_stateful_widget(table, area, &mut table_state);
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
            match client.get_audit_logs().await {
                Ok(data) => {
                    let json = serde_json::json!(data
                        .iter()
                        .map(|e| serde_json::json!({
                            "id": e.id,
                            "timestamp": e.timestamp,
                            "action": e.action,
                            "user": e.user,
                            "details": e.details,
                        }))
                        .collect::<Vec<_>>());
                    let payload = serde_json::to_string(&json).unwrap_or_default();
                    let _ = tx.send(Event::Data {
                        screen: "audit",
                        payload,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Event::ApiError {
                        screen: "audit",
                        message: e.to_string(),
                    });
                }
            }
        });
    }

    fn on_data(&mut self, payload: &str) {
        match serde_json::from_str::<Vec<AuditEntry>>(payload) {
            Ok(entries) => {
                self.table.set_total(entries.len());
                self.entries = entries;
                self.data = serde_json::from_str(payload).ok();
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
        "j/k:navigate  g/G:top/bottom"
    }
}
