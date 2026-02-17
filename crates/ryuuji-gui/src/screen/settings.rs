use iced::widget::{button, column, container, pick_list, row, rule, text, text_input, toggler};
use iced::{Alignment, Element, Length, Task};

use ryuuji_core::config::{AppConfig, ThemeMode};
use ryuuji_core::models::WatchStatus;

use ryuuji_core::debug_log::SharedEventLog;

use crate::app;
use crate::db::DbHandle;
use crate::screen::debug;
use crate::screen::Action;
use crate::style;
use crate::theme::{self, available_themes, ColorScheme};
use crate::toast::ToastKind;

// ── Settings Sections ─────────────────────────────────────────────

/// Settings sidebar sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsSection {
    #[default]
    Appearance,
    General,
    Library,
    WatchFolders,
    Services,
    Torrents,
    Integrations,
    Data,
    About,
    Debug,
}

impl SettingsSection {
    pub const ALL: &'static [SettingsSection] = &[
        SettingsSection::Appearance,
        SettingsSection::General,
        SettingsSection::Library,
        SettingsSection::WatchFolders,
        SettingsSection::Services,
        SettingsSection::Torrents,
        SettingsSection::Integrations,
        SettingsSection::Data,
        SettingsSection::About,
        SettingsSection::Debug,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Appearance => "Appearance",
            Self::General => "General",
            Self::Library => "Library",
            Self::WatchFolders => "Watch Folders",
            Self::Services => "Services",
            Self::Torrents => "Torrents",
            Self::Integrations => "Integrations",
            Self::Data => "Data",
            Self::About => "About",
            Self::Debug => "Debug",
        }
    }
}

// ── Stats DTO ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct LibraryStats {
    pub total: usize,
    pub watching: usize,
    pub completed: usize,
    pub on_hold: usize,
    pub dropped: usize,
    pub plan_to_watch: usize,
}

// ── State ──────────────────────────────────────────────────────────

/// Settings screen state.
pub struct Settings {
    pub active_section: SettingsSection,
    // Appearance
    pub selected_theme: String,
    pub selected_mode: ThemeMode,
    pub available_theme_names: Vec<String>,
    // General
    pub interval_input: String,
    pub close_to_tray: bool,
    // Library
    pub auto_update: bool,
    pub confirm_update: bool,
    // Services
    pub primary_service: String,
    pub primary_service_options: Vec<String>,
    // AniList
    pub anilist_enabled: bool,
    pub anilist_client_id: String,
    pub anilist_client_secret: String,
    pub anilist_authenticated: bool,
    pub anilist_status: String,
    pub anilist_busy: bool,
    // Kitsu
    pub kitsu_enabled: bool,
    pub kitsu_authenticated: bool,
    pub kitsu_status: String,
    pub kitsu_busy: bool,
    pub kitsu_username: String,
    pub kitsu_password: String,
    // MAL
    pub mal_enabled: bool,
    pub mal_client_id: String,
    pub mal_authenticated: bool,
    pub mal_status: String,
    pub mal_busy: bool,
    // Torrents
    pub torrent_enabled: bool,
    pub torrent_interval_input: String,
    // Integrations
    pub discord_enabled: bool,
    // Watch folders
    pub watch_folders: Vec<String>,
    pub new_folder_input: String,
    pub scan_on_startup: bool,
    pub scan_busy: bool,
    pub scan_status: String,
    // Data
    pub library_stats: Option<LibraryStats>,
    pub export_status: String,
    pub export_busy: bool,
    // Debug
    pub debug: debug::Debug,
}

// ── Messages ───────────────────────────────────────────────────────

/// Messages handled by the Settings screen.
#[derive(Debug, Clone)]
pub enum Message {
    // Navigation
    SectionChanged(SettingsSection),
    // Appearance
    ThemeChanged(String),
    ModeChanged(ThemeMode),
    // General
    IntervalChanged(String),
    IntervalSubmitted,
    CloseToTrayToggled(bool),
    // Library
    AutoUpdateToggled(bool),
    ConfirmUpdateToggled(bool),
    // Services
    PrimaryServiceChanged(String),
    // AniList
    AniListEnabledToggled(bool),
    AniListClientIdChanged(String),
    AniListClientIdSubmitted,
    AniListClientSecretChanged(String),
    AniListClientSecretSubmitted,
    AniListLogin,
    AniListLoginResult(Result<(), String>),
    AniListImport,
    AniListImportResult(Result<usize, String>),
    AniListTokenChecked(bool),
    // Kitsu
    KitsuEnabledToggled(bool),
    KitsuUsernameChanged(String),
    KitsuPasswordChanged(String),
    KitsuLogin,
    KitsuLoginResult(Result<(), String>),
    KitsuImport,
    KitsuImportResult(Result<usize, String>),
    KitsuTokenChecked(bool),
    // MAL
    MalEnabledToggled(bool),
    MalClientIdChanged(String),
    MalClientIdSubmitted,
    MalLogin,
    MalLoginResult(Result<(), String>),
    MalImport,
    MalImportResult(Result<usize, String>),
    MalTokenChecked(bool),
    // Torrents
    TorrentEnabledToggled(bool),
    TorrentIntervalChanged(String),
    TorrentIntervalSubmitted,
    // Watch folders
    NewFolderInputChanged(String),
    AddWatchFolder,
    RemoveWatchFolder(usize),
    ScanOnStartupToggled(bool),
    ScanNow,
    ScanResult(Result<String, String>),
    // Integrations
    DiscordEnabledToggled(bool),
    // Data
    StatsLoaded(Result<LibraryStats, String>),
    ExportLibrary,
    ExportResult(Result<String, String>),
    // About
    OpenLogsFolder,
    // Debug
    DebugMsg(debug::Message),
}

// ── Implementation ─────────────────────────────────────────────────

