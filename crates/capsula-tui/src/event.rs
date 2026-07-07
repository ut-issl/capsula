use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind, MouseEvent};

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
        Ok(classify_event(&event::read()?))
    } else {
        Ok(AppEvent::Tick)
    }
}

/// Translate a raw crossterm [`Event`] into an [`AppEvent`].
///
/// Only key *press* events are surfaced as [`AppEvent::Key`]. On Windows the
/// console backend reports both key press and key release events (Unix reports
/// only presses), so the key-release event from the Enter used to launch
/// `capsula tui` would otherwise be read on startup and immediately activate
/// the focused button, starting the run without user input (see issue #1091).
/// Everything that is not a key press or a mouse event becomes a [`AppEvent::Tick`].
fn classify_event(event: &Event) -> AppEvent {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => AppEvent::Key(*key),
        Event::Mouse(mouse) => AppEvent::Mouse(*mouse),
        _ => AppEvent::Tick,
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton};

    use super::*;

    #[test]
    fn key_press_is_forwarded() {
        let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        match classify_event(&event) {
            AppEvent::Key(key) => assert_eq!(key.code, KeyCode::Enter),
            _ => panic!("expected a key event to be forwarded"),
        }
    }

    #[test]
    fn key_release_is_ignored() {
        // On Windows crossterm reports key-release events; treating them as
        // presses causes the run to start immediately on launch (issue #1091).
        let event = Event::Key(KeyEvent::new_with_kind(
            KeyCode::Enter,
            KeyModifiers::NONE,
            KeyEventKind::Release,
        ));
        assert!(matches!(classify_event(&event), AppEvent::Tick));
    }

    #[test]
    fn key_repeat_is_ignored() {
        let event = Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char(' '),
            KeyModifiers::NONE,
            KeyEventKind::Repeat,
        ));
        assert!(matches!(classify_event(&event), AppEvent::Tick));
    }

    #[test]
    fn mouse_event_is_forwarded() {
        let mouse = crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(MouseButton::Left),
            column: 3,
            row: 4,
            modifiers: KeyModifiers::NONE,
        };
        assert!(matches!(
            classify_event(&Event::Mouse(mouse)),
            AppEvent::Mouse(_)
        ));
    }
}
