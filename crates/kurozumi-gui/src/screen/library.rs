use iced::widget::{
    button, column, container, pick_list, row, rule, scrollable, text, text_input,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Task, Theme};

use kurozumi_core::models::WatchStatus;
use kurozumi_core::storage::LibraryRow;

use crate::app;
use crate::db::DbHandle;
use crate::screen::{Action, ModalKind};
use crate::style;
use crate::theme::{self, ColorScheme};
use crate::widgets::context_menu::context_menu;

/// Sort mode for library list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibrarySort {
    #[default]
    Alphabetical,
    RecentlyUpdated,
}

impl LibrarySort {
    pub const ALL: &[LibrarySort] = &[Self::Alphabetical, Self::RecentlyUpdated];
}

impl std::fmt::Display for LibrarySort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Alphabetical => write!(f, "A-Z"),
            Self::RecentlyUpdated => write!(f, "Recent"),
        }
    }
}

/// Library view mode: grid (cover cards) or list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibraryViewMode {
    #[default]
    Grid,
    List,
}

/// Actions available in the library context menu.
#[derive(Debug, Clone)]
pub enum ContextAction {
    ChangeStatus(WatchStatus),
    Delete,
}

/// Library screen state.
pub struct Library {
    pub tab: WatchStatus,
    pub entries: Vec<LibraryRow>,
    pub selected_anime: Option<i64>,
    pub sort: LibrarySort,
    pub view_mode: LibraryViewMode,
    pub score_input: String,
}

/// Messages handled by the Library screen.
#[derive(Debug, Clone)]
pub enum Message {
    TabChanged(WatchStatus),
    AnimeSelected(i64),
    EpisodeIncrement(i64),
    EpisodeDecrement(i64),
    StatusChanged(i64, WatchStatus),
    ScoreInputChanged(String),
    ScoreSubmitted(i64),
    SortChanged(LibrarySort),
    ViewModeToggled,
    ContextAction(i64, ContextAction),
    ConfirmDelete(i64),
    CancelModal,
    // Async result messages (errors stringified for Clone)
    LibraryRefreshed(Result<Vec<LibraryRow>, String>),
    DbOperationDone(Result<(), String>),
}

impl Library {
    pub fn new() -> Self {
        Self {
            tab: WatchStatus::Watching,
            entries: Vec::new(),
            selected_anime: None,
            sort: LibrarySort::default(),
            view_mode: LibraryViewMode::default(),
            score_input: String::new(),
        }
    }

