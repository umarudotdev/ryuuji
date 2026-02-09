use iced::widget::{center, column, container, progress_bar, row, text};
use iced::{Alignment, Element, Length};

use kurozumi_core::models::DetectedMedia;
use kurozumi_core::orchestrator::UpdateOutcome;
use kurozumi_core::storage::LibraryRow;

use crate::cover_cache::CoverCache;
use crate::format;
use crate::style;
use crate::theme::{self, ColorScheme};
use crate::widgets;

/// Now Playing screen state.
pub struct NowPlaying {
    pub detected: Option<DetectedMedia>,
    pub last_outcome: Option<UpdateOutcome>,
    pub matched_row: Option<LibraryRow>,
}

/// Messages handled by the Now Playing screen.
#[derive(Debug, Clone)]
pub enum Message {
    LibraryRowFetched(Box<Option<LibraryRow>>),
}

impl NowPlaying {
    pub fn new() -> Self {
        Self {
            detected: None,
            last_outcome: None,
            matched_row: None,
        }
    }

    pub fn view<'a>(&'a self, cs: &ColorScheme, covers: &'a CoverCache) -> Element<'a, Message> {
        let content: Element<'a, Message> = match &self.detected {
            Some(media) => playing_dashboard(cs, media, self.matched_row.as_ref(), covers),
            None => empty_state(cs),
        };

        content
    }
}

// ── Reusable helpers ──────────────────────────────────────────────

/// Label:value row helper for info cards.
fn info_row<'a>(cs: &ColorScheme, label: &'a str, value: String) -> Element<'a, Message> {
    row![
        text(label)
            .size(style::TEXT_SM)
            .color(cs.on_surface_variant)
            .line_height(style::LINE_HEIGHT_LOOSE)
            .width(Length::Fixed(120.0)),
        text(value)
            .size(style::TEXT_SM)
            .line_height(style::LINE_HEIGHT_LOOSE),
    ]
    .spacing(style::SPACE_SM)
    .align_y(Alignment::Center)
    .into()
}

/// Section heading inside a card.
fn section_heading<'a>(cs: &ColorScheme, label: &'a str) -> Element<'a, Message> {
    text(label)
        .size(style::TEXT_SM)
        .font(style::FONT_HEADING)
        .color(cs.on_surface_variant)
        .line_height(style::LINE_HEIGHT_LOOSE)
        .into()
}

