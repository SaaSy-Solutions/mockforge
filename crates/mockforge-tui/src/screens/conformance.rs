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
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
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

/// Round 25 — identity key for re-anchoring the selected violation
/// after a refresh. Server-side violations get a microsecond-resolution
/// timestamp plus method+path+status; that tuple is enough to find the
/// same record after the refresh replaces `self.violations`. Tuple is
/// `(timestamp_secs, timestamp_nanos, method, path, status)` to avoid
/// pulling in `DateTime`'s Hash/Ord trickiness as keys.
fn violation_key(v: &ConformanceViolation) -> (i64, u32, String, String, u16) {
    (
        v.timestamp.timestamp(),
        v.timestamp.timestamp_subsec_nanos(),
        v.method.clone(),
        v.path.clone(),
        v.status,
    )
}

/// Round 25 — identity key for unknown-path entries; symmetric with
/// `violation_key`. Status is omitted because UnknownPathRequest
/// doesn't carry one.
fn unknown_key(r: &UnknownPathRequest) -> (i64, u32, String, String) {
    (
        r.timestamp.timestamp(),
        r.timestamp.timestamp_subsec_nanos(),
        r.method.clone(),
        r.path.clone(),
    )
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

/// Group key for export dedup — same (method, path, category, reason)
/// rolls up. Aliased so clippy doesn't flag the inner HashMap value.
type DedupKey = (String, String, String, String);

/// Round 16 + Round 37 (#876 follow-up) — collapse a slice of
/// `ConformanceViolation`s into the on-disk export shape.
///
/// Round 16 introduced the dedup by `(method, path, category, reason)`,
/// the occurrence count, and the first/last-seen window. Round 37
/// extends each group with the client-side stamps so a user grepping
/// the bench JSONL's `client_sent_at` finds the matching server-side
/// entry, even after the dedup. Sort is count DESC then path ASC,
/// identical to the prior behaviour so existing tooling keeps working.
///
/// Pulled out of `export_filtered` to keep that function focused on
/// "stitch index -> file path" while the math sits behind a pure
/// function the unit tests can drive directly.
/// Round 45 (#79) — local mirror of `mockforge_foundation::conformance_violations::summarize_reason`
/// for TUI builds. The TUI doesn't depend on mockforge-foundation (it's
/// the standalone admin client), so we re-implement the collapse here
/// to keep the dedup export self-contained. The logic stays in lock-
/// step with the foundation copy; both produce the same one-line
/// `<N> <category> violation(s): <name> (<rule>), ...` string built
/// from the validator's `details[]` payload. Empty when `reason`
/// doesn't carry a parseable `{"details":[...]}` envelope (e.g. the
/// older heuristic fallback path or content-type rejections).
fn summarize_reason_local(reason: &str) -> String {
    use serde_json::Value;

    let json_start = reason.find('{');
    let parsed: Option<Value> = json_start
        .and_then(|i| serde_json::from_str(reason[i..].trim()).ok())
        .or_else(|| serde_json::from_str(reason).ok());

    let details = parsed
        .as_ref()
        .and_then(|v| v.get("details"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if details.is_empty() {
        return String::new();
    }

    let mut by_loc: std::collections::BTreeMap<String, Vec<(String, String)>> =
        std::collections::BTreeMap::new();

    for d in &details {
        let path = d.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let code = d.get("code").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let msg = d.get("message").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
        let (loc, name) = match path.split_once('.') {
            Some((l, n)) => (l.to_string(), n.to_string()),
            None if !path.is_empty() => (path.clone(), String::new()),
            _ => ("body".to_string(), String::new()),
        };
        let rule = match code.as_str() {
            "schema_validation" => {
                if msg.contains("is not one of") {
                    "enum".to_string()
                } else if msg.contains("not of type") || msg.contains("expected type") {
                    "type".to_string()
                } else if msg.contains("less than") || msg.contains("minimum") {
                    "min".to_string()
                } else if msg.contains("greater than") || msg.contains("maximum") {
                    "max".to_string()
                } else if msg.contains("pattern") {
                    "pattern".to_string()
                } else if msg.contains("required") {
                    "required".to_string()
                } else {
                    "schema".to_string()
                }
            }
            "required" => "required".to_string(),
            other => other.to_string(),
        };
        by_loc.entry(loc).or_default().push((name, rule));
    }

    let total = details.len();
    // Round 45 — keep in lock-step with foundation's `summarize_reason`:
    // pick the loc category by the classifier's priority order
    // (query > header > cookie > path > body), not BTreeMap alphabetical.
    let primary_loc = ["query", "header", "cookie", "path", "body"]
        .iter()
        .find(|loc| by_loc.contains_key(**loc))
        .map(|s| s.to_string())
        .or_else(|| by_loc.keys().next().cloned())
        .unwrap_or_else(|| "validation".to_string());
    let primary_label = match primary_loc.as_str() {
        "query" => "query",
        "header" => "header",
        "cookie" => "cookie",
        "path" => "path parameter",
        "body" => "request-body",
        other => other,
    };

    let mut items: Vec<String> = Vec::new();
    for (loc, names) in &by_loc {
        for (name, rule) in names {
            let head = if name.is_empty() {
                loc.clone()
            } else {
                format!("{}.{}", loc, name)
            };
            if rule.is_empty() {
                items.push(head);
            } else {
                items.push(format!("{} ({})", head, rule));
            }
        }
    }

    const MAX_VISIBLE: usize = 5;
    let visible: Vec<String> = items.iter().take(MAX_VISIBLE).cloned().collect();
    let suffix = if items.len() > MAX_VISIBLE {
        format!(", +{} more", items.len() - MAX_VISIBLE)
    } else {
        String::new()
    };

    format!("{} {} violation(s): {}{}", total, primary_label, visible.join(", "), suffix)
}

fn dedup_violations(violations: &[&ConformanceViolation]) -> Vec<DedupedViolation> {
    let mut groups: HashMap<DedupKey, (DedupedViolation, u32, chrono::DateTime<chrono::Utc>)> =
        HashMap::new();
    for v in violations {
        let key = (v.method.clone(), v.path.clone(), v.category.clone(), v.reason.clone());
        let hits = v.occurrences.max(1);
        match groups.get_mut(&key) {
            Some(slot) => {
                slot.1 = slot.1.saturating_add(hits);
                slot.0.count = slot.0.count.saturating_add(hits);
                if v.timestamp < slot.0.first_seen {
                    slot.0.first_seen = v.timestamp;
                }
                if v.timestamp > slot.2 {
                    slot.2 = v.timestamp;
                    slot.0.last_seen = v.timestamp;
                }
                // Round 37 — fold client stamps into the group. We
                // always keep the latest observed
                // `client_mockforge_version` so an upgrade mid-run
                // flips to the newer version. The min/max bracket the
                // client-side send window across the dedup group.
                if v.client_mockforge_version.is_some() {
                    slot.0.client_mockforge_version = v.client_mockforge_version.clone();
                }
                if let Some(cs) = v.client_sent_at {
                    slot.0.min_client_sent_at = Some(match slot.0.min_client_sent_at {
                        Some(existing) if existing <= cs => existing,
                        _ => cs,
                    });
                    slot.0.max_client_sent_at = Some(match slot.0.max_client_sent_at {
                        Some(existing) if existing >= cs => existing,
                        _ => cs,
                    });
                }
            }
            None => {
                // Round 45 (#79) — populate `summary` for this group.
                // Older mockforge versions don't send the field, so
                // build it locally from `reason` when the wire payload
                // didn't carry one. Saves the TUI from re-running the
                // parser on every render.
                let summary = if v.summary.is_empty() {
                    summarize_reason_local(&v.reason)
                } else {
                    v.summary.clone()
                };
                groups.insert(
                    key,
                    (
                        DedupedViolation {
                            method: v.method.clone(),
                            path: v.path.clone(),
                            category: v.category.clone(),
                            reason: v.reason.clone(),
                            summary,
                            status: v.status,
                            count: hits,
                            first_seen: v.timestamp,
                            last_seen: v.timestamp,
                            client_mockforge_version: v.client_mockforge_version.clone(),
                            min_client_sent_at: v.client_sent_at,
                            max_client_sent_at: v.client_sent_at,
                        },
                        hits,
                        v.timestamp,
                    ),
                );
            }
        }
    }
    let mut deduped: Vec<DedupedViolation> = groups.into_iter().map(|(_, (d, _, _))| d).collect();
    deduped.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.path.cmp(&b.path)));
    deduped
}

/// Round 16 — JSON shape written by the export action (`e`). Same
/// fields as `ConformanceViolation` minus `client_ip` (which is always
/// `"unknown"` for now and just adds noise to the file), plus a `count`
/// and a `first_seen` / `last_seen` window. Same-shape lines are
/// collapsed by `export_filtered`.
#[derive(Debug, serde::Serialize)]
struct DedupedViolation {
    method: String,
    path: String,
    category: String,
    reason: String,
    /// Round 45 (#79) — Srikanth on 0.3.189: "I am not seeing any
    /// summary info" in his TUI export. v0.3.189 added `summary` to
    /// `ServerConformanceViolation` but the TUI dedup wrapper
    /// (`DedupedViolation`) didn't propagate it, so every TUI-exported
    /// JSON file dropped the field on the floor. Now persisted on
    /// every group; identical across all rows of a single signature
    /// (`reason` is part of the dedup key) so we can just lift it
    /// from the first observed hit. Empty when the underlying
    /// validator didn't supply a parseable `details` payload.
    summary: String,
    status: u16,
    count: u32,
    first_seen: chrono::DateTime<chrono::Utc>,
    last_seen: chrono::DateTime<chrono::Utc>,
    /// Round 37 (#876 follow-up / Srikanth on 0.3.181) — client-side
    /// stamps observed across the dedup group. `client_mockforge_version`
    /// is the most recently observed value (in practice all dups carry
    /// the same value because they came from the same bench run). The
    /// `min_client_sent_at` / `max_client_sent_at` pair brackets the
    /// time window the *client* sent these probes, which is what a
    /// user wants to grep against the bench JSONL — not the server's
    /// `first_seen` / `last_seen` (when the *server recorded* the
    /// violation). Skipped when none of the group's hits carried the
    /// headers (older bench, real proxy traffic).
    #[serde(skip_serializing_if = "Option::is_none")]
    client_mockforge_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_client_sent_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_client_sent_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct ConformanceScreen {
    loaded: bool,
    /// Full snapshot from the server, newest-first.
    violations: Vec<ConformanceViolation>,
    total: usize,
    /// Round-15: lifetime count of violations seen server-side (the
    /// buffer caps `total` at 256; this is the true total).
    total_seen: u64,
    /// Round-17.1: lifetime count of requests that passed the validator.
    /// With `total_seen` this gives the actual pass/fail ratio.
    total_ok: u64,
    /// Round-13: unmatched-path requests captured by the HTTP fallback.
    unknown_paths: Vec<UnknownPathRequest>,
    unknown_total: usize,
    /// Round-15: lifetime count of unknown-path requests seen.
    unknown_total_seen: u64,
    /// Toggled by `u` between violations and unknown-paths views.
    view_mode: ViewMode,
    table: TableState,
    error: Option<String>,
    last_fetch: Option<Instant>,
    detail_open: bool,
    detail_scroll: u16,
    /// Round 26 — snapshot of the violation / unknown-path that was
    /// selected when Enter opened the modal. The modal renders from
    /// this cached text instead of recomputing from
    /// `selected_violation()` every frame, so when the 5 s refresh
    /// tick prepends new rows or evicts the user's clicked entry from
    /// the 256-cap buffer, the modal keeps showing what they clicked.
    /// Reset to None on Esc / view-toggle. The plain `String` form
    /// avoids holding a reference to a violation that the next refresh
    /// might evict.
    detail_snapshot: Option<String>,
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
            total_seen: 0,
            total_ok: 0,
            unknown_paths: Vec::new(),
            unknown_total: 0,
            unknown_total_seen: 0,
            view_mode: ViewMode::Violations,
            table: TableState::new(),
            error: None,
            last_fetch: None,
            detail_open: false,
            detail_scroll: 0,
            detail_snapshot: None,
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

    /// Round 22.1 — symmetric accessor for the unknown-paths view.
    /// Indexes through `filtered_unknown_indices()` so the table
    /// selection cursor maps to the right row regardless of which
    /// filters are active.
    fn selected_unknown(&self) -> Option<&UnknownPathRequest> {
        let idx = *self.filtered_unknown_indices().get(self.table.selected)?;
        self.unknown_paths.get(idx)
    }

    /// View-aware detail text. Pre-round-22.1, this always returned
    /// the violation detail, so pressing Enter while toggled to the
    /// Unknown view (`u`) opened the violations modal anyway: a
    /// confusing screen-content swap that Srikanth flagged.
    /// Now dispatches on `view_mode`: Violations → violation detail,
    /// UnknownPaths → unknown-request detail.
    fn selected_detail(&self) -> Option<String> {
        match self.view_mode {
            ViewMode::Violations => self.selected_violation_detail(),
            ViewMode::UnknownPaths => self.selected_unknown_detail(),
        }
    }

    fn selected_violation_detail(&self) -> Option<String> {
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

    /// Round 22.1 — unknown-path Enter detail. No `reason` /
    /// `category` (the spec didn't know the path at all), but the
    /// query string is useful for diagnosing proxy-replay mismatches.
    fn selected_unknown_detail(&self) -> Option<String> {
        let r = self.selected_unknown()?;
        Some(format!(
            "Timestamp:  {}\nMethod:     {}\nPath:       {}\nQuery:      {}\nClient IP:  {}\nStatus:     {}\n\n(No spec entry matched this path. In shadow mode the server returned 200; otherwise 404.)\n",
            r.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            r.method,
            r.path,
            if r.query.is_empty() { "(none)" } else { r.query.as_str() },
            r.client_ip,
            r.status,
        ))
    }

    /// Round 17.1 — `c` in the detail view: copy the selected
    /// violation as pretty-printed JSON to the system clipboard via
    /// `arboard`. Useful for pasting into bug reports / proxy logs /
    /// Slack. Surfaces success and failure through the same `flash`
    /// strip the export action uses.
    ///
    /// Failure modes worth knowing about:
    /// - No clipboard daemon (pure-SSH terminal with no `xclip` /
    ///   `wl-clipboard` / Pasteboard available) → arboard returns an
    ///   error; we report it so the user knows the keystroke didn't
    ///   silently no-op.
    /// - The `arboard::Clipboard` handle is created per-press rather
    ///   than cached, so the TUI doesn't hold any clipboard server
    ///   connection while idle.
    fn copy_selected_to_clipboard(&mut self) {
        let Some(v) = self.selected_violation() else {
            self.flash = Some(("no violation selected".to_string(), Instant::now()));
            return;
        };
        let payload =
            serde_json::to_string_pretty(v).unwrap_or_else(|_| "(serialise failed)".to_string());
        match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(payload)) {
            Ok(()) => {
                self.flash = Some((
                    format!("copied violation ({} {}) to clipboard", v.method, v.path),
                    Instant::now(),
                ));
            }
            Err(e) => {
                self.flash = Some((format!("clipboard unavailable: {e}"), Instant::now()));
            }
        }
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
    ///
    /// Round 16 — exports are **deduplicated** by
    /// `(method, path, category, reason)`, sorted by occurrence count
    /// descending, so a 500k-request run no longer produces a 256-entry
    /// JSON of mostly-identical rows. Each group keeps the *first* and
    /// *last* timestamps so you can still see the time window. The TUI
    /// keeps showing each occurrence — only the export collapses.
    fn export_filtered(&mut self) {
        let now = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
        let path = PathBuf::from(format!("conformance-violations-{}.json", now));
        let indices = self.filtered_indices();
        let selected: Vec<&ConformanceViolation> =
            indices.iter().filter_map(|&i| self.violations.get(i)).collect();
        let deduped = dedup_violations(&selected);
        let total_occurrences: u32 = deduped.iter().map(|d| d.count).sum();
        let unique_groups = deduped.len();

        match serde_json::to_string_pretty(&deduped) {
            Ok(json) => match std::fs::write(&path, json) {
                Ok(()) => {
                    self.flash = Some((
                        format!(
                            "exported {} unique violation(s) ({} occurrences) to {}",
                            unique_groups,
                            total_occurrences,
                            path.display()
                        ),
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
                    // Round 26 — drop the snapshot so the next Enter
                    // re-captures from the current selected row.
                    self.detail_snapshot = None;
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
                KeyCode::Char('c') => {
                    // Round 17.1 — Srikanth's (c-i) ask: copy the
                    // current violation to system clipboard. arboard
                    // handles X11/Wayland/macOS/Win32 in default
                    // features; on a TTY with no clipboard backend it
                    // returns an error we surface via the flash strip.
                    self.copy_selected_to_clipboard();
                    return true;
                }
                _ => return true,
            }
        }

        match key.code {
            KeyCode::Enter => {
                // Round 22.1 — gate on the right collection for the
                // active view. Pre-round-22.1, Enter opened the
                // violation modal even when `u` had toggled to
                // unknown-paths, because we checked `violations`
                // unconditionally. Now the modal opens iff the
                // current view has at least one row.
                let has_rows = match self.view_mode {
                    ViewMode::Violations => !self.violations.is_empty(),
                    ViewMode::UnknownPaths => !self.unknown_paths.is_empty(),
                };
                if has_rows {
                    self.detail_open = true;
                    self.detail_scroll = 0;
                    // Round 26 — snapshot the currently-selected row's
                    // detail text NOW, before the next refresh tick has
                    // a chance to replace `self.violations`. The modal
                    // renders from this snapshot, so the user keeps
                    // reading what they clicked even if the underlying
                    // entry is later evicted by the 256-cap buffer or
                    // shifted by new prepended traffic.
                    self.detail_snapshot = self.selected_detail();
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
            // Round 26 — render the cached snapshot captured at Enter
            // time, NOT the live `selected_detail()`. Without this the
            // modal text changes under the user whenever the 5 s tick
            // refreshes the buffer and shifts/evicts what's at the
            // selected index. Snapshot is None only if the screen is
            // somehow opened without a row selected (defensive).
            let detail = self
                .detail_snapshot
                .clone()
                .or_else(|| self.selected_detail())
                .unwrap_or_else(|| "(no row selected)".to_string());
            // Issue #79 round 15 — wrap long lines so big Microsoft
            // Graph paths and validation reasons are fully readable
            // (Srikanth couldn't see the full path/reason). j/k still
            // scroll vertically through the wrapped detail.
            //
            // Round 22.1 — title now reflects the active view, so
            // pressing Enter while toggled to Unknown surfaces as
            // "Unknown Path Detail" instead of the (wrong)
            // "Violation Detail".
            let title = match self.view_mode {
                ViewMode::Violations => " Violation Detail (Esc:close  j/k:scroll  c:copy) ",
                ViewMode::UnknownPaths => " Unknown Path Detail (Esc:close  j/k:scroll) ",
            };
            let para = Paragraph::new(detail)
                .wrap(Wrap { trim: false })
                .scroll((self.detail_scroll, 0))
                .block(
                    Block::default()
                        .title(title)
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
                        "total_seen": resp.total_seen,
                        "total_ok": resp.total_ok,
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
                        "unknown_total_seen": resp.total_seen,
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
            #[serde(default)]
            unknown_total_seen: u64,
        }
        if let Ok(parsed) = serde_json::from_str::<UnknownWire>(payload) {
            // Round 25 (Srikanth follow-up after r24) — preserve the
            // selected row's identity across refresh. Without this, a
            // 5-second tick that prepends new entries silently scrolls
            // the user's investigation cursor to a different request.
            let prev_key = if matches!(self.view_mode, ViewMode::UnknownPaths) {
                self.selected_unknown().map(unknown_key)
            } else {
                None
            };
            self.unknown_paths = parsed.unknown_requests;
            self.unknown_total = parsed.unknown_total;
            self.unknown_total_seen = parsed.unknown_total_seen;
            if matches!(self.view_mode, ViewMode::UnknownPaths) {
                self.table.set_total(self.current_row_count());
                if let Some(key) = prev_key {
                    if let Some(new_pos) = self.filtered_unknown_indices().iter().position(|&i| {
                        self.unknown_paths.get(i).map(unknown_key) == Some(key.clone())
                    }) {
                        self.table.selected = new_pos;
                    }
                }
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
            total_seen: u64,
            #[serde(default)]
            total_ok: u64,
            #[serde(default)]
            cleared: Option<usize>,
        }
        match serde_json::from_str::<Wire>(payload) {
            Ok(parsed) => {
                // Round 25 (Srikanth follow-up after r24) — same
                // identity-preserving refresh as the unknown-paths
                // branch above. Without this, the 5s refresh tick
                // scrolls the user's selection to a different
                // violation row whenever new traffic arrives.
                let prev_key = if matches!(self.view_mode, ViewMode::Violations) {
                    self.selected_violation().map(violation_key)
                } else {
                    None
                };
                self.violations = parsed.violations;
                self.total = parsed.total;
                self.total_seen = parsed.total_seen;
                self.total_ok = parsed.total_ok;
                if matches!(self.view_mode, ViewMode::Violations) {
                    self.table.set_total(self.current_row_count());
                    if let Some(key) = prev_key {
                        if let Some(new_pos) = self.filtered_indices().iter().position(|&i| {
                            self.violations.get(i).map(violation_key) == Some(key.clone())
                        }) {
                            self.table.selected = new_pos;
                        }
                    }
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
            "Esc:close  j/k:scroll  c:copy-to-clipboard"
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
        // Round-15: lifetime `total_seen` alongside the buffered count
        // so a 656k-request run doesn't look like "only 256".
        // Round-17.1: also surface `total_ok` (conformant requests) so
        // the user sees the real pass/fail ratio. Format chosen to
        // stay short — N violations / M ok = N+M total validated.
        let seen_suffix = if self.total_seen as usize > self.total || self.total_ok > 0 {
            let validated = self.total_seen + self.total_ok;
            format!(", {}/{} validated failed", self.total_seen, validated)
        } else {
            String::new()
        };
        let title = if self.total > filtered_count {
            format!(
                " Conformance Violations ({} buffered, {} shown{}{}) ",
                self.total, filtered_count, seen_suffix, filter_suffix
            )
        } else {
            format!(" Conformance Violations ({}{}{}) ", filtered_count, seen_suffix, filter_suffix)
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
            Cell::from("Status").style(Theme::dim()),
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
                    Cell::from(r.status.to_string()).style(Theme::status_code(r.status)),
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
            Constraint::Length(7),
            Constraint::Min(12),
            Constraint::Length(16),
        ];

        let filtered_count = indices.len();
        let filter_suffix = self.filter_label_suffix();
        // Round-15: lifetime total so the 256-cap buffer doesn't read
        // as the whole story (Srikanth's 656k-vs-256 question).
        let seen_suffix = if self.unknown_total_seen as usize > self.unknown_total {
            format!(", {} seen total", self.unknown_total_seen)
        } else {
            String::new()
        };
        let title = if self.unknown_total > filtered_count {
            format!(
                " Unknown Paths ({} buffered, {} shown{}{}) ",
                self.unknown_total, filtered_count, seen_suffix, filter_suffix
            )
        } else {
            format!(" Unknown Paths ({}{}{}) ", filtered_count, seen_suffix, filter_suffix)
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
        // Issue #79 round 15 — wrap so long `METHOD /very/long/graph/path`
        // entries don't get clipped at the panel's right edge.
        let para = Paragraph::new(body).style(Theme::dim()).wrap(Wrap { trim: false }).block(
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use crossterm::event::KeyCode;

    /// Round 45 (#79) — Srikanth's TUI export on 0.3.189 was missing
    /// the `summary` field that v0.3.189's server-side
    /// `ServerConformanceViolation` shipped. The TUI dedup wrapper
    /// (`DedupedViolation`) had to learn the field and either lift the
    /// server's pre-built summary or build one locally from `reason`.
    /// This test pins both branches: when the server already supplied
    /// a `summary`, dedup keeps it verbatim; when `summary` is empty,
    /// dedup falls back to `summarize_reason_local` so older mockforge
    /// servers still get a populated export.
    #[test]
    fn dedup_violations_carries_summary_or_rebuilds_from_reason() {
        let reason = r#"Validation error: {"details":[{"code":"schema_validation","message":"Validation error: \"zzz\" is not one of \"en\" or \"fr\"","path":"query.lang"}],"errors":["..."]}"#;

        // Case A: server already supplied a summary — keep verbatim.
        let mut v_with = violation(1000, 0, "/x", reason);
        v_with.summary = "1 query violation(s): query.lang (enum)".into();
        let out_a = dedup_violations(&[&v_with]);
        assert_eq!(out_a.len(), 1);
        assert_eq!(out_a[0].summary, "1 query violation(s): query.lang (enum)");

        // Case B: server didn't (older mockforge) — rebuild locally.
        let v_without = violation(2000, 0, "/x", reason);
        let out_b = dedup_violations(&[&v_without]);
        assert_eq!(out_b.len(), 1);
        assert_eq!(out_b[0].summary, "1 query violation(s): query.lang (enum)");

        // Case C: reason carries no parseable details — summary stays empty.
        let v_empty = violation(3000, 0, "/x", "raw text, no JSON envelope");
        let out_c = dedup_violations(&[&v_empty]);
        assert_eq!(out_c.len(), 1);
        assert_eq!(out_c[0].summary, "");
    }

    fn violation(secs: i64, nanos: u32, path: &str, reason: &str) -> ConformanceViolation {
        ConformanceViolation {
            timestamp: Utc.timestamp_opt(secs, nanos).unwrap(),
            method: "POST".into(),
            path: path.into(),
            client_ip: "1.2.3.4".into(),
            status: 400,
            reason: reason.into(),
            category: "request-body".into(),
            occurrences: 1,
            client_mockforge_version: None,
            client_sent_at: None,
            summary: String::new(),
        }
    }

    /// Round 37 — variant of `violation` that stamps the client-side
    /// headers, so tests for the export's `client_mockforge_version`
    /// and `min_client_sent_at` / `max_client_sent_at` aggregation can
    /// construct realistic inputs without disturbing the existing
    /// `violation` callers.
    fn stamped_violation(
        secs: i64,
        path: &str,
        reason: &str,
        version: &str,
        client_sent_secs: i64,
    ) -> ConformanceViolation {
        let mut v = violation(secs, 0, path, reason);
        v.client_mockforge_version = Some(version.into());
        v.client_sent_at = Some(Utc.timestamp_opt(client_sent_secs, 0).unwrap());
        v
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, crossterm::event::KeyModifiers::NONE)
    }

    /// Round 37 (#876 follow-up / Srikanth on 0.3.181) — when the
    /// server-side buffer recorded the bench's client stamps on each
    /// violation, the TUI's export must surface them on the dedup
    /// group so the user can correlate the export back to the bench
    /// JSONL line that produced it. Two stamped hits with different
    /// `client_sent_at` collapse into one group with
    /// `min_client_sent_at` = the earlier of the two and
    /// `max_client_sent_at` = the later; the `client_mockforge_version`
    /// is carried verbatim (both hits share the same version in
    /// practice).
    #[test]
    fn dedup_violations_aggregates_client_stamps_across_group() {
        let a = stamped_violation(1_000, "/users", "missing email", "0.3.181", 500);
        let b = stamped_violation(1_005, "/users", "missing email", "0.3.181", 800);
        let result = dedup_violations(&[&a, &b]);
        assert_eq!(result.len(), 1);
        let g = &result[0];
        assert_eq!(g.count, 2);
        assert_eq!(g.client_mockforge_version.as_deref(), Some("0.3.181"));
        assert_eq!(g.min_client_sent_at, Some(Utc.timestamp_opt(500, 0).unwrap()));
        assert_eq!(g.max_client_sent_at, Some(Utc.timestamp_opt(800, 0).unwrap()));
    }

    /// Round 37 — older mockforge instances (pre-#876) sent the
    /// violation payload without the new fields. The dedup must keep
    /// `client_mockforge_version` and `min/max_client_sent_at` as
    /// `None` so the export's `skip_serializing_if = "Option::is_none"`
    /// suppresses them entirely, rather than writing JSON `null`s
    /// that would clutter the output for users on the old server.
    #[test]
    fn dedup_violations_leaves_stamps_none_when_violations_unstamped() {
        let a = violation(1_000, 0, "/users", "missing email");
        let b = violation(1_005, 0, "/users", "missing email");
        let result = dedup_violations(&[&a, &b]);
        assert_eq!(result.len(), 1);
        let g = &result[0];
        assert!(g.client_mockforge_version.is_none());
        assert!(g.min_client_sent_at.is_none());
        assert!(g.max_client_sent_at.is_none());
        // Serializing it should NOT emit the stamp keys at all.
        let json = serde_json::to_string(g).unwrap();
        assert!(!json.contains("client_mockforge_version"));
        assert!(!json.contains("client_sent_at"));
    }

    /// Round 37 — a mid-run upgrade where the first hit came from an
    /// older bench (0.3.180) and the second from a newer bench
    /// (0.3.181) keeps the LATEST observed version on the group, not
    /// the first. Lets a reader spot a version skew across hits.
    #[test]
    fn dedup_violations_picks_latest_version_on_mixed_group() {
        let mut a = stamped_violation(1_000, "/users", "missing email", "0.3.180", 500);
        a.client_mockforge_version = Some("0.3.180".into());
        let mut b = stamped_violation(1_005, "/users", "missing email", "0.3.181", 800);
        b.client_mockforge_version = Some("0.3.181".into());
        let result = dedup_violations(&[&a, &b]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].client_mockforge_version.as_deref(), Some("0.3.181"));
    }

    /// Round 26 — Srikanth's reply on 0.3.169: opening the detail
    /// modal with Enter and then waiting for the 5 s tick still
    /// showed a different request, because the modal re-fetched
    /// from `self.violations[selected]` every frame. The r26 fix
    /// snapshots the detail string at Enter time and renders from
    /// the snapshot. This test simulates: 3 violations → Enter on
    /// row 1 (the middle one) → refresh replaces violations so the
    /// originally-clicked row's content is no longer at the same
    /// index AND has been evicted. The snapshot must still hold the
    /// original violation's detail.
    #[test]
    fn detail_modal_snapshots_at_enter_time() {
        let mut screen = ConformanceScreen::new();
        screen.view_mode = ViewMode::Violations;
        screen.violations = vec![
            violation(1_700_000_000, 0, "/a", "reason-a"),
            violation(1_700_000_001, 0, "/b", "reason-b"),
            violation(1_700_000_002, 0, "/c", "reason-c"),
        ];
        screen.table.set_total(screen.current_row_count());
        screen.table.selected = 1;
        // Sanity check: clicked-row detail mentions `/b`.
        let live_before = screen.selected_detail().expect("live detail before Enter");
        assert!(
            live_before.contains("/b"),
            "pre-Enter live detail must reference /b: {live_before}"
        );

        // Simulate Enter: opens detail modal AND captures snapshot.
        assert!(screen.handle_key(key(KeyCode::Enter)));
        assert!(screen.detail_open);
        let snap = screen.detail_snapshot.as_deref().expect("snapshot captured");
        assert!(snap.contains("/b"), "snapshot should contain /b: {snap}");

        // 5 s tick replaces the violations vec — the originally
        // clicked /b has been evicted by the 256-cap buffer in the
        // real server; we simulate by simply dropping it.
        screen.violations = vec![
            violation(1_700_000_010, 0, "/x", "reason-x"),
            violation(1_700_000_011, 0, "/y", "reason-y"),
            violation(1_700_000_012, 0, "/z", "reason-z"),
        ];
        screen.table.set_total(screen.current_row_count());
        // selected is still 1 → live detail now points at /y, but
        // the snapshot must still show /b.
        let live_after = screen.selected_detail().expect("live detail after refresh");
        assert!(
            live_after.contains("/y"),
            "live detail post-refresh is /y (the new row at index 1): {live_after}"
        );
        let snap_after = screen.detail_snapshot.as_deref().expect("snapshot retained");
        assert!(
            snap_after.contains("/b"),
            "snapshot must still hold pre-refresh /b: {snap_after}"
        );
        assert!(
            !snap_after.contains("/y"),
            "snapshot must NOT leak post-refresh /y: {snap_after}"
        );

        // Esc clears the snapshot so the next Enter re-captures fresh.
        assert!(screen.handle_key(key(KeyCode::Esc)));
        assert!(!screen.detail_open);
        assert!(screen.detail_snapshot.is_none());
    }

    /// Round 26 — same behaviour, but driven through the public
    /// `on_data` payload path that the live refresh tick actually
    /// uses. Exercises the JSON deserialiser + the identity-key
    /// re-anchor + the snapshot, end to end.
    #[test]
    fn detail_snapshot_survives_on_data_payload() {
        let mut screen = ConformanceScreen::new();
        screen.view_mode = ViewMode::Violations;
        let initial = r#"{"violations":[
            {"timestamp":"2026-06-06T13:20:00Z","method":"POST","path":"/a","client_ip":"1.2.3.4","status":400,"reason":"reason-a","category":"request-body"},
            {"timestamp":"2026-06-06T13:20:01Z","method":"POST","path":"/b","client_ip":"1.2.3.4","status":400,"reason":"reason-b","category":"request-body"}
        ],"total":2,"total_seen":2,"total_ok":0}"#;
        screen.on_data(initial);
        assert_eq!(screen.violations.len(), 2);
        screen.table.selected = 1;
        screen.handle_key(key(KeyCode::Enter));
        let snap = screen.detail_snapshot.clone().unwrap();
        assert!(snap.contains("/b"));

        // Now a fresh payload arrives where /b has been evicted.
        let refresh = r#"{"violations":[
            {"timestamp":"2026-06-06T13:20:10Z","method":"POST","path":"/x","client_ip":"1.2.3.4","status":400,"reason":"reason-x","category":"request-body"},
            {"timestamp":"2026-06-06T13:20:11Z","method":"POST","path":"/y","client_ip":"1.2.3.4","status":400,"reason":"reason-y","category":"request-body"}
        ],"total":2,"total_seen":4,"total_ok":0}"#;
        screen.on_data(refresh);
        // Live detail at index 1 would be /y; snapshot still /b.
        let snap_after = screen.detail_snapshot.clone().unwrap();
        assert!(
            snap_after.contains("/b"),
            "snapshot should be unchanged across on_data: {snap_after}"
        );
    }
}
