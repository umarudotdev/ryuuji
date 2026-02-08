use iced::widget::{button, column, container, row, text};
use iced::window;
use iced::{Alignment, Element, Length, Subscription, Task, Theme};

use chrono::Utc;
use kurozumi_core::config::AppConfig;
use kurozumi_core::models::{
    Anime, AnimeIds, AnimeTitle, DetectedMedia, LibraryEntry, WatchStatus,
};
use kurozumi_core::orchestrator::UpdateOutcome;
use kurozumi_core::storage::LibraryRow;

use crate::cover_cache::{self, CoverCache, CoverState};
use crate::db::DbHandle;
use crate::screen::{library, now_playing, search, settings, Action, ModalKind, Page};
use crate::style;
use crate::subscription;
use kurozumi_core::config::ThemeMode;

use crate::theme::{self, ColorScheme, KurozumiTheme};
use crate::window_state::WindowState;

/// Application state — slim router that delegates to screens.
pub struct Kurozumi {
    page: Page,
    config: AppConfig,
    db: Option<DbHandle>,
    // Theme
    current_theme: KurozumiTheme,
    // Screens
    now_playing: now_playing::NowPlaying,
    library: library::Library,
    search: search::Search,
    settings: settings::Settings,
    // Cover images
    cover_cache: CoverCache,
    // App-level chrome
    modal_state: Option<ModalKind>,
    status_message: String,
    // Window persistence
    window_state: WindowState,
}

impl Default for Kurozumi {
    fn default() -> Self {
        let config = AppConfig::load().unwrap_or_default();
        let settings_screen = settings::Settings::from_config(&config);
        let db = AppConfig::ensure_db_path()
            .ok()
            .and_then(|path| DbHandle::open(&path));

        // Resolve initial theme from config.
        let theme_name = config.appearance.theme.as_str();
        let current_theme = theme::find_theme(theme_name)
            .unwrap_or_else(|| KurozumiTheme::for_mode(config.appearance.mode));

        Self {
            page: Page::default(),
            config,
            db,
            current_theme,
            now_playing: now_playing::NowPlaying::new(),
            library: library::Library::new(),
            search: search::Search::new(),
            settings: settings_screen,
            cover_cache: CoverCache::default(),
            modal_state: None,
            status_message: "Ready".into(),
            window_state: WindowState::load(),
        }
    }
}

/// All messages the application can handle.
#[derive(Debug, Clone)]
pub enum Message {
    NavigateTo(Page),
    CoverLoaded {
        anime_id: i64,
        result: Result<std::path::PathBuf, String>,
    },
    DetectionTick,
    DetectionResult(Option<DetectedMedia>),
    DetectionProcessed(Result<UpdateOutcome, String>),
    AppearanceChanged(ThemeMode),
    WindowEvent(window::Event),
    NowPlaying(now_playing::Message),
    Library(library::Message),
    Search(search::Message),
    Settings(settings::Message),
}

impl Kurozumi {
    pub fn new() -> (Self, Task<Message>) {
        let app = Self::default();
        // Check if MAL token exists on startup.
        let task = if let Some(db) = &app.db {
            let db = db.clone();
            Task::perform(
                async move { db.get_mal_token().await.ok().flatten().is_some() },
                |has_token| Message::Settings(settings::Message::MalTokenChecked(has_token)),
            )
        } else {
            Task::none()
        };
        (app, task)
    }

