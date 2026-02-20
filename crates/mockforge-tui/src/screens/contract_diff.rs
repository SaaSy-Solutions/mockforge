//! Contract diff captures table screen.

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use tokio::sync::mpsc;

use crate::api::client::MockForgeClient;
use crate::api::models::ContractDiffCapture;
use crate::event::Event;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::widgets::json_viewer;
use crate::widgets::table::TableState;

const FETCH_INTERVAL: u64 = 30;

pub struct ContractDiffScreen {
    data: Option<serde_json::Value>,
    captures: Vec<ContractDiffCapture>,
    table: TableState,
    error: Option<String>,
    last_fetch: Option<Instant>,
    detail_open: bool,
    detail_scroll: u16,
}

impl ContractDiffScreen {
    pub fn new() -> Self {
        Self {
            data: None,
            captures: Vec::new(),
            table: TableState::new(),
            error: None,
            last_fetch: None,
            detail_open: false,
            detail_scroll: 0,
        }
    }

    fn selected_capture_json(&self) -> Option<serde_json::Value> {
        let capture = self.captures.get(self.table.selected)?;
        Some(serde_json::json!({
            "id": capture.id,
            "path": capture.path,
            "method": capture.method,
            "diff_status": capture.diff_status,
            "captured_at": capture.captured_at,
        }))
    }
}

impl Screen for ContractDiffScreen {
    fn title(&self) -> &str {
        "Contract Diff"
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.detail_open {
            match key.code {
                KeyCode::Esc => {
                    self.detail_open = false;
                    self.detail_scroll = 0;
                    return true;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.detail_scroll = self.detail_scroll.saturating_add(1);
                    return true;
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.detail_scroll = self.detail_scroll.saturating_sub(1);
                    return true;
                }
                _ => return true,
            }
        }

        match key.code {
            KeyCode::Enter => {
                if !self.captures.is_empty() {
                    self.detail_open = true;
                    self.detail_scroll = 0;
                }
                true
            }
            _ => self.table.handle_key(key),
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if self.data.is_none() {
            let loading =
                Paragraph::new("Loading contract diff captures...").style(Theme::dim()).block(
                    Block::default()
                        .title(" Contract Diff ")
                        .borders(Borders::ALL)
                        .border_style(Theme::dim()),
                );
            frame.render_widget(loading, area);
            return;
        }

        // Split: table on top, detail on bottom when open.
        let chunks = if self.detail_open {
            Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area)
        } else {
            Layout::vertical([Constraint::Min(0), Constraint::Length(0)]).split(area)
        };

        // Table pane.
        self.render_table(frame, chunks[0]);

        // Detail pane (when open).
        if self.detail_open {
            if let Some(json) = self.selected_capture_json() {
                json_viewer::render_scrollable(
                    frame,
                    chunks[1],
                    "Capture Detail",
                    &json,
                    self.detail_scroll,
                );
            }
        }
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
            match client.get_contract_diff_captures().await {
                Ok(data) => {
                    let json = serde_json::json!(data
                        .iter()
                        .map(|c| serde_json::json!({
                            "id": c.id,
                            "path": c.path,
                            "method": c.method,
                            "diff_status": c.diff_status,
                            "captured_at": c.captured_at,
                        }))
                        .collect::<Vec<_>>());
                    let payload = serde_json::to_string(&json).unwrap_or_default();
                    let _ = tx.send(Event::Data {
                        screen: "contract_diff",
                        payload,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Event::ApiError {
                        screen: "contract_diff",
                        message: e.to_string(),
                    });
                }
            }
        });
    }

    fn on_data(&mut self, payload: &str) {
        match serde_json::from_str::<Vec<ContractDiffCapture>>(payload) {
            Ok(captures) => {
                self.table.set_total(captures.len());
                self.captures = captures;
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
        if self.detail_open {
            "Esc:close  j/k:scroll"
        } else {
            "j/k:navigate  g/G:top/bottom  Enter:detail"
        }
    }
}

impl ContractDiffScreen {
    fn render_table(&self, frame: &mut Frame, area: Rect) {
        let header = Row::new(vec![
            Cell::from("ID").style(Theme::dim()),
            Cell::from("Path").style(Theme::dim()),
            Cell::from("Method").style(Theme::dim()),
            Cell::from("Diff Status").style(Theme::dim()),
            Cell::from("Captured At").style(Theme::dim()),
        ])
        .height(1);

        let rows: Vec<Row> = self
            .captures
            .iter()
            .skip(self.table.offset)
            .take(self.table.visible_height)
            .map(|capture| {
                let diff_style = match capture.diff_status.as_str() {
                    "match" | "identical" => Theme::success(),
                    "mismatch" | "changed" | "breaking" => Theme::error(),
                    _ => Theme::dim(),
                };
                let captured_at = capture
                    .captured_at
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "--".to_string());
                Row::new(vec![
                    Cell::from(capture.id.clone()),
                    Cell::from(capture.path.clone()),
                    Cell::from(capture.method.clone()).style(Theme::http_method(&capture.method)),
                    Cell::from(capture.diff_status.clone()).style(diff_style),
                    Cell::from(captured_at),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(12),
            Constraint::Min(20),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(20),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .row_highlight_style(Theme::highlight())
            .block(
                Block::default()
                    .title(format!(" Contract Diff ({}) ", self.captures.len()))
                    .title_style(Theme::title())
                    .borders(Borders::ALL)
                    .border_style(Theme::dim())
                    .style(Theme::surface()),
            );

        let mut table_state = self.table.to_ratatui_state();
        frame.render_stateful_widget(table, area, &mut table_state);
    }
}
