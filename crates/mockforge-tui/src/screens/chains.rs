//! Chain list table screen with execution support.

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use tokio::sync::mpsc;

use crate::api::client::MockForgeClient;
use crate::api::models::ChainInfo;
use crate::event::Event;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::widgets::confirm::ConfirmDialog;
use crate::widgets::table::TableState;

const FETCH_INTERVAL: u64 = 30;

pub struct ChainsScreen {
    data: Option<serde_json::Value>,
    chains: Vec<ChainInfo>,
    table: TableState,
    error: Option<String>,
    last_fetch: Option<Instant>,
    confirm: ConfirmDialog,
    pending_execution: Option<String>,
    last_result: Option<ExecutionResult>,
    show_result: bool,
}

struct ExecutionResult {
    chain_name: String,
    success: bool,
    message: String,
}

impl ChainsScreen {
    pub fn new() -> Self {
        Self {
            data: None,
            chains: Vec::new(),
            table: TableState::new(),
            error: None,
            last_fetch: None,
            confirm: ConfirmDialog::new(),
            pending_execution: None,
            last_result: None,
            show_result: false,
        }
    }

    fn selected_chain(&self) -> Option<&ChainInfo> {
        self.chains.get(self.table.selected)
    }

    fn render_result_overlay(&self, frame: &mut Frame) {
        if let Some(ref result) = self.last_result {
            let style = if result.success {
                Theme::success()
            } else {
                Theme::error()
            };
            let lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Chain: ", Theme::dim()),
                    Span::styled(&result.chain_name, Theme::base()),
                ]),
                Line::from(vec![
                    Span::styled("  Result: ", Theme::dim()),
                    Span::styled(&result.message, style),
                ]),
                Line::from(""),
                Line::from(Span::styled("  Press Esc to dismiss", Theme::dim())),
            ];

            let block = Block::default()
                .title(" Execution Result ")
                .title_style(Theme::title())
                .borders(Borders::ALL)
                .border_style(Theme::dim())
                .style(Theme::surface());

            let popup_area = centered_rect(50, 30, frame.area());
            frame.render_widget(ratatui::widgets::Clear, popup_area);
            frame.render_widget(Paragraph::new(lines).block(block), popup_area);
        }
    }
}

impl Screen for ChainsScreen {
    fn title(&self) -> &str {
        "Chains"
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Confirm dialog takes priority.
        if self.confirm.visible {
            if let Some(confirmed) = self.confirm.handle_key(key) {
                if confirmed {
                    if let Some(chain) = self.selected_chain() {
                        self.pending_execution = Some(chain.id.clone());
                    }
                }
                return true;
            }
            return true;
        }

        // Dismiss result overlay.
        if self.show_result {
            if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')) {
                self.show_result = false;
                return true;
            }
            return true;
        }

        match key.code {
            KeyCode::Char('x') | KeyCode::Enter => {
                if let Some(chain) = self.selected_chain() {
                    let name = chain.name.clone();
                    self.confirm.show("Execute Chain", format!("Execute chain \"{name}\"?"));
                }
                true
            }
            _ => self.table.handle_key(key),
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if self.data.is_none() {
            let loading = Paragraph::new("Loading chains...").style(Theme::dim()).block(
                Block::default()
                    .title(" Chains ")
                    .borders(Borders::ALL)
                    .border_style(Theme::dim()),
            );
            frame.render_widget(loading, area);
            return;
        }

        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).split(area);

        // Chain table
        let header = Row::new(vec![
            Cell::from("ID").style(Theme::dim()),
            Cell::from("Name").style(Theme::dim()),
            Cell::from("Steps").style(Theme::dim()),
            Cell::from("Description").style(Theme::dim()),
        ])
        .height(1);

