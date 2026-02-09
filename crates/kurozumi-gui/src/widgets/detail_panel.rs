use iced::widget::{button, column, container, pick_list, progress_bar, row, text};
use iced::{Alignment, Element, Length};

use kurozumi_api::traits::AnimeSearchResult;
use kurozumi_core::models::WatchStatus;
use kurozumi_core::storage::LibraryRow;

use crate::cover_cache::CoverCache;
use crate::format;
use crate::style;
use crate::theme::{self, ColorScheme};
use crate::widgets;

/// Shared anime detail panel used by both library and search screens.
///
/// Takes callback closures to map interactions to the caller's message type.
/// `score_text` / `episode_text` are owned String buffers for intermediate typing.
#[allow(clippy::too_many_arguments)]
pub fn detail_panel<'a, Message: Clone + 'static>(
    cs: &ColorScheme,
    lib_row: &'a LibraryRow,
    on_close: Message,
    on_status_changed: impl Fn(WatchStatus) -> Message + 'static + Clone,
    on_score_changed: impl Fn(f32) -> Message + 'static + Clone,
    on_episode_changed: impl Fn(u32) -> Message + 'static + Clone,
    score_text: &str,
    on_score_input: impl Fn(String) -> Message + 'a,
    on_score_submit: Message,
    episode_text: &str,
    on_episode_input: impl Fn(String) -> Message + 'a,
    on_episode_submit: Message,
    covers: &'a CoverCache,
) -> Element<'a, Message> {
    let anime = &lib_row.anime;
    let entry = &lib_row.entry;

    // ── Cover image (rounded) ────────────────────────────────────
    let cover = widgets::rounded_cover(
        cs,
        covers,
        anime.id,
        style::COVER_WIDTH,
        style::COVER_HEIGHT,
        style::RADIUS_LG,
    );

    // ── Title section ────────────────────────────────────────────
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

    // Meta line: media type + season/year
    let mut meta_parts: Vec<String> = Vec::new();
    if let Some(mt) = &anime.media_type {
        meta_parts.push(format::media_type(mt));
    }
    if let Some(season) = &anime.season {
        meta_parts.push(season.clone());
    }
    if let Some(year) = anime.year {
        meta_parts.push(year.to_string());
    }
    if !meta_parts.is_empty() {
        title_section = title_section.push(
            text(meta_parts.join("  \u{00B7}  "))
                .size(style::TEXT_SM)
                .color(cs.outline)
                .line_height(style::LINE_HEIGHT_LOOSE),
        );
    }

    // Genre badges (wrapping)
    if !anime.genres.is_empty() {
        let badges: Vec<Element<'_, Message>> = anime
            .genres
            .iter()
            .map(|g| {
                container(
                    text(g.as_str())
                        .size(style::TEXT_XS)
                        .line_height(style::LINE_HEIGHT_NORMAL),
                )
                .style(theme::metadata_badge(cs))
                .padding([style::SPACE_XXS, style::BADGE_PADDING_H])
                .center_y(Length::Fixed(style::BADGE_HEIGHT))
                .into()
            })
            .collect();

        let wrap = iced_aw::Wrap::with_elements(badges)
            .spacing(style::SPACE_XS)
            .line_spacing(style::SPACE_XS);
        title_section = title_section.push(wrap);
    }

    // Community score
    if let Some(score) = anime.mean_score {
        title_section = title_section.push(
            text(format!("\u{2605} {score:.2}"))
                .size(style::TEXT_SM)
                .color(cs.primary)
                .line_height(style::LINE_HEIGHT_LOOSE),
        );
    }

    // ── Close button ──────────────────────────────────────────────
    let close_size = style::TEXT_SM + style::SPACE_XS * 2.0;
    let close_btn = button(
        container(
            lucide_icons::iced::icon_x()
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill),
    )
    .on_press(on_close)
    .padding(0)
    .width(Length::Fixed(close_size))
    .height(Length::Fixed(close_size))
    .style(theme::icon_button(cs));

    let top_bar = row![container("").width(Length::Fill), close_btn]
        .align_y(Alignment::Center);

    // ── Synopsis snippet ─────────────────────────────────────────
    let mut detail_content = column![
        top_bar,
        row![cover, title_section]
            .spacing(style::SPACE_LG)
            .align_y(Alignment::Start),
    ]
    .spacing(style::SPACE_LG)
    .padding(style::SPACE_LG);

    if let Some(synopsis) = &anime.synopsis {
        if !synopsis.is_empty() {
            let truncated: String = synopsis.chars().take(150).collect();
            let display = if synopsis.chars().count() > 150 {
                format!("{truncated}\u{2026}")
            } else {
                synopsis.clone()
            };
            let synopsis_card = container(
                column![
                    text("Synopsis")
                        .size(style::TEXT_XS)
                        .font(style::FONT_HEADING)
                        .color(cs.on_surface_variant)
                        .line_height(style::LINE_HEIGHT_LOOSE),
                    text(display)
                        .size(style::TEXT_XS)
                        .color(cs.outline)
                        .line_height(style::LINE_HEIGHT_LOOSE),
                ]
                .spacing(style::SPACE_XS),
            )
            .style(theme::card(cs))
            .padding(style::SPACE_LG)
            .width(Length::Fill);

            detail_content = detail_content.push(synopsis_card);
        }
    }

    // ── Status & Score card ──────────────────────────────────────
    let score_val = entry.score.unwrap_or(0.0);

    let score_dec = if score_val > 0.0 {
        Some(on_score_changed.clone()(score_val - 0.5))
    } else {
        None
    };
    let score_inc = if score_val < 10.0 {
        Some(on_score_changed(score_val + 0.5))
    } else {
        None
    };

    let status_card = container(
        column![
            text("Status & Score")
                .size(style::TEXT_SM)
                .font(style::FONT_HEADING)
                .color(cs.on_surface_variant)
                .line_height(style::LINE_HEIGHT_LOOSE),
            row![
                text("Watch Status")
                    .size(style::TEXT_SM)
                    .color(cs.on_surface)
                    .line_height(style::LINE_HEIGHT_NORMAL)
                    .width(Length::Fill),
                pick_list(WatchStatus::ALL, Some(entry.status), move |s| {
                    on_status_changed(s)
                })
                .text_size(style::TEXT_SM)
                .padding([style::SPACE_SM, style::SPACE_MD])
                .style(theme::pick_list_style(cs))
                .menu_style(theme::pick_list_menu_style(cs)),
            ]
            .align_y(Alignment::Center)
            .spacing(style::SPACE_SM),
            row![
                text("Your Score")
                    .size(style::TEXT_SM)
                    .color(cs.on_surface)
                    .line_height(style::LINE_HEIGHT_NORMAL)
                    .width(Length::Fill),
                widgets::stepper(
                    cs,
                    score_text,
                    on_score_input,
                    on_score_submit,
                    score_dec,
                    score_inc,
                ),
            ]
            .align_y(Alignment::Center)
            .spacing(style::SPACE_SM),
        ]
        .spacing(style::SPACE_MD),
    )
    .style(theme::card(cs))
    .padding(style::SPACE_LG)
    .width(Length::Fill);

    detail_content = detail_content.push(status_card);

    // ── Episode Progress card ────────────────────────────────────
    let ep_text = match anime.episodes {
        Some(total) => format!("Episode {} / {}", entry.watched_episodes, total),
        None => format!("Episode {}", entry.watched_episodes),
    };
    let max_ep = anime.episodes.unwrap_or(u32::MAX);

    let (pct, pct_text) = match anime.episodes {
        Some(total) if total > 0 => {
            let p = (entry.watched_episodes as f32 / total as f32).clamp(0.0, 1.0);
            (p, format!("{:.0}%", p * 100.0))
        }
        _ => (0.0, "\u{2014}".into()),
    };

    let ep_dec = if entry.watched_episodes > 0 {
        Some(on_episode_changed.clone()(entry.watched_episodes - 1))
    } else {
        None
    };
    let ep_inc = if entry.watched_episodes < max_ep {
        Some(on_episode_changed(entry.watched_episodes + 1))
    } else {
        None
    };

    let progress_card = container(
        column![
            text("Episode Progress")
                .size(style::TEXT_SM)
                .font(style::FONT_HEADING)
                .color(cs.on_surface_variant)
                .line_height(style::LINE_HEIGHT_LOOSE),
            row![
                text(ep_text)
                    .size(style::TEXT_SM)
                    .line_height(style::LINE_HEIGHT_NORMAL),
                text(pct_text)
                    .size(style::TEXT_SM)
                    .color(cs.primary)
                    .line_height(style::LINE_HEIGHT_NORMAL),
            ]
            .spacing(style::SPACE_SM)
            .align_y(Alignment::Center),
            progress_bar(0.0..=1.0, pct)
                .girth(Length::Fixed(style::PROGRESS_HEIGHT))
                .style(theme::episode_progress(cs)),
            row![
                text("Set Episode")
                    .size(style::TEXT_SM)
                    .color(cs.on_surface)
                    .line_height(style::LINE_HEIGHT_NORMAL)
                    .width(Length::Fill),
                widgets::stepper(
                    cs,
                    episode_text,
                    on_episode_input,
                    on_episode_submit,
                    ep_dec,
                    ep_inc,
                ),
            ]
            .align_y(Alignment::Center)
            .spacing(style::SPACE_SM),
        ]
        .spacing(style::SPACE_SM),
    )
    .style(theme::card(cs))
    .padding(style::SPACE_LG)
    .width(Length::Fill);

    detail_content = detail_content.push(progress_card);

    super::styled_scrollable(detail_content, cs)
        .height(Length::Fill)
        .into()
}

