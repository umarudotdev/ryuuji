use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length, Subscription, Task, Theme};

use kurozumi_core::config::AppConfig;
use kurozumi_core::models::{DetectedMedia, WatchStatus};
use kurozumi_core::orchestrator::{self, UpdateOutcome};
use kurozumi_core::storage::{LibraryRow, Storage};

use crate::pages;
use crate::pages::settings::SettingsState;
use crate::style;
use crate::subscription;
use crate::theme::{self, ColorScheme, ThemeMode};

/// Which page is currently displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    NowPlaying,
    Library,
    Search,
    Settings,
}

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

/// Application state.
pub struct Kurozumi {
    page: Page,
    config: AppConfig,
    storage: Option<Storage>,
    // Theme
    theme_mode: ThemeMode,
    colors: ColorScheme,
    // Now Playing state
    detected: Option<DetectedMedia>,
    last_outcome: Option<UpdateOutcome>,
    // Library state
    library_tab: WatchStatus,
    library_entries: Vec<LibraryRow>,
    selected_anime: Option<i64>,
    library_sort: LibrarySort,
    library_view_mode: LibraryViewMode,
    score_input: String,
    // Settings state
    settings: SettingsState,
    // Modal state
    modal_state: Option<ModalKind>,
    // Status bar
    status_message: String,
}

impl Default for Kurozumi {
    fn default() -> Self {
        let config = AppConfig::load().unwrap_or_default();
        let settings = SettingsState::from_config(&config);
        let storage = AppConfig::ensure_db_path()
            .ok()
            .and_then(|path| {
                Storage::open(&path)
                    .map_err(|e| tracing::error!("Failed to open database: {e}"))
                    .ok()
            });

        let theme_mode = ThemeMode::default();
        let colors = ColorScheme::for_mode(theme_mode);

        Self {
            page: Page::default(),
            config,
            storage,
            theme_mode,
            colors,
            detected: None,
            last_outcome: None,
            library_tab: WatchStatus::Watching,
            library_entries: Vec::new(),
            selected_anime: None,
            library_sort: LibrarySort::default(),
            library_view_mode: LibraryViewMode::default(),
            score_input: String::new(),
            settings,
            modal_state: None,
            status_message: "Ready".into(),
        }
    }
}

/// All messages the application can handle.
#[derive(Debug, Clone)]
pub enum Message {
    NavigateTo(Page),
    DetectionTick,
    DetectionResult(Option<DetectedMedia>),
    Library(LibraryMsg),
    Settings(SettingsMsg),
}

/// Library page messages.
#[derive(Debug, Clone)]
pub enum LibraryMsg {
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
}

/// Actions available in the library context menu.
#[derive(Debug, Clone)]
pub enum ContextAction {
    ChangeStatus(WatchStatus),
    Delete,
}

/// What kind of modal is currently shown.
#[derive(Debug, Clone)]
pub enum ModalKind {
    ConfirmDelete { anime_id: i64, title: String },
}

/// Settings page messages.
#[derive(Debug, Clone)]
pub enum SettingsMsg {
    IntervalChanged(String),
    AutoUpdateToggled(bool),
    ConfirmUpdateToggled(bool),
    MalEnabledToggled(bool),
    MalClientIdChanged(String),
    ThemeModeChanged(ThemeMode),
    Save,
}