impl Settings {
    /// Initialize form state from the current config.
    pub fn from_config(config: &AppConfig) -> Self {
        let theme_names: Vec<String> = available_themes().iter().map(|t| t.name.clone()).collect();

        Self {
            active_section: SettingsSection::default(),
            selected_theme: config.appearance.theme.clone(),
            selected_mode: config.appearance.mode,
            available_theme_names: theme_names,
            interval_input: config.general.detection_interval.to_string(),
            close_to_tray: config.general.close_to_tray,
            auto_update: config.library.auto_update,
            confirm_update: config.library.confirm_update,
            primary_service: config.services.primary.clone(),
            primary_service_options: vec!["anilist".into(), "kitsu".into(), "mal".into()],
            // AniList
            anilist_enabled: config.services.anilist.enabled,
            anilist_client_id: config
                .services
                .anilist
                .client_id
                .clone()
                .unwrap_or_default(),
            anilist_client_secret: config
                .services
                .anilist
                .client_secret
                .clone()
                .unwrap_or_default(),
            anilist_authenticated: false,
            anilist_status: String::new(),
            anilist_busy: false,
            // Kitsu
            kitsu_enabled: config.services.kitsu.enabled,
            kitsu_authenticated: false,
            kitsu_status: String::new(),
            kitsu_busy: false,
            kitsu_username: String::new(),
            kitsu_password: String::new(),
            // MAL
            mal_enabled: config.services.mal.enabled,
            mal_client_id: config.services.mal.client_id.clone().unwrap_or_default(),
            mal_authenticated: false,
            mal_status: String::new(),
            mal_busy: false,
            torrent_enabled: config.torrent.enabled,
            torrent_interval_input: config.torrent.auto_check_interval.to_string(),
            discord_enabled: config.discord.enabled,
            watch_folders: config.library.watch_folders.clone(),
            new_folder_input: String::new(),
            scan_on_startup: config.library.scan_on_startup,
            scan_busy: false,
            scan_status: String::new(),
            library_stats: None,
            export_status: String::new(),
            export_busy: false,
            debug: debug::Debug::new(),
        }
    }

