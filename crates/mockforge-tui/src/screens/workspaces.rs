//! Workspace list table screen with activation support.

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
use crate::api::models::WorkspaceInfo;
use crate::event::Event;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::widgets::confirm::ConfirmDialog;
use crate::widgets::table::TableState;

const FETCH_INTERVAL: u64 = 30;

pub struct WorkspacesScreen {
    data: Option<serde_json::Value>,
    workspaces: Vec<WorkspaceInfo>,
    table: TableState,
    error: Option<String>,
    last_fetch: Option<Instant>,
    confirm: ConfirmDialog,
    pending_activation: Option<String>,
    status_message: Option<(bool, String)>,
}

impl WorkspacesScreen {
    pub fn new() -> Self {
        Self {
            data: None,
            workspaces: Vec::new(),
            table: TableState::new(),
            error: None,
            last_fetch: None,
            confirm: ConfirmDialog::new(),
            pending_activation: None,
            status_message: None,
        }
    }

    fn selected_workspace(&self) -> Option<&WorkspaceInfo> {
        self.workspaces.get(self.table.selected)
    }
}

impl Screen for WorkspacesScreen {
    fn title(&self) -> &str {
        "Workspaces"
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Confirm dialog takes priority.
        if self.confirm.visible {
            if let Some(confirmed) = self.confirm.handle_key(key) {
                if confirmed {
                    if let Some(ws) = self.selected_workspace() {
                        self.pending_activation = Some(ws.id.clone());
                    }
                }
                return true;
            }
            return true;
        }

        match key.code {
            KeyCode::Char('a') | KeyCode::Enter => {
                if let Some(ws) = self.selected_workspace() {
                    if ws.active {
                        self.status_message =
                            Some((true, format!("\"{}\" is already active", ws.name)));
                    } else {
                        let name = ws.name.clone();
                        self.confirm
                            .show("Activate Workspace", format!("Activate workspace \"{name}\"?"));
                    }
                }
                true
            }
            KeyCode::Char('c') => {
                self.status_message = None;
                true
            }
            _ => self.table.handle_key(key),
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if self.data.is_none() {
            let loading = Paragraph::new("Loading workspaces...").style(Theme::dim()).block(
                Block::default()
                    .title(" Workspaces ")
                    .borders(Borders::ALL)
                    .border_style(Theme::dim()),
            );
            frame.render_widget(loading, area);
            self.confirm.render(frame);
            return;
        }

        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).split(area);

        let header = Row::new(vec![
            Cell::from("ID").style(Theme::dim()),
            Cell::from("Name").style(Theme::dim()),
            Cell::from("Description").style(Theme::dim()),
            Cell::from("Active").style(Theme::dim()),
            Cell::from("Environments").style(Theme::dim()),
        ])
        .height(1);

        let rows: Vec<Row> = self
            .workspaces
            .iter()
            .skip(self.table.offset)
            .take(self.table.visible_height)
            .map(|ws| {
                let active_style = if ws.active {
                    Theme::success()
                } else {
                    Theme::dim()
                };
                Row::new(vec![
                    Cell::from(ws.id.clone()),
                    Cell::from(ws.name.clone()),
                    Cell::from(ws.description.clone()),
                    Cell::from(if ws.active { "yes" } else { "no" }).style(active_style),
                    Cell::from(ws.environments.join(", ")),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(12),
            Constraint::Length(20),
            Constraint::Min(20),
            Constraint::Length(8),
            Constraint::Length(20),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .row_highlight_style(Theme::highlight())
            .block(
                Block::default()
                    .title(format!(" Workspaces ({}) ", self.workspaces.len()))
                    .title_style(Theme::title())
                    .borders(Borders::ALL)
                    .border_style(Theme::dim())
                    .style(Theme::surface()),
            );

        let mut table_state = self.table.to_ratatui_state();
        frame.render_stateful_widget(table, chunks[0], &mut table_state);

        // Status message bar.
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
        // Handle pending activation.
        if let Some(workspace_id) = self.pending_activation.take() {
            let ws_name = self
                .workspaces
                .iter()
                .find(|w| w.id == workspace_id)
                .map(|w| w.name.clone())
                .unwrap_or_else(|| workspace_id.clone());
            let client = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let result = match client.activate_workspace(&workspace_id).await {
                    Ok(msg) => serde_json::json!({
                        "type": "activation_result",
                        "success": true,
                        "message": if msg.is_empty() {
                            format!("Workspace \"{ws_name}\" activated")
                        } else {
                            msg
                        },
                    }),
                    Err(e) => serde_json::json!({
                        "type": "activation_result",
                        "success": false,
                        "message": e.to_string(),
                    }),
                };
                let _ = tx.send(Event::Data {
                    screen: "workspaces",
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
            match client.get_workspaces().await {
                Ok(data) => {
                    let json = serde_json::json!(data
                        .iter()
                        .map(|ws| serde_json::json!({
                            "id": ws.id,
                            "name": ws.name,
                            "description": ws.description,
                            "active": ws.active,
                            "environments": ws.environments,
                        }))
                        .collect::<Vec<_>>());
                    let payload = serde_json::to_string(&json).unwrap_or_default();
                    let _ = tx.send(Event::Data {
                        screen: "workspaces",
                        payload,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Event::ApiError {
                        screen: "workspaces",
                        message: e.to_string(),
                    });
                }
            }
        });
    }

    fn on_data(&mut self, payload: &str) {
        // Check for activation result.
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(payload) {
            if val.get("type").and_then(|v| v.as_str()) == Some("activation_result") {
                let success = val.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
                let message =
                    val.get("message").and_then(|v| v.as_str()).unwrap_or("done").to_string();
                self.status_message = Some((success, message));
                // Force refresh to see updated active state.
                self.last_fetch = None;
                return;
            }
        }

        // Normal workspace list data.
        match serde_json::from_str::<Vec<WorkspaceInfo>>(payload) {
            Ok(workspaces) => {
                self.table.set_total(workspaces.len());
                self.workspaces = workspaces;
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
        "j/k:navigate  a/Enter:activate  c:clear-message  g/G:top/bottom"
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

    fn sample_workspaces() -> Vec<WorkspaceInfo> {
        vec![
            WorkspaceInfo {
                id: "ws-1".into(),
                name: "Development".into(),
                description: "Dev workspace".into(),
                active: true,
                created_at: None,
                environments: vec!["dev".into(), "staging".into()],
            },
            WorkspaceInfo {
                id: "ws-2".into(),
                name: "Production".into(),
                description: "Prod workspace".into(),
                active: false,
                created_at: None,
                environments: vec!["prod".into()],
            },
        ]
    }

    #[test]
    fn new_creates_empty_screen() {
        let s = WorkspacesScreen::new();
        assert!(s.workspaces.is_empty());
        assert!(s.pending_activation.is_none());
        assert!(s.status_message.is_none());
    }

    #[test]
    fn on_data_parses_workspace_list() {
        let mut s = WorkspacesScreen::new();
        let workspaces = sample_workspaces();
        let payload = serde_json::to_string(&workspaces).unwrap();
        s.on_data(&payload);
        assert_eq!(s.workspaces.len(), 2);
        assert!(s.error.is_none());
    }

    #[test]
    fn activate_already_active_shows_message() {
        let mut s = WorkspacesScreen::new();
        let workspaces = sample_workspaces();
        let payload = serde_json::to_string(&workspaces).unwrap();
        s.on_data(&payload);

        // First workspace (index 0) is active. selected starts at 0.
        s.handle_key(key(KeyCode::Char('a')));
        // Should NOT show confirm, should show status message.
        assert!(!s.confirm.visible);
        assert!(s.status_message.is_some());
        let (success, msg) = s.status_message.as_ref().unwrap();
        assert!(success);
        assert!(msg.contains("already active"));
    }

    #[test]
    fn activate_inactive_shows_confirm() {
        let mut s = WorkspacesScreen::new();
        let workspaces = sample_workspaces();
        let payload = serde_json::to_string(&workspaces).unwrap();
        s.on_data(&payload);

        // Navigate to second workspace (inactive) at index 1.
        s.handle_key(key(KeyCode::Char('j')));
        s.handle_key(key(KeyCode::Char('a')));
        assert!(s.confirm.visible);
    }

    #[test]
    fn confirm_yes_sets_pending_activation() {
        let mut s = WorkspacesScreen::new();
        let workspaces = sample_workspaces();
        let payload = serde_json::to_string(&workspaces).unwrap();
        s.on_data(&payload);

        // Navigate to inactive workspace (index 1).
        s.handle_key(key(KeyCode::Char('j')));
        s.handle_key(key(KeyCode::Char('a')));
        s.handle_key(key(KeyCode::Char('y')));
        assert!(!s.confirm.visible);
        assert_eq!(s.pending_activation, Some("ws-2".into()));
    }

    #[test]
    fn confirm_no_does_not_activate() {
        let mut s = WorkspacesScreen::new();
        let workspaces = sample_workspaces();
        let payload = serde_json::to_string(&workspaces).unwrap();
        s.on_data(&payload);

        // Navigate to inactive workspace (index 1).
        s.handle_key(key(KeyCode::Char('j')));
        s.handle_key(key(KeyCode::Char('a')));
        s.handle_key(key(KeyCode::Char('n')));
        assert!(s.pending_activation.is_none());
    }

    #[test]
    fn activation_result_sets_status_message() {
        let mut s = WorkspacesScreen::new();
        let result = serde_json::json!({
            "type": "activation_result",
            "success": true,
            "message": "Workspace activated",
        });
        s.on_data(&serde_json::to_string(&result).unwrap());
        assert!(s.status_message.is_some());
        let (success, msg) = s.status_message.as_ref().unwrap();
        assert!(success);
        assert_eq!(msg, "Workspace activated");
    }

    #[test]
    fn c_key_clears_status_message() {
        let mut s = WorkspacesScreen::new();
        s.status_message = Some((true, "Test".into()));
        s.handle_key(key(KeyCode::Char('c')));
        assert!(s.status_message.is_none());
    }

    #[test]
    fn status_hint_shows_activate() {
        let s = WorkspacesScreen::new();
        assert!(s.status_hint().contains("activate"));
    }

    #[test]
    fn force_refresh_clears_last_fetch() {
        let mut s = WorkspacesScreen::new();
        s.last_fetch = Some(Instant::now());
        s.force_refresh();
        assert!(s.last_fetch.is_none());
    }
}
