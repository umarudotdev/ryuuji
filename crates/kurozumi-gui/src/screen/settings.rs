use iced::widget::{button, column, container, pick_list, row, rule, text, text_input, toggler};
use iced::{Alignment, Element, Length, Task};

use kurozumi_core::config::{AppConfig, ThemeMode};
use kurozumi_core::models::WatchStatus;

use crate::app;
use crate::db::DbHandle;
use crate::screen::Action;
use crate::style;
use crate::theme::{self, available_themes, ColorScheme};

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
    // MAL
    pub mal_enabled: bool,
    pub mal_client_id: String,
    pub mal_authenticated: bool,
    pub mal_status: String,
    pub mal_busy: bool,
    // Integrations
    pub discord_enabled: bool,
    // Data
    pub library_stats: Option<LibraryStats>,
    pub export_status: String,
    pub export_busy: bool,
}

// ── Messages ───────────────────────────────────────────────────────

/// Messages handled by the Settings screen.
#[derive(Debug, Clone)]
pub enum Message {
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
    MalEnabledToggled(bool),
    MalClientIdChanged(String),
    MalClientIdSubmitted,
    // MAL actions
    MalLogin,
    MalLoginResult(Result<(), String>),
    MalImport,
    MalImportResult(Result<usize, String>),
    MalTokenChecked(bool),
    // Integrations
    DiscordEnabledToggled(bool),
    // Data
    StatsLoaded(Result<LibraryStats, String>),
    ExportLibrary,
    ExportResult(Result<String, String>),
}

// ── Implementation ─────────────────────────────────────────────────

impl Settings {
    /// Initialize form state from the current config.
    pub fn from_config(config: &AppConfig) -> Self {
        let theme_names: Vec<String> = available_themes().iter().map(|t| t.name.clone()).collect();

        Self {
            selected_theme: config.appearance.theme.clone(),
            selected_mode: config.appearance.mode,
            available_theme_names: theme_names,
            interval_input: config.general.detection_interval.to_string(),
            close_to_tray: config.general.close_to_tray,
            auto_update: config.library.auto_update,
            confirm_update: config.library.confirm_update,
            primary_service: config.services.primary.clone(),
            primary_service_options: vec!["anilist".into(), "kitsu".into(), "mal".into()],
            mal_enabled: config.services.mal.enabled,
            mal_client_id: config.services.mal.client_id.clone().unwrap_or_default(),
            mal_authenticated: false,
            mal_status: String::new(),
            mal_busy: false,
            discord_enabled: config.discord.enabled,
            library_stats: None,
            export_status: String::new(),
            export_busy: false,
        }
    }

