//! Conformance violations screen — server-side spec violations on
//! incoming requests.
//!
//! Issue #79 round 12 — Srikanth's ask: surface conformance failures
//! that the MockForge server detects on incoming traffic, so users can
//! cross-check the server's view against their proxy's. The bench-side
//! conformance suite already had its own report; this screen handles
//! the *serve-time* equivalent.
//!
//! Extras shipped on top of the bare request (round 12 follow-on):
//!  - keyboard filters (m / s / c) for method, status, category
//!  - pause auto-refresh (p) so the table doesn't jump while investigating
//!  - export current (filtered) view to JSON (e) for offline cross-check
//!    with proxy logs
//!  - clear the server-side buffer (D)
//!  - per-endpoint count panel showing top offending `METHOD path` pairs

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use tokio::sync::mpsc;

use crate::api::client::MockForgeClient;
use crate::api::models::{ConformanceViolation, UnknownPathRequest};
use crate::event::Event;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::widgets::table::TableState;

const FETCH_INTERVAL: u64 = 5;

/// Method-filter cycle state. `All` means no method filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MethodFilter {
    All,
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl MethodFilter {
    fn next(self) -> Self {
        match self {
            Self::All => Self::Get,
            Self::Get => Self::Post,
            Self::Post => Self::Put,
            Self::Put => Self::Patch,
            Self::Patch => Self::Delete,
            Self::Delete => Self::All,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::All => "any",
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
        }
    }
    fn matches(self, method: &str) -> bool {
        match self {
            Self::All => true,
            _ => method.eq_ignore_ascii_case(self.label()),
        }
    }
}

/// Status-filter cycle: all → 4xx → 422 → 5xx → all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusFilter {
    All,
    Client4xx,
    Exact422,
    Server5xx,
}

impl StatusFilter {
    fn next(self) -> Self {
        match self {
            Self::All => Self::Client4xx,
            Self::Client4xx => Self::Exact422,
            Self::Exact422 => Self::Server5xx,
            Self::Server5xx => Self::All,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::All => "any",
            Self::Client4xx => "4xx",
            Self::Exact422 => "422",
            Self::Server5xx => "5xx",
        }
    }
    fn matches(self, status: u16) -> bool {
        match self {
            Self::All => true,
            Self::Client4xx => (400..500).contains(&status),
            Self::Exact422 => status == 422,
            Self::Server5xx => (500..600).contains(&status),
        }
    }
}

/// Which feed the screen is showing — request-side spec violations
/// (default) or the round-13 unknown-paths feed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Violations,
    UnknownPaths,
}

pub struct ConformanceScreen {
    loaded: bool,
    /// Full snapshot from the server, newest-first.
    violations: Vec<ConformanceViolation>,
    total: usize,
    /// Round-13: unmatched-path requests captured by the HTTP fallback.
    unknown_paths: Vec<UnknownPathRequest>,
    unknown_total: usize,
    /// Toggled by `u` between violations and unknown-paths views.
    view_mode: ViewMode,
    table: TableState,
    error: Option<String>,
    last_fetch: Option<Instant>,
    detail_open: bool,
    detail_scroll: u16,
    /// Filter state (Issue #79 round 12 extras).
    method_filter: MethodFilter,
    status_filter: StatusFilter,
    /// Cycles through the categories observed in the current snapshot
    /// plus `"any"`. Stored as the chosen category string (lowercased)
    /// or `None` for "any".
    category_filter: Option<String>,
    paused: bool,
    /// Transient status message shown briefly after an action (e.g.
    /// "exported 23 violations to …").
    flash: Option<(String, Instant)>,
    /// Set by the `D` keystroke; the next tick fires the DELETE call.
    pending_clear: bool,
}

impl ConformanceScreen {
    pub fn new() -> Self {
        Self {
            loaded: false,
            violations: Vec::new(),
            total: 0,
            unknown_paths: Vec::new(),
            unknown_total: 0,
            view_mode: ViewMode::Violations,
            table: TableState::new(),
            error: None,
            last_fetch: None,
            detail_open: false,
            detail_scroll: 0,
            method_filter: MethodFilter::All,
            status_filter: StatusFilter::All,
            category_filter: None,
            paused: false,
            flash: None,
            pending_clear: false,
        }
    }

