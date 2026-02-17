//! Global keyboard shortcuts.
//!
//! Maps key combinations to semantic `Shortcut` variants that the app
//! router dispatches based on the current page and modal state.

use iced::keyboard;
use iced::Subscription;

use crate::app::Message;

/// Application-level keyboard shortcuts.
#[derive(Debug, Clone)]
pub enum Shortcut {
    /// F5 — refresh current screen data.
    Refresh,
    /// Ctrl+C — copy selected anime title to clipboard.
    CopyTitle,
    /// Numpad+ or Ctrl+Up — increment episode for selected anime.
    IncrementEpisode,
    /// Numpad- or Ctrl+Down — decrement episode for selected anime.
    DecrementEpisode,
    /// Ctrl+1-9 → scores 1.0-9.0, Ctrl+0 → 10.0.
    SetScore(u8),
    /// Ctrl+F — focus the search/filter input.
    FocusSearch,
    /// Escape — deselect current selection or dismiss modal.
    Escape,
}

/// Subscription that converts keyboard events to `Message::Shortcut`.
pub fn keyboard_subscription() -> Subscription<Message> {
    iced::event::listen_with(|event, _status, _id| match event {
        iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
            map_shortcut(key, modifiers)
        }
        _ => None,
    })
}

fn map_shortcut(key: keyboard::Key, modifiers: keyboard::Modifiers) -> Option<Message> {
    use keyboard::key::Named;
    use keyboard::Key;

    let ctrl = modifiers.control();

    match key {
        Key::Named(Named::F5) => Some(Shortcut::Refresh),
        Key::Named(Named::Escape) => Some(Shortcut::Escape),
        Key::Named(Named::ArrowUp) if ctrl => Some(Shortcut::IncrementEpisode),
        Key::Named(Named::ArrowDown) if ctrl => Some(Shortcut::DecrementEpisode),
        Key::Character(ref c) if ctrl => match c.as_str() {
            "c" => Some(Shortcut::CopyTitle),
            "f" => Some(Shortcut::FocusSearch),
            "1" => Some(Shortcut::SetScore(1)),
            "2" => Some(Shortcut::SetScore(2)),
            "3" => Some(Shortcut::SetScore(3)),
            "4" => Some(Shortcut::SetScore(4)),
            "5" => Some(Shortcut::SetScore(5)),
            "6" => Some(Shortcut::SetScore(6)),
            "7" => Some(Shortcut::SetScore(7)),
            "8" => Some(Shortcut::SetScore(8)),
            "9" => Some(Shortcut::SetScore(9)),
            "0" => Some(Shortcut::SetScore(10)),
            _ => None,
        },
        _ => None,
    }
    .map(Message::Shortcut)
}