    /// Handle a library message, returning an Action for the app router.
    pub fn update(&mut self, msg: Message, db: Option<&DbHandle>) -> Action {
        match msg {
            Message::TabChanged(status) => {
                self.tab = status;
                self.selected_anime = None;
                self.refresh_task(db)
            }
            Message::AnimeSelected(id) => {
                self.selected_anime = Some(id);
                if let Some(row) = self.entries.iter().find(|r| r.anime.id == id) {
                    self.score_input = row
                        .entry
                        .score
                        .map(|s| format!("{s:.0}"))
                        .unwrap_or_default();
                }
                Action::None
            }
            Message::EpisodeIncrement(anime_id) => {
                if let Some(db) = db {
                    if let Some(entry) = self.entries.iter().find(|r| r.anime.id == anime_id) {
                        let new_ep = entry.entry.watched_episodes + 1;
                        let db = db.clone();
                        return Action::RunTask(Task::perform(
                            async move {
                                let _ = db.update_episode_count(anime_id, new_ep).await;
                                let _ = db.record_watch(anime_id, new_ep).await;
                            },
                            |_| app::Message::Library(Message::DbOperationDone(Ok(()))),
                        ));
                    }
                }
                Action::None
            }
            Message::EpisodeDecrement(anime_id) => {
                if let Some(db) = db {
                    if let Some(entry) = self.entries.iter().find(|r| r.anime.id == anime_id) {
                        if entry.entry.watched_episodes > 0 {
                            let new_ep = entry.entry.watched_episodes - 1;
                            let db = db.clone();
                            return Action::RunTask(Task::perform(
                                async move {
                                    let _ = db.update_episode_count(anime_id, new_ep).await;
                                },
                                |_| app::Message::Library(Message::DbOperationDone(Ok(()))),
                            ));
                        }
                    }
                }
                Action::None
            }
            Message::StatusChanged(anime_id, new_status) => {
                if let Some(db) = db {
                    let db = db.clone();
                    return Action::RunTask(Task::perform(
                        async move { db.update_library_status(anime_id, new_status).await },
                        |r| app::Message::Library(Message::DbOperationDone(r.map_err(|e| e.to_string()))),
                    ));
                }
                Action::None
            }
            Message::ScoreInputChanged(val) => {
                self.score_input = val;
                Action::None
            }
            Message::ScoreSubmitted(anime_id) => {
                if let Some(db) = db {
                    if let Ok(score) = self.score_input.parse::<f32>() {
                        let score = score.clamp(0.0, 10.0);
                        let db = db.clone();
                        return Action::RunTask(Task::perform(
                            async move { db.update_library_score(anime_id, score).await },
                            |r| app::Message::Library(Message::DbOperationDone(r.map_err(|e| e.to_string()))),
                        ));
                    }
                }
                Action::None
            }
            Message::SortChanged(sort) => {
                self.sort = sort;
                self.refresh_task(db)
            }
            Message::ViewModeToggled => {
                self.view_mode = match self.view_mode {
                    LibraryViewMode::Grid => LibraryViewMode::List,
                    LibraryViewMode::List => LibraryViewMode::Grid,
                };
                Action::None
            }
            Message::ContextAction(anime_id, action) => match action {
                ContextAction::ChangeStatus(new_status) => {
                    if let Some(db) = db {
                        let db = db.clone();
                        return Action::RunTask(Task::perform(
                            async move { db.update_library_status(anime_id, new_status).await },
                            |r| app::Message::Library(Message::DbOperationDone(r.map_err(|e| e.to_string()))),
                        ));
                    }
                    Action::None
                }
                ContextAction::Delete => {
                    let title = self
                        .entries
                        .iter()
                        .find(|r| r.anime.id == anime_id)
                        .map(|r| r.anime.title.preferred().to_string())
                        .unwrap_or_else(|| "this anime".into());
                    Action::ShowModal(ModalKind::ConfirmDelete { anime_id, title })
                }
            },
            Message::ConfirmDelete(anime_id) => {
                if let Some(db) = db {
                    if self.selected_anime == Some(anime_id) {
                        self.selected_anime = None;
                    }
                    let db = db.clone();
                    return Action::RunTask(Task::perform(
                        async move { db.delete_library_entry(anime_id).await },
                        |r| app::Message::Library(Message::DbOperationDone(r.map_err(|e| e.to_string()))),
                    ));
                }
                Action::None
            }
            Message::CancelModal => Action::DismissModal,
            Message::LibraryRefreshed(result) => {
                if let Ok(mut entries) = result {
                    self.sort_entries(&mut entries);
                    self.entries = entries;
                }
                Action::None
            }
            Message::DbOperationDone(_result) => {
                // After any DB write, refresh the library.
                self.refresh_task(db)
            }
        }
    }

    /// Build a task that fetches fresh library entries from the DB.
    pub fn refresh_task(&self, db: Option<&DbHandle>) -> Action {
        if let Some(db) = db {
            let db = db.clone();
            let tab = self.tab;
            Action::RunTask(Task::perform(
                async move { db.get_library_by_status(tab).await },
                |r| app::Message::Library(Message::LibraryRefreshed(r.map_err(|e| e.to_string()))),
            ))
        } else {
            Action::None
        }
    }

    fn sort_entries(&self, entries: &mut Vec<LibraryRow>) {
        match self.sort {
            LibrarySort::Alphabetical => {
                entries.sort_by(|a, b| {
                    a.anime
                        .title
                        .preferred()
                        .to_lowercase()
                        .cmp(&b.anime.title.preferred().to_lowercase())
                });
            }
            LibrarySort::RecentlyUpdated => {
                entries.sort_by(|a, b| b.entry.updated_at.cmp(&a.entry.updated_at));
            }
        }
    }

