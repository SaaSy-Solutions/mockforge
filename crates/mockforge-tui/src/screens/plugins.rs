//! Plugin list table screen.

use std::time::Instant;

use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Rect},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use tokio::sync::mpsc;

use crate::api::client::MockForgeClient;
use crate::api::models::PluginInfo;
use crate::event::Event;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::widgets::table::TableState;

const FETCH_INTERVAL: u64 = 60;

pub struct PluginsScreen {
    data: Option<serde_json::Value>,
    plugins: Vec<PluginInfo>,
    table: TableState,
    error: Option<String>,
    last_fetch: Option<Instant>,
}

impl PluginsScreen {
    pub fn new() -> Self {
        Self {
            data: None,
            plugins: Vec::new(),
            table: TableState::new(),
            error: None,
            last_fetch: None,
        }
    }
}

impl Screen for PluginsScreen {
    fn title(&self) -> &str {
        "Plugins"
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        self.table.handle_key(key)
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if self.data.is_none() {
            let loading = Paragraph::new("Loading plugins...").style(Theme::dim()).block(
                Block::default()
                    .title(" Plugins ")
                    .borders(Borders::ALL)
                    .border_style(Theme::dim()),
            );
            frame.render_widget(loading, area);
            return;
        }

        let header = Row::new(vec![
            Cell::from("ID").style(Theme::dim()),
            Cell::from("Name").style(Theme::dim()),
            Cell::from("Version").style(Theme::dim()),
            Cell::from("Status").style(Theme::dim()),
            Cell::from("Healthy").style(Theme::dim()),
        ])
        .height(1);

        let rows: Vec<Row> = self
            .plugins
            .iter()
            .skip(self.table.offset)
            .take(self.table.visible_height)
            .map(|plugin| {
                let healthy_style = if plugin.healthy {
                    Theme::success()
                } else {
                    Theme::error()
                };
                Row::new(vec![
                    Cell::from(plugin.id.clone()),
                    Cell::from(plugin.name.clone()),
                    Cell::from(plugin.version.clone()),
                    Cell::from(plugin.status.clone()),
                    Cell::from(if plugin.healthy { "yes" } else { "no" }).style(healthy_style),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(12),
            Constraint::Length(20),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(8),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .row_highlight_style(Theme::highlight())
            .block(
                Block::default()
                    .title(format!(" Plugins ({}) ", self.plugins.len()))
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
            match client.get_plugins().await {
                Ok(data) => {
                    let json = serde_json::json!(data
                        .iter()
                        .map(|p| serde_json::json!({
                            "id": p.id,
                            "name": p.name,
                            "version": p.version,
                            "status": p.status,
                            "healthy": p.healthy,
                            "description": p.description,
                            "author": p.author,
                        }))
                        .collect::<Vec<_>>());
                    let payload = serde_json::to_string(&json).unwrap_or_default();
                    let _ = tx.send(Event::Data {
                        screen: "plugins",
                        payload,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Event::ApiError {
                        screen: "plugins",
                        message: e.to_string(),
                    });
                }
            }
        });
    }

    fn on_data(&mut self, payload: &str) {
        match serde_json::from_str::<Vec<PluginInfo>>(payload) {
            Ok(plugins) => {
                self.table.set_total(plugins.len());
                self.plugins = plugins;
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
