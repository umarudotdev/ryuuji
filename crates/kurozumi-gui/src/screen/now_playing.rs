use iced::widget::{center, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use kurozumi_core::models::DetectedMedia;
use kurozumi_core::orchestrator::UpdateOutcome;
use kurozumi_core::storage::LibraryRow;

use crate::style;
use crate::theme::{self, ColorScheme};

/// Now Playing screen state.
pub struct NowPlaying {
    pub detected: Option<DetectedMedia>,
    pub last_outcome: Option<UpdateOutcome>,
    pub matched_row: Option<LibraryRow>,
}

/// Messages handled by the Now Playing screen.
#[derive(Debug, Clone)]
pub enum Message {
    LibraryRowFetched(Option<LibraryRow>),
}

impl NowPlaying {
    pub fn new() -> Self {
        Self {
            detected: None,
            last_outcome: None,
            matched_row: None,
        }
    }

    pub fn view<'a>(&'a self, cs: &ColorScheme, status: &'a str) -> Element<'a, Message> {
        let content: Element<'a, Message> = match &self.detected {
            Some(media) => playing_dashboard(cs, media, self.matched_row.as_ref(), status),
            None => empty_state(cs),
        };

        container(content)
            .padding(style::SPACE_XL)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

/// Label:value row helper for info cards.
fn info_row<'a>(cs: &ColorScheme, label: &'a str, value: String) -> Element<'a, Message> {
    row![
        text(label)
            .size(style::TEXT_SM)
            .color(cs.on_surface_variant)
            .width(Length::Fixed(120.0)),
        text(value).size(style::TEXT_SM),
    ]
    .spacing(style::SPACE_SM)
    .align_y(Alignment::Center)
    .into()
}

/// Multi-card dashboard for active playback.
fn playing_dashboard<'a>(
    cs: &ColorScheme,
    media: &'a DetectedMedia,
    matched_row: Option<&'a LibraryRow>,
    status: &'a str,
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

    let now_playing_card = container(
        column![
            text(display_title).size(style::TEXT_3XL),
            text(episode_text).size(style::TEXT_LG).color(cs.primary),
            text(meta_line)
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant),
        ]
        .spacing(style::SPACE_SM),
    )
    .style(theme::card(cs))
    .padding(style::SPACE_2XL)
    .width(Length::Fill);

    let mut page = column![now_playing_card].spacing(style::SPACE_LG);

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
        if let Some(mal_id) = anime.ids.mal {
            info_rows.push(info_row(cs, "MAL ID", mal_id.to_string()));
        }

        if !info_rows.is_empty() {
            let mut info_col = column![text("Anime Info")
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant),]
            .spacing(style::SPACE_SM);

            for r in info_rows {
                info_col = info_col.push(r);
            }

            let anime_info_card = container(info_col)
                .style(theme::card(cs))
                .padding(style::SPACE_LG)
                .width(Length::Fill);

            page = page.push(anime_info_card);
        }

        // ── Library Progress card ───────────────────────────────
        let entry = &lib_row.entry;
        let status_color = theme::status_color(cs, entry.status);
        let status_label = format!("{:?}", entry.status);

        let ep_text = match anime.episodes {
            Some(total) => format!("{} / {}", entry.watched_episodes, total),
            None => format!("{}", entry.watched_episodes),
        };

        let score_text = match entry.score {
            Some(s) if s > 0.0 => format!("{s:.1}"),
            _ => "Not rated".into(),
        };

        let progress_card = container(
            column![
                text("Library Progress")
                    .size(style::TEXT_SM)
                    .color(cs.on_surface_variant),
                row![
                    text("\u{2022}").size(style::TEXT_SM).color(status_color),
                    text(status_label).size(style::TEXT_SM).color(status_color),
                ]
                .spacing(style::SPACE_XS)
                .align_y(Alignment::Center),
                info_row(cs, "Episode", ep_text),
                info_row(cs, "Score", score_text),
            ]
            .spacing(style::SPACE_SM),
        )
        .style(theme::card(cs))
        .padding(style::SPACE_LG)
        .width(Length::Fill);

        page = page.push(progress_card);
    }

    // ── Status card ─────────────────────────────────────────────
    if !status.is_empty() {
        let dot_color = if status.starts_with("Error") {
            cs.error
        } else if status.starts_with("Updated") || status.starts_with("Added") {
            cs.status_completed
        } else {
            cs.on_surface_variant
        };

        let status_card = container(
            row![
                text("\u{2022}").size(style::TEXT_SM).color(dot_color),
                text(status).size(style::TEXT_SM).color(dot_color),
            ]
            .spacing(style::SPACE_SM)
            .align_y(Alignment::Center),
        )
        .style(theme::card(cs))
        .padding([style::SPACE_SM, style::SPACE_LG])
        .width(Length::Fill);

        page = page.push(status_card);
    }

    scrollable(page).height(Length::Fill).into()
}

/// Empty state when nothing is playing.
fn empty_state<'a>(cs: &ColorScheme) -> Element<'a, Message> {
    let content = column![
        lucide_icons::iced::icon_play().size(56.0).color(cs.outline),
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
