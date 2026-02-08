use iced::widget::{center, column, container, row, text};
use iced::{Alignment, Element, Length};

use kurozumi_core::models::DetectedMedia;

use crate::app::Message;
use crate::style;
use crate::theme::{self, ColorScheme};

/// Render the "Now Playing" page.
pub fn view<'a>(
    cs: &ColorScheme,
    detected: &'a Option<DetectedMedia>,
    status: &'a str,
) -> Element<'a, Message> {
    let content: Element<'a, Message> = match detected {
        Some(media) => playing_card(cs, media, status),
        None => empty_state(cs),
    };

    container(content)
        .padding(style::SPACE_XL)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Card layout for active playback.
fn playing_card<'a>(
    cs: &ColorScheme,
    media: &'a DetectedMedia,
    status: &'a str,
) -> Element<'a, Message> {
    let title = media
        .anime_title
        .as_deref()
        .unwrap_or(&media.raw_title);

    let episode_text = media
        .episode
        .map(|ep| format!("Episode {ep}"))
        .unwrap_or_default();

    // Metadata row: player . release group . quality
    let mut meta_parts: Vec<String> = vec![media.player_name.clone()];
    if let Some(group) = &media.release_group {
        meta_parts.push(group.clone());
    }
    if let Some(res) = &media.resolution {
        meta_parts.push(res.clone());
    }
    let meta_line = meta_parts.join("  \u{00B7}  ");

    // Main info card with generous padding
    let info_card = container(
        column![
            text(title).size(style::TEXT_3XL),
            text(episode_text)
                .size(style::TEXT_LG)
                .color(cs.primary),
            text(meta_line)
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant),
        ]
        .spacing(style::SPACE_SM),
    )
    .style(theme::card(cs))
    .padding(style::SPACE_2XL)
    .width(Length::Fill);

    let mut page = column![info_card].spacing(style::SPACE_LG);

    // Status message (only if non-empty).
    if !status.is_empty() {
        let status_color = if status.starts_with("Error") {
            cs.error
        } else if status.starts_with("Updated") || status.starts_with("Added") {
            cs.status_completed
        } else {
            cs.on_surface_variant
        };

        let status_card = container(
            row![
                text("\u{2022}").size(style::TEXT_SM).color(status_color),
                text(status).size(style::TEXT_SM).color(status_color),
            ]
            .spacing(style::SPACE_SM)
            .align_y(Alignment::Center),
        )
        .style(theme::card(cs))
        .padding([style::SPACE_SM, style::SPACE_LG])
        .width(Length::Fill);

        page = page.push(status_card);
    }

    page.into()
}

/// Empty state when nothing is playing.
fn empty_state<'a>(cs: &ColorScheme) -> Element<'a, Message> {
    let content = column![
        text("\u{25B6}") // Play triangle
            .size(56.0)
            .color(cs.outline),
        text("Nothing playing")
            .size(style::TEXT_XL)
            .color(cs.on_surface_variant),
        text("Start a media player with an anime file to see it here.")
            .size(style::TEXT_SM)
            .color(cs.outline),
    ]
    .spacing(style::SPACE_MD)
    .align_x(Alignment::Center);

    center(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
