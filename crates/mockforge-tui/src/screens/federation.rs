//! Federation peer list screen with detail viewer.

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use tokio::sync::mpsc;

use crate::api::client::MockForgeClient;
use crate::api::models::FederationPeer;
use crate::event::Event;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::widgets::table::TableState;

const FETCH_INTERVAL: u64 = 30;

pub struct FederationScreen {
    data: Option<serde_json::Value>,
    peers: Vec<FederationPeer>,
    table: TableState,
    error: Option<String>,
    last_fetch: Option<Instant>,
    show_detail: bool,
}

impl FederationScreen {
    pub fn new() -> Self {
        Self {
            data: None,
            peers: Vec::new(),
            table: TableState::new(),
            error: None,
            last_fetch: None,
            show_detail: false,
        }
    }

    fn selected_peer(&self) -> Option<&FederationPeer> {
        self.peers.get(self.table.selected)
    }
}

impl Screen for FederationScreen {
    fn title(&self) -> &str {
        "Federation"
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Detail overlay.
        if self.show_detail {
            if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')) {
                self.show_detail = false;
                return true;
            }
            return true;
        }

        match key.code {
            KeyCode::Enter | KeyCode::Char('d') => {
                if self.selected_peer().is_some() {
                    self.show_detail = true;
                }
                true
            }
            _ => self.table.handle_key(key),
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if self.data.is_none() {
            let loading = Paragraph::new("Loading federation peers...").style(Theme::dim()).block(
                Block::default()
                    .title(" Federation ")
                    .borders(Borders::ALL)
                    .border_style(Theme::dim()),
            );
            frame.render_widget(loading, area);
            return;
        }

        let header = Row::new(vec![
            Cell::from("ID").style(Theme::dim()),
            Cell::from("URL").style(Theme::dim()),
            Cell::from("Status").style(Theme::dim()),
            Cell::from("Last Sync").style(Theme::dim()),
        ])
        .height(1);

        let rows: Vec<Row> = self
            .peers
            .iter()
            .skip(self.table.offset)
            .take(self.table.visible_height)
            .map(|peer| {
                let status_style = match peer.status.as_str() {
                    "connected" | "healthy" | "up" => Theme::success(),
                    "disconnected" | "down" => Theme::error(),
                    _ => Theme::dim(),
                };
                let last_sync = peer
                    .last_sync
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "--".to_string());
                Row::new(vec![
                    Cell::from(peer.id.clone()),
                    Cell::from(peer.url.clone()),
                    Cell::from(peer.status.clone()).style(status_style),
                    Cell::from(last_sync),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(12),
            Constraint::Min(25),
            Constraint::Length(14),
            Constraint::Length(20),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .row_highlight_style(Theme::highlight())
            .block(
                Block::default()
                    .title(format!(" Federation Peers ({}) ", self.peers.len()))
                    .title_style(Theme::title())
                    .borders(Borders::ALL)
                    .border_style(Theme::dim())
                    .style(Theme::surface()),
            );

        let mut table_state = self.table.to_ratatui_state();
        frame.render_stateful_widget(table, area, &mut table_state);

        // Detail overlay.
        if self.show_detail {
            if let Some(peer) = self.selected_peer() {
                self.render_detail_overlay(frame, peer);
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
            match client.get_federation_peers().await {
                Ok(data) => {
                    let json = serde_json::json!(data
                        .iter()
                        .map(|p| serde_json::json!({
                            "id": p.id,
                            "url": p.url,
                            "status": p.status,
                            "last_sync": p.last_sync,
                        }))
                        .collect::<Vec<_>>());
                    let payload = serde_json::to_string(&json).unwrap_or_default();
                    let _ = tx.send(Event::Data {
                        screen: "federation",
                        payload,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Event::ApiError {
                        screen: "federation",
                        message: e.to_string(),
                    });
                }
            }
        });
    }

    fn on_data(&mut self, payload: &str) {
        match serde_json::from_str::<Vec<FederationPeer>>(payload) {
            Ok(peers) => {
                self.table.set_total(peers.len());
                self.peers = peers;
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
        "j/k:navigate  Enter/d:details  g/G:top/bottom"
    }
}

impl FederationScreen {
    fn render_detail_overlay(&self, frame: &mut Frame, peer: &FederationPeer) {
        let status_style = match peer.status.as_str() {
            "connected" | "healthy" | "up" => Theme::success(),
            "disconnected" | "down" => Theme::error(),
            _ => Theme::dim(),
        };
        let last_sync = peer
            .last_sync
            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "--".to_string());

        let lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  ID:        ", Theme::dim()),
                Span::styled(&peer.id, Style::default().fg(Theme::FG)),
            ]),
            Line::from(vec![
                Span::styled("  URL:       ", Theme::dim()),
                Span::styled(&peer.url, Style::default().fg(Theme::BLUE)),
            ]),
            Line::from(vec![
                Span::styled("  Status:    ", Theme::dim()),
                Span::styled(&peer.status, status_style),
            ]),
            Line::from(vec![
                Span::styled("  Last Sync: ", Theme::dim()),
                Span::styled(last_sync, Style::default().fg(Theme::FG)),
            ]),
            Line::from(""),
            Line::from(Span::styled("  Press Esc to dismiss", Theme::dim())),
        ];

        let block = Block::default()
            .title(" Peer Details ")
            .title_style(Theme::title())
            .borders(Borders::ALL)
            .border_style(Theme::dim())
            .style(Theme::surface());

        let popup_area = centered_rect(50, 35, frame.area());
        frame.render_widget(ratatui::widgets::Clear, popup_area);
        frame.render_widget(Paragraph::new(lines).block(block), popup_area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(ratatui::layout::Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(ratatui::layout::Flex::Center)
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

    fn sample_peers() -> Vec<FederationPeer> {
        vec![
            FederationPeer {
                id: "peer-1".into(),
                url: "http://peer1:9080".into(),
                status: "connected".into(),
                last_sync: None,
            },
            FederationPeer {
                id: "peer-2".into(),
                url: "http://peer2:9080".into(),
                status: "disconnected".into(),
                last_sync: None,
            },
        ]
    }

    #[test]
    fn new_creates_empty_screen() {
        let s = FederationScreen::new();
        assert!(s.peers.is_empty());
        assert!(!s.show_detail);
    }

    #[test]
    fn on_data_parses_peer_list() {
        let mut s = FederationScreen::new();
        let peers = sample_peers();
        let payload = serde_json::to_string(&peers).unwrap();
        s.on_data(&payload);
        assert_eq!(s.peers.len(), 2);
        assert!(s.error.is_none());
    }

    #[test]
    fn enter_on_empty_does_not_show_detail() {
        let mut s = FederationScreen::new();
        s.handle_key(key(KeyCode::Enter));
        assert!(!s.show_detail);
    }

    #[test]
    fn enter_with_peers_shows_detail() {
        let mut s = FederationScreen::new();
        let peers = sample_peers();
        let payload = serde_json::to_string(&peers).unwrap();
        s.on_data(&payload);

        s.handle_key(key(KeyCode::Enter));
        assert!(s.show_detail);
    }

    #[test]
    fn d_key_shows_detail() {
        let mut s = FederationScreen::new();
        let peers = sample_peers();
        let payload = serde_json::to_string(&peers).unwrap();
        s.on_data(&payload);

        s.handle_key(key(KeyCode::Char('d')));
        assert!(s.show_detail);
    }

    #[test]
    fn esc_dismisses_detail() {
        let mut s = FederationScreen::new();
        let peers = sample_peers();
        let payload = serde_json::to_string(&peers).unwrap();
        s.on_data(&payload);

        s.handle_key(key(KeyCode::Enter));
        assert!(s.show_detail);
        s.handle_key(key(KeyCode::Esc));
        assert!(!s.show_detail);
    }

    #[test]
    fn selected_peer_returns_correct_peer() {
        let mut s = FederationScreen::new();
        let peers = sample_peers();
        let payload = serde_json::to_string(&peers).unwrap();
        s.on_data(&payload);

        assert_eq!(s.selected_peer().unwrap().id, "peer-1");
        s.handle_key(key(KeyCode::Char('j')));
        assert_eq!(s.selected_peer().unwrap().id, "peer-2");
    }

    #[test]
    fn status_hint_shows_details() {
        let s = FederationScreen::new();
        assert!(s.status_hint().contains("details"));
    }

    #[test]
    fn force_refresh_clears_last_fetch() {
        let mut s = FederationScreen::new();
        s.last_fetch = Some(Instant::now());
        s.force_refresh();
        assert!(s.last_fetch.is_none());
    }
}
