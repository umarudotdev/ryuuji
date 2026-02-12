use iced::widget::{button, center, column, container, row, rule, text, Space};
use iced::{Alignment, Element, Length, Task};

use chrono::{Local, NaiveDate};
use ryuuji_core::models::WatchStatus;
use ryuuji_core::storage::{HistoryRow, LibraryRow};

use crate::app;
use crate::cover_cache::CoverCache;
use crate::db::DbHandle;
use crate::screen::{Action, ContextAction, ModalKind};
use crate::style;
use crate::theme::{self, ColorScheme};
use crate::widgets;

/// History screen state.
pub struct History {
    pub entries: Vec<HistoryRow>,
    pub selected_anime: Option<i64>,
    /// Full library row for the selected anime (fetched on demand).
    pub selected_row: Option<LibraryRow>,
    pub score_input: String,
    pub episode_input: String,
    pub start_date_input: String,
    pub finish_date_input: String,
    pub notes_input: String,
    pub rewatch_count_input: String,
}

/// Messages handled by the History screen.
#[derive(Debug, Clone)]
#[allow(dead_code)] // ContextAction is infrastructure for future context menus
pub enum Message {
    HistoryRefreshed(Result<Vec<HistoryRow>, String>),
    AnimeSelected(i64),
    CloseDetail,
    LibraryRowFetched(Box<Result<Option<LibraryRow>, String>>),
    // Detail panel editing
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
    // Context menu
    ContextAction(i64, ContextAction),
    ConfirmDelete(i64),
    CancelModal,
    DbOperationDone(Result<(), String>),
}

