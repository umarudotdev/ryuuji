//! Subscription composition — batches all app subscriptions into one.

use std::time::Duration;

use iced::Subscription;
use kurozumi_core::config::ThemeMode;

use crate::app::Message;

/// Compose all application subscriptions.
///
/// - Detection tick (always active)
/// - OS appearance check (only when ThemeMode::System)
/// - Window events (resize/move for state persistence)
pub fn subscriptions(interval_secs: u64, theme_mode: ThemeMode) -> Subscription<Message> {
    let mut subs = vec![detection_tick(interval_secs), window_events()];

    if theme_mode == ThemeMode::System {
        subs.push(appearance_check());
    }

    Subscription::batch(subs)
}

/// Ticks every `interval` seconds, triggering media player detection.
fn detection_tick(interval_secs: u64) -> Subscription<Message> {
    iced::time::every(Duration::from_secs(interval_secs)).map(|_| Message::DetectionTick)
}

/// Forwards window resize and move events for state persistence.
fn window_events() -> Subscription<Message> {
    iced::window::events().map(|(_id, event)| Message::WindowEvent(event))
}

/// Polls the OS dark/light mode every 5 seconds.
fn appearance_check() -> Subscription<Message> {
    iced::time::every(Duration::from_secs(5)).map(|_| {
        let mode = match dark_light::detect() {
            Ok(dark_light::Mode::Light) => ThemeMode::Light,
            _ => ThemeMode::Dark,
        };
        Message::AppearanceChanged(mode)
    })
}