    /// Handle a settings message, returning an Action for the app router.
    pub fn update(&mut self, msg: Message, config: &mut AppConfig) -> Action {
        match msg {
            // ── Navigation ───────────────────────────────────────
            Message::SectionChanged(section) => {
                self.active_section = section;
                Action::None
            }

            // ── Appearance ──────────────────────────────────────
            Message::ThemeChanged(name) => {
                self.selected_theme = name.clone();
                config.appearance.theme = name;
                let _ = config.save();
                Action::None
            }
            Message::ModeChanged(mode) => {
                self.selected_mode = mode;
                config.appearance.mode = mode;
                let _ = config.save();
                Action::None
            }

            // ── General ─────────────────────────────────────────
            Message::IntervalChanged(val) => {
                self.interval_input = val;
                Action::None
            }
            Message::IntervalSubmitted => {
                let interval = self
                    .interval_input
                    .parse::<u64>()
                    .unwrap_or(config.general.detection_interval)
                    .clamp(1, 300);
                self.interval_input = interval.to_string();
                config.general.detection_interval = interval;
                let _ = config.save();
                Action::None
            }
            Message::CloseToTrayToggled(val) => {
                self.close_to_tray = val;
                config.general.close_to_tray = val;
                let _ = config.save();
                Action::None
            }

            // ── Library ─────────────────────────────────────────
            Message::AutoUpdateToggled(val) => {
                self.auto_update = val;
                config.library.auto_update = val;
                let _ = config.save();
                Action::None
            }
            Message::ConfirmUpdateToggled(val) => {
                self.confirm_update = val;
                config.library.confirm_update = val;
                let _ = config.save();
                Action::None
            }

            // ── Services ────────────────────────────────────────
            Message::PrimaryServiceChanged(svc) => {
                self.primary_service = svc.clone();
                config.services.primary = svc;
                let _ = config.save();
                Action::None
            }

            // ── AniList ─────────────────────────────────────────
            Message::AniListEnabledToggled(val) => {
                self.anilist_enabled = val;
                config.services.anilist.enabled = val;
                let _ = config.save();
                Action::None
            }
            Message::AniListClientIdChanged(val) => {
                self.anilist_client_id = val;
                Action::None
            }
            Message::AniListClientIdSubmitted => {
                config.services.anilist.client_id = if self.anilist_client_id.trim().is_empty() {
                    None
                } else {
                    Some(self.anilist_client_id.trim().to_string())
                };
                let _ = config.save();
                Action::None
            }
            Message::AniListClientSecretChanged(val) => {
                self.anilist_client_secret = val;
                Action::None
            }
            Message::AniListClientSecretSubmitted => {
                config.services.anilist.client_secret =
                    if self.anilist_client_secret.trim().is_empty() {
                        None
                    } else {
                        Some(self.anilist_client_secret.trim().to_string())
                    };
                let _ = config.save();
                Action::None
            }
            Message::AniListLogin => {
                self.anilist_busy = true;
                self.anilist_status = "Opening browser for AniList login...".into();
                Action::None
            }
            Message::AniListLoginResult(result) => {
                self.anilist_busy = false;
                match result {
                    Ok(()) => {
                        self.anilist_authenticated = true;
                        self.anilist_status = "Logged in to AniList.".into();
                    }
                    Err(e) => {
                        tracing::warn!(service = "anilist", error = %e, "Login failed");
                        self.anilist_status = format!("Login failed: {e}");
                    }
                }
                Action::None
            }
            Message::AniListImport => {
                self.anilist_busy = true;
                self.anilist_status = "Importing anime list from AniList...".into();
                Action::None
            }
            Message::AniListImportResult(result) => {
                self.anilist_busy = false;
                match result {
                    Ok(count) => {
                        self.anilist_status = format!("Imported {count} anime from AniList.");
                        Action::RefreshLibrary
                    }
                    Err(e) => {
                        tracing::warn!(service = "anilist", error = %e, "Import failed");
                        self.anilist_status = format!("Import failed: {e}");
                        Action::ShowToast(format!("AniList import failed: {e}"), ToastKind::Error)
                    }
                }
            }
            Message::AniListTokenChecked(has_token) => {
                self.anilist_authenticated = has_token;
                Action::None
            }

            // ── Kitsu ───────────────────────────────────────────
            Message::KitsuEnabledToggled(val) => {
                self.kitsu_enabled = val;
                config.services.kitsu.enabled = val;
                let _ = config.save();
                Action::None
            }
            Message::KitsuUsernameChanged(val) => {
                self.kitsu_username = val;
                Action::None
            }
            Message::KitsuPasswordChanged(val) => {
                self.kitsu_password = val;
                Action::None
            }
            Message::KitsuLogin => {
                self.kitsu_busy = true;
                self.kitsu_status = "Logging in to Kitsu...".into();
                Action::None
            }
            Message::KitsuLoginResult(result) => {
                self.kitsu_busy = false;
                self.kitsu_password.clear(); // Don't keep password in memory
                match result {
                    Ok(()) => {
                        self.kitsu_authenticated = true;
                        self.kitsu_status = "Logged in to Kitsu.".into();
                    }
                    Err(e) => {
                        tracing::warn!(service = "kitsu", error = %e, "Login failed");
                        self.kitsu_status = format!("Login failed: {e}");
                    }
                }
                Action::None
            }
            Message::KitsuImport => {
                self.kitsu_busy = true;
                self.kitsu_status = "Importing anime list from Kitsu...".into();
                Action::None
            }
            Message::KitsuImportResult(result) => {
                self.kitsu_busy = false;
                match result {
                    Ok(count) => {
                        self.kitsu_status = format!("Imported {count} anime from Kitsu.");
                        Action::RefreshLibrary
                    }
                    Err(e) => {
                        tracing::warn!(service = "kitsu", error = %e, "Import failed");
                        self.kitsu_status = format!("Import failed: {e}");
                        Action::ShowToast(format!("Kitsu import failed: {e}"), ToastKind::Error)
                    }
                }
            }
            Message::KitsuTokenChecked(has_token) => {
                self.kitsu_authenticated = has_token;
                Action::None
            }

            // ── MAL ─────────────────────────────────────────────
            Message::MalEnabledToggled(val) => {
                self.mal_enabled = val;
                config.services.mal.enabled = val;
                let _ = config.save();
                Action::None
            }
            Message::MalClientIdChanged(val) => {
                self.mal_client_id = val;
                Action::None
            }
            Message::MalClientIdSubmitted => {
                config.services.mal.client_id = if self.mal_client_id.trim().is_empty() {
                    None
                } else {
                    Some(self.mal_client_id.trim().to_string())
                };
                let _ = config.save();
                Action::None
            }

            // ── MAL actions ─────────────────────────────────────
            Message::MalLogin => {
                self.mal_busy = true;
                self.mal_status = "Opening browser for MAL login...".into();
                Action::None // app.rs handles the async task
            }
            Message::MalLoginResult(result) => {
                self.mal_busy = false;
                match result {
                    Ok(()) => {
                        self.mal_authenticated = true;
                        self.mal_status = "Logged in to MAL.".into();
                    }
                    Err(e) => {
                        tracing::warn!(service = "mal", error = %e, "Login failed");
                        self.mal_status = format!("Login failed: {e}");
                    }
                }
                Action::None
            }
            Message::MalImport => {
                self.mal_busy = true;
                self.mal_status = "Importing anime list from MAL...".into();
                Action::None // app.rs handles the async task
            }
            Message::MalImportResult(result) => {
                self.mal_busy = false;
                match result {
                    Ok(count) => {
                        self.mal_status = format!("Imported {count} anime from MAL.");
                        Action::RefreshLibrary
                    }
                    Err(e) => {
                        tracing::warn!(service = "mal", error = %e, "Import failed");
                        self.mal_status = format!("Import failed: {e}");
                        Action::ShowToast(format!("MAL import failed: {e}"), ToastKind::Error)
                    }
                }
            }
            Message::MalTokenChecked(has_token) => {
                self.mal_authenticated = has_token;
                Action::None
            }

            // ── Torrents ──────────────────────────────────────────
            Message::TorrentEnabledToggled(val) => {
                self.torrent_enabled = val;
                config.torrent.enabled = val;
                let _ = config.save();
                Action::None
            }
            Message::TorrentIntervalChanged(val) => {
                self.torrent_interval_input = val;
                Action::None
            }
            Message::TorrentIntervalSubmitted => {
                let interval = self
                    .torrent_interval_input
                    .parse::<u64>()
                    .unwrap_or(config.torrent.auto_check_interval)
                    .clamp(0, 1440);
                self.torrent_interval_input = interval.to_string();
                config.torrent.auto_check_interval = interval;
                let _ = config.save();
                Action::None
            }

            // ── Watch Folders ────────────────────────────────────
            Message::NewFolderInputChanged(val) => {
                self.new_folder_input = val;
                Action::None
            }
            Message::AddWatchFolder => {
                let path = self.new_folder_input.trim().to_string();
                if !path.is_empty() && !self.watch_folders.contains(&path) {
                    self.watch_folders.push(path);
                    self.new_folder_input.clear();
                    config.library.watch_folders = self.watch_folders.clone();
                    let _ = config.save();
                }
                Action::None
            }
            Message::RemoveWatchFolder(index) => {
                if index < self.watch_folders.len() {
                    self.watch_folders.remove(index);
                    config.library.watch_folders = self.watch_folders.clone();
                    let _ = config.save();
                }
                Action::None
            }
            Message::ScanOnStartupToggled(val) => {
                self.scan_on_startup = val;
                config.library.scan_on_startup = val;
                let _ = config.save();
                Action::None
            }
            Message::ScanNow => {
                self.scan_busy = true;
                self.scan_status = "Scanning watch folders...".into();
                Action::None // app.rs handles the async task
            }
            Message::ScanResult(result) => {
                self.scan_busy = false;
                match result {
                    Ok(summary) => {
                        self.scan_status = summary;
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Watch folder scan failed");
                        self.scan_status = format!("Scan failed: {e}");
                    }
                }
                Action::None
            }

            // ── Integrations ────────────────────────────────────
            Message::DiscordEnabledToggled(val) => {
                self.discord_enabled = val;
                config.discord.enabled = val;
                let _ = config.save();
                Action::None
            }

            // ── Data ────────────────────────────────────────────
            Message::StatsLoaded(result) => {
                match result {
                    Ok(stats) => self.library_stats = Some(stats),
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to load library stats");
                        self.library_stats = None;
                    }
                }
                Action::None
            }
            Message::ExportLibrary => {
                self.export_busy = true;
                self.export_status = "Exporting...".into();
                Action::None // app.rs handles the async task
            }
            Message::ExportResult(result) => {
                self.export_busy = false;
                match result {
                    Ok(path) => {
                        self.export_status = format!("Exported to {path}");
                        Action::ShowToast(format!("Exported to {path}"), ToastKind::Success)
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Library export failed");
                        self.export_status = format!("Export failed: {e}");
                        Action::ShowToast(format!("Export failed: {e}"), ToastKind::Error)
                    }
                }
            }

            // ── About ────────────────────────────────────────────
            Message::OpenLogsFolder => {
                let log_dir = AppConfig::log_dir();
                if let Err(e) = open::that(&log_dir) {
                    tracing::warn!(path = %log_dir.display(), error = %e, "Failed to open logs folder");
                }
                Action::None
            }

            // ── Debug ────────────────────────────────────────────
            Message::DebugMsg(msg) => self.debug.update(msg),
        }
    }

    /// Refresh the embedded debug panel from the shared event log.
    pub fn refresh_debug(&mut self, event_log: &SharedEventLog, db: Option<&DbHandle>) -> Action {
        self.debug.refresh(event_log, db, |msg| {
            app::Message::Settings(Message::DebugMsg(msg))
        })
    }

    /// Kick off an async task to load library statistics.
    pub fn load_stats(&self, db: Option<&DbHandle>) -> Action {
        let Some(db) = db.cloned() else {
            return Action::None;
        };
        Action::RunTask(Task::perform(
            async move {
                let rows = db.get_all_library().await.map_err(|e| e.to_string())?;
                let mut stats = LibraryStats {
                    total: rows.len(),
                    ..Default::default()
                };
                for row in &rows {
                    match row.entry.status {
                        WatchStatus::Watching => stats.watching += 1,
                        WatchStatus::Completed => stats.completed += 1,
                        WatchStatus::OnHold => stats.on_hold += 1,
                        WatchStatus::Dropped => stats.dropped += 1,
                        WatchStatus::PlanToWatch => stats.plan_to_watch += 1,
                    }
                }
                Ok(stats)
            },
            |result| app::Message::Settings(Message::StatsLoaded(result)),
        ))
    }

    // ── View ────────────────────────────────────────────────────────

    pub fn view<'a>(&'a self, cs: &ColorScheme) -> Element<'a, Message> {
        // ── Sidebar navigation ───────────────────────────────
        let heading = text("Settings")
            .size(style::TEXT_LG)
            .font(style::FONT_HEADING)
            .line_height(style::LINE_HEIGHT_TIGHT);

        let mut sidebar = column![heading].spacing(style::SPACE_XS);

        for &section in SettingsSection::ALL {
            let is_active = section == self.active_section;
            let label_color = if is_active {
                cs.primary
            } else {
                cs.on_surface_variant
            };

            let item = button(
                text(section.label())
                    .size(style::TEXT_SM)
                    .color(label_color)
                    .line_height(style::LINE_HEIGHT_NORMAL),
            )
            .on_press(Message::SectionChanged(section))
            .padding([style::SPACE_SM, style::SPACE_MD])
            .width(Length::Fill)
            .style(theme::settings_nav_item(cs, is_active));

            sidebar = sidebar.push(item);
        }

        let sidebar_container = container(sidebar)
            .width(Length::Fixed(style::SETTINGS_SIDEBAR_WIDTH))
            .padding([style::SPACE_XL, style::SPACE_MD]);

        // ── Active section content ───────────────────────────
        let section_content: Element<'_, Message> = match self.active_section {
            SettingsSection::Appearance => self.appearance_card(cs),
            SettingsSection::General => self.general_card(cs),
            SettingsSection::Library => self.library_card(cs),
            SettingsSection::WatchFolders => self.watch_folders_card(cs),
            SettingsSection::Services => self.services_card(cs),
            SettingsSection::Torrents => self.torrent_card(cs),
            SettingsSection::Integrations => self.integrations_card(cs),
            SettingsSection::Data => self.data_card(cs),
            SettingsSection::About => self.about_card(cs),
            SettingsSection::Debug => self.debug.view(cs).map(Message::DebugMsg),
        };

        let content_pane = column![section_content]
            .spacing(style::SPACE_LG)
            .padding(style::SPACE_XL)
            .width(Length::Fill);

        let content_scroll = crate::widgets::styled_scrollable(content_pane, cs)
            .height(Length::Fill);

        row![sidebar_container, content_scroll]
            .height(Length::Fill)
            .into()
    }

    // ── Card builders ───────────────────────────────────────────────

    fn appearance_card<'a>(&'a self, cs: &ColorScheme) -> Element<'a, Message> {
        container(
            column![
                text("Appearance")
                    .size(style::TEXT_XS)
                    .font(style::FONT_HEADING)
                    .color(cs.on_surface_variant)
                    .line_height(style::LINE_HEIGHT_LOOSE),
                row![
                    text("Theme")
                        .size(style::TEXT_BASE)
                        .line_height(style::LINE_HEIGHT_NORMAL)
                        .width(Length::Fill),
                    pick_list(
                        self.available_theme_names.as_slice(),
                        Some(&self.selected_theme),
                        |name: String| Message::ThemeChanged(name),
                    )
                    .text_size(style::TEXT_SM)
                    .padding([style::SPACE_XS, style::SPACE_SM])
                    .style(theme::pick_list_style(cs))
                    .menu_style(theme::pick_list_menu_style(cs)),
                ]
                .align_y(Alignment::Center)
                .spacing(style::SPACE_MD),
                row![
                    text("Mode")
                        .size(style::TEXT_BASE)
                        .line_height(style::LINE_HEIGHT_NORMAL)
                        .width(Length::Fill),
                    pick_list(
                        ThemeMode::ALL,
                        Some(self.selected_mode),
                        |mode: ThemeMode| Message::ModeChanged(mode),
                    )
                    .text_size(style::TEXT_SM)
                    .padding([style::SPACE_XS, style::SPACE_SM])
                    .style(theme::pick_list_style(cs))
                    .menu_style(theme::pick_list_menu_style(cs)),
                ]
                .align_y(Alignment::Center)
                .spacing(style::SPACE_MD),
            ]
            .spacing(style::SPACE_SM),
        )
        .style(theme::card(cs))
        .padding(style::SPACE_LG)
        .width(Length::Fill)
        .into()
    }

    fn general_card<'a>(&'a self, cs: &ColorScheme) -> Element<'a, Message> {
        container(
            column![
                text("General")
                    .size(style::TEXT_XS)
                    .font(style::FONT_HEADING)
                    .color(cs.on_surface_variant)
                    .line_height(style::LINE_HEIGHT_LOOSE),
                row![
                    text("Detection interval (seconds)")
                        .size(style::TEXT_BASE)
                        .line_height(style::LINE_HEIGHT_NORMAL)
                        .width(Length::Fill),
                    text_input("5", &self.interval_input)
                        .on_input(Message::IntervalChanged)
                        .on_submit(Message::IntervalSubmitted)
                        .size(style::INPUT_FONT_SIZE)
                        .padding(style::INPUT_PADDING)
                        .width(Length::Fixed(80.0))
                        .style(theme::text_input_style(cs)),
                ]
                .align_y(Alignment::Center)
                .spacing(style::SPACE_MD),
                toggler(self.close_to_tray)
                    .label("Close to system tray")
                    .text_size(style::TEXT_BASE)
                    .on_toggle(Message::CloseToTrayToggled)
                    .spacing(style::SPACE_SM)
                    .size(22.0)
                    .style(theme::toggler_style(cs)),
            ]
            .spacing(style::SPACE_SM),
        )
        .style(theme::card(cs))
        .padding(style::SPACE_LG)
        .width(Length::Fill)
        .into()
    }

    fn library_card<'a>(&'a self, cs: &ColorScheme) -> Element<'a, Message> {
        container(
            column![
                text("Library")
                    .size(style::TEXT_XS)
                    .font(style::FONT_HEADING)
                    .color(cs.on_surface_variant)
                    .line_height(style::LINE_HEIGHT_LOOSE),
                toggler(self.auto_update)
                    .label("Auto-update progress from detected playback")
                    .text_size(style::TEXT_BASE)
                    .on_toggle(Message::AutoUpdateToggled)
                    .spacing(style::SPACE_SM)
                    .size(22.0)
                    .style(theme::toggler_style(cs)),
                toggler(self.confirm_update)
                    .label("Confirm before updating")
                    .text_size(style::TEXT_BASE)
                    .on_toggle(Message::ConfirmUpdateToggled)
                    .spacing(style::SPACE_SM)
                    .size(22.0)
                    .style(theme::toggler_style(cs)),
            ]
            .spacing(style::SPACE_SM),
        )
        .style(theme::card(cs))
        .padding(style::SPACE_LG)
        .width(Length::Fill)
        .into()
    }

    fn services_card<'a>(&'a self, cs: &ColorScheme) -> Element<'a, Message> {
        let mut content = column![
            text("Services")
                .size(style::TEXT_XS)
                .font(style::FONT_HEADING)
                .color(cs.on_surface_variant)
                .line_height(style::LINE_HEIGHT_LOOSE),
            row![
                text("Primary service")
                    .size(style::TEXT_BASE)
                    .line_height(style::LINE_HEIGHT_NORMAL)
                    .width(Length::Fill),
                pick_list(
                    self.primary_service_options.as_slice(),
                    Some(&self.primary_service),
                    |svc: String| Message::PrimaryServiceChanged(svc),
                )
                .text_size(style::TEXT_SM)
                .padding([style::SPACE_SM, style::SPACE_MD])
                .style(theme::pick_list_style(cs))
                .menu_style(theme::pick_list_menu_style(cs)),
            ]
            .align_y(Alignment::Center)
            .spacing(style::SPACE_MD),
        ]
        .spacing(style::SPACE_SM);

        // AniList sub-section
        content = content.push(rule::horizontal(1));
        content = content.push(
            text("AniList")
                .size(style::TEXT_XS)
                .font(style::FONT_HEADING)
                .color(cs.on_surface_variant)
                .line_height(style::LINE_HEIGHT_LOOSE),
        );
        content = content.push(
            toggler(self.anilist_enabled)
                .label("Enable AniList sync")
                .text_size(style::TEXT_BASE)
                .on_toggle(Message::AniListEnabledToggled)
                .spacing(style::SPACE_SM)
                .size(22.0)
                .style(theme::toggler_style(cs)),
        );

        if self.anilist_enabled {
            content = content.push(
                row![
                    text("Client ID")
                        .size(style::TEXT_BASE)
                        .line_height(style::LINE_HEIGHT_NORMAL)
                        .width(Length::Fill),
                    text_input("your-client-id", &self.anilist_client_id)
                        .on_input(Message::AniListClientIdChanged)
                        .on_submit(Message::AniListClientIdSubmitted)
                        .size(style::INPUT_FONT_SIZE)
                        .padding(style::INPUT_PADDING)
                        .width(Length::Fixed(240.0))
                        .style(theme::text_input_style(cs)),
                ]
                .align_y(Alignment::Center)
                .spacing(style::SPACE_MD),
            );
            content = content.push(
                row![
                    text("Client Secret")
                        .size(style::TEXT_BASE)
                        .line_height(style::LINE_HEIGHT_NORMAL)
                        .width(Length::Fill),
                    text_input("your-client-secret", &self.anilist_client_secret)
                        .on_input(Message::AniListClientSecretChanged)
                        .on_submit(Message::AniListClientSecretSubmitted)
                        .size(style::INPUT_FONT_SIZE)
                        .padding(style::INPUT_PADDING)
                        .width(Length::Fixed(240.0))
                        .secure(true)
                        .style(theme::text_input_style(cs)),
                ]
                .align_y(Alignment::Center)
                .spacing(style::SPACE_MD),
            );

            let has_credentials = !self.anilist_client_id.trim().is_empty()
                && !self.anilist_client_secret.trim().is_empty();

            if has_credentials {
                let mut actions = row![].spacing(style::SPACE_SM);

                if !self.anilist_authenticated {
                    let mut login_btn = button(text("Login to AniList").size(style::TEXT_SM))
                        .padding([style::SPACE_SM, style::SPACE_XL])
                        .style(theme::primary_button(cs));
                    if !self.anilist_busy {
                        login_btn = login_btn.on_press(Message::AniListLogin);
                    }
                    actions = actions.push(login_btn);
                } else {
                    let mut import_btn = button(text("Import Library").size(style::TEXT_SM))
                        .padding([style::SPACE_SM, style::SPACE_XL])
                        .style(theme::primary_button(cs));
                    if !self.anilist_busy {
                        import_btn = import_btn.on_press(Message::AniListImport);
                    }
                    actions = actions.push(import_btn);
                }

                content = content.push(actions);
            }

            if !self.anilist_status.is_empty() {
                let color = if self.anilist_status.contains("failed")
                    || self.anilist_status.contains("Error")
                {
                    cs.error
                } else {
                    cs.status_completed
                };
                content = content.push(
                    text(&self.anilist_status)
                        .size(style::TEXT_SM)
                        .color(color)
                        .line_height(style::LINE_HEIGHT_LOOSE),
                );
            }
        }

        // Kitsu sub-section
        content = content.push(rule::horizontal(1));
        content = content.push(
            text("Kitsu")
                .size(style::TEXT_XS)
                .font(style::FONT_HEADING)
                .color(cs.on_surface_variant)
                .line_height(style::LINE_HEIGHT_LOOSE),
        );
        content = content.push(
            toggler(self.kitsu_enabled)
                .label("Enable Kitsu sync")
                .text_size(style::TEXT_BASE)
                .on_toggle(Message::KitsuEnabledToggled)
                .spacing(style::SPACE_SM)
                .size(22.0)
                .style(theme::toggler_style(cs)),
        );

        if self.kitsu_enabled {
            content = content.push(
                row![
                    text("Username")
                        .size(style::TEXT_BASE)
                        .line_height(style::LINE_HEIGHT_NORMAL)
                        .width(Length::Fill),
                    text_input("email or username", &self.kitsu_username)
                        .on_input(Message::KitsuUsernameChanged)
                        .size(style::INPUT_FONT_SIZE)
                        .padding(style::INPUT_PADDING)
                        .width(Length::Fixed(240.0))
                        .style(theme::text_input_style(cs)),
                ]
                .align_y(Alignment::Center)
                .spacing(style::SPACE_MD),
            );
            content = content.push(
                row![
                    text("Password")
                        .size(style::TEXT_BASE)
                        .line_height(style::LINE_HEIGHT_NORMAL)
                        .width(Length::Fill),
                    text_input("password", &self.kitsu_password)
                        .on_input(Message::KitsuPasswordChanged)
                        .size(style::INPUT_FONT_SIZE)
                        .padding(style::INPUT_PADDING)
                        .width(Length::Fixed(240.0))
                        .secure(true)
                        .style(theme::text_input_style(cs)),
                ]
                .align_y(Alignment::Center)
                .spacing(style::SPACE_MD),
            );

            if !self.kitsu_authenticated {
                let has_credentials = !self.kitsu_username.trim().is_empty()
                    && !self.kitsu_password.trim().is_empty();

                let mut login_btn = button(text("Login to Kitsu").size(style::TEXT_SM))
                    .padding([style::SPACE_SM, style::SPACE_XL])
                    .style(theme::primary_button(cs));
                if !self.kitsu_busy && has_credentials {
                    login_btn = login_btn.on_press(Message::KitsuLogin);
                }
                content = content.push(login_btn);
            } else {
                let mut import_btn = button(text("Import Library").size(style::TEXT_SM))
                    .padding([style::SPACE_SM, style::SPACE_XL])
                    .style(theme::primary_button(cs));
                if !self.kitsu_busy {
                    import_btn = import_btn.on_press(Message::KitsuImport);
                }
                content = content.push(import_btn);
            }

            if !self.kitsu_status.is_empty() {
                let color = if self.kitsu_status.contains("failed")
                    || self.kitsu_status.contains("Error")
                {
                    cs.error
                } else {
                    cs.status_completed
                };
                content = content.push(
                    text(&self.kitsu_status)
                        .size(style::TEXT_SM)
                        .color(color)
                        .line_height(style::LINE_HEIGHT_LOOSE),
                );
            }
        }

        // MAL sub-section
        content = content.push(rule::horizontal(1));
        content = content.push(
            text("MyAnimeList")
                .size(style::TEXT_XS)
                .font(style::FONT_HEADING)
                .color(cs.on_surface_variant)
                .line_height(style::LINE_HEIGHT_LOOSE),
        );
        content = content.push(
            toggler(self.mal_enabled)
                .label("Enable MAL sync")
                .text_size(style::TEXT_BASE)
                .on_toggle(Message::MalEnabledToggled)
                .spacing(style::SPACE_SM)
                .size(22.0)
                .style(theme::toggler_style(cs)),
        );

        if self.mal_enabled {
            content = content.push(
                row![
                    text("Client ID")
                        .size(style::TEXT_BASE)
                        .line_height(style::LINE_HEIGHT_NORMAL)
                        .width(Length::Fill),
                    text_input("your-client-id", &self.mal_client_id)
                        .on_input(Message::MalClientIdChanged)
                        .on_submit(Message::MalClientIdSubmitted)
                        .size(style::INPUT_FONT_SIZE)
                        .padding(style::INPUT_PADDING)
                        .width(Length::Fixed(240.0))
                        .style(theme::text_input_style(cs)),
                ]
                .align_y(Alignment::Center)
                .spacing(style::SPACE_MD),
            );

            if !self.mal_client_id.trim().is_empty() {
                let mut actions = row![].spacing(style::SPACE_SM);

                if !self.mal_authenticated {
                    let mut login_btn = button(text("Login to MAL").size(style::TEXT_SM))
                        .padding([style::SPACE_SM, style::SPACE_XL])
                        .style(theme::primary_button(cs));
                    if !self.mal_busy {
                        login_btn = login_btn.on_press(Message::MalLogin);
                    }
                    actions = actions.push(login_btn);
                } else {
                    let mut import_btn = button(text("Import Library").size(style::TEXT_SM))
                        .padding([style::SPACE_SM, style::SPACE_XL])
                        .style(theme::primary_button(cs));
                    if !self.mal_busy {
                        import_btn = import_btn.on_press(Message::MalImport);
                    }
                    actions = actions.push(import_btn);
                }

                content = content.push(actions);
            }

            if !self.mal_status.is_empty() {
                let color =
                    if self.mal_status.contains("failed") || self.mal_status.contains("Error") {
                        cs.error
                    } else {
                        cs.status_completed
                    };
                content = content.push(
                    text(&self.mal_status)
                        .size(style::TEXT_SM)
                        .color(color)
                        .line_height(style::LINE_HEIGHT_LOOSE),
                );
            }
        }

        container(content)
            .style(theme::card(cs))
            .padding(style::SPACE_LG)
            .width(Length::Fill)
            .into()
    }

    fn torrent_card<'a>(&'a self, cs: &ColorScheme) -> Element<'a, Message> {
        let mut content = column![
            text("Torrents")
                .size(style::TEXT_XS)
                .font(style::FONT_HEADING)
                .color(cs.on_surface_variant)
                .line_height(style::LINE_HEIGHT_LOOSE),
            toggler(self.torrent_enabled)
                .label("Enable torrent RSS feature")
                .text_size(style::TEXT_BASE)
                .on_toggle(Message::TorrentEnabledToggled)
                .spacing(style::SPACE_SM)
                .size(22.0)
                .style(theme::toggler_style(cs)),
        ]
        .spacing(style::SPACE_SM);

        if self.torrent_enabled {
            content = content.push(
                row![
                    text("Auto-check interval (minutes, 0 = off)")
                        .size(style::TEXT_BASE)
                        .line_height(style::LINE_HEIGHT_NORMAL)
                        .width(Length::Fill),
                    text_input("0", &self.torrent_interval_input)
                        .on_input(Message::TorrentIntervalChanged)
                        .on_submit(Message::TorrentIntervalSubmitted)
                        .size(style::INPUT_FONT_SIZE)
                        .padding(style::INPUT_PADDING)
                        .width(Length::Fixed(80.0))
                        .style(theme::text_input_style(cs)),
                ]
                .align_y(Alignment::Center)
                .spacing(style::SPACE_MD),
            );
        }

        container(content)
            .style(theme::card(cs))
            .padding(style::SPACE_LG)
            .width(Length::Fill)
            .into()
    }

    fn integrations_card<'a>(&'a self, cs: &ColorScheme) -> Element<'a, Message> {
        container(
            column![
                text("Integrations")
                    .size(style::TEXT_XS)
                    .font(style::FONT_HEADING)
                    .color(cs.on_surface_variant)
                    .line_height(style::LINE_HEIGHT_LOOSE),
                toggler(self.discord_enabled)
                    .label("Discord Rich Presence")
                    .text_size(style::TEXT_BASE)
                    .on_toggle(Message::DiscordEnabledToggled)
                    .spacing(style::SPACE_SM)
                    .size(22.0)
                    .style(theme::toggler_style(cs)),
            ]
            .spacing(style::SPACE_SM),
        )
        .style(theme::card(cs))
        .padding(style::SPACE_LG)
        .width(Length::Fill)
        .into()
    }

    fn data_card<'a>(&'a self, cs: &ColorScheme) -> Element<'a, Message> {
        let mut content = column![text("Data")
            .size(style::TEXT_XS)
            .font(style::FONT_HEADING)
            .color(cs.on_surface_variant)
            .line_height(style::LINE_HEIGHT_LOOSE),]
        .spacing(style::SPACE_SM);

        // Stats
        if let Some(stats) = &self.library_stats {
            content = content.push(
                column![
                    text(format!("Total entries: {}", stats.total))
                        .size(style::TEXT_BASE)
                        .line_height(style::LINE_HEIGHT_NORMAL),
                    row![
                        text(format!("Watching: {}", stats.watching))
                            .size(style::TEXT_SM)
                            .color(cs.on_surface_variant)
                            .line_height(style::LINE_HEIGHT_LOOSE),
                        text(format!("Completed: {}", stats.completed))
                            .size(style::TEXT_SM)
                            .color(cs.on_surface_variant)
                            .line_height(style::LINE_HEIGHT_LOOSE),
                        text(format!("On Hold: {}", stats.on_hold))
                            .size(style::TEXT_SM)
                            .color(cs.on_surface_variant)
                            .line_height(style::LINE_HEIGHT_LOOSE),
                        text(format!("Dropped: {}", stats.dropped))
                            .size(style::TEXT_SM)
                            .color(cs.on_surface_variant)
                            .line_height(style::LINE_HEIGHT_LOOSE),
                        text(format!("Plan to Watch: {}", stats.plan_to_watch))
                            .size(style::TEXT_SM)
                            .color(cs.on_surface_variant)
                            .line_height(style::LINE_HEIGHT_LOOSE),
                    ]
                    .spacing(style::SPACE_LG),
                ]
                .spacing(style::SPACE_SM),
            );
        } else {
            content = content.push(
                text("Loading library stats...")
                    .size(style::TEXT_SM)
                    .color(cs.on_surface_variant)
                    .line_height(style::LINE_HEIGHT_LOOSE),
            );
        }

        // Export
        content = content.push(rule::horizontal(1));

        let mut export_btn = button(text("Export library as JSON").size(style::TEXT_SM))
            .padding([style::SPACE_SM, style::SPACE_XL])
            .style(theme::primary_button(cs));
        if !self.export_busy {
            export_btn = export_btn.on_press(Message::ExportLibrary);
        }
        content = content.push(export_btn);

        if !self.export_status.is_empty() {
            let color = if self.export_status.contains("failed") {
                cs.error
            } else {
                cs.status_completed
            };
            content = content.push(
                text(&self.export_status)
                    .size(style::TEXT_SM)
                    .color(color)
                    .line_height(style::LINE_HEIGHT_LOOSE),
            );
        }

        container(content)
            .style(theme::card(cs))
            .padding(style::SPACE_LG)
            .width(Length::Fill)
            .into()
    }

    fn watch_folders_card<'a>(&'a self, cs: &ColorScheme) -> Element<'a, Message> {
        let mut content = column![
            text("Watch Folders")
                .size(style::TEXT_XS)
                .font(style::FONT_HEADING)
                .color(cs.on_surface_variant)
                .line_height(style::LINE_HEIGHT_LOOSE),
            toggler(self.scan_on_startup)
                .label("Scan on startup")
                .text_size(style::TEXT_BASE)
                .on_toggle(Message::ScanOnStartupToggled)
                .spacing(style::SPACE_SM)
                .size(22.0)
                .style(theme::toggler_style(cs)),
        ]
        .spacing(style::SPACE_SM);

        // Existing folders
        for (i, folder) in self.watch_folders.iter().enumerate() {
            content = content.push(
                row![
                    text(folder)
                        .size(style::TEXT_SM)
                        .line_height(style::LINE_HEIGHT_NORMAL)
                        .width(Length::Fill),
                    button(text("\u{2715}").size(style::TEXT_SM))
                        .on_press(Message::RemoveWatchFolder(i))
                        .padding([style::SPACE_XXS, style::SPACE_SM])
                        .style(theme::ghost_button(cs)),
                ]
                .align_y(Alignment::Center)
                .spacing(style::SPACE_SM),
            );
        }

        // Add folder input
        content = content.push(
            row![
                text_input("/path/to/anime", &self.new_folder_input)
                    .on_input(Message::NewFolderInputChanged)
                    .on_submit(Message::AddWatchFolder)
                    .size(style::INPUT_FONT_SIZE)
                    .padding(style::INPUT_PADDING)
                    .width(Length::Fill)
                    .style(theme::text_input_style(cs)),
                button(text("Add").size(style::TEXT_SM))
                    .on_press(Message::AddWatchFolder)
                    .padding([style::SPACE_SM, style::SPACE_XL])
                    .style(theme::primary_button(cs)),
            ]
            .align_y(Alignment::Center)
            .spacing(style::SPACE_SM),
        );

        // Scan button
        if !self.watch_folders.is_empty() {
            let mut scan_btn = button(text("Scan Now").size(style::TEXT_SM))
                .padding([style::SPACE_SM, style::SPACE_XL])
                .style(theme::primary_button(cs));
            if !self.scan_busy {
                scan_btn = scan_btn.on_press(Message::ScanNow);
            }
            content = content.push(scan_btn);
        }

        if !self.scan_status.is_empty() {
            let color = if self.scan_status.contains("failed") {
                cs.error
            } else {
                cs.status_completed
            };
            content = content.push(
                text(&self.scan_status)
                    .size(style::TEXT_SM)
                    .color(color)
                    .line_height(style::LINE_HEIGHT_LOOSE),
            );
        }

        container(content)
            .style(theme::card(cs))
            .padding(style::SPACE_LG)
            .width(Length::Fill)
            .into()
    }

    fn about_card(&self, cs: &ColorScheme) -> Element<'_, Message> {
        let version = env!("CARGO_PKG_VERSION");
        let config_path = AppConfig::config_path().display().to_string();
        let db_path = AppConfig::db_path().display().to_string();
        let log_dir = AppConfig::log_dir().display().to_string();

        container(
            column![
                text("About")
                    .size(style::TEXT_XS)
                    .font(style::FONT_HEADING)
                    .color(cs.on_surface_variant)
                    .line_height(style::LINE_HEIGHT_LOOSE),
                text(format!("ryuuji v{version}"))
                    .size(style::TEXT_BASE)
                    .line_height(style::LINE_HEIGHT_NORMAL),
                row![
                    text("Config:")
                        .size(style::TEXT_SM)
                        .color(cs.on_surface_variant)
                        .line_height(style::LINE_HEIGHT_LOOSE),
                    text(config_path)
                        .size(style::TEXT_SM)
                        .color(cs.outline)
                        .line_height(style::LINE_HEIGHT_LOOSE),
                ]
                .spacing(style::SPACE_SM),
                row![
                    text("Database:")
                        .size(style::TEXT_SM)
                        .color(cs.on_surface_variant)
                        .line_height(style::LINE_HEIGHT_LOOSE),
                    text(db_path)
                        .size(style::TEXT_SM)
                        .color(cs.outline)
                        .line_height(style::LINE_HEIGHT_LOOSE),
                ]
                .spacing(style::SPACE_SM),
                row![
                    text("Logs:")
                        .size(style::TEXT_SM)
                        .color(cs.on_surface_variant)
                        .line_height(style::LINE_HEIGHT_LOOSE),
                    text(log_dir)
                        .size(style::TEXT_SM)
                        .color(cs.outline)
                        .line_height(style::LINE_HEIGHT_LOOSE),
                    button(text("Open").size(style::TEXT_SM))
                        .on_press(Message::OpenLogsFolder)
                        .padding([style::SPACE_XXS, style::SPACE_SM])
                        .style(theme::ghost_button(cs)),
                ]
                .spacing(style::SPACE_SM)
                .align_y(Alignment::Center),
            ]
            .spacing(style::SPACE_SM),
        )
        .style(theme::card(cs))
        .padding(style::SPACE_LG)
        .width(Length::Fill)
        .into()
    }
}