    fn filtered_unknown_indices(&self) -> Vec<usize> {
        self.unknown_paths
            .iter()
            .enumerate()
            .filter(|(_, r)| self.method_filter.matches(&r.method))
            .map(|(i, _)| i)
            .collect()
    }

    fn current_row_count(&self) -> usize {
        match self.view_mode {
            ViewMode::Violations => self.filtered_indices().len(),
            ViewMode::UnknownPaths => self.filtered_unknown_indices().len(),
        }
    }

    fn filtered_indices(&self) -> Vec<usize> {
        self.violations
            .iter()
            .enumerate()
            .filter(|(_, v)| self.method_filter.matches(&v.method))
            .filter(|(_, v)| self.status_filter.matches(v.status))
            .filter(|(_, v)| match &self.category_filter {
                None => true,
                Some(want) => v.category.eq_ignore_ascii_case(want),
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn selected_violation(&self) -> Option<&ConformanceViolation> {
        let idx = *self.filtered_indices().get(self.table.selected)?;
        self.violations.get(idx)
    }

    fn selected_detail(&self) -> Option<String> {
        let v = self.selected_violation()?;
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

    /// Cycle the category filter through every distinct category in the
    /// current snapshot, plus `"any"`. Keeps the keymap to a single key
    /// (`c`) regardless of how many categories the spec validator
    /// produces.
    fn cycle_category(&mut self) {
        let mut cats: Vec<String> = self
            .violations
            .iter()
            .map(|v| v.category.clone())
            .filter(|c| !c.is_empty())
            .collect();
        cats.sort();
        cats.dedup();
        if cats.is_empty() {
            self.category_filter = None;
            return;
        }
        self.category_filter = match &self.category_filter {
            None => Some(cats[0].clone()),
            Some(current) => match cats.iter().position(|c| c == current) {
                Some(i) if i + 1 < cats.len() => Some(cats[i + 1].clone()),
                _ => None,
            },
        };
    }

    /// Export the current filtered snapshot to a JSON file in CWD so
    /// the user can drop it next to their proxy logs and grep across
    /// both sides. Returns the path or an error message via `flash`.
    fn export_filtered(&mut self) {
        let now = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
        let path = PathBuf::from(format!("conformance-violations-{}.json", now));
        let indices = self.filtered_indices();
        let snapshot: Vec<&ConformanceViolation> =
            indices.iter().filter_map(|&i| self.violations.get(i)).collect();
        match serde_json::to_string_pretty(&snapshot) {
            Ok(json) => match std::fs::write(&path, json) {
                Ok(()) => {
                    self.flash = Some((
                        format!("exported {} violation(s) to {}", snapshot.len(), path.display()),
                        Instant::now(),
                    ));
                }
                Err(e) => {
                    self.flash = Some((format!("export failed: {e}"), Instant::now()));
                }
            },
            Err(e) => {
                self.flash = Some((format!("serialise failed: {e}"), Instant::now()));
            }
        }
    }

    /// Compute the top-N most-frequent `METHOD path` pairs in the
    /// current filtered view. Used by the right-side breakdown panel.
    /// View-aware: aggregates violations or unknown-paths depending on
    /// the active mode.
    fn top_endpoints(&self, n: usize) -> Vec<(String, usize)> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        match self.view_mode {
            ViewMode::Violations => {
                for &idx in &self.filtered_indices() {
                    if let Some(v) = self.violations.get(idx) {
                        let key = format!("{} {}", v.method, v.path);
                        *counts.entry(key).or_insert(0) += 1;
                    }
                }
            }
            ViewMode::UnknownPaths => {
                for &idx in &self.filtered_unknown_indices() {
                    if let Some(r) = self.unknown_paths.get(idx) {
                        let key = format!("{} {}", r.method, r.path);
                        *counts.entry(key).or_insert(0) += 1;
                    }
                }
            }
        }
        let mut pairs: Vec<(String, usize)> = counts.into_iter().collect();
        pairs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        pairs.truncate(n);
        pairs
    }

    fn flash_str(&self) -> Option<&str> {
        let (msg, at) = self.flash.as_ref()?;
        if at.elapsed().as_secs() < 6 {
            Some(msg.as_str())
        } else {
            None
        }
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
            KeyCode::Char('m') => {
                self.method_filter = self.method_filter.next();
                self.table.set_total(self.current_row_count());
                true
            }
            KeyCode::Char('s') => {
                self.status_filter = self.status_filter.next();
                self.table.set_total(self.current_row_count());
                true
            }
            KeyCode::Char('c') => {
                self.cycle_category();
                self.table.set_total(self.current_row_count());
                true
            }
            KeyCode::Char('u') => {
                // Round-13: cycle between Violations (request-side spec
                // failures) and UnknownPaths (404s for paths not in the
                // loaded spec at all) views.
                self.view_mode = match self.view_mode {
                    ViewMode::Violations => ViewMode::UnknownPaths,
                    ViewMode::UnknownPaths => ViewMode::Violations,
                };
                self.table.set_total(self.current_row_count());
                self.last_fetch = None;
                true
            }
            KeyCode::Char('p') => {
                self.paused = !self.paused;
                true
            }
            KeyCode::Char('e') => {
                self.export_filtered();
                true
            }
            KeyCode::Char('D') => {
                self.pending_clear = true;
                self.flash =
                    Some(("clear requested — refreshing on next tick".to_string(), Instant::now()));
                self.last_fetch = None;
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

        // Two-column body (table | top endpoints) over a status strip.
        let vchunks = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]).split(area);
        let hchunks =
            Layout::horizontal([Constraint::Min(40), Constraint::Length(34)]).split(vchunks[0]);
        self.render_table(frame, hchunks[0]);
        self.render_top_endpoints(frame, hchunks[1]);
        self.render_summary(frame, vchunks[1]);
    }

    fn tick(&mut self, client: &MockForgeClient, tx: &mpsc::UnboundedSender<Event>) {
        if self.pending_clear {
            self.pending_clear = false;
            let client_clone = client.clone();
            let tx_clone = tx.clone();
            let view = self.view_mode;
            tokio::spawn(async move {
                // Clear only the active feed so the other one stays
                // intact — round-13 added unknown-paths alongside
                // violations and both have separate buffers.
                let result = match view {
                    ViewMode::Violations => client_clone.clear_conformance_violations().await,
                    ViewMode::UnknownPaths => client_clone.clear_unknown_paths().await,
                };
                match result {
                    Ok(n) => {
                        let payload = match view {
                            ViewMode::Violations => {
                                format!(r#"{{"violations":[],"total":0,"cleared":{n}}}"#)
                            }
                            ViewMode::UnknownPaths => format!(
                                r#"{{"unknown_requests":[],"unknown_total":0,"cleared":{n}}}"#
                            ),
                        };
                        let _ = tx_clone.send(Event::Data {
                            screen: "conformance",
                            payload,
                        });
                    }
                    Err(err) => {
                        let _ = tx_clone.send(Event::ApiError {
                            screen: "conformance",
                            message: format!("clear failed: {err}"),
                        });
                    }
                }
            });
        }
        if self.paused {
            return;
        }
        let should_fetch = match self.last_fetch {
            Some(t) => t.elapsed().as_secs() >= FETCH_INTERVAL,
            None => true,
        };
        if !should_fetch {
            return;
        }
        self.last_fetch = Some(Instant::now());

        // Always fetch both feeds so toggling `u` is instant. Both
        // calls are cheap GETs against bounded ring buffers.
        let client_v = client.clone();
        let tx_v = tx.clone();
        tokio::spawn(async move {
            match client_v.get_conformance_violations().await {
                Ok(resp) => {
                    if let Ok(payload) = serde_json::to_string(&serde_json::json!({
                        "violations": resp.violations,
                        "total": resp.total,
                    })) {
                        let _ = tx_v.send(Event::Data {
                            screen: "conformance",
                            payload,
                        });
                    }
                }
                Err(err) => {
                    let _ = tx_v.send(Event::ApiError {
                        screen: "conformance",
                        message: err.to_string(),
                    });
                }
            }
        });
        let client_u = client.clone();
        let tx_u = tx.clone();
        tokio::spawn(async move {
            match client_u.get_unknown_paths().await {
                Ok(resp) => {
                    if let Ok(payload) = serde_json::to_string(&serde_json::json!({
                        "unknown_requests": resp.requests,
                        "unknown_total": resp.total,
                    })) {
                        let _ = tx_u.send(Event::Data {
                            screen: "conformance",
                            payload,
                        });
                    }
                }
                Err(_) => {
                    // Unknown-paths is a round-13 endpoint; older
                    // servers don't have it. Silently ignore so older
                    // server versions don't surface a confusing error.
                }
            }
        });
    }

    fn on_data(&mut self, payload: &str) {
        // Two payload shapes share this screen — violations (round 12)
        // and unknown_requests (round 13). Try the unknown-paths shape
        // first since it's narrower; fall through to the violations
        // decode on miss.
        #[derive(serde::Deserialize)]
        struct UnknownWire {
            unknown_requests: Vec<UnknownPathRequest>,
            #[serde(default)]
            unknown_total: usize,
        }
        if let Ok(parsed) = serde_json::from_str::<UnknownWire>(payload) {
            self.unknown_paths = parsed.unknown_requests;
            self.unknown_total = parsed.unknown_total;
            if matches!(self.view_mode, ViewMode::UnknownPaths) {
                self.table.set_total(self.current_row_count());
            }
            self.loaded = true;
            self.error = None;
            return;
        }

        #[derive(serde::Deserialize)]
        struct Wire {
            violations: Vec<ConformanceViolation>,
            #[serde(default)]
            total: usize,
            #[serde(default)]
            cleared: Option<usize>,
        }
        match serde_json::from_str::<Wire>(payload) {
            Ok(parsed) => {
                self.violations = parsed.violations;
                self.total = parsed.total;
                if matches!(self.view_mode, ViewMode::Violations) {
                    self.table.set_total(self.current_row_count());
                }
                self.loaded = true;
                self.error = None;
                if let Some(n) = parsed.cleared {
                    self.flash =
                        Some((format!("cleared {n} server-side violation(s)"), Instant::now()));
                    self.last_fetch = None;
                }
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
        } else if self.paused {
            "[paused]  p:resume  m/s/c:filter  e:export  D:clear  Enter:detail"
        } else {
            "j/k:navigate  m/s/c:filter  p:pause  e:export  D:clear  u:unknown-paths  Enter:detail"
        }
    }
}

impl ConformanceScreen {
    fn render_table(&self, frame: &mut Frame, area: Rect) {
        match self.view_mode {
            ViewMode::Violations => self.render_violations_table(frame, area),
            ViewMode::UnknownPaths => self.render_unknown_paths_table(frame, area),
        }
    }

    fn render_violations_table(&self, frame: &mut Frame, area: Rect) {
        let header = Row::new(vec![
            Cell::from("When").style(Theme::dim()),
            Cell::from("Method").style(Theme::dim()),
            Cell::from("Path").style(Theme::dim()),
            Cell::from("Status").style(Theme::dim()),
            Cell::from("Category").style(Theme::dim()),
            Cell::from("Client").style(Theme::dim()),
        ])
        .height(1);

        let indices = self.filtered_indices();
        let rows: Vec<Row> = indices
            .iter()
            .skip(self.table.offset)
            .take(self.table.visible_height)
            .filter_map(|&i| self.violations.get(i))
            .map(|v| {
                let category = if v.category.is_empty() {
                    "(uncategorised)".to_string()
                } else {
                    v.category.clone()
                };
                Row::new(vec![
                    Cell::from(v.timestamp.format("%H:%M:%S").to_string()),
                    Cell::from(v.method.clone()).style(Theme::http_method(&v.method)),
                    Cell::from(v.path.clone()),
                    Cell::from(v.status.to_string()).style(Theme::status_code(v.status)),
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

        let filtered_count = indices.len();
        let filter_suffix = self.filter_label_suffix();
        let title = if self.total > filtered_count {
            format!(
                " Conformance Violations ({} buffered, {} shown{}) ",
                self.total, filtered_count, filter_suffix
            )
        } else {
            format!(" Conformance Violations ({}{}) ", filtered_count, filter_suffix)
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

    fn render_unknown_paths_table(&self, frame: &mut Frame, area: Rect) {
        let header = Row::new(vec![
            Cell::from("When").style(Theme::dim()),
            Cell::from("Method").style(Theme::dim()),
            Cell::from("Path").style(Theme::dim()),
            Cell::from("Query").style(Theme::dim()),
            Cell::from("Client").style(Theme::dim()),
        ])
        .height(1);

        let indices = self.filtered_unknown_indices();
        let rows: Vec<Row> = indices
            .iter()
            .skip(self.table.offset)
            .take(self.table.visible_height)
            .filter_map(|&i| self.unknown_paths.get(i))
            .map(|r| {
                Row::new(vec![
                    Cell::from(r.timestamp.format("%H:%M:%S").to_string()),
                    Cell::from(r.method.clone()).style(Theme::http_method(&r.method)),
                    Cell::from(r.path.clone()),
                    Cell::from(if r.query.is_empty() {
                        "-".to_string()
                    } else {
                        r.query.clone()
                    }),
                    Cell::from(r.client_ip.clone()),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Min(20),
            Constraint::Min(15),
            Constraint::Length(16),
        ];

        let filtered_count = indices.len();
        let filter_suffix = self.filter_label_suffix();
        let title = if self.unknown_total > filtered_count {
            format!(
                " Unknown Paths ({} buffered, {} shown{}) ",
                self.unknown_total, filtered_count, filter_suffix
            )
        } else {
            format!(" Unknown Paths ({}{}) ", filtered_count, filter_suffix)
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

    fn render_top_endpoints(&self, frame: &mut Frame, area: Rect) {
        let top = self.top_endpoints(8);
        let body = if top.is_empty() {
            "no violations in view".to_string()
        } else {
            top.into_iter()
                .map(|(endpoint, n)| format!("{:>4}  {}", n, endpoint))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let para = Paragraph::new(body).style(Theme::dim()).block(
            Block::default()
                .title(" Top Offending Endpoints ")
                .title_style(Theme::title())
                .borders(Borders::ALL)
                .border_style(Theme::dim()),
        );
        frame.render_widget(para, area);
    }

    fn render_summary(&self, frame: &mut Frame, area: Rect) {
        let body = if let Some(flash) = self.flash_str() {
            flash.to_string()
        } else {
            match self.view_mode {
                ViewMode::Violations => self.violations_summary(),
                ViewMode::UnknownPaths => self.unknown_paths_summary(),
            }
        };

        let para = Paragraph::new(body)
            .style(Theme::dim())
            .block(Block::default().borders(Borders::ALL).border_style(Theme::dim()));
        frame.render_widget(para, area);
    }

    fn violations_summary(&self) -> String {
        if self.violations.is_empty() {
            return "No spec violations recorded — every incoming request matched the loaded OpenAPI spec. (`u`: view unknown-path requests instead)".to_string();
        }
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for &i in &self.filtered_indices() {
            let Some(v) = self.violations.get(i) else {
                continue;
            };
            let key = if v.category.is_empty() {
                "(uncategorised)"
            } else {
                v.category.as_str()
            };
            *counts.entry(key).or_insert(0) += 1;
        }
        let mut pairs: Vec<(&&str, &usize)> = counts.iter().collect();
        pairs.sort_by(|a, b| b.1.cmp(a.1));
        let body = pairs
            .into_iter()
            .take(3)
            .map(|(k, v)| format!("{} ({})", k, v))
            .collect::<Vec<_>>()
            .join(", ");
        format!("Top categories: {}", body)
    }

    fn unknown_paths_summary(&self) -> String {
        if self.unknown_paths.is_empty() {
            return "No unknown-path 404s recorded — every incoming request matched a route in the loaded spec. (`u`: switch back to violations)".to_string();
        }
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for &i in &self.filtered_unknown_indices() {
            let Some(r) = self.unknown_paths.get(i) else {
                continue;
            };
            *counts.entry(r.method.as_str()).or_insert(0) += 1;
        }
        let mut pairs: Vec<(&&str, &usize)> = counts.iter().collect();
        pairs.sort_by(|a, b| b.1.cmp(a.1));
        let body = pairs
            .into_iter()
            .take(5)
            .map(|(k, v)| format!("{} ({})", k, v))
            .collect::<Vec<_>>()
            .join(", ");
        format!("Top methods: {}", body)
    }

    fn filter_label_suffix(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.method_filter != MethodFilter::All {
            parts.push(format!("method={}", self.method_filter.label()));
        }
        if self.status_filter != StatusFilter::All {
            parts.push(format!("status={}", self.status_filter.label()));
        }
        if let Some(cat) = &self.category_filter {
            parts.push(format!("category={cat}"));
        }
        if self.paused {
            parts.push("paused".to_string());
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!(", filter: {}", parts.join(" / "))
        }
    }
}
