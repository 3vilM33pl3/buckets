// SPDX-License-Identifier: MIT

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::time::Duration;

use crate::message::AppMessage;

/// Poll for a crossterm event and convert it to an AppMessage.
/// Returns None if no event is available within the timeout.
pub fn poll_event(timeout: Duration) -> Option<AppMessage> {
    if event::poll(timeout).unwrap_or(false) {
        if let Ok(Event::Key(key)) = event::read() {
            // Ctrl+C always quits
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                return Some(AppMessage::Quit);
            }
            return Some(AppMessage::KeyPress(key));
        }
    }
    None
}