    pub fn view<'a>(&'a self, cs: &'a ColorScheme) -> Element<'a, Message> {
        let count_text = format!(
            "{} {}",
            self.entries.len(),
            if self.entries.len() == 1 { "entry" } else { "entries" }
        );

        let view_icon = match self.view_mode {
            LibraryViewMode::Grid => lucide_icons::iced::icon_list(),
            LibraryViewMode::List => lucide_icons::iced::icon_layout_grid(),
        };

        let header = row![
            chip_bar(cs, self.tab),
            text(count_text)
                .size(style::TEXT_XS)
                .color(cs.outline)
                .width(Length::Fill),
            button(view_icon.size(style::TEXT_BASE))
                .padding([style::SPACE_XS, style::SPACE_SM])
                .on_press(Message::ViewModeToggled)
                .style(theme::ghost_button(cs)),
            pick_list(LibrarySort::ALL, Some(self.sort), |s| {
                Message::SortChanged(s)
            })
            .text_size(style::TEXT_SM)
            .padding([style::SPACE_XS, style::SPACE_SM]),
        ]
        .spacing(style::SPACE_SM)
        .align_y(Alignment::Center)
        .padding([style::SPACE_SM, style::SPACE_LG]);

        let list: Element<'_, Message> = if self.entries.is_empty() {
            container(
                column![
                    text("No anime in this list.")
                        .size(style::TEXT_SM)
                        .color(cs.on_surface_variant),
                ]
                .align_x(Alignment::Center),
            )
            .padding(style::SPACE_3XL)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .into()
        } else {
            match self.view_mode {
                LibraryViewMode::List => {
                    let items: Vec<Element<'a, Message>> = self.entries
                        .iter()
                        .map(|r| anime_list_item(cs, r, self.selected_anime))
                        .collect();

                    scrollable(
                        column(items)
                            .spacing(style::SPACE_XXS)
                            .padding([style::SPACE_XS, style::SPACE_LG]),
                    )
                    .height(Length::Fill)
                    .into()
                }
                LibraryViewMode::Grid => {
                    let mut cards: Vec<Element<'a, Message>> = self.entries
                        .iter()
                        .map(|r| grid_card(cs, r, self.selected_anime))
                        .collect();

                    let mut grid_rows: Vec<Element<'a, Message>> = Vec::new();
                    let mut drain = cards.drain(..);
                    loop {
                        let mut grid_row = row![].spacing(style::SPACE_MD);
                        let mut count = 0;
                        for _ in 0..style::GRID_COLUMNS {
                            if let Some(card) = drain.next() {
                                grid_row = grid_row.push(card);
                                count += 1;
                            }
                        }
                        if count == 0 {
                            break;
                        }
                        for _ in count..style::GRID_COLUMNS {
                            grid_row = grid_row.push(
                                container(text(""))
                                    .width(Length::Fixed(style::GRID_CARD_WIDTH)),
                            );
                        }
                        grid_rows.push(grid_row.into());
                    }

                    scrollable(
                        column(grid_rows)
                            .spacing(style::SPACE_MD)
                            .padding([style::SPACE_SM, style::SPACE_LG]),
                    )
                    .height(Length::Fill)
                    .into()
                }
            }
        };

        let content = column![header, rule::horizontal(1), list]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill);

        if let Some(anime_id) = self.selected_anime {
            if let Some(lib_row) = self.entries.iter().find(|r| r.anime.id == anime_id) {
                let detail = anime_detail(cs, lib_row, &self.score_input);
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
}

/// Filter chip bar for watch status filtering.
fn chip_bar(cs: &ColorScheme, active: WatchStatus) -> Element<'static, Message> {
    let chips: Vec<Element<'_, Message>> = WatchStatus::ALL
        .iter()
        .map(|&status| {
            let is_selected = status == active;
            let base_label = match status {
                WatchStatus::PlanToWatch => "Plan".to_string(),
                other => other.as_str().to_string(),
            };
            let label = if is_selected {
                format!("\u{2713} {base_label}")
            } else {
                base_label
            };

            button(
                text(label)
                    .size(style::TEXT_XS)
                    .center(),
            )
            .height(Length::Fixed(style::CHIP_HEIGHT))
            .padding([style::SPACE_XS, style::SPACE_MD])
            .on_press(Message::TabChanged(status))
            .style(theme::filter_chip(is_selected, cs))
            .into()
        })
        .collect();

    row(chips).spacing(style::SPACE_XS).into()
}

