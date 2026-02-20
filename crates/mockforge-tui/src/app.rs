//! App state machine and main event loop.

use std::time::Duration;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::api::client::MockForgeClient;
use crate::config::TuiConfig;
use crate::event::{Event, EventHandler};
use crate::keybindings::{self, Action};
use crate::screens::{self, Screen, ScreenId};
use crate::theme::Theme;
use crate::tui;
use crate::widgets::command_palette::{CommandPalette, PaletteAction};
use crate::widgets::{help, status_bar};

/// Top-level application state.
pub struct App {
    config: TuiConfig,
    admin_url: String,
    client: MockForgeClient,
    screens: Vec<Box<dyn Screen>>,
    active_tab: usize,
    show_help: bool,
    command_palette: CommandPalette,
    connected: bool,
    error_count: usize,
    should_quit: bool,
    /// Y-offset of the tab bar for mouse click detection.
    tab_bar_y: u16,
    /// Last time we checked connectivity.
    last_health_check: std::time::Instant,
}

impl App {
    /// Create the app from a config and optional auth token.
    pub fn new(config: TuiConfig, token: Option<String>) -> Self {
        Theme::init(config.is_light_theme());

        let client =
            MockForgeClient::new(config.admin_url.clone(), token).expect("failed to build client");

        let admin_url = config.admin_url.clone();
        let initial_tab = config.last_tab.unwrap_or(0);

        let screens: Vec<Box<dyn Screen>> = vec![
            Box::new(screens::dashboard::DashboardScreen::new()),
            Box::new(screens::logs::LogsScreen::new()),
            Box::new(screens::routes::RoutesScreen::new()),
            Box::new(screens::metrics::MetricsScreen::new()),
            Box::new(screens::config::ConfigScreen::new()),
            Box::new(screens::chaos::ChaosScreen::new()),
            Box::new(screens::workspaces::WorkspacesScreen::new()),
            Box::new(screens::plugins::PluginsScreen::new()),
            Box::new(screens::fixtures::FixturesScreen::new()),
            Box::new(screens::health::HealthScreen::new()),
            Box::new(screens::smoke_tests::SmokeTestsScreen::new()),
            Box::new(screens::time_travel::TimeTravelScreen::new()),
            Box::new(screens::chains::ChainsScreen::new()),
            Box::new(screens::verification::VerificationScreen::new()),
            Box::new(screens::analytics::AnalyticsScreen::new()),
            Box::new(screens::recorder::RecorderScreen::new()),
            Box::new(screens::import::ImportScreen::new()),
            Box::new(screens::audit::AuditScreen::new()),
            Box::new(screens::world_state::WorldStateScreen::new()),
            Box::new(screens::contract_diff::ContractDiffScreen::new()),
            Box::new(screens::federation::FederationScreen::new()),
            Box::new(screens::behavioral_cloning::BehavioralCloningScreen::new()),
        ];

        let active_tab = if initial_tab < screens.len() {
            initial_tab
        } else {
            0
        };

        Self {
            config,
            admin_url,
            client,
            screens,
            active_tab,
            show_help: false,
            command_palette: CommandPalette::new(),
            connected: false,
            error_count: 0,
            should_quit: false,
            tab_bar_y: 1,
            last_health_check: std::time::Instant::now(),
        }
    }

