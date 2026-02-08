use iced::widget::{column, container, pick_list, row, scrollable, text};
use iced::{Alignment, Element, Length};
use iced_aw::NumberInput;

use kurozumi_core::models::WatchStatus;
use kurozumi_core::storage::LibraryRow;

use crate::style;
use crate::theme::{self, ColorScheme};

/// Shared anime detail panel used by both library and search screens.
///
/// Takes callback closures to map interactions to the caller's message type.
pub fn detail_panel<'a, Message: Clone + 'static>(
    cs: &ColorScheme,
    lib_row: &'a LibraryRow,
    on_status_changed: impl Fn(WatchStatus) -> Message + 'static + Clone,
    on_score_changed: impl Fn(f32) -> Message + 'static + Clone,
    on_episode_changed: impl Fn(u32) -> Message + 'static + Clone,
) -> Element<'a, Message> {
    let anime = &lib_row.anime;
    let entry = &lib_row.entry;

    let cover = container(
        text("\u{1F3AC}")
            .size(style::TEXT_3XL)
            .color(cs.outline)
            .center(),
    )
    .width(Length::Fixed(style::COVER_WIDTH))
    .height(Length::Fixed(style::COVER_HEIGHT))
    .center_x(Length::Fixed(style::COVER_WIDTH))
    .center_y(Length::Fixed(style::COVER_HEIGHT))
    .style(theme::cover_placeholder(cs));

    let mut title_section = column![text(anime.title.preferred())
        .size(style::TEXT_XL)
        .font(style::FONT_HEADING)
        .line_height(style::LINE_HEIGHT_TIGHT),]
    .spacing(style::SPACE_XS);

    if let Some(english) = &anime.title.english {
        if Some(english.as_str()) != anime.title.romaji.as_deref() {
            title_section = title_section.push(
                text(english.as_str())
                    .size(style::TEXT_SM)
                    .color(cs.on_surface_variant)
                    .line_height(style::LINE_HEIGHT_LOOSE),
            );
        }
    }

    let mut meta_parts: Vec<String> = Vec::new();
    if let Some(season) = &anime.season {
        meta_parts.push(season.clone());
    }
    if let Some(year) = anime.year {
        meta_parts.push(year.to_string());
    }
    if !meta_parts.is_empty() {
        title_section = title_section.push(
            text(meta_parts.join(" "))
                .size(style::TEXT_SM)
                .color(cs.outline)
                .line_height(style::LINE_HEIGHT_LOOSE),
        );
    }

    let score_val = entry.score.unwrap_or(0.0);

    let on_score_cb = on_score_changed;
    let status_card = container(
        column![
            text("Status")
                .size(style::TEXT_XS)
                .color(cs.on_surface_variant)
                .line_height(style::LINE_HEIGHT_LOOSE),
            pick_list(WatchStatus::ALL, Some(entry.status), move |s| {
                on_status_changed(s)
            })
            .text_size(style::TEXT_SM)
            .padding([style::SPACE_XS, style::SPACE_SM]),
            text("Score")
                .size(style::TEXT_XS)
                .color(cs.on_surface_variant)
                .line_height(style::LINE_HEIGHT_LOOSE),
            NumberInput::new(&score_val, 0.0..=10.0, on_score_cb)
                .step(0.5)
                .width(Length::Fixed(100.0))
                .set_size(style::TEXT_SM)
                .padding([style::SPACE_XS, style::SPACE_SM])
                .style(theme::aw_number_input_style(cs))
                .input_style(theme::text_input_style(cs)),
        ]
        .spacing(style::SPACE_SM),
    )
    .style(theme::card(cs))
    .padding(style::SPACE_LG)
    .width(Length::Fill);

    let ep_text = match anime.episodes {
        Some(total) => format!("Episode {} / {}", entry.watched_episodes, total),
        None => format!("Episode {}", entry.watched_episodes),
    };
    let max_ep = anime.episodes.unwrap_or(u32::MAX);

    let progress_card = container(
        column![
            text(ep_text)
                .size(style::TEXT_BASE)
                .line_height(style::LINE_HEIGHT_NORMAL),
            NumberInput::new(&entry.watched_episodes, 0..=max_ep, move |ep| {
                on_episode_changed(ep)
            })
            .step(1u32)
            .width(Length::Fixed(120.0))
            .set_size(style::TEXT_SM)
            .padding([style::SPACE_XS, style::SPACE_SM])
            .style(theme::aw_number_input_style(cs))
            .input_style(theme::text_input_style(cs)),
        ]
        .spacing(style::SPACE_SM),
    )
    .style(theme::card(cs))
    .padding(style::SPACE_LG)
    .width(Length::Fill);

    let detail = column![
        row![cover, title_section]
            .spacing(style::SPACE_LG)
            .align_y(Alignment::Start),
        status_card,
        progress_card,
    ]
    .spacing(style::SPACE_LG)
    .padding(style::SPACE_LG);

    scrollable(detail).height(Length::Fill).into()
}
