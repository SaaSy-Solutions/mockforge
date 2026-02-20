//! Fixture browser table screen.

use std::time::Instant;

use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Rect},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use tokio::sync::mpsc;

use crate::api::client::MockForgeClient;
use crate::api::models::FixtureInfo;
use crate::event::Event;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::widgets::table::TableState;

const FETCH_INTERVAL: u64 = 30;

pub struct FixturesScreen {
    data: Option<serde_json::Value>,
    fixtures: Vec<FixtureInfo>,
    table: TableState,
    error: Option<String>,
    last_fetch: Option<Instant>,
}

impl FixturesScreen {
    pub fn new() -> Self {
        Self {
            data: None,
            fixtures: Vec::new(),
            table: TableState::new(),
            error: None,
            last_fetch: None,
        }
    }
}

impl Screen for FixturesScreen {
    fn title(&self) -> &str {
        "Fixtures"
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        self.table.handle_key(key)
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if self.data.is_none() {
            let loading = Paragraph::new("Loading fixtures...").style(Theme::dim()).block(
                Block::default()
                    .title(" Fixtures ")
                    .borders(Borders::ALL)
                    .border_style(Theme::dim()),
            );
            frame.render_widget(loading, area);
            return;
        }

        let header = Row::new(vec![
            Cell::from("Protocol").style(Theme::dim()),
            Cell::from("Method").style(Theme::dim()),
            Cell::from("Path").style(Theme::dim()),
            Cell::from("Size").style(Theme::dim()),
        ])
        .height(1);

        let rows: Vec<Row> = self
            .fixtures
            .iter()
            .skip(self.table.offset)
            .take(self.table.visible_height)
            .map(|fixture| {
                Row::new(vec![
                    Cell::from(fixture.protocol.clone()),
                    Cell::from(fixture.method.clone()).style(Theme::http_method(&fixture.method)),
                    Cell::from(fixture.path.clone()),
                    Cell::from(format_size(fixture.file_size)),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Min(20),
            Constraint::Length(10),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .row_highlight_style(Theme::highlight())
            .block(
                Block::default()
                    .title(format!(" Fixtures ({}) ", self.fixtures.len()))
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
            match client.get_fixtures().await {
                Ok(data) => {
                    let json = serde_json::json!(data
                        .iter()
                        .map(|f| serde_json::json!({
                            "id": f.id,
                            "protocol": f.protocol,
                            "method": f.method,
                            "path": f.path,
                            "file_size": f.file_size,
                            "file_path": f.file_path,
                            "fingerprint": f.fingerprint,
                        }))
                        .collect::<Vec<_>>());
                    let payload = serde_json::to_string(&json).unwrap_or_default();
                    let _ = tx.send(Event::Data {
                        screen: "fixtures",
                        payload,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Event::ApiError {
                        screen: "fixtures",
                        message: e.to_string(),
                    });
                }
            }
        });
    }

    fn on_data(&mut self, payload: &str) {
        match serde_json::from_str::<Vec<FixtureInfo>>(payload) {
            Ok(fixtures) => {
                self.table.set_total(fixtures.len());
                self.fixtures = fixtures;
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

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
