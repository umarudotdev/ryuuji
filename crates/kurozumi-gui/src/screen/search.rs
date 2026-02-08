use iced::widget::{button, column, container, row, rule, scrollable, text, text_input};
use iced::{Alignment, Background, Border, Color, Element, Length, Task, Theme};
use iced_aw::ContextMenu;

use kurozumi_core::models::WatchStatus;
use kurozumi_core::storage::LibraryRow;

use crate::app;
use crate::db::DbHandle;
use crate::screen::{Action, ModalKind, Page};
use crate::style;
use crate::theme::{self, ColorScheme};
use crate::widgets::detail_panel;

/// Actions available in the search context menu (mirrors library).
#[derive(Debug, Clone)]
pub enum ContextAction {
    ChangeStatus(WatchStatus),
    Delete,
}

/// Search screen state.
pub struct Search {
    query: String,
    all_entries: Vec<LibraryRow>,
    filtered_indices: Vec<usize>,
    loaded: bool,
    selected_anime: Option<i64>,
}

/// Messages handled by the Search screen.
#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(String),
    EntriesLoaded(Result<Vec<LibraryRow>, String>),
    AnimeSelected(i64),
    EpisodeChanged(i64, u32),
    StatusChanged(i64, WatchStatus),
    ScoreChanged(i64, f32),
    ContextAction(i64, ContextAction),
    ConfirmDelete(i64),
    CancelModal,
    DbOperationDone(Result<(), String>),
}