/// A single anime grid card.
fn grid_card<'a>(
    cs: &ColorScheme,
    lib_row: &'a LibraryRow,
    selected: Option<i64>,
) -> Element<'a, Message> {
    let title = lib_row.anime.title.preferred();
    let progress = match lib_row.anime.episodes {
        Some(total) => format!("{} / {}", lib_row.entry.watched_episodes, total),
        None => format!("{}", lib_row.entry.watched_episodes),
    };
    let is_selected = selected == Some(lib_row.anime.id);
    let anime_id = lib_row.anime.id;
    let status_col = theme::status_color(cs, lib_row.entry.status);

    let status_bar = container(text("").size(1))
        .width(Length::Fill)
        .height(Length::Fixed(3.0))
        .style(theme::status_bar_accent(status_col));

    let cover = container(
        text("\u{1F3AC}")
            .size(style::TEXT_3XL)
            .color(cs.outline)
            .center(),
    )
    .width(Length::Fill)
    .height(Length::Fixed(style::GRID_COVER_HEIGHT))
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .style(theme::grid_cover_placeholder(cs));

    let info = column![
        text(title)
            .size(style::TEXT_SM)
            .width(Length::Fill),
        text(progress)
            .size(style::TEXT_XS)
            .color(cs.on_surface_variant),
    ]
    .spacing(style::SPACE_XXS)
    .padding([style::SPACE_SM, style::SPACE_SM]);

    let card_content = column![status_bar, cover, info];

    button(card_content)
        .width(Length::Fixed(style::GRID_CARD_WIDTH))
        .padding(0)
        .on_press(Message::AnimeSelected(anime_id))
        .style(theme::grid_card(is_selected, cs))
        .into()
}

/// A single anime list item with context menu.
fn anime_list_item<'a>(
    cs: &'a ColorScheme,
    lib_row: &'a LibraryRow,
    selected: Option<i64>,
) -> Element<'a, Message> {
    let title = lib_row.anime.title.preferred();
    let progress = match lib_row.anime.episodes {
        Some(total) => format!("{} / {}", lib_row.entry.watched_episodes, total),
        None => format!("{}", lib_row.entry.watched_episodes),
    };

    let is_selected = selected == Some(lib_row.anime.id);
    let anime_id = lib_row.anime.id;
    let status_col = theme::status_color(cs, lib_row.entry.status);

    let status_bar = container(text("").size(1))
        .width(Length::Fixed(3.0))
        .height(Length::Fill)
        .style(theme::status_bar_accent(status_col));

    let content = row![
        status_bar,
        text(title).size(style::TEXT_BASE).width(Length::Fill),
        text(progress).size(style::TEXT_SM).color(cs.on_surface_variant),
    ]
    .spacing(style::SPACE_SM)
    .align_y(Alignment::Center);

    let base = button(content)
        .width(Length::Fill)
        .padding([style::SPACE_SM, style::SPACE_MD])
        .on_press(Message::AnimeSelected(anime_id))
        .style(theme::list_item(is_selected, cs));

    let primary = cs.primary;
    let on_primary = cs.on_primary;
    let on_surface = cs.on_surface;
    let error = cs.error;
    let on_error = cs.on_error;
    let menu_bg = cs.surface_container_high;
    let menu_border = cs.outline;
    let menu_item = move |label: &'a str, msg: Message| -> Element<'a, Message> {
        button(text(label).size(style::TEXT_SM))
            .width(Length::Fill)
            .padding([style::SPACE_XS, style::SPACE_MD])
            .on_press(msg)
            .style(move |_theme: &Theme, status| {
                let (bg, tc) = match status {
                    button::Status::Hovered => (Some(Background::Color(primary)), on_primary),
                    _ => (None, on_surface),
                };
                button::Style {
                    background: bg,
                    text_color: tc,
                    border: Border {
                        radius: style::RADIUS_SM.into(),
                        ..Border::default()
                    },
                    ..Default::default()
                }
            })
            .into()
    };

    context_menu(base, move || {
        container(
            column![
                menu_item(
                    "Watching",
                    Message::ContextAction(
                        anime_id,
                        ContextAction::ChangeStatus(WatchStatus::Watching),
                    ),
                ),
                menu_item(
                    "Completed",
                    Message::ContextAction(
                        anime_id,
                        ContextAction::ChangeStatus(WatchStatus::Completed),
                    ),
                ),
                menu_item(
                    "On Hold",
                    Message::ContextAction(
                        anime_id,
                        ContextAction::ChangeStatus(WatchStatus::OnHold),
                    ),
                ),
                menu_item(
                    "Dropped",
                    Message::ContextAction(
                        anime_id,
                        ContextAction::ChangeStatus(WatchStatus::Dropped),
                    ),
                ),
                menu_item(
                    "Plan to Watch",
                    Message::ContextAction(
                        anime_id,
                        ContextAction::ChangeStatus(WatchStatus::PlanToWatch),
                    ),
                ),
                rule::horizontal(1),
                button(text("Delete").size(style::TEXT_SM))
                    .width(Length::Fill)
                    .padding([style::SPACE_XS, style::SPACE_MD])
                    .on_press(Message::ContextAction(
                        anime_id,
                        ContextAction::Delete,
                    ))
                    .style(move |_theme: &Theme, status| {
                        let (bg, tc) = match status {
                            button::Status::Hovered => (Some(Background::Color(error)), on_error),
                            _ => (None, error),
                        };
                        button::Style {
                            background: bg,
                            text_color: tc,
                            border: Border {
                                radius: style::RADIUS_SM.into(),
                                ..Border::default()
                            },
                            ..Default::default()
                        }
                    }),
            ]
            .spacing(style::SPACE_XXS)
            .width(Length::Fixed(160.0)),
        )
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(menu_bg)),
            border: Border {
                color: menu_border,
                width: 1.0,
                radius: style::RADIUS_MD.into(),
            },
            shadow: iced::Shadow {
                color: Color::BLACK,
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 16.0,
            },
            ..Default::default()
        })
        .padding(style::SPACE_XS)
        .into()
    })
}

