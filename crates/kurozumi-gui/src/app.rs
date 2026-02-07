use iced::widget::{button, column, container, row, text, Rule};
use iced::{Element, Length, Subscription, Task, Theme};

use kurozumi_core::config::AppConfig;
use kurozumi_core::models::{DetectedMedia, WatchStatus};
use kurozumi_core::orchestrator::{self, UpdateOutcome};
use kurozumi_core::storage::{LibraryRow, Storage};

use crate::pages;
use crate::subscription;

/// Which page is currently displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    NowPlaying,
    Library,
    Search,
    Settings,
}

/// Application state.
pub struct Kurozumi {
    page: Page,
    config: AppConfig,
    storage: Option<Storage>,
    // Now Playing state
    detected: Option<DetectedMedia>,
    last_outcome: Option<UpdateOutcome>,
    // Library state
    library_tab: WatchStatus,
    library_entries: Vec<LibraryRow>,
    selected_anime: Option<i64>,
    // Status bar
    status_message: String,
}

impl Default for Kurozumi {
    fn default() -> Self {
        let config = AppConfig::load().unwrap_or_default();
        let storage = AppConfig::ensure_db_path()
            .ok()
            .and_then(|path| {
                Storage::open(&path)
                    .map_err(|e| tracing::error!("Failed to open database: {e}"))
                    .ok()
            });

        Self {
            page: Page::default(),
            config,
            storage,
            detected: None,
            last_outcome: None,
            library_tab: WatchStatus::Watching,
            library_entries: Vec::new(),
            selected_anime: None,
            status_message: String::new(),
        }
    }
}

/// All messages the application can handle.
#[derive(Debug, Clone)]
pub enum Message {
    // Navigation
    NavigateTo(Page),
    // Detection
    DetectionTick,
    DetectionResult(Option<DetectedMedia>),
    // Library
    LibraryTabChanged(WatchStatus),
    AnimeSelected(i64),
    EpisodeIncrement(i64),
    EpisodeDecrement(i64),
}

impl Kurozumi {
    pub fn title(&self) -> String {
        String::from("kurozumi")
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NavigateTo(page) => {
                self.page = page;
                if page == Page::Library {
                    self.refresh_library();
                }
                Task::none()
            }
            Message::DetectionTick => {
                Task::perform(detect_and_parse(), Message::DetectionResult)
            }
            Message::DetectionResult(media) => {
                self.detected = media.clone();
                // Run orchestrator if we have storage and a detection.
                if let (Some(storage), Some(detected)) = (&self.storage, &media) {
                    match orchestrator::process_detection(detected, storage, &self.config) {
                        Ok(outcome) => {
                            self.status_message = match &outcome {
                                UpdateOutcome::Updated { anime_title, episode } => {
                                    format!("Updated {anime_title} to ep {episode}")
                                }
                                UpdateOutcome::AddedToLibrary { anime_title, episode } => {
                                    format!("Added {anime_title} (ep {episode}) to library")
                                }
                                UpdateOutcome::AlreadyCurrent { .. } => String::new(),
                                UpdateOutcome::Unrecognized { raw_title } => {
                                    format!("Unrecognized: {raw_title}")
                                }
                                UpdateOutcome::NothingPlaying => String::new(),
                            };
                            self.last_outcome = Some(outcome);
                        }
                        Err(e) => {
                            self.status_message = format!("Error: {e}");
                        }
                    }
                    // Refresh library if we're on that page.
                    if self.page == Page::Library {
                        self.refresh_library();
                    }
                }
                Task::none()
            }
            Message::LibraryTabChanged(status) => {
                self.library_tab = status;
                self.selected_anime = None;
                self.refresh_library();
                Task::none()
            }
            Message::AnimeSelected(id) => {
                self.selected_anime = Some(id);
                Task::none()
            }
            Message::EpisodeIncrement(anime_id) => {
                if let Some(storage) = &self.storage {
                    if let Some(entry) = self
                        .library_entries
                        .iter()
                        .find(|r| r.anime.id == anime_id)
                    {
                        let new_ep = entry.entry.watched_episodes + 1;
                        let _ = storage.update_episode_count(anime_id, new_ep);
                        let _ = storage.record_watch(anime_id, new_ep);
                        self.refresh_library();
                    }
                }
                Task::none()
            }
            Message::EpisodeDecrement(anime_id) => {
                if let Some(storage) = &self.storage {
                    if let Some(entry) = self
                        .library_entries
                        .iter()
                        .find(|r| r.anime.id == anime_id)
                    {
                        if entry.entry.watched_episodes > 0 {
                            let new_ep = entry.entry.watched_episodes - 1;
                            let _ = storage.update_episode_count(anime_id, new_ep);
                            self.refresh_library();
                        }
                    }
                }
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let sidebar = self.sidebar();

        let page_content: Element<'_, Message> = match self.page {
            Page::NowPlaying => pages::now_playing::view(&self.detected, &self.status_message),
            Page::Library => {
                pages::library::view(self.library_tab, &self.library_entries, self.selected_anime)
            }
            Page::Search => container(text("Search — coming in Phase 3"))
                .padding(20)
                .into(),
            Page::Settings => container(text("Settings — coming in Phase 2"))
                .padding(20)
                .into(),
        };

        row![sidebar, Rule::vertical(1), page_content]
            .height(Length::Fill)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        subscription::detection_tick(self.config.general.detection_interval.max(1))
    }

    pub fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let nav_button = |label: &'static str, page: Page, active: bool| {
            let btn = button(text(label).size(14))
                .width(Length::Fill)
                .on_press(Message::NavigateTo(page));
            if active {
                btn.style(button::primary)
            } else {
                btn.style(button::text)
            }
        };

        let sidebar = column![
            text("kurozumi").size(18),
            Rule::horizontal(1),
            nav_button("Now Playing", Page::NowPlaying, self.page == Page::NowPlaying),
            nav_button("Library", Page::Library, self.page == Page::Library),
            nav_button("Search", Page::Search, self.page == Page::Search),
            nav_button("Settings", Page::Settings, self.page == Page::Settings),
        ]
        .spacing(4)
        .padding(10)
        .width(Length::Fixed(160.0));

        container(sidebar).height(Length::Fill).into()
    }

    fn refresh_library(&mut self) {
        if let Some(storage) = &self.storage {
            self.library_entries = storage
                .get_library_by_status(self.library_tab)
                .unwrap_or_default();
        }
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