impl Search {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            all_entries: Vec::new(),
            filtered_indices: Vec::new(),
            loaded: false,
            selected_anime: None,
        }
    }

    /// Fire an async task to load all library entries.
    pub fn load_entries(&self, db: Option<&DbHandle>) -> Action {
        if let Some(db) = db {
            let db = db.clone();
            Action::RunTask(Task::perform(
                async move { db.get_all_library().await },
                |r| app::Message::Search(Message::EntriesLoaded(r.map_err(|e| e.to_string()))),
            ))
        } else {
            Action::None
        }
    }

    /// Rebuild `filtered_indices` from the current query.
    fn refilter(&mut self) {
        let q = self.query.to_lowercase();
        if q.is_empty() {
            self.filtered_indices = (0..self.all_entries.len()).collect();
        } else {
            self.filtered_indices = self
                .all_entries
                .iter()
                .enumerate()
                .filter(|(_, row)| matches_query(row, &q))
                .map(|(i, _)| i)
                .collect();
        }
    }

    /// Handle a search message, returning an Action for the app router.
    pub fn update(&mut self, msg: Message, db: Option<&DbHandle>) -> Action {
        match msg {
            Message::QueryChanged(new_query) => {
                self.query = new_query;
                self.refilter();
                // Deselect if selected anime is no longer in filtered results
                if let Some(id) = self.selected_anime {
                    let still_visible = self
                        .filtered_indices
                        .iter()
                        .any(|&i| self.all_entries[i].anime.id == id);
                    if !still_visible {
                        self.selected_anime = None;
                    }
                }
                Action::None
            }
            Message::EntriesLoaded(result) => {
                if let Ok(mut entries) = result {
                    entries.sort_by(|a, b| {
                        a.anime
                            .title
                            .preferred()
                            .to_lowercase()
                            .cmp(&b.anime.title.preferred().to_lowercase())
                    });
                    self.all_entries = entries;
                    self.loaded = true;
                    self.refilter();
                }
                Action::None
            }
            Message::AnimeSelected(id) => {
                self.selected_anime = Some(id);
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
                        |_| app::Message::Search(Message::DbOperationDone(Ok(()))),
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
                            app::Message::Search(Message::DbOperationDone(
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
                            app::Message::Search(Message::DbOperationDone(
                                r.map_err(|e| e.to_string()),
                            ))
                        },
                    ));
                }
                Action::None
            }
            Message::ContextAction(anime_id, action) => match action {
                ContextAction::ChangeStatus(new_status) => {
                    if let Some(db) = db {
                        let db = db.clone();
                        return Action::RunTask(Task::perform(
                            async move { db.update_library_status(anime_id, new_status).await },
                            |r| {
                                app::Message::Search(Message::DbOperationDone(
                                    r.map_err(|e| e.to_string()),
                                ))
                            },
                        ));
                    }
                    Action::None
                }
                ContextAction::Delete => {
                    let title = self
                        .all_entries
                        .iter()
                        .find(|r| r.anime.id == anime_id)
                        .map(|r| r.anime.title.preferred().to_string())
                        .unwrap_or_else(|| "this anime".into());
                    Action::ShowModal(ModalKind::ConfirmDelete {
                        anime_id,
                        title,
                        source: Page::Search,
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
                            app::Message::Search(Message::DbOperationDone(
                                r.map_err(|e| e.to_string()),
                            ))
                        },
                    ));
                }
                Action::None
            }
            Message::CancelModal => Action::DismissModal,
            Message::DbOperationDone(_result) => {
                // After any DB write, reload entries.
                self.load_entries(db)
            }
        }
    }

    pub fn view<'a>(&'a self, cs: &'a ColorScheme) -> Element<'a, Message> {
        let search_icon = lucide_icons::iced::icon_search()
            .size(style::TEXT_BASE)
            .color(cs.on_surface_variant);

        let search_input = text_input("Search library...", &self.query)
            .on_input(Message::QueryChanged)
            .size(style::TEXT_BASE)
            .padding([style::SPACE_SM, style::SPACE_MD])
            .width(Length::Fill)
            .style(theme::text_input_style(cs));

        let header = row![search_icon, search_input]
            .spacing(style::SPACE_SM)
            .align_y(Alignment::Center)
            .padding([style::SPACE_SM, style::SPACE_LG]);

        let result_count = if self.query.is_empty() {
            format!(
                "{} {}",
                self.all_entries.len(),
                if self.all_entries.len() == 1 {
                    "entry"
                } else {
                    "entries"
                }
            )
        } else {
            format!(
                "{} {}",
                self.filtered_indices.len(),
                if self.filtered_indices.len() == 1 {
                    "result"
                } else {
                    "results"
                }
            )
        };

        let count_row = container(
            text(result_count)
                .size(style::TEXT_XS)
                .color(cs.outline)
                .line_height(style::LINE_HEIGHT_LOOSE),
        )
        .padding([0.0, style::SPACE_LG]);

        let list: Element<'_, Message> = if !self.loaded {
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
        } else if self.filtered_indices.is_empty() {
            let msg = if self.query.is_empty() {
                "Your library is empty."
            } else {
                "No matching anime found."
            };
            container(
                text(msg)
                    .size(style::TEXT_SM)
                    .color(cs.on_surface_variant)
                    .line_height(style::LINE_HEIGHT_LOOSE),
            )
            .padding(style::SPACE_3XL)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .into()
        } else {
            let items: Vec<Element<'a, Message>> = self
                .filtered_indices
                .iter()
                .map(|&i| search_list_item(cs, &self.all_entries[i], self.selected_anime))
                .collect();

            scrollable(
                column(items)
                    .spacing(style::SPACE_XXS)
                    .padding([style::SPACE_XS, style::SPACE_LG]),
            )
            .height(Length::Fill)
            .into()
        };

        let content = column![header, count_row, rule::horizontal(1), list]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill);

        if let Some(anime_id) = self.selected_anime {
            if let Some(lib_row) = self.all_entries.iter().find(|r| r.anime.id == anime_id) {
                let anime_id = lib_row.anime.id;
                let detail = detail_panel(
                    cs,
                    lib_row,
                    move |s| Message::StatusChanged(anime_id, s),
                    move |v| Message::ScoreChanged(anime_id, v),
                    move |ep| Message::EpisodeChanged(anime_id, ep),
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
}

/// Check if a library row matches the search query (case-insensitive substring).
fn matches_query(row: &LibraryRow, query: &str) -> bool {
    let title = &row.anime.title;

    if title.preferred().to_lowercase().contains(query) {
        return true;
    }
    if let Some(en) = &title.english {
        if en.to_lowercase().contains(query) {
            return true;
        }
    }
    if let Some(romaji) = &title.romaji {
        if romaji.to_lowercase().contains(query) {
            return true;
        }
    }
    if let Some(native) = &title.native {
        if native.to_lowercase().contains(query) {
            return true;
        }
    }
    for synonym in &row.anime.synonyms {
        if synonym.to_lowercase().contains(query) {
            return true;
        }
    }

    false
}

/// A single search result list item with context menu.
fn search_list_item<'a>(
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

    let status_label = text(lib_row.entry.status.as_str())
        .size(style::TEXT_XS)
        .color(cs.on_surface_variant)
        .line_height(style::LINE_HEIGHT_LOOSE);

    let status_bar = container(text("").size(1))
        .width(Length::Fixed(3.0))
        .height(Length::Fill)
        .style(theme::status_bar_accent(status_col));

    let content = row![
        status_bar,
        text(title)
            .size(style::TEXT_BASE)
            .line_height(style::LINE_HEIGHT_NORMAL)
            .width(Length::Fill),
        status_label,
        text(progress)
            .size(style::TEXT_SM)
            .color(cs.on_surface_variant)
            .line_height(style::LINE_HEIGHT_LOOSE),
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
        button(
            text(label)
                .size(style::TEXT_SM)
                .line_height(style::LINE_HEIGHT_LOOSE),
        )
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

    ContextMenu::new(base, move || {
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
                button(
                    text("Delete")
                        .size(style::TEXT_SM)
                        .line_height(style::LINE_HEIGHT_LOOSE),
                )
                .width(Length::Fill)
                .padding([style::SPACE_XS, style::SPACE_MD])
                .on_press(Message::ContextAction(anime_id, ContextAction::Delete))
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
    .style(theme::aw_context_menu_style(cs))
    .into()
}
