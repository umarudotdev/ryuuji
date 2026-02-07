use iced::widget::{column, container, progress_bar, row, text};
use iced::{Alignment, Element, Length};

use ryuuji_core::models::DetectedMedia;
use ryuuji_core::orchestrator::UpdateOutcome;
use ryuuji_core::storage::LibraryRow;

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
    pub episode_input: String,
}

/// Messages handled by the Now Playing screen.
#[derive(Debug, Clone)]
pub enum Message {
    LibraryRowFetched(Box<Option<LibraryRow>>),
    EpisodeChanged(i64, u32),
    EpisodeInputChanged(String),
    EpisodeInputSubmitted,
}

impl NowPlaying {
    pub fn new() -> Self {
        Self {
            detected: None,
            last_outcome: None,
            matched_row: None,
            episode_input: String::new(),
        }
    }

    pub fn view<'a>(&'a self, cs: &ColorScheme, covers: &'a CoverCache) -> Element<'a, Message> {
        match &self.detected {
            Some(media) => playing_dashboard(
                cs,
                media,
                self.matched_row.as_ref(),
                covers,
                &self.episode_input,
            ),
            None => np_empty_state(cs),
        }
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
            .width(Length::Fixed(style::INPUT_LABEL_WIDTH)),
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
        .size(style::TEXT_XS)
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
            .width(Length::Fixed(style::INPUT_LABEL_WIDTH)),
        wrap,
    ]
    .spacing(style::SPACE_SM)
    .into()
}

// ── Main dashboard ────────────────────────────────────────────────