impl History {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            selected_anime: None,
            selected_row: None,
            score_input: String::new(),
            episode_input: String::new(),
            start_date_input: String::new(),
            finish_date_input: String::new(),
            notes_input: String::new(),
            rewatch_count_input: String::new(),
        }
    }

    /// Handle a history message, returning an Action for the app router.
    pub fn update(&mut self, msg: Message, db: Option<&DbHandle>) -> Action {
        match msg {
            Message::HistoryRefreshed(Ok(entries)) => {
                self.entries = entries;
                Action::None
            }
            Message::HistoryRefreshed(Err(e)) => Action::SetStatus(format!("History error: {e}")),
            Message::AnimeSelected(id) => {
                self.selected_anime = Some(id);
                self.fetch_library_row(db, id)
            }
            Message::CloseDetail => {
                self.selected_anime = None;
                self.selected_row = None;
                Action::None
            }
            Message::LibraryRowFetched(result) => {
                if let Ok(Some(row)) = *result {
                    self.score_input = format!("{:.1}", row.entry.score.unwrap_or(0.0));
                    self.episode_input = row.entry.watched_episodes.to_string();
                    self.start_date_input = row.entry.start_date.clone().unwrap_or_default();
                    self.finish_date_input = row.entry.finish_date.clone().unwrap_or_default();
                    self.notes_input = row.entry.notes.clone().unwrap_or_default();
                    self.rewatch_count_input = row.entry.rewatch_count.to_string();
                    self.selected_row = Some(row);
                } else {
                    // Anime no longer in library — close the detail panel.
                    self.selected_anime = None;
                    self.selected_row = None;
                }
                Action::None
            }
            // ── Detail panel editing ─────────────────────────────
            Message::EpisodeChanged(anime_id, new_ep) => {
                if let Some(db) = db {
                    let db = db.clone();
                    return Action::RunTask(Task::perform(
                        async move {
                            let _ = db.update_episode_count(anime_id, new_ep).await;
                            let _ = db.record_watch(anime_id, new_ep).await;
                        },
                        |_| app::Message::History(Message::DbOperationDone(Ok(()))),
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
                            app::Message::History(Message::DbOperationDone(
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
                            app::Message::History(Message::DbOperationDone(
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
                        .selected_row
                        .as_ref()
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
                                app::Message::History(Message::DbOperationDone(
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
                                app::Message::History(Message::DbOperationDone(
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
                                app::Message::History(Message::DbOperationDone(
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
                            app::Message::History(Message::DbOperationDone(
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
                        .selected_row
                        .as_ref()
                        .map(|r| r.entry.rewatching)
                        .unwrap_or(false);
                    return Action::RunTask(Task::perform(
                        async move { db.update_library_rewatch(anime_id, rewatching, count).await },
                        |r| {
                            app::Message::History(Message::DbOperationDone(
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
            // ── Context menu ─────────────────────────────────────
            Message::ContextAction(anime_id, action) => match action {
                ContextAction::ChangeStatus(new_status) => {
                    if let Some(db) = db {
                        let db = db.clone();
                        return Action::RunTask(Task::perform(
                            async move { db.update_library_status(anime_id, new_status).await },
                            |r| {
                                app::Message::History(Message::DbOperationDone(
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
                        source: super::Page::History,
                    })
                }
            },
            Message::ConfirmDelete(anime_id) => {
                if let Some(db) = db {
                    if self.selected_anime == Some(anime_id) {
                        self.selected_anime = None;
                        self.selected_row = None;
                    }
                    let db = db.clone();
                    return Action::RunTask(Task::perform(
                        async move { db.delete_library_entry(anime_id).await },
                        |r| {
                            app::Message::History(Message::DbOperationDone(
                                r.map_err(|e| e.to_string()),
                            ))
                        },
                    ));
                }
                Action::None
            }
            Message::CancelModal => Action::DismissModal,
            Message::DbOperationDone(_result) => self.refresh_all(db),
        }
    }

    // ── Async actions ────────────────────────────────────────────

    /// Fire a task to load watch history from the DB.
    pub fn load_history(&self, db: Option<&DbHandle>) -> Action {
        let Some(db) = db else {
            return Action::None;
        };
        let db = db.clone();
        Action::RunTask(Task::perform(
            async move { db.get_watch_history(500).await.map_err(|e| e.to_string()) },
            |result| app::Message::History(Message::HistoryRefreshed(result)),
        ))
    }

    /// Fetch the full LibraryRow for the detail panel.
    fn fetch_library_row(&self, db: Option<&DbHandle>, anime_id: i64) -> Action {
        let Some(db) = db else {
            return Action::None;
        };
        let db = db.clone();
        Action::RunTask(Task::perform(
            async move {
                db.get_library_row(anime_id)
                    .await
                    .map_err(|e| e.to_string())
            },
            |r| app::Message::History(Message::LibraryRowFetched(Box::new(r))),
        ))
    }

    /// Re-fetch both history and the selected row after a DB write.
    fn refresh_all(&self, db: Option<&DbHandle>) -> Action {
        let Some(db) = db else {
            return Action::None;
        };
        let db1 = db.clone();
        let history_task = Task::perform(
            async move { db1.get_watch_history(500).await.map_err(|e| e.to_string()) },
            |r| app::Message::History(Message::HistoryRefreshed(r)),
        );

        let row_task = if let Some(id) = self.selected_anime {
            let db2 = db.clone();
            Task::perform(
                async move { db2.get_library_row(id).await.map_err(|e| e.to_string()) },
                |r| app::Message::History(Message::LibraryRowFetched(Box::new(r))),
            )
        } else {
            Task::none()
        };

        Action::RunTask(Task::batch([history_task, row_task]))
    }

    // ── View ─────────────────────────────────────────────────────

    pub fn view<'a>(
        &'a self,
        cs: &ColorScheme,
        cover_cache: &'a CoverCache,
    ) -> Element<'a, Message> {
        if self.entries.is_empty() {
            return empty_state(cs);
        }

        let today = Local::now().date_naive();
        let yesterday = today.pred_opt().unwrap_or(today);

        let mut content = column![].spacing(style::SPACE_XS).width(Length::Fill);
        let mut current_date: Option<NaiveDate> = None;
        let mut is_first_section = true;

        for entry in &self.entries {
            let entry_date = entry.watched_at.with_timezone(&Local).date_naive();

            // Insert date header when the date changes.
            if current_date != Some(entry_date) {
                current_date = Some(entry_date);
                let label = if entry_date == today {
                    "Today".to_string()
                } else if entry_date == yesterday {
                    "Yesterday".to_string()
                } else {
                    entry_date.format("%B %d, %Y").to_string()
                };

                if !is_first_section {
                    content = content.push(Space::new().height(style::SPACE_SM));
                }
                is_first_section = false;
                content = content.push(
                    text(label)
                        .size(style::TEXT_SM)
                        .font(style::FONT_HEADING)
                        .color(cs.on_surface_variant)
                        .line_height(style::LINE_HEIGHT_TIGHT),
                );
            }

            content = content.push(history_item(entry, cs, cover_cache, self.selected_anime));
        }

        let scrollable_content = crate::widgets::styled_scrollable(
            container(content)
                .padding([style::SPACE_LG, style::SPACE_XL])
                .width(Length::Fill),
            cs,
        )
        .height(Length::Fill);

        let list = column![
            // Header
            container(
                text("History")
                    .size(style::TEXT_XL)
                    .font(style::FONT_HEADING)
                    .line_height(style::LINE_HEIGHT_TIGHT),
            )
            .padding(
                iced::Padding::new(style::SPACE_XL)
                    .top(style::SPACE_LG)
                    .bottom(style::SPACE_SM),
            ),
            scrollable_content,
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        // Detail panel (shown when a library row has been fetched for the selection)
        if let Some(lib_row) = &self.selected_row {
            let anime_id = lib_row.anime.id;
            let detail = widgets::detail_panel(
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
                cover_cache,
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
                container(list)
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

        container(list)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

// ── History item widget ──────────────────────────────────────────

/// A single history item row with cover thumbnail, metadata, and selection support.
fn history_item<'a>(
    entry: &'a HistoryRow,
    cs: &ColorScheme,
    cover_cache: &'a CoverCache,
    selected: Option<i64>,
) -> Element<'a, Message> {
    let time_str = entry
        .watched_at
        .with_timezone(&Local)
        .format("%H:%M")
        .to_string();
    let title = entry.anime.title.preferred();
    let episode_text = format!("Episode {}", entry.episode);
    let is_selected = selected == Some(entry.anime.id);
    let anime_id = entry.anime.id;

    let thumb = widgets::rounded_cover(
        cs,
        cover_cache,
        anime_id,
        style::THUMB_WIDTH,
        style::THUMB_HEIGHT,
        style::RADIUS_SM,
    );

    // Title + metadata column
    let mut meta_parts: Vec<String> = Vec::new();
    if let Some(mt) = &entry.anime.media_type {
        meta_parts.push(crate::format::media_type(mt));
    }
    if let Some(year) = entry.anime.year {
        meta_parts.push(year.to_string());
    }
    let genre_str: String = entry
        .anime
        .genres
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if !genre_str.is_empty() {
        meta_parts.push(genre_str);
    }

    let mut info_col = column![text(title)
        .size(style::TEXT_BASE)
        .font(style::FONT_HEADING)
        .line_height(style::LINE_HEIGHT_NORMAL)
        .wrapping(iced::widget::text::Wrapping::None)]
    .spacing(style::SPACE_XXS)
    .clip(true);

    if !meta_parts.is_empty() {
        info_col = info_col.push(
            text(meta_parts.join("  \u{00B7}  "))
                .size(style::TEXT_XS)
                .color(cs.outline)
                .line_height(style::LINE_HEIGHT_LOOSE),
        );
    }

    // Right side: episode badge + timestamp
    let right_col = column![
        text(episode_text)
            .size(style::TEXT_SM)
            .color(cs.on_surface_variant)
            .line_height(style::LINE_HEIGHT_NORMAL),
        text(time_str)
            .size(style::TEXT_XS)
            .color(cs.outline)
            .line_height(style::LINE_HEIGHT_LOOSE),
    ]
    .spacing(style::SPACE_XXS)
    .align_x(Alignment::End);

    let content = row![thumb, info_col.width(Length::Fill), right_col]
        .spacing(style::SPACE_SM)
        .align_y(Alignment::Center);

    button(content)
        .width(Length::Fill)
        .padding([style::SPACE_XS, style::SPACE_MD])
        .on_press(Message::AnimeSelected(anime_id))
        .style(theme::list_item(is_selected, cs))
        .into()
}

/// Empty state when no history exists.
fn empty_state(cs: &ColorScheme) -> Element<'static, Message> {
    center(
        column![
            text("No watch history yet")
                .size(style::TEXT_LG)
                .font(style::FONT_HEADING)
                .color(cs.on_surface_variant)
                .line_height(style::LINE_HEIGHT_TIGHT),
            text("Start watching anime and your history will appear here.")
                .size(style::TEXT_SM)
                .color(cs.outline)
                .line_height(style::LINE_HEIGHT_LOOSE),
        ]
        .spacing(style::SPACE_SM)
        .align_x(iced::Alignment::Center),
    )
    .into()
}
