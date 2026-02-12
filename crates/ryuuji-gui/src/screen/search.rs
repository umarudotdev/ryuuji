use iced::widget::{button, column, container, pick_list, row, rule, text, text_input};
use iced::{Alignment, Element, Length, Task};

use ryuuji_api::traits::AnimeSearchResult;
use ryuuji_core::models::WatchStatus;
use ryuuji_core::storage::LibraryRow;

use crate::app;
use crate::cover_cache::CoverCache;
use crate::db::DbHandle;
use crate::format;
use crate::screen::{Action, ContextAction, ModalKind, Page};
use crate::style;
use crate::theme::{self, ColorScheme};
use crate::widgets::{self, detail_panel, online_detail_panel};

// ── Sort ──────────────────────────────────────────────────────────

/// Sort mode for search results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchSort {
    #[default]
    Alphabetical,
    Score,
    RecentlyUpdated,
}

impl SearchSort {
    pub const ALL: &[SearchSort] = &[Self::Alphabetical, Self::Score, Self::RecentlyUpdated];
}

impl std::fmt::Display for SearchSort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Alphabetical => write!(f, "A-Z"),
            Self::Score => write!(f, "Score"),
            Self::RecentlyUpdated => write!(f, "Recent"),
        }
    }
}

// ── Search mode ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchMode {
    #[default]
    Local,
    Online,
}

// ── State ─────────────────────────────────────────────────────────

/// Search screen state.
pub struct Search {
    query: String,
    pub all_entries: Vec<LibraryRow>,
    filtered_indices: Vec<usize>,
    loaded: bool,
    pub selected_anime: Option<i64>,
    pub score_input: String,
    pub episode_input: String,
    // Filtering & sorting
    status_filter: Option<WatchStatus>,
    sort: SearchSort,
    // Online search
    pub search_mode: SearchMode,
    pub online_results: Vec<AnimeSearchResult>,
    pub online_loading: bool,
    online_error: Option<String>,
    pub service_authenticated: bool,
    selected_online: Option<usize>,
    pub start_date_input: String,
    pub finish_date_input: String,
    pub notes_input: String,
    pub rewatch_count_input: String,
}

// ── Messages ──────────────────────────────────────────────────────

/// Messages handled by the Search screen.
#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(String),
    EntriesLoaded(Result<Vec<LibraryRow>, String>),
    AnimeSelected(i64),
    EpisodeChanged(i64, u32),
    StatusChanged(i64, WatchStatus),
    ScoreChanged(i64, f32),
    ScoreInputChanged(String),
    ScoreInputSubmitted,
    EpisodeInputChanged(String),
    EpisodeInputSubmitted,
    ContextAction(i64, ContextAction),
    ConfirmDelete(i64),
    CancelModal,
    DbOperationDone(Result<(), String>),
    ClearQuery,
    CloseDetail,
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
    // Filter & sort
    StatusFilterChanged(Option<WatchStatus>),
    SortChanged(SearchSort),
    // Online search
    SearchOnline,
    OnlineResultsLoaded(Result<Vec<AnimeSearchResult>, String>),
    BackToLocal,
    OnlineSelected(usize),
    AddToLibrary(usize),
    AddedToLibrary(Result<(), String>),
}

// ── Implementation ────────────────────────────────────────────────