/// Hero-style dashboard for active playback.
fn playing_dashboard<'a>(
    cs: &ColorScheme,
    media: &'a DetectedMedia,
    matched_row: Option<&'a LibraryRow>,
    covers: &'a CoverCache,
    episode_input: &str,
) -> Element<'a, Message> {
    // ── Title & metadata ────────────────────────────────────────
    let display_title = matched_row
        .map(|r| r.anime.title.preferred().to_string())
        .or_else(|| media.anime_title.clone())
        .unwrap_or_else(|| media.raw_title.clone());

    // Cover image — hero size.
    let cover_element: Element<'_, Message> = if let Some(lib_row) = matched_row {
        widgets::rounded_cover(
            cs,
            covers,
            lib_row.anime.id,
            style::HERO_COVER_WIDTH,
            style::HERO_COVER_HEIGHT,
            style::RADIUS_LG,
        )
    } else {
        container(text("").size(1)).width(Length::Fixed(0.0)).into()
    };

    // ── Right side: title + episode progress + stepper ────────
    let mut title_block = column![text(display_title)
        .size(style::TEXT_2XL)
        .font(style::FONT_HEADING)
        .line_height(style::LINE_HEIGHT_TIGHT),]
    .spacing(style::SPACE_SM);

    // English subtitle
    if let Some(lib_row) = matched_row {
        if let Some(english) = &lib_row.anime.title.english {
            if Some(english.as_str()) != lib_row.anime.title.romaji.as_deref() {
                title_block = title_block.push(
                    text(english.as_str())
                        .size(style::TEXT_SM)
                        .color(cs.on_surface_variant)
                        .line_height(style::LINE_HEIGHT_LOOSE),
                );
            }
        }
    }

    // Episode progress bar
    if let Some(lib_row) = matched_row {
        let entry = &lib_row.entry;
        let (pct, ep_label) = match lib_row.anime.episodes {
            Some(t) if t > 0 => {
                let p = (entry.watched_episodes as f32 / t as f32).clamp(0.0, 1.0);
                (
                    p,
                    format!("Ep {} / {}  ({:.0}%)", entry.watched_episodes, t, p * 100.0),
                )
            }
            _ => (0.0, format!("Ep {}", entry.watched_episodes)),
        };

        title_block = title_block.push(
            column![
                text(ep_label)
                    .size(style::TEXT_SM)
                    .line_height(style::LINE_HEIGHT_LOOSE),
                progress_bar(0.0..=1.0, pct)
                    .girth(Length::Fixed(style::PROGRESS_HEIGHT))
                    .style(theme::episode_progress(cs)),
            ]
            .spacing(style::SPACE_XS),
        );

        // Inline episode stepper
        let anime_id = lib_row.anime.id;
        let max_ep = lib_row.anime.episodes.unwrap_or(u32::MAX);

        let ep_dec = if entry.watched_episodes > 0 {
            Some(Message::EpisodeChanged(
                anime_id,
                entry.watched_episodes - 1,
            ))
        } else {
            None
        };
        let ep_inc = if entry.watched_episodes < max_ep {
            Some(Message::EpisodeChanged(
                anime_id,
                entry.watched_episodes + 1,
            ))
        } else {
            None
        };

        title_block = title_block.push(widgets::stepper(
            cs,
            episode_input,
            Message::EpisodeInputChanged,
            Message::EpisodeInputSubmitted,
            ep_dec,
            ep_inc,
        ));
    } else {
        // Show episode from detection for unmatched media
        let episode_text = media
            .episode
            .map(|ep| format!("Episode {ep}"))
            .unwrap_or_default();
        if !episode_text.is_empty() {
            title_block = title_block.push(
                text(episode_text)
                    .size(style::TEXT_LG)
                    .color(cs.primary)
                    .line_height(style::LINE_HEIGHT_NORMAL),
            );
        }
    }

    // Player info line
    let player_label = if let Some(service) = &media.service_name {
        format!("{} \u{00B7} {service}", media.player_name)
    } else {
        media.player_name.clone()
    };
    let mut meta_parts: Vec<String> = vec![player_label];
    if let Some(group) = &media.release_group {
        meta_parts.push(group.clone());
    }
    if let Some(res) = &media.resolution {
        meta_parts.push(res.clone());
    }
    let meta_line = meta_parts.join("  \u{00B7}  ");

    title_block = title_block.push(
        text(meta_line)
            .size(style::TEXT_XS)
            .color(cs.outline)
            .line_height(style::LINE_HEIGHT_LOOSE),
    );

    let hero_row = row![cover_element, title_block.width(Length::Fill)]
        .spacing(style::SPACE_XL)
        .align_y(Alignment::Start);

    let hero_card = container(hero_row)
        .style(theme::card(cs))
        .padding(style::SPACE_2XL)
        .width(Length::Fill);

    let mut page = column![hero_card]
        .spacing(style::SPACE_LG)
        .padding(style::SPACE_XL);

    // ── Synopsis card ──────────────────────────────────────────
    if let Some(lib_row) = matched_row {
        if let Some(synopsis) = &lib_row.anime.synopsis {
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

    // ── Two-column info cards ──────────────────────────────────
    if let Some(lib_row) = matched_row {
        let anime = &lib_row.anime;
        let entry = &lib_row.entry;

        // Left column: Anime Info
        let mut info_rows: Vec<Element<'_, Message>> = Vec::new();
        if let Some(mt) = &anime.media_type {
            info_rows.push(info_row(cs, "Type", format::media_type(mt)));
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
        if !anime.studios.is_empty() {
            info_rows.push(badge_row(cs, "Studios", &anime.studios));
        }
        if let Some(score) = anime.mean_score {
            info_rows.push(info_row(cs, "Score", format!("\u{2605} {score:.2}")));
        }
        if !anime.genres.is_empty() {
            info_rows.push(badge_row(cs, "Genres", &anime.genres));
        }
        if let Some(as_) = &anime.airing_status {
            info_rows.push(info_row(cs, "Status", format::airing_status(as_)));
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

        // Right column: Your Entry
        let status_color = theme::status_color(cs, entry.status);
        let status_label = entry.status.to_string();
        let score_text = match entry.score {
            Some(s) if s > 0.0 => format!("{s:.1}"),
            _ => "\u{2014}".into(),
        };
        let start = entry
            .start_date
            .as_deref()
            .unwrap_or("\u{2014}")
            .to_string();
        let finish = entry
            .finish_date
            .as_deref()
            .unwrap_or("\u{2014}")
            .to_string();

        let mut entry_col = column![
            section_heading(cs, "Your Entry"),
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
            info_row(cs, "Score", score_text),
            info_row(cs, "Started", start),
            info_row(cs, "Finished", finish),
        ]
        .spacing(style::SPACE_SM);

        if entry.rewatching {
            entry_col = entry_col.push(info_row(cs, "Rewatch #", entry.rewatch_count.to_string()));
        }

        let entry_card = container(entry_col)
            .style(theme::card(cs))
            .padding(style::SPACE_LG)
            .width(Length::FillPortion(1));

        if !info_rows.is_empty() {
            let mut info_col = column![section_heading(cs, "Anime Info")].spacing(style::SPACE_SM);
            for r in info_rows {
                info_col = info_col.push(r);
            }
            let info_card = container(info_col)
                .style(theme::card(cs))
                .padding(style::SPACE_LG)
                .width(Length::FillPortion(1));

            page = page.push(
                row![info_card, entry_card]
                    .spacing(style::SPACE_LG)
                    .width(Length::Fill),
            );
        } else {
            page = page.push(entry_card);
        }
    }

    crate::widgets::styled_scrollable(page, cs)
        .height(Length::Fill)
        .into()
}

/// Enhanced empty state when nothing is playing.
fn np_empty_state<'a>(cs: &ColorScheme) -> Element<'a, Message> {
    let icon = lucide_icons::iced::icon_circle_play()
        .size(64.0)
        .color(cs.outline)
        .into();
    widgets::empty_state(
        cs,
        icon,
        "Nothing playing",
        "Start a media player and Ryuuji will automatically detect it",
    )
}