/// Detail panel for an online search result (no library entry yet).
pub fn online_detail_panel<'a, Message: Clone + 'static>(
    cs: &ColorScheme,
    result: &'a AnimeSearchResult,
    on_close: Message,
    on_add: Message,
    covers: &'a CoverCache,
    cover_key: i64,
) -> Element<'a, Message> {
    // Cover image
    let cover = widgets::rounded_cover(
        cs,
        covers,
        cover_key,
        style::COVER_WIDTH,
        style::COVER_HEIGHT,
        style::RADIUS_LG,
    );

    // Title section
    let mut title_section = column![text(result.title.as_str())
        .size(style::TEXT_XL)
        .font(style::FONT_HEADING)
        .line_height(style::LINE_HEIGHT_TIGHT),]
    .spacing(style::SPACE_XS);

    if let Some(english) = &result.title_english {
        if english != &result.title {
            title_section = title_section.push(
                text(english.as_str())
                    .size(style::TEXT_SM)
                    .color(cs.on_surface_variant)
                    .line_height(style::LINE_HEIGHT_LOOSE),
            );
        }
    }

    // Meta line
    let mut meta_parts: Vec<String> = Vec::new();
    if let Some(mt) = &result.media_type {
        meta_parts.push(format::media_type(mt));
    }
    if let Some(season) = &result.season {
        meta_parts.push(season.clone());
    }
    if let Some(year) = result.year {
        meta_parts.push(year.to_string());
    }
    if let Some(eps) = result.episodes {
        meta_parts.push(format!("{eps} eps"));
    }
    if !meta_parts.is_empty() {
        title_section = title_section.push(
            text(meta_parts.join("  \u{00B7}  "))
                .size(style::TEXT_SM)
                .color(cs.outline)
                .line_height(style::LINE_HEIGHT_LOOSE),
        );
    }

    // Genre badges
    if !result.genres.is_empty() {
        let badges: Vec<Element<'_, Message>> = result
            .genres
            .iter()
            .map(|g| {
                container(
                    text(g.as_str())
                        .size(style::TEXT_XS)
                        .line_height(style::LINE_HEIGHT_NORMAL),
                )
                .style(theme::metadata_badge(cs))
                .padding([style::SPACE_XXS, style::BADGE_PADDING_H])
                .center_y(Length::Fixed(style::BADGE_HEIGHT))
                .into()
            })
            .collect();

        let wrap = iced_aw::Wrap::with_elements(badges)
            .spacing(style::SPACE_XS)
            .line_spacing(style::SPACE_XS);
        title_section = title_section.push(wrap);
    }

    // Community score
    if let Some(score) = result.mean_score {
        title_section = title_section.push(
            text(format!("\u{2605} {score:.2}"))
                .size(style::TEXT_SM)
                .color(cs.primary)
                .line_height(style::LINE_HEIGHT_LOOSE),
        );
    }

    // Close button
    let close_size = style::TEXT_SM + style::SPACE_XS * 2.0;
    let close_btn = button(
        container(
            lucide_icons::iced::icon_x()
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill),
    )
    .on_press(on_close)
    .padding(0)
    .width(Length::Fixed(close_size))
    .height(Length::Fixed(close_size))
    .style(theme::icon_button(cs));

    let top_bar = row![container("").width(Length::Fill), close_btn]
        .align_y(Alignment::Center);

    let mut detail_content = column![
        top_bar,
        row![cover, title_section]
            .spacing(style::SPACE_LG)
            .align_y(Alignment::Start),
    ]
    .spacing(style::SPACE_LG)
    .padding(style::SPACE_LG);

    // Synopsis
    if let Some(synopsis) = &result.synopsis {
        if !synopsis.is_empty() {
            let synopsis_card = container(
                column![
                    text("Synopsis")
                        .size(style::TEXT_XS)
                        .font(style::FONT_HEADING)
                        .color(cs.on_surface_variant)
                        .line_height(style::LINE_HEIGHT_LOOSE),
                    text(synopsis.as_str())
                        .size(style::TEXT_XS)
                        .color(cs.outline)
                        .line_height(style::LINE_HEIGHT_LOOSE),
                ]
                .spacing(style::SPACE_XS),
            )
            .style(theme::card(cs))
            .padding(style::SPACE_LG)
            .width(Length::Fill);

            detail_content = detail_content.push(synopsis_card);
        }
    }

    // "Add to Library" button
    let add_btn = button(
        row![
            lucide_icons::iced::icon_plus()
                .size(style::TEXT_BASE)
                .center(),
            text("Add to Library")
                .size(style::TEXT_SM)
                .line_height(style::LINE_HEIGHT_NORMAL),
        ]
        .spacing(style::SPACE_SM)
        .align_y(Alignment::Center),
    )
    .padding([style::SPACE_SM, style::SPACE_XL])
    .on_press(on_add)
    .style(theme::primary_button(cs))
    .width(Length::Fill);

    detail_content = detail_content.push(add_btn);

    super::styled_scrollable(detail_content, cs)
        .height(Length::Fill)
        .into()
}
