use iced::widget::{button, column, container, pick_list, row, rule, text};
use iced::{Alignment, Element, Length, Task};

use crate::widgets::anime_card;

use ryuuji_core::models::WatchStatus;
use ryuuji_core::storage::LibraryRow;

use crate::app;
use crate::cover_cache::CoverCache;
use crate::db::DbHandle;
use crate::screen::{Action, ContextAction, ModalKind};
use crate::style;
use crate::theme::{self, ColorScheme};
use crate::widgets::{self, detail_panel};

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

/// Toggle between list and grid display modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    List,
    Grid,
}

/// Library screen state.
pub struct Library {
    pub tab: WatchStatus,
    pub entries: Vec<LibraryRow>,
    pub selected_anime: Option<i64>,
    pub sort: LibrarySort,
    pub view_mode: ViewMode,
    pub score_input: String,
    pub episode_input: String,
    pub start_date_input: String,
    pub finish_date_input: String,
    pub notes_input: String,
    pub rewatch_count_input: String,
}

/// Messages handled by the Library screen.
#[derive(Debug, Clone)]
pub enum Message {
    TabChanged(WatchStatus),
    AnimeSelected(i64),
    EpisodeChanged(i64, u32),
    StatusChanged(i64, WatchStatus),
    ScoreChanged(i64, f32),
    ScoreInputChanged(String),
    ScoreInputSubmitted,
    EpisodeInputChanged(String),
    EpisodeInputSubmitted,
    StartDateInputChanged(String),
    StartDateInputSubmitted,
    FinishDateInputChanged(String),
    FinishDateInputSubmitted,
    NotesInputChanged(String),
    NotesInputSubmitted,
    RewatchToggled(i64, bool),
    RewatchCountChanged(i64, u32),
    RewatchCountInputChanged(String),
    RewatchCountInputSubmitted,
    SortChanged(LibrarySort),
    ViewModeChanged(ViewMode),
    CloseDetail,
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
            view_mode: ViewMode::default(),
            score_input: String::new(),
            episode_input: String::new(),
            start_date_input: String::new(),
            finish_date_input: String::new(),
            notes_input: String::new(),
            rewatch_count_input: String::new(),
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
            Message::CloseDetail => {
                self.selected_anime = None;
                Action::None
            }
            Message::AnimeSelected(id) => {
                self.selected_anime = Some(id);
                // Sync stepper text buffers to the selected entry
                if let Some(row) = self.entries.iter().find(|r| r.anime.id == id) {
                    self.score_input = format!("{:.1}", row.entry.score.unwrap_or(0.0));
                    self.episode_input = row.entry.watched_episodes.to_string();
                    self.start_date_input = row.entry.start_date.clone().unwrap_or_default();
                    self.finish_date_input = row.entry.finish_date.clone().unwrap_or_default();
                    self.notes_input = row.entry.notes.clone().unwrap_or_default();
                    self.rewatch_count_input = row.entry.rewatch_count.to_string();
                }
                Action::None
            }
            Message::EpisodeChanged(anime_id, new_ep) => {
                if let Some(db) = db {
                    let db = db.clone();
                    return Action::RunTask(Task::perform(
                        async move {
                            let _ = db.update_episode_count(anime_id, new_ep).await;
                            let _ = db.record_watch(anime_id, new_ep).await;
                        },
                        |_| app::Message::Library(Message::DbOperationDone(Ok(()))),
                    ));
                }
                Action::None
            }
            Message::StatusChanged(anime_id, new_status) => {
                if let Some(db) = db {
                    let db = db.clone();
                    return Action::RunTask(Task::perform(
                        async move { db.update_library_status(anime_id, new_status).await },
                        |r| {
                            app::Message::Library(Message::DbOperationDone(
                                r.map_err(|e| e.to_string()),
                            ))
                        },
                    ));
                }
                Action::None
            }
            Message::ScoreChanged(anime_id, score) => {
                if let Some(db) = db {
                    let db = db.clone();
                    return Action::RunTask(Task::perform(
                        async move { db.update_library_score(anime_id, score).await },
                        |r| {
                            app::Message::Library(Message::DbOperationDone(
                                r.map_err(|e| e.to_string()),
                            ))
                        },
                    ));
                }
                Action::None
            }
            Message::ScoreInputChanged(val) => {
                self.score_input = val;
                Action::None
            }
            Message::ScoreInputSubmitted => {
                if let Some(anime_id) = self.selected_anime {
                    let score = self
                        .score_input
                        .parse::<f32>()
                        .unwrap_or(0.0)
                        .clamp(0.0, 10.0);
                    self.score_input = format!("{score:.1}");
                    return self.update(Message::ScoreChanged(anime_id, score), db);
                }
                Action::None
            }
            Message::EpisodeInputChanged(val) => {
                self.episode_input = val;
                Action::None
            }
            Message::EpisodeInputSubmitted => {
                if let Some(anime_id) = self.selected_anime {
                    let max_ep = self
                        .entries
                        .iter()
                        .find(|r| r.anime.id == anime_id)
                        .and_then(|r| r.anime.episodes)
                        .unwrap_or(u32::MAX);
                    let ep = self.episode_input.parse::<u32>().unwrap_or(0).min(max_ep);
                    self.episode_input = ep.to_string();
                    return self.update(Message::EpisodeChanged(anime_id, ep), db);
                }
                Action::None
            }
            Message::StartDateInputChanged(val) => {
                self.start_date_input = val;
                Action::None
            }
            Message::StartDateInputSubmitted => {
                if let Some(anime_id) = self.selected_anime {
                    if let Some(db) = db {
                        let db = db.clone();
                        let start = if self.start_date_input.is_empty() {
                            None
                        } else {
                            Some(self.start_date_input.clone())
                        };
                        let finish = if self.finish_date_input.is_empty() {
                            None
                        } else {
                            Some(self.finish_date_input.clone())
                        };
                        return Action::RunTask(Task::perform(
                            async move { db.update_library_dates(anime_id, start, finish).await },
                            |r| {
                                app::Message::Library(Message::DbOperationDone(
                                    r.map_err(|e| e.to_string()),
                                ))
                            },
                        ));
                    }
                }
                Action::None
            }
            Message::FinishDateInputChanged(val) => {
                self.finish_date_input = val;
                Action::None
            }
            Message::FinishDateInputSubmitted => {
                if let Some(anime_id) = self.selected_anime {
                    if let Some(db) = db {
                        let db = db.clone();
                        let start = if self.start_date_input.is_empty() {
                            None
                        } else {
                            Some(self.start_date_input.clone())
                        };
                        let finish = if self.finish_date_input.is_empty() {
                            None
                        } else {
                            Some(self.finish_date_input.clone())
                        };
                        return Action::RunTask(Task::perform(
                            async move { db.update_library_dates(anime_id, start, finish).await },
                            |r| {
                                app::Message::Library(Message::DbOperationDone(
                                    r.map_err(|e| e.to_string()),
                                ))
                            },
                        ));
                    }
                }
                Action::None
            }
            Message::NotesInputChanged(val) => {
                self.notes_input = val;
                Action::None
            }
            Message::NotesInputSubmitted => {
                if let Some(anime_id) = self.selected_anime {
                    if let Some(db) = db {
                        let db = db.clone();
                        let notes = if self.notes_input.is_empty() {
                            None
                        } else {
                            Some(self.notes_input.clone())
                        };
                        return Action::RunTask(Task::perform(
                            async move { db.update_library_notes(anime_id, notes).await },
                            |r| {
                                app::Message::Library(Message::DbOperationDone(
                                    r.map_err(|e| e.to_string()),
                                ))
                            },
                        ));
                    }
                }
                Action::None
            }
            Message::RewatchToggled(anime_id, toggled) => {
                if let Some(db) = db {
                    let db = db.clone();
                    let count = self.rewatch_count_input.parse::<u32>().unwrap_or(0);
                    return Action::RunTask(Task::perform(
                        async move { db.update_library_rewatch(anime_id, toggled, count).await },
                        |r| {
                            app::Message::Library(Message::DbOperationDone(
                                r.map_err(|e| e.to_string()),
                            ))
                        },
                    ));
                }
                Action::None
            }
            Message::RewatchCountChanged(anime_id, count) => {
                self.rewatch_count_input = count.to_string();
                if let Some(db) = db {
                    let db = db.clone();
                    let rewatching = self
                        .entries
                        .iter()
                        .find(|r| r.anime.id == anime_id)
                        .map(|r| r.entry.rewatching)
                        .unwrap_or(false);
                    return Action::RunTask(Task::perform(
                        async move { db.update_library_rewatch(anime_id, rewatching, count).await },
                        |r| {
                            app::Message::Library(Message::DbOperationDone(
                                r.map_err(|e| e.to_string()),
                            ))
                        },
                    ));
                }
                Action::None
            }
            Message::RewatchCountInputChanged(val) => {
                self.rewatch_count_input = val;
                Action::None
            }
            Message::RewatchCountInputSubmitted => {
                if let Some(anime_id) = self.selected_anime {
                    let count = self.rewatch_count_input.parse::<u32>().unwrap_or(0);
                    self.rewatch_count_input = count.to_string();
                    return self.update(Message::RewatchCountChanged(anime_id, count), db);
                }
                Action::None
            }
            Message::SortChanged(sort) => {
                self.sort = sort;
                self.refresh_task(db)
            }
            Message::ViewModeChanged(mode) => {
                self.view_mode = mode;
                Action::None
            }
            Message::ContextAction(anime_id, action) => match action {
                ContextAction::ChangeStatus(new_status) => {
                    if let Some(db) = db {
                        let db = db.clone();
                        return Action::RunTask(Task::perform(
                            async move { db.update_library_status(anime_id, new_status).await },
                            |r| {
                                app::Message::Library(Message::DbOperationDone(
                                    r.map_err(|e| e.to_string()),
                                ))
                            },
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
                    Action::ShowModal(ModalKind::ConfirmDelete {
                        anime_id,
                        title,
                        source: super::Page::Library,
                    })
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
                        |r| {
                            app::Message::Library(Message::DbOperationDone(
                                r.map_err(|e| e.to_string()),
                            ))
                        },
                    ));
                }
                Action::None
            }
            Message::CancelModal => Action::DismissModal,
            Message::LibraryRefreshed(result) => {
                if let Ok(mut entries) = result {
                    self.sort_entries(&mut entries);
                    self.entries = entries;
                    // Re-sync stepper text buffers to the (possibly updated) selected entry
                    if let Some(id) = self.selected_anime {
                        if let Some(row) = self.entries.iter().find(|r| r.anime.id == id) {
                            self.score_input = format!("{:.1}", row.entry.score.unwrap_or(0.0));
                            self.episode_input = row.entry.watched_episodes.to_string();
                            self.start_date_input =
                                row.entry.start_date.clone().unwrap_or_default();
                            self.finish_date_input =
                                row.entry.finish_date.clone().unwrap_or_default();
                            self.notes_input = row.entry.notes.clone().unwrap_or_default();
                            self.rewatch_count_input = row.entry.rewatch_count.to_string();
                        }
                    }
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

    fn sort_entries(&self, entries: &mut [LibraryRow]) {
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

    pub fn view<'a>(&'a self, cs: &'a ColorScheme, covers: &'a CoverCache) -> Element<'a, Message> {
        let count_text = format!(
            "{} {}",
            self.entries.len(),
            if self.entries.len() == 1 {
                "entry"
            } else {
                "entries"
            }
        );

        // View mode toggle buttons
        let list_icon = lucide_icons::iced::icon_list().size(style::TEXT_SM).color(
            if self.view_mode == ViewMode::List {
                cs.primary
            } else {
                cs.on_surface_variant
            },
        );
        let grid_icon = lucide_icons::iced::icon_layout_grid()
            .size(style::TEXT_SM)
            .color(if self.view_mode == ViewMode::Grid {
                cs.primary
            } else {
                cs.on_surface_variant
            });

        let view_toggle = row![
            button(container(list_icon).center(Length::Fill))
                .width(Length::Fixed(28.0))
                .height(Length::Fixed(28.0))
                .padding(0)
                .on_press(Message::ViewModeChanged(ViewMode::List))
                .style(theme::icon_button(cs)),
            button(container(grid_icon).center(Length::Fill))
                .width(Length::Fixed(28.0))
                .height(Length::Fixed(28.0))
                .padding(0)
                .on_press(Message::ViewModeChanged(ViewMode::Grid))
                .style(theme::icon_button(cs)),
        ]
        .spacing(style::SPACE_XXS);

        let header = row![
            chip_bar(cs, self.tab),
            text(count_text)
                .size(style::TEXT_XS)
                .color(cs.outline)
                .line_height(style::LINE_HEIGHT_LOOSE)
                .width(Length::Fill),
            view_toggle,
            pick_list(LibrarySort::ALL, Some(self.sort), |s| {
                Message::SortChanged(s)
            })
            .text_size(style::TEXT_SM)
            .padding([style::SPACE_SM, style::SPACE_MD])
            .style(theme::pick_list_style(cs))
            .menu_style(theme::pick_list_menu_style(cs)),
        ]
        .spacing(style::SPACE_SM)
        .align_y(Alignment::Center)
        .padding([style::SPACE_SM, style::SPACE_LG]);

        let list: Element<'_, Message> = if self.entries.is_empty() {
            let icon = lucide_icons::iced::icon_library()
                .size(48.0)
                .color(cs.outline)
                .into();
            widgets::empty_state(
                cs,
                icon,
                "No anime yet",
                "Import your library from a service or start watching something.",
            )
        } else {
            match self.view_mode {
                ViewMode::List => {
                    let items: Vec<Element<'a, Message>> = self
                        .entries
                        .iter()
                        .map(|r| {
                            widgets::anime_list_item(
                                cs,
                                r,
                                self.selected_anime,
                                covers,
                                Message::AnimeSelected,
                                Message::ContextAction,
                            )
                        })
                        .collect();

                    crate::widgets::styled_scrollable(
                        column(items)
                            .spacing(style::SPACE_XXS)
                            .padding([style::SPACE_XS, style::SPACE_LG]),
                        cs,
                    )
                    .height(Length::Fill)
                    .into()
                }
                ViewMode::Grid => {
                    let cards: Vec<Element<'a, Message>> = self
                        .entries
                        .iter()
                        .map(|r| {
                            anime_card::library_card(
                                cs,
                                r,
                                covers,
                                Message::AnimeSelected(r.anime.id),
                            )
                        })
                        .collect();

                    let wrap = iced_aw::Wrap::with_elements(cards)
                        .spacing(style::SPACE_SM)
                        .line_spacing(style::SPACE_SM);

                    crate::widgets::styled_scrollable(
                        container(wrap).padding([style::SPACE_SM, style::SPACE_LG]),
                        cs,
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
                let anime_id = lib_row.anime.id;
                let detail = detail_panel(
                    cs,
                    lib_row,
                    Message::CloseDetail,
                    move |s| Message::StatusChanged(anime_id, s),
                    move |v| Message::ScoreChanged(anime_id, v),
                    move |ep| Message::EpisodeChanged(anime_id, ep),
                    &self.score_input,
                    Message::ScoreInputChanged,
                    Message::ScoreInputSubmitted,
                    &self.episode_input,
                    Message::EpisodeInputChanged,
                    Message::EpisodeInputSubmitted,
                    covers,
                    &self.start_date_input,
                    Message::StartDateInputChanged,
                    Message::StartDateInputSubmitted,
                    &self.finish_date_input,
                    Message::FinishDateInputChanged,
                    Message::FinishDateInputSubmitted,
                    &self.notes_input,
                    Message::NotesInputChanged,
                    Message::NotesInputSubmitted,
                    move |b| Message::RewatchToggled(anime_id, b),
                    &self.rewatch_count_input,
                    Message::RewatchCountInputChanged,
                    Message::RewatchCountInputSubmitted,
                    move |c| Message::RewatchCountChanged(anime_id, c),
                );
                return row![
                    container(content)
                        .width(Length::FillPortion(3))
                        .height(Length::Fill),
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
            let mut chip_content = row![].spacing(style::SPACE_XXS).align_y(Alignment::Center);
            if is_selected {
                chip_content =
                    chip_content.push(lucide_icons::iced::icon_check().size(style::TEXT_XS));
            }
            chip_content = chip_content.push(
                text(base_label)
                    .size(style::TEXT_XS)
                    .line_height(style::LINE_HEIGHT_LOOSE),
            );

            button(container(chip_content).center_y(Length::Fill))
                .height(Length::Fixed(style::CHIP_HEIGHT))
                .padding([style::SPACE_XS, style::SPACE_MD])
                .on_press(Message::TabChanged(status))
                .style(theme::filter_chip(is_selected, cs))
                .into()
        })
        .collect();

    row(chips).spacing(style::SPACE_XS).into()
}
