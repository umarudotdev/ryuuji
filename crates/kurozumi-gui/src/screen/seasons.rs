use iced::widget::{button, column, container, pick_list, row, rule, text};
use iced::{Alignment, Element, Length};

use kurozumi_api::traits::{AnimeSearchResult, AnimeSeason};

use crate::cover_cache::CoverCache;
use crate::format;
use crate::screen::Action;
use crate::style;
use crate::theme::{self, ColorScheme};
use crate::widgets;

// ── Sort ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SeasonSort {
    #[default]
    Popularity,
    Score,
    Title,
}

impl SeasonSort {
    pub const ALL: &[SeasonSort] = &[Self::Popularity, Self::Score, Self::Title];
}

impl std::fmt::Display for SeasonSort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Popularity => write!(f, "Popularity"),
            Self::Score => write!(f, "Score"),
            Self::Title => write!(f, "A-Z"),
        }
    }
}

// ── State ─────────────────────────────────────────────────────────

pub struct Seasons {
    pub season: AnimeSeason,
    pub year: u32,
    pub entries: Vec<AnimeSearchResult>,
    pub loading: bool,
    pub error: Option<String>,
    pub selected: Option<usize>,
    genre_filter: Option<String>,
    sort: SeasonSort,
    /// Whether any service token is available for browsing.
    pub service_authenticated: bool,
}

// ── Messages ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    SeasonChanged(AnimeSeason),
    YearPrev,
    YearNext,
    DataLoaded(Result<Vec<AnimeSearchResult>, String>),
    AnimeSelected(usize),
    CloseDetail,
    AddToLibrary(usize),
    AddedToLibrary(Result<(), String>),
    GenreFilterChanged(Option<String>),
    SortChanged(SeasonSort),
    Refresh,
}

// ── Implementation ────────────────────────────────────────────────

