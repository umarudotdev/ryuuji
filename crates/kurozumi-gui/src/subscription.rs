use iced::Subscription;
use std::time::Duration;

use crate::app::Message;

/// Creates a subscription that ticks every `interval` seconds,
/// triggering media player detection.
pub fn detection_tick(interval_secs: u64) -> Subscription<Message> {
    iced::time::every(Duration::from_secs(interval_secs)).map(|_| Message::DetectionTick)
}