impl Search {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            all_entries: Vec::new(),
            filtered_indices: Vec::new(),
            loaded: false,
            selected_anime: None,
            score_input: String::new(),
            episode_input: String::new(),
            status_filter: None,
            sort: SearchSort::default(),
            search_mode: SearchMode::Local,
            online_results: Vec::new(),
            online_loading: false,
            online_error: None,
            service_authenticated: false,
            selected_online: None,
            start_date_input: String::new(),
            finish_date_input: String::new(),
            notes_input: String::new(),
            rewatch_count_input: String::new(),
        }
    }

    /// Current search query text.
    pub fn query(&self) -> &str {
        &self.query
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

    /// Rebuild `filtered_indices` from the current query and status filter, then sort.
    fn refilter(&mut self) {
        let q = self.query.to_lowercase();
        self.filtered_indices = self
            .all_entries
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                // Status filter
                if let Some(status) = self.status_filter {
                    if row.entry.status != status {
                        return false;
                    }
                }
                // Text query
                if !q.is_empty() && !matches_query(row, &q) {
                    return false;
                }
                true
            })
            .map(|(i, _)| i)
            .collect();

        // Sort filtered indices
        let entries = &self.all_entries;
        match self.sort {
            SearchSort::Alphabetical => {
                self.filtered_indices.sort_by(|&a, &b| {
                    entries[a]
                        .anime
                        .title
                        .preferred()
                        .to_lowercase()
                        .cmp(&entries[b].anime.title.preferred().to_lowercase())
                });
            }
            SearchSort::Score => {
                self.filtered_indices.sort_by(|&a, &b| {
                    let sa = entries[a].entry.score.unwrap_or(0.0);
                    let sb = entries[b].entry.score.unwrap_or(0.0);
                    sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SearchSort::RecentlyUpdated => {
                self.filtered_indices.sort_by(|&a, &b| {
                    entries[b]
                        .entry
                        .updated_at
                        .cmp(&entries[a].entry.updated_at)
                });
            }
        }
    }

    /// Handle a search message, returning an Action for the app router.
    pub fn update(&mut self, msg: Message, db: Option<&DbHandle>) -> Action {
        match msg {
            Message::ClearQuery => {
                self.query.clear();
                if self.search_mode == SearchMode::Local {
                    self.refilter();
                }
                Action::None
            }
            Message::CloseDetail => {
                self.selected_anime = None;
                self.selected_online = None;
                Action::None
            }
            Message::QueryChanged(new_query) => {
                self.query = new_query;
                if self.search_mode == SearchMode::Local {
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
                }
                Action::None
            }
            Message::EntriesLoaded(result) => {
                if let Ok(entries) = result {
                    self.all_entries = entries;
                    self.loaded = true;
                    self.refilter();
                    // Re-sync stepper text buffers to the (possibly updated) selected entry
                    if let Some(id) = self.selected_anime {
                        if let Some(row) = self.all_entries.iter().find(|r| r.anime.id == id) {
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
            Message::AnimeSelected(id) => {
                self.selected_anime = Some(id);
                if let Some(row) = self.all_entries.iter().find(|r| r.anime.id == id) {
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
                        .all_entries
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
            Message::StatusFilterChanged(filter) => {
                self.status_filter = filter;
                self.refilter();
                Action::None
            }
            Message::SortChanged(sort) => {
                self.sort = sort;
                self.refilter();
                Action::None
            }
            // ── Online search messages ────────────────────────────
            Message::SearchOnline => {
                self.search_mode = SearchMode::Online;
                self.online_loading = true;
                self.online_error = None;
                self.online_results.clear();
                self.selected_online = None;
                // The actual MAL call is handled by app.rs
                Action::None
            }
            Message::OnlineResultsLoaded(result) => {
                self.online_loading = false;
                match result {
                    Ok(results) => {
                        self.online_results = results;
                        self.online_error = None;
                    }
                    Err(e) => {
                        self.online_error = Some(e);
                    }
                }
                Action::None
            }
            Message::BackToLocal => {
                self.search_mode = SearchMode::Local;
                self.selected_online = None;
                Action::None
            }
            Message::OnlineSelected(idx) => {
                self.selected_online = Some(idx);
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
                                app::Message::Search(Message::DbOperationDone(
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
                                app::Message::Search(Message::DbOperationDone(
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
                                app::Message::Search(Message::DbOperationDone(
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
                            app::Message::Search(Message::DbOperationDone(
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
                        .all_entries
                        .iter()
                        .find(|r| r.anime.id == anime_id)
                        .map(|r| r.entry.rewatching)
                        .unwrap_or(false);
                    return Action::RunTask(Task::perform(
                        async move { db.update_library_rewatch(anime_id, rewatching, count).await },
                        |r| {
                            app::Message::Search(Message::DbOperationDone(
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
            Message::AddToLibrary(_idx) => {
                // Handled by app.rs which has access to DB
                Action::None
            }
            Message::AddedToLibrary(result) => {
                match result {
                    Ok(()) => {
                        self.search_mode = SearchMode::Local;
                        self.selected_online = None;
                    }
                    Err(e) => {
                        self.online_error = Some(e);
                    }
                }
                // Reload local entries to reflect the new addition
                self.load_entries(db)
            }
        }
    }

    // ── View ──────────────────────────────────────────────────────

    pub fn view<'a>(&'a self, cs: &'a ColorScheme, covers: &'a CoverCache) -> Element<'a, Message> {
        match self.search_mode {
            SearchMode::Local => self.view_local(cs, covers),
            SearchMode::Online => self.view_online(cs, covers),
        }
    }

    fn view_local<'a>(
        &'a self,
        cs: &'a ColorScheme,
        covers: &'a CoverCache,
    ) -> Element<'a, Message> {
        let search_icon = lucide_icons::iced::icon_search()
            .size(style::TEXT_BASE)
            .color(cs.on_surface_variant);

        let search_input = text_input("Search library...", &self.query)
            .on_input(Message::QueryChanged)
            .size(style::TEXT_BASE)
            .padding([style::SPACE_XS, style::SPACE_SM])
            .width(Length::Fill)
            .style(theme::text_input_borderless(cs));

        let mut search_row = row![search_icon, search_input]
            .spacing(style::SPACE_SM)
            .align_y(Alignment::Center);

        if !self.query.is_empty() {
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
            .on_press(Message::ClearQuery)
            .padding(0)
            .width(Length::Fixed(clear_size))
            .height(Length::Fixed(clear_size))
            .style(theme::icon_button(cs));
            search_row = search_row.push(clear_btn);
        }

        let header = container(search_row)
            .style(theme::search_bar(cs))
            .padding([style::SPACE_SM, style::SPACE_MD])
            .width(Length::Fill);

        let header = container(header).padding([style::SPACE_SM, style::SPACE_LG]);

        // Status filter chip bar + result count + sort picker
        let result_count = format!(
            "{} {}",
            self.filtered_indices.len(),
            if self.filtered_indices.len() == 1 {
                "result"
            } else {
                "results"
            }
        );

        let filter_bar = row![
            status_chip_bar(cs, self.status_filter),
            text(result_count)
                .size(style::TEXT_XS)
                .color(cs.outline)
                .line_height(style::LINE_HEIGHT_LOOSE)
                .width(Length::Fill),
            pick_list(SearchSort::ALL, Some(self.sort), Message::SortChanged)
                .text_size(style::TEXT_SM)
                .padding([style::SPACE_SM, style::SPACE_MD])
                .style(theme::pick_list_style(cs))
                .menu_style(theme::pick_list_menu_style(cs)),
        ]
        .spacing(style::SPACE_SM)
        .align_y(Alignment::Center)
        .padding([style::SPACE_XS, style::SPACE_LG]);

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
            let msg = if self.query.is_empty() && self.status_filter.is_none() {
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
                .map(|&i| {
                    widgets::anime_list_item(
                        cs,
                        &self.all_entries[i],
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
        };

        // Online search prompt
        let mal_prompt = self.online_search_prompt(cs);

        let content = column![header, filter_bar, rule::horizontal(1), list, mal_prompt]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill);

        if let Some(anime_id) = self.selected_anime {
            if let Some(lib_row) = self.all_entries.iter().find(|r| r.anime.id == anime_id) {
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

    /// Render the "Search online" call-to-action at the bottom of local results.
    fn online_search_prompt(&self, cs: &ColorScheme) -> Element<'_, Message> {
        if self.query.trim().is_empty() {
            return container(text("").size(1)).height(Length::Shrink).into();
        }

        let inner: Element<'_, Message> = if self.service_authenticated {
            let label = format!("Search online for \"{}\"", self.query.trim());
            button(
                row![
                    lucide_icons::iced::icon_globe()
                        .size(style::TEXT_SM)
                        .center(),
                    text(label)
                        .size(style::TEXT_SM)
                        .line_height(style::LINE_HEIGHT_NORMAL),
                ]
                .spacing(style::SPACE_SM)
                .align_y(Alignment::Center),
            )
            .padding([style::SPACE_SM, style::SPACE_XL])
            .on_press(Message::SearchOnline)
            .style(theme::ghost_button(cs))
            .into()
        } else {
            text("Log in to a service in Settings to search online")
                .size(style::TEXT_XS)
                .color(cs.outline)
                .line_height(style::LINE_HEIGHT_LOOSE)
                .into()
        };

        container(inner)
            .padding([style::SPACE_SM, style::SPACE_LG])
            .width(Length::Fill)
            .center_x(Length::Fill)
            .into()
    }

    // ── Online view ───────────────────────────────────────────────

    fn view_online<'a>(
        &'a self,
        cs: &'a ColorScheme,
        covers: &'a CoverCache,
    ) -> Element<'a, Message> {
        // Header: back button + "MAL Results"
        let back = button(
            row![
                lucide_icons::iced::icon_arrow_left()
                    .size(style::TEXT_SM)
                    .center(),
                text("Back to library")
                    .size(style::TEXT_SM)
                    .line_height(style::LINE_HEIGHT_NORMAL),
            ]
            .spacing(style::SPACE_XS)
            .align_y(Alignment::Center),
        )
        .padding([style::SPACE_SM, style::SPACE_MD])
        .on_press(Message::BackToLocal)
        .style(theme::ghost_button(cs));

        let title_text = format!("MAL results for \"{}\"", self.query.trim());
        let header = row![
            back,
            text(title_text)
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant)
                .line_height(style::LINE_HEIGHT_LOOSE)
                .width(Length::Fill),
        ]
        .spacing(style::SPACE_SM)
        .align_y(Alignment::Center)
        .padding([style::SPACE_SM, style::SPACE_LG]);

        // Content area
        let body: Element<'_, Message> = if self.online_loading {
            container(
                text("Searching...")
                    .size(style::TEXT_SM)
                    .color(cs.on_surface_variant)
                    .line_height(style::LINE_HEIGHT_LOOSE),
            )
            .padding(style::SPACE_3XL)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .into()
        } else if let Some(err) = &self.online_error {
            container(
                column![
                    text(err.as_str())
                        .size(style::TEXT_SM)
                        .color(cs.error)
                        .line_height(style::LINE_HEIGHT_NORMAL),
                    button(text("Retry").size(style::TEXT_SM))
                        .padding([style::SPACE_SM, style::SPACE_XL])
                        .on_press(Message::SearchOnline)
                        .style(theme::ghost_button(cs)),
                ]
                .spacing(style::SPACE_MD)
                .align_x(Alignment::Center),
            )
            .padding(style::SPACE_3XL)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .into()
        } else if self.online_results.is_empty() {
            container(
                text("No results found.")
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
                .online_results
                .iter()
                .enumerate()
                .map(|(idx, result)| {
                    online_list_item(cs, result, idx, self.selected_online, covers)
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
        };

        let content = column![header, rule::horizontal(1), body]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill);

        // Show detail panel for selected online result
        if let Some(idx) = self.selected_online {
            if let Some(result) = self.online_results.get(idx) {
                let cover_key = online_cover_key(result.service_id);
                let detail = online_detail_panel(
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
}

// ── Helper functions ──────────────────────────────────────────────

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

/// Status filter chip bar with "All" option.
fn status_chip_bar(cs: &ColorScheme, active: Option<WatchStatus>) -> Element<'static, Message> {
    let mut chips: Vec<Element<'_, Message>> = Vec::with_capacity(6);

    // "All" chip
    let is_all = active.is_none();
    let all_chip = {
        let mut chip_content = row![].spacing(style::SPACE_XXS).align_y(Alignment::Center);
        if is_all {
            chip_content = chip_content.push(lucide_icons::iced::icon_check().size(style::TEXT_XS));
        }
        chip_content = chip_content.push(
            text("All")
                .size(style::TEXT_XS)
                .line_height(style::LINE_HEIGHT_LOOSE),
        );
        button(container(chip_content).center_y(Length::Fill))
            .height(Length::Fixed(style::CHIP_HEIGHT))
            .padding([style::SPACE_XS, style::SPACE_MD])
            .on_press(Message::StatusFilterChanged(None))
            .style(theme::filter_chip(is_all, cs))
    };
    chips.push(all_chip.into());

    // Status chips
    for &status in WatchStatus::ALL {
        let is_selected = active == Some(status);
        let label = match status {
            WatchStatus::PlanToWatch => "Plan".to_string(),
            other => other.as_str().to_string(),
        };
        let mut chip_content = row![].spacing(style::SPACE_XXS).align_y(Alignment::Center);
        if is_selected {
            chip_content = chip_content.push(lucide_icons::iced::icon_check().size(style::TEXT_XS));
        }
        chip_content = chip_content.push(
            text(label)
                .size(style::TEXT_XS)
                .line_height(style::LINE_HEIGHT_LOOSE),
        );

        chips.push(
            button(container(chip_content).center_y(Length::Fill))
                .height(Length::Fixed(style::CHIP_HEIGHT))
                .padding([style::SPACE_XS, style::SPACE_MD])
                .on_press(Message::StatusFilterChanged(Some(status)))
                .style(theme::filter_chip(is_selected, cs))
                .into(),
        );
    }

    row(chips).spacing(style::SPACE_XS).into()
}

/// Compute a cover cache key for an online result (negative to avoid colliding with local IDs).
pub fn online_cover_key(service_id: u64) -> i64 {
    -(service_id as i64)
}

/// A single online search result list item.
fn online_list_item<'a>(
    cs: &'a ColorScheme,
    result: &'a AnimeSearchResult,
    idx: usize,
    selected: Option<usize>,
    covers: &'a CoverCache,
) -> Element<'a, Message> {
    let is_selected = selected == Some(idx);
    let cover_key = online_cover_key(result.service_id);

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
    if let Some(year) = result.year {
        meta_parts.push(year.to_string());
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
            text(format!("\u{2605} {score:.2}"))
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
        .on_press(Message::OnlineSelected(idx))
        .style(theme::list_item(is_selected, cs))
        .into()
}