    /// Run the terminal UI event loop.
    pub async fn run(mut self) -> Result<()> {
        let mut terminal = tui::init()?;
        let tick_rate = Duration::from_millis(250);
        let mut events = EventHandler::new(tick_rate);
        let tx = events.sender();

        // Initial connectivity check.
        self.connected = self.client.ping().await;

        loop {
            // Render.
            terminal.draw(|frame| self.render(frame))?;

            // Wait for next event.
            let event = events.next().await?;

            match event {
                Event::Key(key) => {
                    // Ctrl+C always quits.
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && matches!(key.code, KeyCode::Char('c'))
                    {
                        self.should_quit = true;
                    } else if self.command_palette.visible {
                        if let Some(action) = self.command_palette.handle_key(key) {
                            self.execute_palette_action(action);
                        }
                    } else if self.show_help {
                        if matches!(key.code, KeyCode::Char('?') | KeyCode::Esc) {
                            self.show_help = false;
                        }
                    } else {
                        // Try screen-specific handling first.
                        let consumed = self.screens[self.active_tab].handle_key(key);
                        if !consumed {
                            self.handle_global_key(key);
                        }
                    }
                }
                Event::Tick => {
                    self.screens[self.active_tab].tick(&self.client, &tx);

                    // Periodic connectivity check every 10 seconds.
                    if self.last_health_check.elapsed() >= Duration::from_secs(10) {
                        self.last_health_check = std::time::Instant::now();
                        let client = self.client.clone();
                        let health_tx = tx.clone();
                        tokio::spawn(async move {
                            let ok = client.ping().await;
                            // Use a special screen key for health check routing.
                            if ok {
                                let _ = health_tx.send(Event::Data {
                                    screen: "_health_check",
                                    payload: String::new(),
                                });
                            } else {
                                let _ = health_tx.send(Event::ApiError {
                                    screen: "_health_check",
                                    message: "Server unreachable".into(),
                                });
                            }
                        });
                    }
                }
                Event::Data { screen, payload } => {
                    self.route_data(screen, &payload);
                }
                Event::ApiError { screen, message } => {
                    self.error_count += 1;
                    self.route_error(screen, &message);
                }
                Event::LogLine(line) => {
                    self.connected = true;
                    if let Some(logs) = self.screens.get_mut(1) {
                        logs.push_log_line(line);
                    }
                }
                Event::Resize(_, _) => {}
                Event::Mouse(mouse) => {
                    self.handle_mouse(mouse);
                }
            }

            if self.should_quit {
                break;
            }
        }

        // Save last-used tab to config file (best-effort).
        self.config.last_tab = Some(self.active_tab);
        let _ = self.config.save();

        tui::restore()?;
        Ok(())
    }

    fn handle_global_key(&mut self, key: crossterm::event::KeyEvent) {
        // `:` opens the command palette (not in keybindings since it's a modal trigger).
        if matches!(key.code, KeyCode::Char(':')) {
            self.command_palette.open();
            return;
        }

        if let Some(action) = keybindings::resolve(key) {
            match action {
                Action::Quit => self.should_quit = true,
                Action::ToggleHelp => self.show_help = !self.show_help,
                Action::NextTab => {
                    self.active_tab = (self.active_tab + 1) % self.screens.len();
                }
                Action::PrevTab => {
                    self.active_tab = if self.active_tab == 0 {
                        self.screens.len() - 1
                    } else {
                        self.active_tab - 1
                    };
                }
                Action::JumpTab(idx) => {
                    if idx < self.screens.len() {
                        self.active_tab = idx;
                    }
                }
                Action::Refresh => {
                    self.screens[self.active_tab].force_refresh();
                }
                _ => {}
            }
        }
    }

    fn execute_palette_action(&mut self, action: PaletteAction) {
        match action {
            PaletteAction::GoToScreen(idx) => {
                if idx < self.screens.len() {
                    self.active_tab = idx;
                }
            }
            PaletteAction::Refresh => {
                self.screens[self.active_tab].force_refresh();
            }
            PaletteAction::ToggleHelp => {
                self.show_help = !self.show_help;
            }
            PaletteAction::Quit => {
                self.should_quit = true;
            }
        }
    }

    fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) {
        use crossterm::event::{MouseButton, MouseEventKind};

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Check if click is on the tab bar row.
                if mouse.row == self.tab_bar_y {
                    self.handle_tab_click(mouse.column);
                }
            }
            MouseEventKind::ScrollUp => {
                // Forward as Up key to the active screen.
                let key = crossterm::event::KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
                self.screens[self.active_tab].handle_key(key);
            }
            MouseEventKind::ScrollDown => {
                let key = crossterm::event::KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
                self.screens[self.active_tab].handle_key(key);
            }
            _ => {}
        }
    }

    fn handle_tab_click(&mut self, column: u16) {
        // Calculate tab boundaries based on rendered tab labels.
        let mut x: u16 = 0;
        for (i, screen) in self.screens.iter().enumerate() {
            let title_len = u16::try_from(screen.title().len()).unwrap_or(u16::MAX);
            let label_len: u16 = if i <= 9 {
                // " N:Title " + " " separator
                title_len.saturating_add(4)
            } else {
                // " Title " + " "
                title_len.saturating_add(3)
            };
            if column >= x && column < x.saturating_add(label_len) {
                self.active_tab = i;
                return;
            }
            x = x.saturating_add(label_len);
        }
    }

    fn route_data(&mut self, screen_key: &str, payload: &str) {
        self.connected = true;

        // Internal health check — not routed to any screen.
        if screen_key == "_health_check" {
            return;
        }

        for (i, sid) in ScreenId::ALL.iter().enumerate() {
            if sid.data_key() == screen_key {
                if let Some(screen) = self.screens.get_mut(i) {
                    screen.on_data(payload);
                }
                return;
            }
        }
    }

    fn route_error(&mut self, screen_key: &str, message: &str) {
        // Internal health check failure — mark disconnected but don't
        // propagate to any screen.
        if screen_key == "_health_check" {
            self.connected = false;
            return;
        }

        for (i, sid) in ScreenId::ALL.iter().enumerate() {
            if sid.data_key() == screen_key {
                if let Some(screen) = self.screens.get_mut(i) {
                    screen.on_error(message);
                }
                return;
            }
        }
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Minimum terminal size check (80x24).
        if area.width < 80 || area.height < 24 {
            let msg = Paragraph::new(format!(
                "Terminal too small ({}x{}). Minimum: 80x24. Please resize.",
                area.width, area.height
            ))
            .style(Style::default().fg(Theme::RED))
            .alignment(Alignment::Center);
            let centered = Rect {
                y: area.height / 2,
                height: 1,
                ..area
            };
            frame.render_widget(msg, centered);
            return;
        }

        let chunks = Layout::vertical([
            Constraint::Length(2), // title bar + tabs
            Constraint::Min(0),    // main content
            Constraint::Length(1), // status bar
        ])
        .split(area);

        self.render_header(frame, chunks[0]);

        // Error banner: show a persistent 1-line banner when the active screen
        // has an error, while still rendering data underneath.
        let content_area = if let Some(err) = self.screens[self.active_tab].error() {
            let parts =
                Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(chunks[1]);
            let banner = Paragraph::new(format!(" Error: {err}"))
                .style(Style::default().fg(Theme::RED).bg(Theme::OVERLAY));
            frame.render_widget(banner, parts[0]);
            parts[1]
        } else {
            chunks[1]
        };

        self.screens[self.active_tab].render(frame, content_area);

        status_bar::render(
            frame,
            chunks[2],
            self.connected,
            self.screens[self.active_tab].status_hint(),
            self.error_count,
            &self.admin_url,
        );

        if self.show_help {
            help::render(frame);
        }

        if self.command_palette.visible {
            self.command_palette.render(frame);
        }
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area);

        // Title bar
        let conn_status = if self.connected {
            "Connected"
        } else {
            "Disconnected"
        };
        let conn_style = if self.connected {
            Theme::success()
        } else {
            Theme::error()
        };
        let title = Line::from(vec![
            Span::styled(" MockForge TUI ", Theme::title()),
            Span::styled(format!("v{}", env!("CARGO_PKG_VERSION")), Theme::dim()),
            Span::raw("  "),
            Span::styled(conn_status, conn_style),
            Span::styled(format!("  {}", self.admin_url), Theme::dim()),
        ]);
        frame.render_widget(Paragraph::new(title).style(Theme::surface()), chunks[0]);

        // Tab bar
        let mut tab_spans = Vec::new();
        for (i, screen) in self.screens.iter().enumerate() {
            let style = if i == self.active_tab {
                Theme::tab_active()
            } else {
                Theme::tab_inactive()
            };
            let label = if i < 9 {
                format!(" {}:{} ", i + 1, screen.title())
            } else if i == 9 {
                format!(" 0:{} ", screen.title())
            } else {
                format!(" {} ", screen.title())
            };
            tab_spans.push(Span::styled(label, style));
            tab_spans.push(Span::raw(" "));
        }
        let tabs = Line::from(tab_spans);
        frame.render_widget(Paragraph::new(tabs).style(Theme::base()), chunks[1]);
    }
}
