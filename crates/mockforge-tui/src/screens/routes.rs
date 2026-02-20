//! Sortable route table screen.

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};
use tokio::sync::mpsc;

use crate::api::client::MockForgeClient;
use crate::api::models::RouteInfo;
use crate::event::Event;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::widgets::filter::FilterInput;
use crate::widgets::table::TableState;

const COLUMNS: &[&str] = &[
    "Method", "Path", "Requests", "Errors", "Latency", "Fixtures",
];

pub struct RoutesScreen {
    routes: Vec<RouteInfo>,
    filtered: Vec<usize>,
    table: TableState,
    filter: FilterInput,
    error: Option<String>,
    last_fetch: Option<Instant>,
}

impl RoutesScreen {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            filtered: Vec::new(),
            table: TableState::new(),
            filter: FilterInput::new(),
            error: None,
            last_fetch: None,
        }
    }

    fn rebuild_filtered(&mut self) {
        self.filtered = self
            .routes
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                let text = format!("{} {}", r.method.as_deref().unwrap_or("ANY"), r.path);
                self.filter.matches(&text)
            })
            .map(|(i, _)| i)
            .collect();
        self.sort_filtered();
        self.table.set_total(self.filtered.len());
    }

    fn sort_filtered(&mut self) {
        let routes = &self.routes;
        let col = self.table.sort_column;
        let asc = self.table.sort_ascending;

        self.filtered.sort_by(|&a, &b| {
            let ra = &routes[a];
            let rb = &routes[b];
            let cmp = match col {
                0 => ra.method.cmp(&rb.method),
                1 => ra.path.cmp(&rb.path),
                2 => ra.request_count.cmp(&rb.request_count),
                3 => ra.error_count.cmp(&rb.error_count),
                4 => ra.latency_ms.cmp(&rb.latency_ms),
                5 => ra.has_fixtures.cmp(&rb.has_fixtures),
                _ => std::cmp::Ordering::Equal,
            };
            if asc {
                cmp
            } else {
                cmp.reverse()
            }
        });
    }
}