    /// Handle a settings message, returning an Action for the app router.
    pub fn update(&mut self, msg: Message, config: &mut AppConfig) -> Action {
        match msg {
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
                        self.mal_status = format!("Import failed: {e}");
                        Action::None
                    }
                }
            }
            Message::MalTokenChecked(has_token) => {
                self.mal_authenticated = has_token;
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
                    Err(_) => self.library_stats = None,
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
                    }
                    Err(e) => {
                        self.export_status = format!("Export failed: {e}");
                    }
                }
                Action::None
            }
        }
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
        let heading = text("Settings").size(style::TEXT_2XL);

        let page = column![
            heading,
            self.appearance_card(cs),
            self.general_card(cs),
            self.library_card(cs),
            self.services_card(cs),
            self.integrations_card(cs),
            self.data_card(cs),
            self.about_card(cs),
        ]
        .spacing(style::SPACE_LG)
        .padding(style::SPACE_XL)
        .width(Length::Fill);

        iced::widget::scrollable(page).height(Length::Fill).into()
    }

    // ── Card builders ───────────────────────────────────────────────

    fn appearance_card<'a>(&'a self, cs: &ColorScheme) -> Element<'a, Message> {
        container(
            column![
                text("Appearance")
                    .size(style::TEXT_XS)
                    .color(cs.on_surface_variant),
                row![
                    text("Theme").size(style::TEXT_BASE).width(Length::Fill),
                    pick_list(
                        self.available_theme_names.as_slice(),
                        Some(&self.selected_theme),
                        |name: String| Message::ThemeChanged(name),
                    )
                    .text_size(style::TEXT_SM)
                    .padding([style::SPACE_XS, style::SPACE_SM]),
                ]
                .align_y(Alignment::Center)
                .spacing(style::SPACE_MD),
                row![
                    text("Mode").size(style::TEXT_BASE).width(Length::Fill),
                    pick_list(
                        ThemeMode::ALL,
                        Some(self.selected_mode),
                        |mode: ThemeMode| Message::ModeChanged(mode),
                    )
                    .text_size(style::TEXT_SM)
                    .padding([style::SPACE_XS, style::SPACE_SM]),
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
                    .color(cs.on_surface_variant),
                row![
                    text("Detection interval (seconds)")
                        .size(style::TEXT_BASE)
                        .width(Length::Fill),
                    text_input("5", &self.interval_input)
                        .on_input(Message::IntervalChanged)
                        .on_submit(Message::IntervalSubmitted)
                        .size(style::TEXT_SM)
                        .padding([style::SPACE_XS, style::SPACE_SM])
                        .width(Length::Fixed(80.0))
                        .style(theme::text_input_style(cs)),
                ]
                .align_y(Alignment::Center)
                .spacing(style::SPACE_MD),
                toggler(self.close_to_tray)
                    .label("Close to system tray")
                    .text_size(style::TEXT_BASE)
                    .on_toggle(Message::CloseToTrayToggled)
                    .spacing(style::SPACE_SM),
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
                    .color(cs.on_surface_variant),
                toggler(self.auto_update)
                    .label("Auto-update progress from detected playback")
                    .text_size(style::TEXT_BASE)
                    .on_toggle(Message::AutoUpdateToggled)
                    .spacing(style::SPACE_SM),
                toggler(self.confirm_update)
                    .label("Confirm before updating")
                    .text_size(style::TEXT_BASE)
                    .on_toggle(Message::ConfirmUpdateToggled)
                    .spacing(style::SPACE_SM),
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
                .color(cs.on_surface_variant),
            row![
                text("Primary service")
                    .size(style::TEXT_BASE)
                    .width(Length::Fill),
                pick_list(
                    self.primary_service_options.as_slice(),
                    Some(&self.primary_service),
                    |svc: String| Message::PrimaryServiceChanged(svc),
                )
                .text_size(style::TEXT_SM)
                .padding([style::SPACE_XS, style::SPACE_SM]),
            ]
            .align_y(Alignment::Center)
            .spacing(style::SPACE_MD),
        ]
        .spacing(style::SPACE_SM);

        // MAL sub-section
        content = content.push(rule::horizontal(1));
        content = content.push(
            text("MyAnimeList")
                .size(style::TEXT_XS)
                .color(cs.on_surface_variant),
        );
        content = content.push(
            toggler(self.mal_enabled)
                .label("Enable MAL sync")
                .text_size(style::TEXT_BASE)
                .on_toggle(Message::MalEnabledToggled)
                .spacing(style::SPACE_SM),
        );

        if self.mal_enabled {
            content = content.push(
                row![
                    text("Client ID").size(style::TEXT_BASE).width(Length::Fill),
                    text_input("your-client-id", &self.mal_client_id)
                        .on_input(Message::MalClientIdChanged)
                        .on_submit(Message::MalClientIdSubmitted)
                        .size(style::TEXT_SM)
                        .padding([style::SPACE_XS, style::SPACE_SM])
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
                content = content.push(text(&self.mal_status).size(style::TEXT_SM).color(color));
            }
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
                    .color(cs.on_surface_variant),
                toggler(self.discord_enabled)
                    .label("Discord Rich Presence")
                    .text_size(style::TEXT_BASE)
                    .on_toggle(Message::DiscordEnabledToggled)
                    .spacing(style::SPACE_SM),
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
            .color(cs.on_surface_variant),]
        .spacing(style::SPACE_SM);

        // Stats
        if let Some(stats) = &self.library_stats {
            content = content.push(
                column![
                    text(format!("Total entries: {}", stats.total)).size(style::TEXT_BASE),
                    row![
                        text(format!("Watching: {}", stats.watching))
                            .size(style::TEXT_SM)
                            .color(cs.on_surface_variant),
                        text(format!("Completed: {}", stats.completed))
                            .size(style::TEXT_SM)
                            .color(cs.on_surface_variant),
                        text(format!("On Hold: {}", stats.on_hold))
                            .size(style::TEXT_SM)
                            .color(cs.on_surface_variant),
                        text(format!("Dropped: {}", stats.dropped))
                            .size(style::TEXT_SM)
                            .color(cs.on_surface_variant),
                        text(format!("Plan to Watch: {}", stats.plan_to_watch))
                            .size(style::TEXT_SM)
                            .color(cs.on_surface_variant),
                    ]
                    .spacing(style::SPACE_LG),
                ]
                .spacing(style::SPACE_SM),
            );
        } else {
            content = content.push(
                text("Loading library stats...")
                    .size(style::TEXT_SM)
                    .color(cs.on_surface_variant),
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
            content = content.push(text(&self.export_status).size(style::TEXT_SM).color(color));
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

        container(
            column![
                text("About")
                    .size(style::TEXT_XS)
                    .color(cs.on_surface_variant),
                text(format!("kurozumi v{version}")).size(style::TEXT_BASE),
                row![
                    text("Config:")
                        .size(style::TEXT_SM)
                        .color(cs.on_surface_variant),
                    text(config_path).size(style::TEXT_SM).color(cs.outline),
                ]
                .spacing(style::SPACE_SM),
                row![
                    text("Database:")
                        .size(style::TEXT_SM)
                        .color(cs.on_surface_variant),
                    text(db_path).size(style::TEXT_SM).color(cs.outline),
                ]
                .spacing(style::SPACE_SM),
            ]
            .spacing(style::SPACE_SM),
        )
        .style(theme::card(cs))
        .padding(style::SPACE_LG)
        .width(Length::Fill)
        .into()
    }
}
