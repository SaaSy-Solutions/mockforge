//! Channel-based async event handler for terminal, SSE, and data poll events.

use anyhow::Result;
use crossterm::event::{self, Event as TermEvent, KeyEvent, MouseEvent};
use std::time::Duration;
use tokio::sync::mpsc;

/// All event types the main loop can receive.
#[derive(Debug)]
pub enum Event {
    /// Terminal key press.
    Key(KeyEvent),
    /// Terminal mouse input.
    Mouse(MouseEvent),
    /// Terminal resize.
    Resize(u16, u16),
    /// Periodic tick for polling / animation.
    Tick,
    /// Data fetched from the admin API (screen id + serialised JSON).
    Data {
        screen: &'static str,
        payload: String,
    },
    /// An API error to display.
    ApiError {
        screen: &'static str,
        message: String,
    },
    /// SSE log line received.
    LogLine(String),
}

/// Spawns a background task that reads terminal events and emits ticks.
pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
    _tx: mpsc::UnboundedSender<Event>,
}

impl EventHandler {
    /// Create the event handler. `tick_rate` controls how often `Tick` events
    /// are generated.
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let event_tx = tx.clone();

        tokio::spawn(async move {
            loop {
                let has_event = event::poll(tick_rate).unwrap_or(false);
                if has_event {
                    match event::read() {
                        Ok(TermEvent::Key(key)) => {
                            if event_tx.send(Event::Key(key)).is_err() {
                                return;
                            }
                        }
                        Ok(TermEvent::Mouse(mouse)) => {
                            if event_tx.send(Event::Mouse(mouse)).is_err() {
                                return;
                            }
                        }
                        Ok(TermEvent::Resize(w, h)) => {
                            if event_tx.send(Event::Resize(w, h)).is_err() {
                                return;
                            }
                        }
                        _ => {}
                    }
                } else {
                    // No terminal event within the tick window — emit tick.
                    if event_tx.send(Event::Tick).is_err() {
                        return;
                    }
                }
            }
        });

        Self { rx, _tx: tx }
    }

    /// Get a clone of the sender so background tasks can push events.
    pub fn sender(&self) -> mpsc::UnboundedSender<Event> {
        self._tx.clone()
    }

    /// Wait for the next event.
    pub async fn next(&mut self) -> Result<Event> {
        self.rx.recv().await.ok_or_else(|| anyhow::anyhow!("event channel closed"))
    }
}