impl Screen for RoutesScreen {
    fn title(&self) -> &str {
        "Routes"
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.filter.active {
            let consumed = self.filter.handle_key(key);
            if consumed {
                self.rebuild_filtered();
            }
            return consumed;
        }

        match key.code {
            KeyCode::Char('/') => {
                self.filter.activate();
                true
            }
            KeyCode::Char('s') => {
                self.table.next_sort(COLUMNS.len());
                self.sort_filtered();
                true
            }
            _ => self.table.handle_key(key),
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).split(area);

        // Filter bar
        self.filter.render(frame, chunks[0]);

        // Column headers
        let header_cells: Vec<Cell> = COLUMNS
            .iter()
            .enumerate()
            .map(|(i, &name)| {
                let style = if i == self.table.sort_column {
                    Theme::title().add_modifier(Modifier::UNDERLINED)
                } else {
                    Theme::dim()
                };
                let arrow = if i == self.table.sort_column {
                    if self.table.sort_ascending {
                        " ▲"
                    } else {
                        " ▼"
                    }
                } else {
                    ""
                };
                Cell::from(format!("{name}{arrow}")).style(style)
            })
            .collect();
        let header = Row::new(header_cells).height(1);

        // Data rows
        let rows: Vec<Row> = self
            .filtered
            .iter()
            .skip(self.table.offset)
            .take(self.table.visible_height)
            .map(|&idx| {
                let route = &self.routes[idx];
                let method = route.method.as_deref().unwrap_or("ANY");
                Row::new(vec![
                    Cell::from(method.to_string()).style(Theme::http_method(method)),
                    Cell::from(route.path.clone()),
                    Cell::from(route.request_count.to_string()),
                    Cell::from(route.error_count.to_string()).style(if route.error_count > 0 {
                        Theme::error()
                    } else {
                        Theme::base()
                    }),
                    Cell::from(route.latency_ms.map_or("—".to_string(), |ms| format!("{ms}ms"))),
                    Cell::from(if route.has_fixtures { "✓" } else { "—" }),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(8),
            Constraint::Min(20),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(9),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .row_highlight_style(Theme::highlight())
            .block(
                Block::default()
                    .title(format!(" Routes ({}) ", self.filtered.len()))
                    .title_style(Theme::title())
                    .borders(Borders::ALL)
                    .border_style(Theme::dim())
                    .style(Theme::surface()),
            );

        let mut table_state = self.table.to_ratatui_state();
        frame.render_stateful_widget(table, chunks[1], &mut table_state);
    }

    fn tick(&mut self, client: &MockForgeClient, tx: &mpsc::UnboundedSender<Event>) {
        let should_fetch = self.last_fetch.map_or(true, |t| t.elapsed().as_secs() >= 30);
        if !should_fetch {
            return;
        }
        self.last_fetch = Some(Instant::now());

        let client = client.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            match client.get_routes().await {
                Ok(routes) => {
                    let json = serde_json::to_string(&routes).unwrap_or_default();
                    let _ = tx.send(Event::Data {
                        screen: "routes",
                        payload: json,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Event::ApiError {
                        screen: "routes",
                        message: e.to_string(),
                    });
                }
            }
        });
    }

    fn on_data(&mut self, payload: &str) {
        match serde_json::from_str::<Vec<RouteInfo>>(payload) {
            Ok(routes) => {
                self.routes = routes;
                self.rebuild_filtered();
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
        "s:sort  /:filter  j/k:scroll  Enter:details"
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

    fn sample_payload() -> String {
        let payload = serde_json::json!([
            {
                "path": "/api/users",
                "method": "GET",
                "request_count": 100,
                "error_count": 2,
                "latency_ms": 45,
                "has_fixtures": true
            },
            {
                "path": "/api/items",
                "method": "POST",
                "request_count": 50,
                "error_count": 0,
                "latency_ms": null,
                "has_fixtures": false
            }
        ]);
        serde_json::to_string(&payload).unwrap()
    }

    #[test]
    fn new_creates_screen_with_expected_defaults() {
        let screen = RoutesScreen::new();
        assert!(screen.routes.is_empty());
        assert!(screen.filtered.is_empty());
        assert!(screen.error.is_none());
        assert!(screen.last_fetch.is_none());
    }

    #[test]
    fn on_data_parses_route_info_json_array() {
        let mut screen = RoutesScreen::new();
        screen.on_data(&sample_payload());

        assert_eq!(screen.routes.len(), 2);
        assert_eq!(screen.filtered.len(), 2);
        assert!(screen.error.is_none());

        assert_eq!(screen.routes[0].path, "/api/users");
        assert_eq!(screen.routes[0].method.as_deref(), Some("GET"));
        assert_eq!(screen.routes[0].request_count, 100);
        assert_eq!(screen.routes[0].error_count, 2);
        assert_eq!(screen.routes[0].latency_ms, Some(45));
        assert!(screen.routes[0].has_fixtures);

        assert_eq!(screen.routes[1].path, "/api/items");
        assert_eq!(screen.routes[1].method.as_deref(), Some("POST"));
        assert_eq!(screen.routes[1].request_count, 50);
        assert_eq!(screen.routes[1].error_count, 0);
        assert!(screen.routes[1].latency_ms.is_none());
        assert!(!screen.routes[1].has_fixtures);
    }

    #[test]
    fn on_data_with_invalid_json_sets_error() {
        let mut screen = RoutesScreen::new();
        screen.on_data("not valid json {{{");

        assert!(screen.error.is_some());
        let err = screen.error.as_ref().unwrap();
        assert!(
            err.contains("Parse error"),
            "Expected error to contain 'Parse error', got: {err}"
        );
    }

    #[test]
    fn sort_cycles_through_columns() {
        let mut screen = RoutesScreen::new();
        screen.on_data(&sample_payload());

        // Initial sort column is 0 (Method), ascending
        assert_eq!(screen.table.sort_column, 0);
        assert!(screen.table.sort_ascending);

        // Press 's' to advance sort column
        screen.handle_key(key(KeyCode::Char('s')));
        assert_eq!(screen.table.sort_column, 1); // Path

        screen.handle_key(key(KeyCode::Char('s')));
        assert_eq!(screen.table.sort_column, 2); // Requests

        screen.handle_key(key(KeyCode::Char('s')));
        assert_eq!(screen.table.sort_column, 3); // Errors

        screen.handle_key(key(KeyCode::Char('s')));
        assert_eq!(screen.table.sort_column, 4); // Latency

        screen.handle_key(key(KeyCode::Char('s')));
        assert_eq!(screen.table.sort_column, 5); // Fixtures
    }

    #[test]
    fn sort_toggles_ascending_descending_on_wrap() {
        let mut screen = RoutesScreen::new();
        screen.on_data(&sample_payload());

        assert!(screen.table.sort_ascending);

        // Cycle through all 6 columns (0..5), then wrap
        for _ in 0..COLUMNS.len() {
            screen.handle_key(key(KeyCode::Char('s')));
        }
        // After wrapping back to column 0, ascending should toggle
        assert_eq!(screen.table.sort_column, 0);
        assert!(!screen.table.sort_ascending);

        // Cycle through all 6 again
        for _ in 0..COLUMNS.len() {
            screen.handle_key(key(KeyCode::Char('s')));
        }
        assert_eq!(screen.table.sort_column, 0);
        assert!(screen.table.sort_ascending);
    }

    #[test]
    fn filter_activates_with_slash_key() {
        let mut screen = RoutesScreen::new();
        assert!(!screen.filter.active);

        let consumed = screen.handle_key(key(KeyCode::Char('/')));
        assert!(consumed);
        assert!(screen.filter.active);
    }

    #[test]
    fn rebuild_filtered_respects_filter_text() {
        let mut screen = RoutesScreen::new();
        screen.on_data(&sample_payload());
        assert_eq!(screen.filtered.len(), 2);

        // Set filter text to match only "GET /api/users"
        screen.filter.text = "GET".to_string();
        screen.rebuild_filtered();
        assert_eq!(screen.filtered.len(), 1);

        // The remaining filtered index should point to the GET route
        let idx = screen.filtered[0];
        assert_eq!(screen.routes[idx].method.as_deref(), Some("GET"));

        // Filter for POST
        screen.filter.text = "POST".to_string();
        screen.rebuild_filtered();
        assert_eq!(screen.filtered.len(), 1);
        let idx = screen.filtered[0];
        assert_eq!(screen.routes[idx].method.as_deref(), Some("POST"));

        // Filter with no match
        screen.filter.text = "DELETE".to_string();
        screen.rebuild_filtered();
        assert_eq!(screen.filtered.len(), 0);

        // Clear filter shows all again
        screen.filter.text.clear();
        screen.rebuild_filtered();
        assert_eq!(screen.filtered.len(), 2);
    }

    #[test]
    fn table_navigation_with_j_k() {
        let mut screen = RoutesScreen::new();
        screen.on_data(&sample_payload());

        // Initial selection is 0
        assert_eq!(screen.table.selected, 0);

        // 'j' scrolls down
        let consumed = screen.handle_key(key(KeyCode::Char('j')));
        assert!(consumed);
        assert_eq!(screen.table.selected, 1);

        // 'k' scrolls back up
        let consumed = screen.handle_key(key(KeyCode::Char('k')));
        assert!(consumed);
        assert_eq!(screen.table.selected, 0);

        // 'k' at top stays at 0
        screen.handle_key(key(KeyCode::Char('k')));
        assert_eq!(screen.table.selected, 0);
    }

    #[test]
    fn force_refresh_clears_last_fetch() {
        let mut screen = RoutesScreen::new();
        screen.last_fetch = Some(Instant::now());
        assert!(screen.last_fetch.is_some());

        screen.force_refresh();
        assert!(screen.last_fetch.is_none());
    }

    #[test]
    fn status_hint_contains_expected_keywords() {
        let screen = RoutesScreen::new();
        let hint = screen.status_hint();
        assert!(hint.contains("sort"), "Expected hint to contain 'sort'");
        assert!(hint.contains("filter"), "Expected hint to contain 'filter'");
        assert!(hint.contains("scroll"), "Expected hint to contain 'scroll'");
    }
}