impl Kurozumi {
    pub fn new() -> (Self, Task<Message>) {
        (Self::default(), Task::none())
    }

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
                                UpdateOutcome::AlreadyCurrent { .. } => self.status_message.clone(),
                                UpdateOutcome::Unrecognized { raw_title } => {
                                    format!("Unrecognized: {raw_title}")
                                }
                                UpdateOutcome::NothingPlaying => self.status_message.clone(),
                            };
                            self.last_outcome = Some(outcome);
                        }
                        Err(e) => {
                            self.status_message = format!("Error: {e}");
                        }
                    }
                    if self.page == Page::Library {
                        self.refresh_library();
                    }
                }
                Task::none()
            }
            Message::Library(msg) => self.handle_library(msg),
            Message::Settings(msg) => self.handle_settings(msg),
        }
    }

    fn handle_library(&mut self, msg: LibraryMsg) -> Task<Message> {
        match msg {
            LibraryMsg::TabChanged(status) => {
                self.library_tab = status;
                self.selected_anime = None;
                self.refresh_library();
            }
            LibraryMsg::AnimeSelected(id) => {
                self.selected_anime = Some(id);
                // Pre-fill score input from existing entry.
                if let Some(row) = self.library_entries.iter().find(|r| r.anime.id == id) {
                    self.score_input = row
                        .entry
                        .score
                        .map(|s| format!("{s:.0}"))
                        .unwrap_or_default();
                }
            }
            LibraryMsg::EpisodeIncrement(anime_id) => {
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
            }
            LibraryMsg::EpisodeDecrement(anime_id) => {
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
            }
            LibraryMsg::StatusChanged(anime_id, new_status) => {
                if let Some(storage) = &self.storage {
                    let _ = storage.update_library_status(anime_id, new_status);
                    self.refresh_library();
                }
            }
            LibraryMsg::ScoreInputChanged(val) => {
                self.score_input = val;
            }
            LibraryMsg::ScoreSubmitted(anime_id) => {
                if let Some(storage) = &self.storage {
                    if let Ok(score) = self.score_input.parse::<f32>() {
                        let score = score.clamp(0.0, 10.0);
                        let _ = storage.update_library_score(anime_id, score);
                        self.refresh_library();
                    }
                }
            }
            LibraryMsg::SortChanged(sort) => {
                self.library_sort = sort;
                self.refresh_library();
            }
            LibraryMsg::ViewModeToggled => {
                self.library_view_mode = match self.library_view_mode {
                    LibraryViewMode::Grid => LibraryViewMode::List,
                    LibraryViewMode::List => LibraryViewMode::Grid,
                };
            }
            LibraryMsg::ContextAction(anime_id, action) => match action {
                ContextAction::ChangeStatus(new_status) => {
                    if let Some(storage) = &self.storage {
                        let _ = storage.update_library_status(anime_id, new_status);
                        self.refresh_library();
                    }
                }
                ContextAction::Delete => {
                    let title = self
                        .library_entries
                        .iter()
                        .find(|r| r.anime.id == anime_id)
                        .map(|r| r.anime.title.preferred().to_string())
                        .unwrap_or_else(|| "this anime".into());
                    self.modal_state =
                        Some(ModalKind::ConfirmDelete { anime_id, title });
                }
            },
            LibraryMsg::ConfirmDelete(anime_id) => {
                if let Some(storage) = &self.storage {
                    let _ = storage.delete_library_entry(anime_id);
                    self.modal_state = None;
                    if self.selected_anime == Some(anime_id) {
                        self.selected_anime = None;
                    }
                    self.refresh_library();
                    self.status_message = "Entry removed from library.".into();
                }
            }
            LibraryMsg::CancelModal => {
                self.modal_state = None;
            }
        }
        Task::none()
    }

    fn handle_settings(&mut self, msg: SettingsMsg) -> Task<Message> {
        match msg {
            SettingsMsg::IntervalChanged(val) => self.settings.interval_input = val,
            SettingsMsg::AutoUpdateToggled(val) => self.settings.auto_update = val,
            SettingsMsg::ConfirmUpdateToggled(val) => self.settings.confirm_update = val,
            SettingsMsg::MalEnabledToggled(val) => self.settings.mal_enabled = val,
            SettingsMsg::MalClientIdChanged(val) => self.settings.mal_client_id = val,
            SettingsMsg::ThemeModeChanged(mode) => {
                self.theme_mode = mode;
                self.colors = ColorScheme::for_mode(mode);
                self.settings.theme_mode = mode;
            }
            SettingsMsg::Save => {
                self.settings.apply_to_config(&mut self.config);
                match self.config.save() {
                    Ok(()) => self.settings.status_message = "Settings saved.".into(),
                    Err(e) => {
                        self.settings.status_message = format!("Save failed: {e}");
                    }
                }
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let cs = &self.colors;
        let nav = self.nav_rail(cs);

        let page_content: Element<'_, Message> = match self.page {
            Page::NowPlaying => pages::now_playing::view(cs, &self.detected, &self.status_message),
            Page::Library => pages::library::view(
                cs,
                self.library_tab,
                &self.library_entries,
                self.selected_anime,
                self.library_sort,
                &self.score_input,
                self.library_view_mode,
            ),
            Page::Search => container(
                column![
                    text("Search").size(style::TEXT_XL).color(cs.on_surface_variant),
                    text("Coming soon")
                        .size(style::TEXT_SM)
                        .color(cs.outline),
                ]
                .spacing(style::SPACE_SM),
            )
            .padding(style::SPACE_XL)
            .into(),
            Page::Settings => pages::settings::view(cs, &self.settings),
        };

        let status_bar = container(
            text(&self.status_message).size(style::TEXT_XS),
        )
        .style(theme::status_bar(cs))
        .width(Length::Fill)
        .height(Length::Fixed(style::STATUS_BAR_HEIGHT))
        .padding([4.0, style::SPACE_MD]);

        let main: Element<'_, Message> = column![
            row![nav, page_content].height(Length::Fill),
            status_bar,
        ]
        .into();

        // Wrap in modal if one is active.
        if let Some(modal_kind) = &self.modal_state {
            let modal_content = self.build_modal_content(cs, modal_kind);
            crate::widgets::modal(main, modal_content, Message::Library(LibraryMsg::CancelModal))
        } else {
            main
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        subscription::detection_tick(self.config.general.detection_interval.max(1))
    }

    pub fn theme(&self) -> Theme {
        theme::build_theme(&self.colors)
    }

    fn build_modal_content<'a>(&self, cs: &ColorScheme, kind: &'a ModalKind) -> Element<'a, Message> {
        match kind {
            ModalKind::ConfirmDelete { anime_id, title } => {
                let anime_id = *anime_id;
                container(
                    column![
                        text("Remove from library?")
                            .size(style::TEXT_LG),
                        text(title.as_str())
                            .size(style::TEXT_SM)
                            .color(cs.on_surface_variant),
                        text("This action cannot be undone.")
                            .size(style::TEXT_XS)
                            .color(cs.outline),
                        row![
                            button(text("Cancel").size(style::TEXT_SM))
                                .padding([style::SPACE_SM, style::SPACE_XL])
                                .on_press(Message::Library(LibraryMsg::CancelModal))
                                .style(theme::ghost_button(cs)),
                            button(text("Delete").size(style::TEXT_SM))
                                .padding([style::SPACE_SM, style::SPACE_XL])
                                .on_press(Message::Library(LibraryMsg::ConfirmDelete(anime_id)))
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
        let nav_item =
            |icon: iced::widget::Text<'static>, label: &'static str, page: Page| {
                let active = self.page == page;
                button(
                    column![
                        icon.size(style::NAV_ICON_SIZE).center(),
                        text(label).size(style::NAV_LABEL_SIZE).center(),
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
            // Branding
            container(
                text("K").size(style::TEXT_XL).color(cs.primary),
            )
            .width(Length::Fill)
            .center_x(Length::Fill)
            .padding([style::SPACE_LG, 0.0]),
            // Navigation items
            column![
                nav_item(icons::icon_play(), "Playing", Page::NowPlaying),
                nav_item(icons::icon_library(), "Library", Page::Library),
                nav_item(icons::icon_search(), "Search", Page::Search),
                nav_item(icons::icon_settings(), "Settings", Page::Settings),
            ]
            .spacing(style::SPACE_XS)
            .align_x(Alignment::Center),
        ]
        .spacing(style::SPACE_SM)
        .align_x(Alignment::Center)
        .width(Length::Fixed(style::NAV_RAIL_WIDTH));

        container(rail)
            .style(theme::nav_rail_bg(cs))
            .height(Length::Fill)
            .into()
    }

    fn refresh_library(&mut self) {
        if let Some(storage) = &self.storage {
            let mut entries = storage
                .get_library_by_status(self.library_tab)
                .unwrap_or_default();

            match self.library_sort {
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

            self.library_entries = entries;
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