impl Seasons {
    pub fn new() -> Self {
        Self {
            season: AnimeSeason::current(),
            year: chrono::Datelike::year(&chrono::Utc::now()) as u32,
            entries: Vec::new(),
            loading: false,
            error: None,
            selected: None,
            genre_filter: None,
            sort: SeasonSort::default(),
            service_authenticated: false,
        }
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::SeasonChanged(season) => {
                self.season = season;
                self.entries.clear();
                self.selected = None;
                self.loading = true;
                self.error = None;
                // App will intercept Refresh to spawn the API call.
                Action::None
            }
            Message::YearPrev => {
                self.year = self.year.saturating_sub(1);
                self.entries.clear();
                self.selected = None;
                self.loading = true;
                self.error = None;
                Action::None
            }
            Message::YearNext => {
                self.year = self.year.saturating_add(1);
                self.entries.clear();
                self.selected = None;
                self.loading = true;
                self.error = None;
                Action::None
            }
            Message::DataLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(items) => {
                        self.entries = items;
                        self.error = None;
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
                Action::None
            }
            Message::AnimeSelected(idx) => {
                self.selected = Some(idx);
                Action::None
            }
            Message::CloseDetail => {
                self.selected = None;
                Action::None
            }
            Message::AddToLibrary(_idx) => {
                // Handled by app.rs
                Action::None
            }
            Message::AddedToLibrary(result) => {
                if let Err(e) = result {
                    self.error = Some(e);
                }
                Action::None
            }
            Message::GenreFilterChanged(genre) => {
                self.genre_filter = genre;
                Action::None
            }
            Message::SortChanged(sort) => {
                self.sort = sort;
                Action::None
            }
            Message::Refresh => {
                self.loading = true;
                self.error = None;
                self.entries.clear();
                self.selected = None;
                Action::None
            }
        }
    }

    /// Get filtered and sorted indices into `self.entries`.
    fn filtered_sorted(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                if let Some(genre) = &self.genre_filter {
                    e.genres.iter().any(|g| g == genre)
                } else {
                    true
                }
            })
            .map(|(i, _)| i)
            .collect();

        match self.sort {
            SeasonSort::Popularity => {} // Already sorted by popularity from AniList
            SeasonSort::Score => {
                let entries = &self.entries;
                indices.sort_by(|&a, &b| {
                    let sa = entries[a].mean_score.unwrap_or(0.0);
                    let sb = entries[b].mean_score.unwrap_or(0.0);
                    sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SeasonSort::Title => {
                let entries = &self.entries;
                indices.sort_by(|&a, &b| {
                    entries[a]
                        .title
                        .to_lowercase()
                        .cmp(&entries[b].title.to_lowercase())
                });
            }
        }

        indices
    }

    /// Collect all unique genres from the current entries.
    fn available_genres(&self) -> Vec<String> {
        let mut genres: Vec<String> = self
            .entries
            .iter()
            .flat_map(|e| e.genres.iter().cloned())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        genres.sort();
        genres
    }

    // ── View ──────────────────────────────────────────────────────

    pub fn view<'a>(&'a self, cs: &'a ColorScheme, covers: &'a CoverCache) -> Element<'a, Message> {
        if !self.service_authenticated {
            return self.view_unauthenticated(cs);
        }

        // ── Season / Year picker header ────────────────────────────
        let season_buttons: Vec<Element<'_, Message>> = AnimeSeason::ALL
            .iter()
            .map(|&s| {
                let is_active = self.season == s;
                let label = s.to_string();
                let mut chip_content = row![].spacing(style::SPACE_XXS).align_y(Alignment::Center);
                if is_active {
                    chip_content =
                        chip_content.push(lucide_icons::iced::icon_check().size(style::TEXT_XS));
                }
                chip_content = chip_content.push(
                    text(label)
                        .size(style::TEXT_XS)
                        .line_height(style::LINE_HEIGHT_LOOSE),
                );
                button(container(chip_content).center_y(Length::Fill))
                    .height(Length::Fixed(style::CHIP_HEIGHT))
                    .padding([style::SPACE_XS, style::SPACE_MD])
                    .on_press(Message::SeasonChanged(s))
                    .style(theme::filter_chip(is_active, cs))
                    .into()
            })
            .collect();

        let year_stepper = row![
            button(
                container(
                    lucide_icons::iced::icon_chevron_left()
                        .size(style::TEXT_SM)
                        .color(cs.on_surface_variant),
                )
                .center_x(Length::Fill)
                .center_y(Length::Fill),
            )
            .on_press(Message::YearPrev)
            .padding(0)
            .width(Length::Fixed(28.0))
            .height(Length::Fixed(28.0))
            .style(theme::icon_button(cs)),
            text(self.year.to_string())
                .size(style::TEXT_BASE)
                .font(style::FONT_HEADING)
                .line_height(style::LINE_HEIGHT_NORMAL),
            button(
                container(
                    lucide_icons::iced::icon_chevron_right()
                        .size(style::TEXT_SM)
                        .color(cs.on_surface_variant),
                )
                .center_x(Length::Fill)
                .center_y(Length::Fill),
            )
            .on_press(Message::YearNext)
            .padding(0)
            .width(Length::Fixed(28.0))
            .height(Length::Fixed(28.0))
            .style(theme::icon_button(cs)),
        ]
        .spacing(style::SPACE_SM)
        .align_y(Alignment::Center);

        let header = row![
            row(season_buttons).spacing(style::SPACE_XS),
            container("").width(Length::Fill),
            year_stepper,
        ]
        .spacing(style::SPACE_SM)
        .align_y(Alignment::Center)
        .padding([style::SPACE_SM, style::SPACE_LG]);

        // ── Genre filter + sort bar ────────────────────────────────
        let filtered = self.filtered_sorted();
        let result_count = format!("{} anime", filtered.len());

        let genre_options: Vec<String> = self.available_genres();
        let mut filter_bar = row![].spacing(style::SPACE_SM).align_y(Alignment::Center);

        if !genre_options.is_empty() {
            // Genre filter as a pick list
            let genre_pick = pick_list(genre_options, self.genre_filter.clone(), |g| {
                Message::GenreFilterChanged(Some(g))
            })
            .text_size(style::TEXT_SM)
            .padding([style::SPACE_SM, style::SPACE_MD])
            .placeholder("All genres")
            .style(theme::pick_list_style(cs))
            .menu_style(theme::pick_list_menu_style(cs));
            filter_bar = filter_bar.push(genre_pick);

            // Clear genre filter button
            if self.genre_filter.is_some() {
                let clear_size = style::TEXT_SM + style::SPACE_XS * 2.0;
                let clear_btn = button(
                    container(
                        lucide_icons::iced::icon_x()
                            .size(style::TEXT_SM)
                            .color(cs.on_surface_variant),
                    )
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
                )
                .on_press(Message::GenreFilterChanged(None))
                .padding(0)
                .width(Length::Fixed(clear_size))
                .height(Length::Fixed(clear_size))
                .style(theme::icon_button(cs));
                filter_bar = filter_bar.push(clear_btn);
            }
        }

        filter_bar = filter_bar
            .push(
                text(result_count)
                    .size(style::TEXT_XS)
                    .color(cs.outline)
                    .line_height(style::LINE_HEIGHT_LOOSE)
                    .width(Length::Fill),
            )
            .push(
                pick_list(SeasonSort::ALL, Some(self.sort), Message::SortChanged)
                    .text_size(style::TEXT_SM)
                    .padding([style::SPACE_SM, style::SPACE_MD])
                    .style(theme::pick_list_style(cs))
                    .menu_style(theme::pick_list_menu_style(cs)),
            );

        let filter_bar = container(filter_bar).padding([style::SPACE_XS, style::SPACE_LG]);

        // ── Content area ───────────────────────────────────────────
        let body: Element<'_, Message> = if self.loading {
            container(
                text("Loading...")
                    .size(style::TEXT_SM)
                    .color(cs.on_surface_variant)
                    .line_height(style::LINE_HEIGHT_LOOSE),
            )
            .padding(style::SPACE_3XL)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .into()
        } else if let Some(err) = &self.error {
            container(
                column![
                    text(err.as_str())
                        .size(style::TEXT_SM)
                        .color(cs.error)
                        .line_height(style::LINE_HEIGHT_NORMAL),
                    button(text("Retry").size(style::TEXT_SM))
                        .padding([style::SPACE_SM, style::SPACE_XL])
                        .on_press(Message::Refresh)
                        .style(theme::ghost_button(cs)),
                ]
                .spacing(style::SPACE_MD)
                .align_x(Alignment::Center),
            )
            .padding(style::SPACE_3XL)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .into()
        } else if self.entries.is_empty() {
            container(
                text("No anime found for this season.")
                    .size(style::TEXT_SM)
                    .color(cs.on_surface_variant)
                    .line_height(style::LINE_HEIGHT_LOOSE),
            )
            .padding(style::SPACE_3XL)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .into()
        } else {
            let items: Vec<Element<'a, Message>> = filtered
                .iter()
                .map(|&idx| season_list_item(cs, &self.entries[idx], idx, self.selected, covers))
                .collect();

            crate::widgets::styled_scrollable(
                column(items)
                    .spacing(style::SPACE_XXS)
                    .padding([style::SPACE_XS, style::SPACE_LG]),
                cs,
            )
            .height(Length::Fill)
            .into()
        };

        let content = column![header, filter_bar, rule::horizontal(1), body]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill);

        // Show detail panel for selected anime
        if let Some(idx) = self.selected {
            if let Some(result) = self.entries.get(idx) {
                let cover_key = season_cover_key(result.service_id);
                let detail = widgets::online_detail_panel(
                    cs,
                    result,
                    Message::CloseDetail,
                    Message::AddToLibrary(idx),
                    covers,
                    cover_key,
                );
                return row![
                    container(content).width(Length::FillPortion(3)),
                    rule::vertical(1),
                    container(detail)
                        .width(Length::FillPortion(2))
                        .height(Length::Fill),
                ]
                .height(Length::Fill)
                .into();
            }
        }

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_unauthenticated(&self, cs: &ColorScheme) -> Element<'_, Message> {
        container(
            column![
                lucide_icons::iced::icon_calendar()
                    .size(style::TEXT_3XL)
                    .color(cs.outline)
                    .center(),
                text("Season Browser")
                    .size(style::TEXT_XL)
                    .font(style::FONT_HEADING)
                    .line_height(style::LINE_HEIGHT_TIGHT)
                    .color(cs.on_surface),
                text("Log in to a service in Settings to browse season charts.")
                    .size(style::TEXT_SM)
                    .color(cs.on_surface_variant)
                    .line_height(style::LINE_HEIGHT_LOOSE),
            ]
            .spacing(style::SPACE_MD)
            .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }
}

