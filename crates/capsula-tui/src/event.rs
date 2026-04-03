use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent, MouseEvent};

/// Application-level event combining crossterm events with a periodic tick.
pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Tick,
}

/// Poll for the next event, returning `Tick` if no event arrives within the timeout.
///
/// The 1-second timeout drives the elapsed-time display during an active run.
pub fn next_event() -> Result<AppEvent> {
    if event::poll(Duration::from_secs(1))? {
        match event::read()? {
            Event::Key(key) => Ok(AppEvent::Key(key)),
            Event::Mouse(mouse) => Ok(AppEvent::Mouse(mouse)),
            _ => Ok(AppEvent::Tick),
        }
    } else {
        Ok(AppEvent::Tick)
    }
}