        let rows: Vec<Row> = self
            .chains
            .iter()
            .skip(self.table.offset)
            .take(self.table.visible_height)
            .map(|chain| {
                Row::new(vec![
                    Cell::from(chain.id.clone()),
                    Cell::from(chain.name.clone()),
                    Cell::from(chain.steps.len().to_string()),
                    Cell::from(chain.description.clone()),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(12),
            Constraint::Length(20),
            Constraint::Length(6),
            Constraint::Min(20),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .row_highlight_style(Theme::highlight())
            .block(
                Block::default()
                    .title(format!(" Chains ({}) ", self.chains.len()))
                    .title_style(Theme::title())
                    .borders(Borders::ALL)
                    .border_style(Theme::dim())
                    .style(Theme::surface()),
            );

        let mut table_state = self.table.to_ratatui_state();
        frame.render_stateful_widget(table, chunks[0], &mut table_state);

        // Last execution result bar
        let result_line = if let Some(ref result) = self.last_result {
            let style = if result.success {
                Theme::success()
            } else {
                Theme::error()
            };
            let icon = if result.success { "OK" } else { "FAIL" };
            Line::from(vec![
                Span::styled(format!("  Last: [{icon}] "), style),
                Span::styled(&result.chain_name, Theme::base()),
                Span::styled(format!(" — {}", result.message), Theme::dim()),
            ])
        } else {
            Line::from(Span::styled("  No executions yet", Theme::dim()))
        };

        let result_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());
        let result_paragraph = Paragraph::new(result_line).block(result_block);
        frame.render_widget(result_paragraph, chunks[1]);

        // Confirm dialog overlay
        self.confirm.render(frame);

        // Result detail overlay
        if self.show_result {
            self.render_result_overlay(frame);
        }
    }

    fn tick(&mut self, client: &MockForgeClient, tx: &mpsc::UnboundedSender<Event>) {
        // Handle pending execution.
        if let Some(chain_id) = self.pending_execution.take() {
            let chain_name = self
                .chains
                .iter()
                .find(|c| c.id == chain_id)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| chain_id.clone());
            let client = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let result = match client.execute_chain(&chain_id).await {
                    Ok(data) => {
                        let msg = data
                            .as_str()
                            .map(String::from)
                            .unwrap_or_else(|| "Executed successfully".into());
                        serde_json::json!({
                            "type": "execution_result",
                            "chain_name": chain_name,
                            "success": true,
                            "message": msg,
                        })
                    }
                    Err(e) => serde_json::json!({
                        "type": "execution_result",
                        "chain_name": chain_name,
                        "success": false,
                        "message": e.to_string(),
                    }),
                };
                let _ = tx.send(Event::Data {
                    screen: "chains",
                    payload: serde_json::to_string(&result).unwrap_or_default(),
                });
            });
        }

        // Periodic data fetch.
        let should_fetch =
            self.last_fetch.map_or(true, |t| t.elapsed().as_secs() >= FETCH_INTERVAL);
        if !should_fetch {
            return;
        }
        self.last_fetch = Some(Instant::now());

