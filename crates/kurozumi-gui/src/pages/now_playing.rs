use iced::widget::{column, container, text, Column};
use iced::{Element, Length, Theme};

use kurozumi_core::models::DetectedMedia;

use crate::app::Message;

/// Render the "Now Playing" page.
pub fn view<'a>(detected: &'a Option<DetectedMedia>, status: &'a str) -> Element<'a, Message> {
    let content: Element<'a, Message> = match detected {
        Some(media) => {
            let title = media
                .anime_title
                .as_deref()
                .unwrap_or(&media.raw_title);

            let episode_text = media
                .episode
                .map(|ep| format!("Episode {ep}"))
                .unwrap_or_default();

            let player_text = format!("Playing in {}", media.player_name);

            let mut col: Column<'_, Message, Theme> = column![
                text(title).size(28),
                text(episode_text).size(20),
                text(player_text).size(14),
            ]
            .spacing(8);

            if let Some(group) = &media.release_group {
                col = col.push(text(format!("Release: {group}")).size(14));
            }
            if let Some(res) = &media.resolution {
                col = col.push(text(format!("Quality: {res}")).size(14));
            }

            if !status.is_empty() {
                col = col.push(text(status).size(12));
            }

            col.into()
        }
        None => {
            column![
                text("Nothing playing").size(24),
                text("Start a media player with an anime file to see it here.").size(14),
            ]
            .spacing(8)
            .into()
        }
    };

    container(content)
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
