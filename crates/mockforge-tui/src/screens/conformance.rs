//! Conformance violations screen — server-side spec violations on
//! incoming requests.
//!
//! Issue #79 round 12 — Srikanth's ask: surface conformance failures
//! that the MockForge server detects on incoming traffic, so users can
//! cross-check the server's view against their proxy's. The bench-side
//! conformance suite already had its own report; this screen handles
//! the *serve-time* equivalent.

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use tokio::sync::mpsc;

use crate::api::client::MockForgeClient;
use crate::api::models::ConformanceViolation;
use crate::event::Event;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::widgets::table::TableState;

const FETCH_INTERVAL: u64 = 5;

pub struct ConformanceScreen {
    loaded: bool,
    violations: Vec<ConformanceViolation>,
    total: usize,
    table: TableState,
    error: Option<String>,
    last_fetch: Option<Instant>,
    detail_open: bool,
    detail_scroll: u16,
}

impl ConformanceScreen {
    pub fn new() -> Self {
        Self {
            loaded: false,
            violations: Vec::new(),
            total: 0,
            table: TableState::new(),
            error: None,
            last_fetch: None,
            detail_open: false,
            detail_scroll: 0,
        }
    }

    fn selected_detail(&self) -> Option<String> {
        let v = self.violations.get(self.table.selected)?;
        Some(format!(
            "Timestamp:  {}\nMethod:     {}\nPath:       {}\nClient IP:  {}\nStatus:     {}\nCategory:   {}\n\nReason:\n{}\n",
            v.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            v.method,
            v.path,
            v.client_ip,
            v.status,
            if v.category.is_empty() { "(uncategorised)" } else { v.category.as_str() },
            v.reason,
        ))
    }
}

impl Default for ConformanceScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for ConformanceScreen {
    fn title(&self) -> &str {
        "Conformance"
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
                if !self.violations.is_empty() {
                    self.detail_open = true;
                    self.detail_scroll = 0;
                }
                true
            }
            _ => self.table.handle_key(key),
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.loaded {
            let placeholder = Paragraph::new(
                "Loading server-side conformance violations...\n\nThis screen lists \
                 incoming requests the mockforge server rejected for spec violations \
                 (status 400/422). Empty until a request triggers a validation \
                 failure.",
            )
            .style(Theme::dim())
            .block(
                Block::default()
                    .title(" Conformance ")
                    .title_style(Theme::title())
                    .borders(Borders::ALL)
                    .border_style(Theme::dim()),
            );
            frame.render_widget(placeholder, area);
            return;
        }

        if self.detail_open {
            let detail =
                self.selected_detail().unwrap_or_else(|| "(no violation selected)".to_string());
            let para = Paragraph::new(detail).scroll((self.detail_scroll, 0)).block(
                Block::default()
                    .title(" Violation Detail (Esc to close) ")
                    .title_style(Theme::title())
                    .borders(Borders::ALL)
                    .border_style(Theme::dim())
                    .style(Theme::surface()),
            );
            frame.render_widget(para, area);
            return;
        }

        let chunks = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]).split(area);
        self.render_table(frame, chunks[0]);
        self.render_summary(frame, chunks[1]);
    }

    fn tick(&mut self, client: &MockForgeClient, tx: &mpsc::UnboundedSender<Event>) {
        let should_fetch = match self.last_fetch {
            Some(t) => t.elapsed().as_secs() >= FETCH_INTERVAL,
            None => true,
        };
        if !should_fetch {
            return;
        }
        self.last_fetch = Some(Instant::now());

        let client = client.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            match client.get_conformance_violations().await {
                Ok(resp) => {
                    if let Ok(payload) = serde_json::to_string(&serde_json::json!({
                        "violations": resp.violations,
                        "total": resp.total,
                    })) {
                        let _ = tx.send(Event::Data {
                            screen: "conformance",
                            payload,
                        });
                    }
                }
                Err(err) => {
                    let _ = tx.send(Event::ApiError {
                        screen: "conformance",
                        message: err.to_string(),
                    });
                }
            }
        });
    }

    fn on_data(&mut self, payload: &str) {
        #[derive(serde::Deserialize)]
        struct Wire {
            violations: Vec<ConformanceViolation>,
            #[serde(default)]
            total: usize,
        }
        match serde_json::from_str::<Wire>(payload) {
            Ok(parsed) => {
                self.table.set_total(parsed.violations.len());
                self.violations = parsed.violations;
                self.total = parsed.total;
                self.loaded = true;
                self.error = None;
            }
            Err(e) => {
                self.error = Some(format!("decode conformance payload: {e}"));
            }
        }
    }

    fn on_error(&mut self, message: &str) {
        self.error = Some(message.to_string());
        self.loaded = true;
    }

    fn force_refresh(&mut self) {
        self.last_fetch = None;
    }

    fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    fn status_hint(&self) -> &str {
        if self.detail_open {
            "Esc:close  j/k:scroll"
        } else {
            "j/k:navigate  g/G:top/bottom  Enter:detail"
        }
    }
}