        let client = client.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            match client.get_chains().await {
                Ok(data) => {
                    let json = serde_json::json!(data
                        .iter()
                        .map(|c| serde_json::json!({
                            "id": c.id,
                            "name": c.name,
                            "description": c.description,
                            "steps": c.steps,
                        }))
                        .collect::<Vec<_>>());
                    let payload = serde_json::to_string(&json).unwrap_or_default();
                    let _ = tx.send(Event::Data {
                        screen: "chains",
                        payload,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Event::ApiError {
                        screen: "chains",
                        message: e.to_string(),
                    });
                }
            }
        });
    }

    fn on_data(&mut self, payload: &str) {
        // Check for execution result message.
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(payload) {
            if val.get("type").and_then(|v| v.as_str()) == Some("execution_result") {
                self.last_result = Some(ExecutionResult {
                    chain_name: val
                        .get("chain_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    success: val.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
                    message: val
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("done")
                        .to_string(),
                });
                self.show_result = true;
                // Force refresh to get updated chain state.
                self.last_fetch = None;
                return;
            }
        }

        // Normal chain list data.
        match serde_json::from_str::<Vec<ChainInfo>>(payload) {
            Ok(chains) => {
                self.table.set_total(chains.len());
                self.chains = chains;
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
        "j/k:navigate  x/Enter:execute  g/G:top/bottom"
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    use ratatui::layout::{Constraint, Flex, Layout};
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
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

    fn sample_chains() -> Vec<ChainInfo> {
        vec![
            ChainInfo {
                id: "chain-1".into(),
                name: "Login Flow".into(),
                steps: vec![serde_json::json!({"action": "login"})],
                description: "Simulates login".into(),
            },
            ChainInfo {
                id: "chain-2".into(),
                name: "CRUD Flow".into(),
                steps: vec![
                    serde_json::json!({"action": "create"}),
                    serde_json::json!({"action": "read"}),
                ],
                description: "Create-read cycle".into(),
            },
        ]
    }

    #[test]
    fn new_creates_empty_screen() {
        let s = ChainsScreen::new();
        assert!(s.chains.is_empty());
        assert!(s.pending_execution.is_none());
        assert!(s.last_result.is_none());
        assert!(!s.show_result);
    }

    #[test]
    fn on_data_parses_chain_list() {
        let mut s = ChainsScreen::new();
        let chains = sample_chains();
        let payload = serde_json::to_string(&chains).unwrap();
        s.on_data(&payload);
        assert_eq!(s.chains.len(), 2);
        assert!(s.error.is_none());
    }

    #[test]
    fn enter_on_empty_list_does_not_crash() {
        let mut s = ChainsScreen::new();
        assert!(s.handle_key(key(KeyCode::Enter)));
        assert!(!s.confirm.visible);
    }

    #[test]
    fn enter_with_selection_shows_confirm() {
        let mut s = ChainsScreen::new();
        let chains = sample_chains();
        let payload = serde_json::to_string(&chains).unwrap();
        s.on_data(&payload);

        // Navigate to first item and press Enter.
        s.handle_key(key(KeyCode::Char('j')));
        assert!(s.handle_key(key(KeyCode::Enter)));
        assert!(s.confirm.visible);
    }

    #[test]
    fn x_key_shows_confirm() {
        let mut s = ChainsScreen::new();
        let chains = sample_chains();
        let payload = serde_json::to_string(&chains).unwrap();
        s.on_data(&payload);

        s.handle_key(key(KeyCode::Char('j')));
        assert!(s.handle_key(key(KeyCode::Char('x'))));
        assert!(s.confirm.visible);
    }

    #[test]
    fn confirm_yes_sets_pending_execution() {
        let mut s = ChainsScreen::new();
        let chains = sample_chains();
        let payload = serde_json::to_string(&chains).unwrap();
        s.on_data(&payload);

        s.handle_key(key(KeyCode::Char('j')));
        s.handle_key(key(KeyCode::Char('x')));
        assert!(s.confirm.visible);

        // Confirm with 'y'.
        s.handle_key(key(KeyCode::Char('y')));
        assert!(!s.confirm.visible);
        assert!(s.pending_execution.is_some());
    }

    #[test]
    fn confirm_no_clears_without_execution() {
        let mut s = ChainsScreen::new();
        let chains = sample_chains();
        let payload = serde_json::to_string(&chains).unwrap();
        s.on_data(&payload);

        s.handle_key(key(KeyCode::Char('j')));
        s.handle_key(key(KeyCode::Char('x')));
        s.handle_key(key(KeyCode::Char('n')));
        assert!(!s.confirm.visible);
        assert!(s.pending_execution.is_none());
    }

    #[test]
    fn execution_result_sets_last_result() {
        let mut s = ChainsScreen::new();
        let result = serde_json::json!({
            "type": "execution_result",
            "chain_name": "Login Flow",
            "success": true,
            "message": "Executed successfully",
        });
        s.on_data(&serde_json::to_string(&result).unwrap());
        assert!(s.last_result.is_some());
        assert!(s.show_result);
        let r = s.last_result.as_ref().unwrap();
        assert!(r.success);
        assert_eq!(r.chain_name, "Login Flow");
    }

    #[test]
    fn esc_dismisses_result_overlay() {
        let mut s = ChainsScreen::new();
        let result = serde_json::json!({
            "type": "execution_result",
            "chain_name": "Test",
            "success": true,
            "message": "ok",
        });
        s.on_data(&serde_json::to_string(&result).unwrap());
        assert!(s.show_result);
        s.handle_key(key(KeyCode::Esc));
        assert!(!s.show_result);
    }

    #[test]
    fn status_hint_shows_execute() {
        let s = ChainsScreen::new();
        assert!(s.status_hint().contains("execute"));
    }

    #[test]
    fn force_refresh_clears_last_fetch() {
        let mut s = ChainsScreen::new();
        s.last_fetch = Some(Instant::now());
        s.force_refresh();
        assert!(s.last_fetch.is_none());
    }
}