/// Detail panel for the selected anime.
fn anime_detail<'a>(cs: &ColorScheme, lib_row: &'a LibraryRow, score_input: &str) -> Element<'a, Message> {
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

    let mut title_section = column![
        text(anime.title.preferred()).size(style::TEXT_XL),
    ]
    .spacing(style::SPACE_XS);

    if let Some(english) = &anime.title.english {
        if Some(english.as_str()) != anime.title.romaji.as_deref() {
            title_section = title_section
                .push(text(english.as_str()).size(style::TEXT_SM).color(cs.on_surface_variant));
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
                .color(cs.outline),
        );
    }

    let anime_id = anime.id;
    let status_card = container(
        column![
            text("Status").size(style::TEXT_XS).color(cs.on_surface_variant),
            pick_list(WatchStatus::ALL, Some(entry.status), move |s| {
                Message::StatusChanged(anime_id, s)
            })
            .text_size(style::TEXT_SM)
            .padding([style::SPACE_XS, style::SPACE_SM]),
            text("Score").size(style::TEXT_XS).color(cs.on_surface_variant),
            row![
                text_input("0-10", score_input)
                    .on_input(|v| Message::ScoreInputChanged(v))
                    .on_submit(Message::ScoreSubmitted(anime_id))
                    .size(style::TEXT_SM)
                    .padding([style::SPACE_XS, style::SPACE_SM])
                    .width(Length::Fixed(80.0))
                    .style(theme::text_input_style(cs)),
            ]
            .spacing(style::SPACE_SM)
            .align_y(Alignment::Center),
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

    let progress_card = container(
        column![
            text(ep_text).size(style::TEXT_BASE),
            row![
                button(text("\u{2212}").size(style::TEXT_SM))
                    .on_press(Message::EpisodeDecrement(anime_id))
                    .style(theme::control_button(cs))
                    .padding([style::SPACE_XS, style::SPACE_LG]),
                button(text("+").size(style::TEXT_SM))
                    .on_press(Message::EpisodeIncrement(anime_id))
                    .style(theme::control_button(cs))
                    .padding([style::SPACE_XS, style::SPACE_LG]),
            ]
            .spacing(style::SPACE_SM),
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
