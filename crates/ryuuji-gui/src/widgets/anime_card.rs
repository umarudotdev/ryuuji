use iced::widget::{button, column, container, progress_bar, text};
use iced::{Element, Length};

use crate::cover_cache::CoverCache;
use crate::style;
use crate::theme::{self, ColorScheme};
use crate::widgets;

/// Card width: cover + horizontal padding inside the card.
pub const CARD_WIDTH: f32 = style::COVER_WIDTH + 2.0 * style::SPACE_SM;

/// A compact anime card widget for grid display.
///
/// Shows cover image, truncated title, episode progress text, and a thin
/// progress bar. Generic over message type via `on_select` closure.
pub fn anime_card<'a, Message: Clone + 'static>(
    cs: &ColorScheme,
    covers: &'a CoverCache,
    cover_key: i64,
    title: &str,
    episode_text: &str,
    progress: Option<f32>,
    status_color: iced::Color,
    on_select: Message,
) -> Element<'a, Message> {
    // Cover image
    let cover = widgets::rounded_cover(
        cs,
        covers,
        cover_key,
        style::COVER_WIDTH,
        style::COVER_HEIGHT,
        style::RADIUS_MD,
    );

    // Title (clipped to 2 lines via container height)
    let title_el = container(
        text(title.to_string())
            .size(style::TEXT_SM)
            .font(style::FONT_HEADING)
            .color(cs.on_surface)
            .line_height(style::LINE_HEIGHT_NORMAL)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
    )
    .height(Length::Fixed(
        style::TEXT_SM * style::LINE_HEIGHT_NORMAL * 2.0 + 2.0,
    ))
    .clip(true);

    // Episode text
    let ep_el = text(episode_text.to_string())
        .size(style::TEXT_XS)
        .color(cs.on_surface_variant)
        .line_height(style::LINE_HEIGHT_LOOSE);

    // Progress bar (only when we have a fraction)
    let mut card_content = column![cover, title_el, ep_el]
        .spacing(style::SPACE_XS)
        .padding(style::SPACE_SM)
        .width(Length::Fixed(CARD_WIDTH));

    if let Some(pct) = progress {
        card_content = card_content.push(
            progress_bar(0.0..=1.0, pct)
                .girth(Length::Fixed(4.0))
                .style(theme::episode_progress(cs)),
        );
    }

    // Status color indicator — thin left border via a styled container
    let inner = container(card_content).style(theme::anime_card_style(cs, status_color));

    button(inner)
        .padding(0)
        .width(Length::Fixed(CARD_WIDTH))
        .on_press(on_select)
        .style(theme::anime_card_button(cs))
        .into()
}

/// Build an anime card from a library row.
pub fn library_card<'a, Message: Clone + 'static>(
    cs: &'a ColorScheme,
    lib_row: &'a ryuuji_core::storage::LibraryRow,
    covers: &'a CoverCache,
    on_select: Message,
) -> Element<'a, Message> {
    let anime = &lib_row.anime;
    let entry = &lib_row.entry;

    let ep_text = match anime.episodes {
        Some(total) => format!("Ep {} / {}", entry.watched_episodes, total),
        None => format!("Ep {}", entry.watched_episodes),
    };

    let progress = anime.episodes.map(|total| {
        if total > 0 {
            (entry.watched_episodes as f32 / total as f32).clamp(0.0, 1.0)
        } else {
            0.0
        }
    });

    let status_col = theme::status_color(cs, entry.status);

    anime_card(
        cs,
        covers,
        anime.id,
        anime.title.preferred(),
        &ep_text,
        progress,
        status_col,
        on_select,
    )
}

/// Build an anime card from an online search result (seasons/search).
pub fn online_card<'a, Message: Clone + 'static>(
    cs: &'a ColorScheme,
    result: &'a ryuuji_api::traits::AnimeSearchResult,
    covers: &'a CoverCache,
    cover_key: i64,
    on_select: Message,
) -> Element<'a, Message> {
    let ep_text = match result.episodes {
        Some(eps) => format!("{eps} eps"),
        None => String::new(),
    };

    let score_text = result
        .mean_score
        .map(|s| format!("\u{2605} {s:.1}"))
        .unwrap_or_default();

    let display = if !ep_text.is_empty() && !score_text.is_empty() {
        format!("{ep_text}  \u{00B7}  {score_text}")
    } else if !ep_text.is_empty() {
        ep_text
    } else {
        score_text
    };

    anime_card(
        cs,
        covers,
        cover_key,
        &result.title,
        &display,
        None,
        cs.outline_variant,
        on_select,
    )
}