// ── Helper functions ──────────────────────────────────────────────

/// Cover cache key for season browse results (negative to avoid colliding with local IDs).
pub fn season_cover_key(service_id: u64) -> i64 {
    -(service_id as i64)
}

/// A single season browse result list item.
fn season_list_item<'a>(
    cs: &'a ColorScheme,
    result: &'a AnimeSearchResult,
    idx: usize,
    selected: Option<usize>,
    covers: &'a CoverCache,
) -> Element<'a, Message> {
    let is_selected = selected == Some(idx);
    let cover_key = season_cover_key(result.service_id);

    let thumb = widgets::rounded_cover(
        cs,
        covers,
        cover_key,
        style::THUMB_WIDTH,
        style::THUMB_HEIGHT,
        style::RADIUS_SM,
    );

    // Title + metadata
    let mut info_col = column![text(result.title.as_str())
        .size(style::TEXT_BASE)
        .font(style::FONT_HEADING)
        .line_height(style::LINE_HEIGHT_NORMAL)]
    .spacing(style::SPACE_XXS);

    let mut meta_parts: Vec<String> = Vec::new();
    if let Some(mt) = &result.media_type {
        meta_parts.push(format::media_type(mt));
    }
    if let Some(status) = &result.status {
        meta_parts.push(format::airing_status(status));
    }
    let genre_str: String = result
        .genres
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if !genre_str.is_empty() {
        meta_parts.push(genre_str);
    }
    if !meta_parts.is_empty() {
        info_col = info_col.push(
            text(meta_parts.join("  \u{00B7}  "))
                .size(style::TEXT_XS)
                .color(cs.outline)
                .line_height(style::LINE_HEIGHT_LOOSE),
        );
    }

    // Episode count + score on the right
    let mut right_col = column![].spacing(style::SPACE_XXS).align_x(Alignment::End);
    if let Some(eps) = result.episodes {
        right_col = right_col.push(
            text(format!("{eps} eps"))
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant)
                .line_height(style::LINE_HEIGHT_LOOSE),
        );
    }
    if let Some(score) = result.mean_score {
        right_col = right_col.push(
            text(format!("\u{2605} {score:.1}"))
                .size(style::TEXT_XS)
                .color(cs.primary)
                .line_height(style::LINE_HEIGHT_LOOSE),
        );
    }

    let content = row![thumb, info_col.width(Length::Fill), right_col,]
        .spacing(style::SPACE_SM)
        .align_y(Alignment::Center);

    button(content)
        .width(Length::Fill)
        .padding([style::SPACE_XS, style::SPACE_MD])
        .on_press(Message::AnimeSelected(idx))
        .style(theme::list_item(is_selected, cs))
        .into()
}
