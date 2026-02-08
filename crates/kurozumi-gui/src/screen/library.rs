use iced::widget::{button, column, container, pick_list, row, rule, scrollable, text};
use iced::{Alignment, Element, Length, Task};

use kurozumi_core::models::WatchStatus;
use kurozumi_core::storage::LibraryRow;

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

/// Library screen state.
pub struct Library {
    pub tab: WatchStatus,
    pub entries: Vec<LibraryRow>,
    pub selected_anime: Option<i64>,
    pub sort: LibrarySort,
    pub score_input: String,
    pub episode_input: String,
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
    SortChanged(LibrarySort),
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
            score_input: String::new(),
            episode_input: String::new(),
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
            Message::SortChanged(sort) => {
                self.sort = sort;
                self.refresh_task(db)
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

        let header = row![
            chip_bar(cs, self.tab),
            text(count_text)
                .size(style::TEXT_XS)
                .color(cs.outline)
                .line_height(style::LINE_HEIGHT_LOOSE)
                .width(Length::Fill),
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
            container(
                column![text("No anime in this list.")
                    .size(style::TEXT_SM)
                    .color(cs.on_surface_variant)
                    .line_height(style::LINE_HEIGHT_LOOSE),]
                .align_x(Alignment::Center),
            )
            .padding(style::SPACE_3XL)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .into()
        } else {
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

            scrollable(
                column(items)
                    .spacing(style::SPACE_XXS)
                    .padding([style::SPACE_XS, style::SPACE_LG]),
            )
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new()
                    .width(6)
                    .scroller_width(4)
                    .margin(2),
            ))
            .style(theme::overlay_scrollbar(cs))
            .height(Length::Fill)
            .into()
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