    pub fn title(&self) -> String {
        String::from("kurozumi")
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NavigateTo(page) => {
                self.page = page;
                if page == Page::Library {
                    let action = self.library.refresh_task(self.db.as_ref());
                    return self.handle_action(action);
                }
                if page == Page::Search {
                    self.search.mal_authenticated = self.settings.mal_authenticated;
                    let action = self.search.load_entries(self.db.as_ref());
                    return self.handle_action(action);
                }
                if page == Page::Settings {
                    let action = self.settings.load_stats(self.db.as_ref());
                    return self.handle_action(action);
                }
                Task::none()
            }
            Message::CoverLoaded { anime_id, result } => {
                match result {
                    Ok(path) => {
                        self.cover_cache
                            .states
                            .insert(anime_id, CoverState::Loaded(path));
                    }
                    Err(_) => {
                        self.cover_cache.states.insert(anime_id, CoverState::Failed);
                    }
                }
                Task::none()
            }
            Message::DetectionTick => Task::perform(detect_and_parse(), Message::DetectionResult),
            Message::DetectionResult(media) => {
                if media.is_none() {
                    self.now_playing.matched_row = None;
                }
                self.now_playing.detected = media.clone();
                if let (Some(db), Some(detected)) = (&self.db, media) {
                    let db = db.clone();
                    let config = self.config.clone();
                    return Task::perform(
                        async move {
                            db.process_detection(detected, config)
                                .await
                                .map_err(|e| e.to_string())
                        },
                        Message::DetectionProcessed,
                    );
                }
                Task::none()
            }
            Message::DetectionProcessed(result) => {
                let mut follow_up = Task::none();
                match result {
                    Ok(outcome) => {
                        self.status_message = match &outcome {
                            UpdateOutcome::Updated {
                                anime_title,
                                episode,
                                ..
                            } => {
                                format!("Updated {anime_title} to ep {episode}")
                            }
                            UpdateOutcome::AddedToLibrary {
                                anime_title,
                                episode,
                                ..
                            } => {
                                format!("Added {anime_title} (ep {episode}) to library")
                            }
                            UpdateOutcome::AlreadyCurrent { .. } => self.status_message.clone(),
                            UpdateOutcome::Unrecognized { raw_title } => {
                                format!("Unrecognized: {raw_title}")
                            }
                            UpdateOutcome::NothingPlaying => self.status_message.clone(),
                        };

                        // Fire follow-up query for matched anime details.
                        let anime_id = match &outcome {
                            UpdateOutcome::Updated { anime_id, .. }
                            | UpdateOutcome::AlreadyCurrent { anime_id, .. }
                            | UpdateOutcome::AddedToLibrary { anime_id, .. } => Some(*anime_id),
                            _ => None,
                        };
                        if let (Some(db), Some(id)) = (&self.db, anime_id) {
                            let db = db.clone();
                            follow_up = Task::perform(
                                async move { db.get_library_row(id).await.ok().flatten() },
                                |row| {
                                    Message::NowPlaying(now_playing::Message::LibraryRowFetched(
                                        Box::new(row),
                                    ))
                                },
                            );
                        }

                        self.now_playing.last_outcome = Some(outcome);
                    }
                    Err(e) => {
                        self.status_message = format!("Error: {e}");
                    }
                }
                if self.page == Page::Library {
                    let action = self.library.refresh_task(self.db.as_ref());
                    let lib_task = self.handle_action(action);
                    return Task::batch([follow_up, lib_task]);
                }
                if self.page == Page::Search {
                    let action = self.search.load_entries(self.db.as_ref());
                    let search_task = self.handle_action(action);
                    return Task::batch([follow_up, search_task]);
                }
                follow_up
            }
            Message::AppearanceChanged(mode) => {
                // System theme changed — pick the matching default theme.
                let target = KurozumiTheme::for_mode(mode);
                if target.name != self.current_theme.name {
                    self.current_theme = target;
                }
                Task::none()
            }
            Message::WindowEvent(event) => {
                match event {
                    window::Event::Resized(size) => {
                        self.window_state.width = size.width;
                        self.window_state.height = size.height;
                        self.window_state.save();
                    }
                    window::Event::Moved(pos) => {
                        self.window_state.x = pos.x;
                        self.window_state.y = pos.y;
                        self.window_state.save();
                    }
                    _ => {}
                }
                Task::none()
            }
            Message::NowPlaying(msg) => match msg {
                now_playing::Message::LibraryRowFetched(row) => {
                    let row = *row;
                    let cover_task = if let Some(r) = &row {
                        self.request_cover(r.anime.id, r.anime.cover_url.as_deref())
                    } else {
                        Task::none()
                    };
                    self.now_playing.matched_row = row;
                    cover_task
                }
            },
            Message::Library(msg) => {
                // Request cover for newly selected anime.
                let cover_task = match &msg {
                    library::Message::AnimeSelected(id) => {
                        let info = self
                            .library
                            .entries
                            .iter()
                            .find(|r| r.anime.id == *id)
                            .map(|r| (r.anime.id, r.anime.cover_url.clone()));
                        if let Some((id, url)) = info {
                            self.request_cover(id, url.as_deref())
                        } else {
                            Task::none()
                        }
                    }
                    _ => Task::none(),
                };
                let action = self.library.update(msg, self.db.as_ref());
                let action_task = self.handle_action(action);
                // Batch-request covers for all visible entries.
                let info = Self::cover_info_from_rows(&self.library.entries);
                let batch_covers = self.batch_request_covers(info);
                Task::batch([cover_task, action_task, batch_covers])
            }
            Message::Search(msg) => {
                // Intercept messages that need app-level access.
                match &msg {
                    search::Message::SearchOnline => {
                        let query = self.search.query().to_string();
                        self.search.update(msg, self.db.as_ref());
                        return self.spawn_mal_search(query);
                    }
                    search::Message::AddToLibrary(idx) => {
                        let idx = *idx;
                        if let Some(result) = self.search.online_results.get(idx).cloned() {
                            return self.spawn_add_to_library(result);
                        }
                        return Task::none();
                    }
                    search::Message::OnlineResultsLoaded(_) => {
                        // After online results load, batch-request covers.
                        let action = self.search.update(msg, self.db.as_ref());
                        let action_task = self.handle_action(action);
                        let online_covers: Vec<(i64, Option<String>)> = self
                            .search
                            .online_results
                            .iter()
                            .map(|r| (search::online_cover_key(r.service_id), r.cover_url.clone()))
                            .collect();
                        let batch = self.batch_request_covers(online_covers);
                        return Task::batch([action_task, batch]);
                    }
                    _ => {}
                }

                // Request cover for newly selected anime.
                let cover_task = match &msg {
                    search::Message::AnimeSelected(id) => {
                        let info = self
                            .search
                            .all_entries
                            .iter()
                            .find(|r| r.anime.id == *id)
                            .map(|r| (r.anime.id, r.anime.cover_url.clone()));
                        if let Some((id, url)) = info {
                            self.request_cover(id, url.as_deref())
                        } else {
                            Task::none()
                        }
                    }
                    search::Message::OnlineSelected(idx) => {
                        let info =
                            self.search.online_results.get(*idx).map(|r| {
                                (search::online_cover_key(r.service_id), r.cover_url.clone())
                            });
                        if let Some((key, url)) = info {
                            self.request_cover(key, url.as_deref())
                        } else {
                            Task::none()
                        }
                    }
                    _ => Task::none(),
                };
                let action = self.search.update(msg, self.db.as_ref());
                let action_task = self.handle_action(action);
                // Batch-request covers for all entries.
                let info = Self::cover_info_from_rows(&self.search.all_entries);
                let batch_covers = self.batch_request_covers(info);
                Task::batch([cover_task, action_task, batch_covers])
            }
            Message::Settings(ref msg) => {
                // Intercept async actions before delegating to settings.
                match msg {
                    settings::Message::MalLogin => {
                        let msg = msg.clone();
                        self.settings.update(msg, &mut self.config);
                        self.spawn_mal_login()
                    }
                    settings::Message::MalImport => {
                        let msg = msg.clone();
                        self.settings.update(msg, &mut self.config);
                        self.spawn_mal_import()
                    }
                    settings::Message::ExportLibrary => {
                        let msg = msg.clone();
                        self.settings.update(msg, &mut self.config);
                        self.spawn_library_export()
                    }
                    _ => {
                        let msg = msg.clone();
                        let action = self.settings.update(msg, &mut self.config);
                        // Sync theme from settings if changed.
                        let wanted = &self.settings.selected_theme;
                        if wanted != &self.current_theme.name {
                            if let Some(new_theme) = theme::find_theme(wanted) {
                                self.current_theme = new_theme;
                            }
                        }
                        self.handle_action(action)
                    }
                }
            }
        }
    }

    /// Spawn the MAL OAuth login flow as an async task.
    fn spawn_mal_login(&self) -> Task<Message> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        let client_id = self.settings.mal_client_id.trim().to_string();
        if client_id.is_empty() {
            return Task::none();
        }

        Task::perform(
            async move {
                // Run the OAuth PKCE flow (opens browser, waits for redirect).
                let token_resp = kurozumi_api::mal::auth::authorize(&client_id)
                    .await
                    .map_err(|e| e.to_string())?;

                // Save tokens to DB.
                let expires_at = token_resp
                    .expires_in
                    .map(|secs| (Utc::now() + chrono::Duration::seconds(secs as i64)).to_rfc3339());
                db.save_mal_token(
                    token_resp.access_token,
                    token_resp.refresh_token,
                    expires_at,
                )
                .await
                .map_err(|e| e.to_string())?;

                Ok(())
            },
            |result| Message::Settings(settings::Message::MalLoginResult(result)),
        )
    }

    /// Spawn the MAL library import flow as an async task.
    fn spawn_mal_import(&self) -> Task<Message> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        let client_id = self.settings.mal_client_id.trim().to_string();
        if client_id.is_empty() {
            return Task::none();
        }

        Task::perform(
            async move {
                // Load token from DB.
                let token = db
                    .get_mal_token()
                    .await
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| "Not logged in to MAL".to_string())?;

                // Fetch user's full anime list from MAL (with all title variants).
                let client = kurozumi_api::mal::MalClient::new(client_id, token);
                let mal_items = client
                    .get_user_list_full()
                    .await
                    .map_err(|e| e.to_string())?;

                // Map MAL items directly to local types, preserving all title data.
                let batch: Vec<(Anime, Option<LibraryEntry>)> = mal_items
                    .into_iter()
                    .map(|item| {
                        let alt = &item.node.alternative_titles;
                        let season = item.node.start_season.as_ref().map(|s| {
                            let mut c = s.season.chars();
                            match c.next() {
                                Some(first) => first.to_uppercase().to_string() + c.as_str(),
                                None => s.season.clone(),
                            }
                        });
                        let year = item.node.start_season.as_ref().map(|s| s.year);

                        let anime = Anime {
                            id: 0,
                            ids: AnimeIds {
                                anilist: None,
                                kitsu: None,
                                mal: Some(item.node.id),
                            },
                            title: AnimeTitle {
                                romaji: Some(item.node.title.clone()),
                                english: alt.as_ref().and_then(|a| a.en.clone()),
                                native: alt.as_ref().and_then(|a| a.ja.clone()),
                            },
                            synonyms: alt
                                .as_ref()
                                .and_then(|a| a.synonyms.clone())
                                .unwrap_or_default(),
                            episodes: item.node.num_episodes,
                            cover_url: item
                                .node
                                .main_picture
                                .as_ref()
                                .and_then(|p| p.medium.clone()),
                            season,
                            year,
                            synopsis: item.node.synopsis.clone(),
                            genres: item
                                .node
                                .genres
                                .as_ref()
                                .map(|g| g.iter().map(|x| x.name.clone()).collect())
                                .unwrap_or_default(),
                            media_type: item.node.media_type.clone(),
                            airing_status: item.node.status.clone(),
                            mean_score: item.node.mean,
                            studios: item
                                .node
                                .studios
                                .as_ref()
                                .map(|s| s.iter().map(|x| x.name.clone()).collect())
                                .unwrap_or_default(),
                            source: item.node.source.clone(),
                            rating: item.node.rating.clone(),
                            start_date: item.node.start_date.clone(),
                            end_date: item.node.end_date.clone(),
                        };

                        let status_str = item.list_status.status.as_deref().unwrap_or("watching");
                        let status =
                            WatchStatus::from_db_str(status_str).unwrap_or(WatchStatus::Watching);
                        let library_entry = LibraryEntry {
                            id: 0,
                            anime_id: 0, // will be set by DB actor
                            status,
                            watched_episodes: item.list_status.num_episodes_watched.unwrap_or(0),
                            score: item.list_status.score.map(|s| s as f32),
                            updated_at: Utc::now(),
                        };

                        (anime, Some(library_entry))
                    })
                    .collect();

                // Bulk upsert into DB.
                db.mal_import_batch(batch).await.map_err(|e| e.to_string())
            },
            |result| Message::Settings(settings::Message::MalImportResult(result)),
        )
    }

    /// Spawn a library JSON export as an async task.
    fn spawn_library_export(&self) -> Task<Message> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };

        Task::perform(
            async move {
                let rows = db.get_all_library().await.map_err(|e| e.to_string())?;
                let json = serde_json::to_string_pretty(&rows).map_err(|e| e.to_string())?;

                let data_dir = AppConfig::db_path()
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| std::path::PathBuf::from("."));
                let export_path = data_dir.join("kurozumi-export.json");
                std::fs::write(&export_path, json).map_err(|e| e.to_string())?;
                Ok(export_path.display().to_string())
            },
            |result| Message::Settings(settings::Message::ExportResult(result)),
        )
    }

    /// Spawn an online MAL search as an async task.
    fn spawn_mal_search(&self, query: String) -> Task<Message> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        let client_id = self.settings.mal_client_id.trim().to_string();
        if client_id.is_empty() {
            return Task::none();
        }

        Task::perform(
            async move {
                use kurozumi_api::traits::AnimeService;

                let token = db
                    .get_mal_token()
                    .await
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| "Not logged in to MAL".to_string())?;

                let client = kurozumi_api::mal::MalClient::new(client_id, token);
                client.search_anime(&query).await.map_err(|e| e.to_string())
            },
            |result| Message::Search(search::Message::OnlineResultsLoaded(result)),
        )
    }

    /// Add an online search result to the local library.
    fn spawn_add_to_library(
        &self,
        result: kurozumi_api::traits::AnimeSearchResult,
    ) -> Task<Message> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };

        Task::perform(
            async move {
                let anime = Anime {
                    id: 0,
                    ids: AnimeIds {
                        anilist: None,
                        kitsu: None,
                        mal: Some(result.service_id),
                    },
                    title: AnimeTitle {
                        romaji: Some(result.title.clone()),
                        english: result.title_english.clone(),
                        native: None,
                    },
                    synonyms: Vec::new(),
                    episodes: result.episodes,
                    cover_url: result.cover_url.clone(),
                    season: result.season.clone(),
                    year: result.year,
                    synopsis: result.synopsis.clone(),
                    genres: result.genres.clone(),
                    media_type: result.media_type.clone(),
                    airing_status: result.status.clone(),
                    mean_score: result.mean_score,
                    studios: Vec::new(),
                    source: None,
                    rating: None,
                    start_date: None,
                    end_date: None,
                };

                let entry = LibraryEntry {
                    id: 0,
                    anime_id: 0,
                    status: WatchStatus::PlanToWatch,
                    watched_episodes: 0,
                    score: None,
                    updated_at: Utc::now(),
                };

                db.mal_import_batch(vec![(anime, Some(entry))])
                    .await
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            },
            |result| Message::Search(search::Message::AddedToLibrary(result)),
        )
    }

    /// Interpret an Action returned by a screen.
    fn handle_action(&mut self, action: Action) -> Task<Message> {
        match action {
            Action::None => Task::none(),
            Action::NavigateTo(page) => {
                self.page = page;
                Task::none()
            }
            Action::RefreshLibrary => {
                let action = self.library.refresh_task(self.db.as_ref());
                self.handle_action(action)
            }
            Action::SetStatus(msg) => {
                self.status_message = msg;
                Task::none()
            }
            Action::ShowModal(kind) => {
                self.modal_state = Some(kind);
                Task::none()
            }
            Action::DismissModal => {
                self.modal_state = None;
                Task::none()
            }
            Action::RunTask(task) => task,
        }
    }

    /// Batch-request cover downloads for a set of (anime_id, cover_url) pairs.
    fn batch_request_covers(&mut self, items: Vec<(i64, Option<String>)>) -> Task<Message> {
        let tasks: Vec<Task<Message>> = items
            .into_iter()
            .map(|(id, url)| self.request_cover(id, url.as_deref()))
            .collect();
        if tasks.is_empty() {
            Task::none()
        } else {
            Task::batch(tasks)
        }
    }

    /// Extract (anime_id, cover_url) pairs from library rows for batch cover requests.
    fn cover_info_from_rows(rows: &[LibraryRow]) -> Vec<(i64, Option<String>)> {
        rows.iter()
            .map(|r| (r.anime.id, r.anime.cover_url.clone()))
            .collect()
    }

    /// Request a cover image download for an anime if not already requested.
    fn request_cover(&mut self, anime_id: i64, cover_url: Option<&str>) -> Task<Message> {
        let Some(url) = cover_url else {
            // No cover URL available — mark as failed so the placeholder renders.
            self.cover_cache
                .states
                .entry(anime_id)
                .or_insert(CoverState::Failed);
            return Task::none();
        };
        if self.cover_cache.states.contains_key(&anime_id) {
            return Task::none();
        }
        // Check disk cache first.
        let path = cover_cache::cover_path(anime_id);
        if path.exists() {
            self.cover_cache
                .states
                .insert(anime_id, CoverState::Loaded(path));
            return Task::none();
        }
        self.cover_cache
            .states
            .insert(anime_id, CoverState::Loading);
        let url = url.to_string();
        Task::perform(
            async move { cover_cache::fetch_cover(anime_id, url).await },
            move |result| Message::CoverLoaded { anime_id, result },
        )
    }

    pub fn view(&self) -> Element<'_, Message> {
        let cs = &self.current_theme.colors;
        let nav = self.nav_rail(cs);

        let page_content: Element<'_, Message> = match self.page {
            Page::NowPlaying => self
                .now_playing
                .view(cs, &self.cover_cache)
                .map(Message::NowPlaying),
            Page::Library => self
                .library
                .view(cs, &self.cover_cache)
                .map(Message::Library),
            Page::Search => self.search.view(cs, &self.cover_cache).map(Message::Search),
            Page::Settings => self.settings.view(cs).map(Message::Settings),
        };

        let status_bar = container(
            text(&self.status_message)
                .size(style::TEXT_XS)
                .line_height(style::LINE_HEIGHT_LOOSE),
        )
        .style(theme::status_bar(cs))
        .width(Length::Fill)
        .height(Length::Fixed(style::STATUS_BAR_HEIGHT))
        .padding([4.0, style::SPACE_MD]);

        let main: Element<'_, Message> =
            column![row![nav, page_content].height(Length::Fill), status_bar,].into();

        // Wrap in modal if one is active.
        if let Some(modal_kind) = &self.modal_state {
            let modal_content = self.build_modal_content(cs, modal_kind);
            let dismiss_msg = match modal_kind {
                ModalKind::ConfirmDelete { source, .. } => match source {
                    Page::Search => Message::Search(search::Message::CancelModal),
                    _ => Message::Library(library::Message::CancelModal),
                },
            };
            crate::widgets::modal(main, modal_content, dismiss_msg)
        } else {
            main
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        subscription::subscriptions(
            self.config.general.detection_interval.max(1),
            self.config.appearance.mode,
        )
    }

    pub fn theme(&self) -> Theme {
        self.current_theme.iced_theme()
    }

    fn build_modal_content<'a>(
        &self,
        cs: &ColorScheme,
        kind: &'a ModalKind,
    ) -> Element<'a, Message> {
        match kind {
            ModalKind::ConfirmDelete {
                anime_id,
                title,
                source,
            } => {
                let anime_id = *anime_id;
                let source = *source;
                let cancel_msg = match source {
                    Page::Search => Message::Search(search::Message::CancelModal),
                    _ => Message::Library(library::Message::CancelModal),
                };
                let confirm_msg = match source {
                    Page::Search => Message::Search(search::Message::ConfirmDelete(anime_id)),
                    _ => Message::Library(library::Message::ConfirmDelete(anime_id)),
                };
                container(
                    column![
                        text("Remove from library?")
                            .size(style::TEXT_LG)
                            .font(style::FONT_HEADING)
                            .line_height(style::LINE_HEIGHT_TIGHT),
                        text(title.as_str())
                            .size(style::TEXT_SM)
                            .color(cs.on_surface_variant)
                            .line_height(style::LINE_HEIGHT_LOOSE),
                        text("This action cannot be undone.")
                            .size(style::TEXT_XS)
                            .color(cs.outline)
                            .line_height(style::LINE_HEIGHT_LOOSE),
                        row![
                            button(text("Cancel").size(style::TEXT_SM))
                                .padding([style::SPACE_SM, style::SPACE_XL])
                                .on_press(cancel_msg)
                                .style(theme::ghost_button(cs)),
                            button(text("Delete").size(style::TEXT_SM))
                                .padding([style::SPACE_SM, style::SPACE_XL])
                                .on_press(confirm_msg)
                                .style(theme::danger_button(cs)),
                        ]
                        .spacing(style::SPACE_SM),
                    ]
                    .spacing(style::SPACE_LG),
                )
                .style(theme::dialog_container(cs))
                .padding(style::SPACE_2XL)
                .into()
            }
        }
    }

    fn nav_rail<'a>(&'a self, cs: &ColorScheme) -> Element<'a, Message> {
        let nav_item = |icon: iced::widget::Text<'static>, label: &'static str, page: Page| {
            let active = self.page == page;
            button(
                column![
                    icon.size(style::NAV_ICON_SIZE).center(),
                    text(label)
                        .size(style::NAV_LABEL_SIZE)
                        .line_height(style::LINE_HEIGHT_LOOSE)
                        .center(),
                ]
                .align_x(Alignment::Center)
                .spacing(style::SPACE_XXS)
                .width(Length::Fill),
            )
            .width(Length::Fixed(64.0))
            .padding([style::SPACE_SM, style::SPACE_XS])
            .on_press(Message::NavigateTo(page))
            .style(theme::nav_rail_item(active, cs))
        };

        use lucide_icons::iced as icons;

        let rail = column![
            column![
                nav_item(icons::icon_play(), "Playing", Page::NowPlaying),
                nav_item(icons::icon_library(), "Library", Page::Library),
                nav_item(icons::icon_search(), "Search", Page::Search),
            ]
            .spacing(style::SPACE_XS)
            .align_x(Alignment::Center),
            iced::widget::Space::new().height(Length::Fill),
            container(nav_item(icons::icon_settings(), "Settings", Page::Settings))
                .align_x(Alignment::Center)
                .width(Length::Fill)
                .padding(iced::Padding::new(0.0).bottom(style::SPACE_SM)),
        ]
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill);

        container(rail)
            .style(theme::nav_rail_bg(cs))
            .width(Length::Fixed(style::NAV_RAIL_WIDTH))
            .height(Length::Fill)
            .padding(iced::Padding::new(0.0).top(style::SPACE_LG))
            .into()
    }
}

/// Perform media detection and filename parsing off the main thread.
async fn detect_and_parse() -> Option<DetectedMedia> {
    let players = kurozumi_detect::detect_players();
    let player = players.into_iter().next()?;

    let raw_title = player
        .file_path
        .as_deref()
        .and_then(|p| {
            std::path::Path::new(p)
                .file_name()
                .and_then(|f| f.to_str())
                .map(|s| s.to_string())
        })
        .or_else(|| player.media_title.clone())?;

    let parsed = kurozumi_parse::parse(&raw_title);

    Some(DetectedMedia {
        player_name: player.player_name,
        anime_title: parsed.title,
        episode: parsed.episode_number,
        release_group: parsed.release_group,
        resolution: parsed.resolution,
        raw_title,
    })
}