/// Render a list of strings as pill-shaped badges with wrapping.
fn badge_row<'a>(cs: &ColorScheme, label: &'a str, items: &'a [String]) -> Element<'a, Message> {
    let badges: Vec<Element<'a, Message>> = items
        .iter()
        .map(|item| {
            container(
                text(item.as_str())
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

    row![
        text(label)
            .size(style::TEXT_SM)
            .color(cs.on_surface_variant)
            .line_height(style::LINE_HEIGHT_LOOSE)
            .width(Length::Fixed(120.0)),
        wrap,
    ]
    .spacing(style::SPACE_SM)
    .into()
}

/// Episode progress bar with label.
fn episode_progress<'a>(
    cs: &ColorScheme,
    current: u32,
    total: Option<u32>,
) -> Element<'a, Message> {
    let (pct, label) = match total {
        Some(t) if t > 0 => {
            let p = (current as f32 / t as f32).clamp(0.0, 1.0);
            (p, format!("{current} / {t}  ({:.0}%)", p * 100.0))
        }
        _ => (0.0, format!("{current} episodes")),
    };

    column![
        text(label)
            .size(style::TEXT_SM)
            .line_height(style::LINE_HEIGHT_LOOSE),
        progress_bar(0.0..=1.0, pct)
            .girth(Length::Fixed(style::PROGRESS_HEIGHT))
            .style(theme::episode_progress(cs)),
    ]
    .spacing(style::SPACE_XS)
    .into()
}

// ── Main dashboard ────────────────────────────────────────────────

/// Multi-card dashboard for active playback.
fn playing_dashboard<'a>(
    cs: &ColorScheme,
    media: &'a DetectedMedia,
    matched_row: Option<&'a LibraryRow>,
    covers: &'a CoverCache,
) -> Element<'a, Message> {
    // ── Now Playing card ────────────────────────────────────────
    let display_title = matched_row
        .map(|r| r.anime.title.preferred().to_string())
        .or_else(|| media.anime_title.clone())
        .unwrap_or_else(|| media.raw_title.clone());

    let episode_text = media
        .episode
        .map(|ep| format!("Episode {ep}"))
        .unwrap_or_default();

    let mut meta_parts: Vec<String> = vec![media.player_name.clone()];
    if let Some(group) = &media.release_group {
        meta_parts.push(group.clone());
    }
    if let Some(res) = &media.resolution {
        meta_parts.push(res.clone());
    }
    let meta_line = meta_parts.join("  \u{00B7}  ");

    // Cover image (rounded corners).
    let cover_element: Element<'_, Message> = if let Some(lib_row) = matched_row {
        widgets::rounded_cover(
            cs,
            covers,
            lib_row.anime.id,
            style::COVER_WIDTH,
            style::COVER_HEIGHT,
            style::RADIUS_LG,
        )
    } else {
        container(text("").size(1)).width(Length::Fixed(0.0)).into()
    };

    let title_block = column![
        text(display_title)
            .size(style::TEXT_3XL)
            .font(style::FONT_HEADING)
            .line_height(style::LINE_HEIGHT_TIGHT),
        text(episode_text)
            .size(style::TEXT_LG)
            .color(cs.primary)
            .line_height(style::LINE_HEIGHT_NORMAL),
        text(meta_line)
            .size(style::TEXT_SM)
            .color(cs.on_surface_variant)
            .line_height(style::LINE_HEIGHT_LOOSE),
    ]
    .spacing(style::SPACE_SM);

    let mut card_content = column![row![cover_element, title_block]
        .spacing(style::SPACE_LG)
        .align_y(Alignment::Start),]
    .spacing(style::SPACE_LG);

    // ── Your Progress (inline in main card) ─────────────────
    if let Some(lib_row) = matched_row {
        let entry = &lib_row.entry;
        let status_color = theme::status_color(cs, entry.status);
        let status_label = format!("{:?}", entry.status);

        let score_text = match entry.score {
            Some(s) if s > 0.0 => format!("{s:.1}"),
            _ => "Not rated".into(),
        };

        let progress_section = column![
            section_heading(cs, "Your Progress"),
            row![
                text("\u{2022}")
                    .size(style::TEXT_SM)
                    .color(status_color)
                    .line_height(style::LINE_HEIGHT_LOOSE),
                text(status_label)
                    .size(style::TEXT_SM)
                    .color(status_color)
                    .line_height(style::LINE_HEIGHT_LOOSE),
            ]
            .spacing(style::SPACE_XS)
            .align_y(Alignment::Center),
            episode_progress(cs, entry.watched_episodes, lib_row.anime.episodes),
            info_row(cs, "Your Score", score_text),
        ]
        .spacing(style::SPACE_SM);

        card_content = card_content.push(progress_section);
    }

    let now_playing_card = container(card_content)
        .style(theme::card(cs))
        .padding(style::SPACE_2XL)
        .width(Length::Fill);

    let mut page = column![now_playing_card]
        .spacing(style::SPACE_LG)
        .padding(style::SPACE_XL);

    // ── Synopsis card (right after the main card) ──────────────
    if let Some(lib_row) = matched_row {
        let anime = &lib_row.anime;

        if let Some(synopsis) = &anime.synopsis {
            if !synopsis.is_empty() {
                let synopsis_card = container(
                    column![
                        section_heading(cs, "Synopsis"),
                        text(synopsis.as_str())
                            .size(style::TEXT_SM)
                            .line_height(style::LINE_HEIGHT_LOOSE),
                    ]
                    .spacing(style::SPACE_SM),
                )
                .style(theme::card(cs))
                .padding(style::SPACE_LG)
                .width(Length::Fill);

                page = page.push(synopsis_card);
            }
        }
    }

    // ── Anime Info card ─────────────────────────────────────────
    if let Some(lib_row) = matched_row {
        let anime = &lib_row.anime;
        let mut info_rows: Vec<Element<'_, Message>> = Vec::new();

        if let Some(english) = &anime.title.english {
            info_rows.push(info_row(cs, "English", english.clone()));
        }
        if let Some(native) = &anime.title.native {
            info_rows.push(info_row(cs, "Native", native.clone()));
        }
        if let Some(mt) = &anime.media_type {
            info_rows.push(info_row(cs, "Type", format::media_type(mt)));
        }
        if let Some(as_) = &anime.airing_status {
            info_rows.push(info_row(cs, "Status", format::airing_status(as_)));
        }
        if anime.season.is_some() || anime.year.is_some() {
            let mut parts = Vec::new();
            if let Some(season) = &anime.season {
                parts.push(season.clone());
            }
            if let Some(year) = anime.year {
                parts.push(year.to_string());
            }
            info_rows.push(info_row(cs, "Season", parts.join(" ")));
        }
        if let Some(total) = anime.episodes {
            info_rows.push(info_row(cs, "Episodes", total.to_string()));
        }
        if let Some(score) = anime.mean_score {
            info_rows.push(info_row(cs, "Score", format!("\u{2605} {score:.2}")));
        }
        if !anime.genres.is_empty() {
            info_rows.push(badge_row(cs, "Genres", &anime.genres));
        }
        if !anime.studios.is_empty() {
            info_rows.push(badge_row(cs, "Studios", &anime.studios));
        }
        if let Some(src) = &anime.source {
            info_rows.push(info_row(cs, "Source", format::source(src)));
        }
        if let Some(rating) = &anime.rating {
            info_rows.push(info_row(cs, "Rating", format::rating(rating)));
        }
        if anime.start_date.is_some() || anime.end_date.is_some() {
            let aired = format!(
                "{} \u{2013} {}",
                anime.start_date.as_deref().unwrap_or("?"),
                anime.end_date.as_deref().unwrap_or("?"),
            );
            info_rows.push(info_row(cs, "Aired", aired));
        }
        if let Some(mal_id) = anime.ids.mal {
            info_rows.push(info_row(cs, "MAL ID", mal_id.to_string()));
        }

        if !info_rows.is_empty() {
            let mut info_col = column![section_heading(cs, "Anime Info")].spacing(style::SPACE_SM);

            for r in info_rows {
                info_col = info_col.push(r);
            }

            let anime_info_card = container(info_col)
                .style(theme::card(cs))
                .padding(style::SPACE_LG)
                .width(Length::Fill);

            page = page.push(anime_info_card);
        }
    }

    crate::widgets::styled_scrollable(page, cs)
        .height(Length::Fill)
        .into()
}

/// Empty state when nothing is playing.
fn empty_state<'a>(cs: &ColorScheme) -> Element<'a, Message> {
    let content = column![
        lucide_icons::iced::icon_play().size(56.0).color(cs.outline),
        text("Nothing playing")
            .size(style::TEXT_XL)
            .font(style::FONT_HEADING)
            .color(cs.on_surface_variant)
            .line_height(style::LINE_HEIGHT_TIGHT),
        text("Start a media player with an anime file to see it here.")
            .size(style::TEXT_SM)
            .color(cs.outline)
            .line_height(style::LINE_HEIGHT_LOOSE),
    ]
    .spacing(style::SPACE_MD)
    .align_x(Alignment::Center);

    center(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