impl ConformanceScreen {
    fn render_table(&self, frame: &mut Frame, area: Rect) {
        let header = Row::new(vec![
            Cell::from("When").style(Theme::dim()),
            Cell::from("Method").style(Theme::dim()),
            Cell::from("Path").style(Theme::dim()),
            Cell::from("Status").style(Theme::dim()),
            Cell::from("Category").style(Theme::dim()),
            Cell::from("Client").style(Theme::dim()),
        ])
        .height(1);

        let rows: Vec<Row> = self
            .violations
            .iter()
            .skip(self.table.offset)
            .take(self.table.visible_height)
            .map(|v| {
                let status_style = Theme::status_code(v.status);
                let category = if v.category.is_empty() {
                    "(uncategorised)".to_string()
                } else {
                    v.category.clone()
                };
                Row::new(vec![
                    Cell::from(v.timestamp.format("%H:%M:%S").to_string()),
                    Cell::from(v.method.clone()).style(Theme::http_method(&v.method)),
                    Cell::from(v.path.clone()),
                    Cell::from(v.status.to_string()).style(status_style),
                    Cell::from(category),
                    Cell::from(v.client_ip.clone()),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Min(20),
            Constraint::Length(8),
            Constraint::Length(16),
            Constraint::Length(16),
        ];

        let title = if self.total > self.violations.len() {
            format!(
                " Conformance Violations ({} buffered, showing {}) ",
                self.total,
                self.violations.len()
            )
        } else {
            format!(" Conformance Violations ({}) ", self.violations.len())
        };

        let table = Table::new(rows, widths)
            .header(header)
            .row_highlight_style(Theme::highlight())
            .block(
                Block::default()
                    .title(title)
                    .title_style(Theme::title())
                    .borders(Borders::ALL)
                    .border_style(Theme::dim())
                    .style(Theme::surface()),
            );

        let mut table_state = self.table.to_ratatui_state();
        frame.render_stateful_widget(table, area, &mut table_state);
    }

    fn render_summary(&self, frame: &mut Frame, area: Rect) {
        let summary = if self.violations.is_empty() {
            "No spec violations recorded — every incoming request matched the loaded OpenAPI spec."
                .to_string()
        } else {
            let by_category = {
                let mut counts: std::collections::HashMap<&str, usize> =
                    std::collections::HashMap::new();
                for v in &self.violations {
                    let key = if v.category.is_empty() {
                        "(uncategorised)"
                    } else {
                        v.category.as_str()
                    };
                    *counts.entry(key).or_insert(0) += 1;
                }
                let mut pairs: Vec<(&&str, &usize)> = counts.iter().collect();
                pairs.sort_by(|a, b| b.1.cmp(a.1));
                pairs
                    .into_iter()
                    .take(3)
                    .map(|(k, v)| format!("{} ({})", k, v))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            format!("Top categories: {}", by_category)
        };

        let para = Paragraph::new(summary)
            .style(Theme::dim())
            .block(Block::default().borders(Borders::ALL).border_style(Theme::dim()));
        frame.render_widget(para, area);
    }
}
