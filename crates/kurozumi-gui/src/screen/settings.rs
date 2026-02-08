use iced::widget::{button, column, pick_list, row, text, text_input, toggler};
use iced::{Alignment, Element, Length};
use iced_aw::Card;

use kurozumi_core::config::{AppConfig, ThemeMode};

use crate::screen::Action;
use crate::style;
use crate::theme::{self, available_themes, ColorScheme};

/// Settings screen state (transient form fields, separate from AppConfig until Save).
pub struct Settings {
    pub interval_input: String,
    pub auto_update: bool,
    pub confirm_update: bool,
    pub mal_enabled: bool,
    pub mal_client_id: String,
    pub selected_theme: String,
    pub selected_mode: ThemeMode,
    pub available_theme_names: Vec<String>,
    pub status_message: String,
    /// Whether we believe MAL is authenticated (token exists in DB).
    pub mal_authenticated: bool,
    /// Status feedback for MAL operations.
    pub mal_status: String,
    /// Whether a MAL operation is in progress.
    pub mal_busy: bool,
}

/// Messages handled by the Settings screen.
#[derive(Debug, Clone)]
pub enum Message {
    IntervalChanged(String),
    AutoUpdateToggled(bool),
    ConfirmUpdateToggled(bool),
    MalEnabledToggled(bool),
    MalClientIdChanged(String),
    ThemeChanged(String),
    ModeChanged(ThemeMode),
    Save,
    // MAL actions
    MalLogin,
    MalLoginResult(Result<(), String>),
    MalImport,
    MalImportResult(Result<usize, String>),
    MalTokenChecked(bool),
}

impl Settings {
    /// Initialize form state from the current config.
    pub fn from_config(config: &AppConfig) -> Self {
        let theme_names: Vec<String> = available_themes().iter().map(|t| t.name.clone()).collect();

        Self {
            interval_input: config.general.detection_interval.to_string(),
            auto_update: config.library.auto_update,
            confirm_update: config.library.confirm_update,
            mal_enabled: config.services.mal.enabled,
            mal_client_id: config.services.mal.client_id.clone().unwrap_or_default(),
            selected_theme: config.appearance.theme.clone(),
            selected_mode: config.appearance.mode,
            available_theme_names: theme_names,
            status_message: String::new(),
            mal_authenticated: false,
            mal_status: String::new(),
            mal_busy: false,
        }
    }

    /// Write form values back into the config.
    pub fn apply_to_config(&self, config: &mut AppConfig) {
        let interval = self
            .interval_input
            .parse::<u64>()
            .unwrap_or(config.general.detection_interval)
            .clamp(1, 300);

        config.general.detection_interval = interval;
        config.library.auto_update = self.auto_update;
        config.library.confirm_update = self.confirm_update;
        config.services.mal.enabled = self.mal_enabled;
        config.services.mal.client_id = if self.mal_client_id.trim().is_empty() {
            None
        } else {
            Some(self.mal_client_id.trim().to_string())
        };
        config.appearance.theme = self.selected_theme.clone();
        config.appearance.mode = self.selected_mode;
    }

    /// Handle a settings message, returning an Action for the app router.
    pub fn update(&mut self, msg: Message, config: &mut AppConfig) -> Action {
        match msg {
            Message::IntervalChanged(val) => {
                self.interval_input = val;
                Action::None
            }
            Message::AutoUpdateToggled(val) => {
                self.auto_update = val;
                Action::None
            }
            Message::ConfirmUpdateToggled(val) => {
                self.confirm_update = val;
                Action::None
            }
            Message::MalEnabledToggled(val) => {
                self.mal_enabled = val;
                Action::None
            }
            Message::MalClientIdChanged(val) => {
                self.mal_client_id = val;
                Action::None
            }
            Message::ThemeChanged(name) => {
                self.selected_theme = name;
                Action::None
            }
            Message::ModeChanged(mode) => {
                self.selected_mode = mode;
                Action::None
            }
            Message::Save => {
                self.apply_to_config(config);
                match config.save() {
                    Ok(()) => {
                        self.status_message = "Settings saved.".into();
                        Action::SetStatus("Settings saved.".into())
                    }
                    Err(e) => {
                        self.status_message = format!("Save failed: {e}");
                        Action::None
                    }
                }
            }
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
        }
    }

    pub fn view<'a>(&'a self, cs: &ColorScheme) -> Element<'a, Message> {
        let heading = text("Settings").size(style::TEXT_2XL);

        let appearance_card: Element<'_, Message> = Card::new(
            text("Appearance")
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant),
            column![
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
            .spacing(style::SPACE_MD),
        )
        .width(Length::Fill)
        .padding_head(style::SPACE_MD.into())
        .padding_body(style::SPACE_LG.into())
        .style(theme::aw_card_style(cs))
        .into();

        let detection_card: Element<'_, Message> = Card::new(
            text("Detection")
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant),
            column![row![
                text("Detection interval (seconds)")
                    .size(style::TEXT_BASE)
                    .width(Length::Fill),
                text_input("5", &self.interval_input)
                    .on_input(Message::IntervalChanged)
                    .size(style::TEXT_SM)
                    .padding([style::SPACE_XS, style::SPACE_SM])
                    .width(Length::Fixed(80.0))
                    .style(theme::text_input_style(cs)),
            ]
            .align_y(Alignment::Center)
            .spacing(style::SPACE_MD),],
        )
        .width(Length::Fill)
        .padding_head(style::SPACE_MD.into())
        .padding_body(style::SPACE_LG.into())
        .style(theme::aw_card_style(cs))
        .into();

        let library_card: Element<'_, Message> = Card::new(
            text("Library")
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant),
            column![
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
            .spacing(style::SPACE_MD),
        )
        .width(Length::Fill)
        .padding_head(style::SPACE_MD.into())
        .padding_body(style::SPACE_LG.into())
        .style(theme::aw_card_style(cs))
        .into();

        let mut mal_content = column![toggler(self.mal_enabled)
            .label("Enable MAL sync")
            .text_size(style::TEXT_BASE)
            .on_toggle(Message::MalEnabledToggled)
            .spacing(style::SPACE_SM),]
        .spacing(style::SPACE_MD);

        if self.mal_enabled {
            mal_content = mal_content.push(
                row![
                    text("Client ID").size(style::TEXT_BASE).width(Length::Fill),
                    text_input("your-client-id", &self.mal_client_id)
                        .on_input(Message::MalClientIdChanged)
                        .size(style::TEXT_SM)
                        .padding([style::SPACE_XS, style::SPACE_SM])
                        .width(Length::Fixed(240.0))
                        .style(theme::text_input_style(cs)),
                ]
                .align_y(Alignment::Center)
                .spacing(style::SPACE_MD),
            );

            // Show MAL action buttons when client ID is present.
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

                mal_content = mal_content.push(actions);
            }

            // Status message for MAL operations.
            if !self.mal_status.is_empty() {
                let color =
                    if self.mal_status.contains("failed") || self.mal_status.contains("Error") {
                        cs.error
                    } else {
                        cs.status_completed
                    };
                mal_content =
                    mal_content.push(text(&self.mal_status).size(style::TEXT_SM).color(color));
            }
        }

        let mal_card: Element<'_, Message> = Card::new(
            text("MyAnimeList")
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant),
            mal_content,
        )
        .width(Length::Fill)
        .padding_head(style::SPACE_MD.into())
        .padding_body(style::SPACE_LG.into())
        .style(theme::aw_card_style(cs))
        .into();

        let mut footer = row![button(text("Save Settings").size(style::TEXT_SM))
            .padding([style::SPACE_SM, style::SPACE_XL])
            .on_press(Message::Save)
            .style(theme::primary_button(cs)),]
        .spacing(style::SPACE_MD)
        .align_y(Alignment::Center);

        if !self.status_message.is_empty() {
            let color = if self.status_message.starts_with("Save failed") {
                cs.error
            } else {
                cs.status_completed
            };
            footer = footer.push(text(&self.status_message).size(style::TEXT_SM).color(color));
        }

        let page = column![
            heading,
            appearance_card,
            detection_card,
            library_card,
            mal_card,
            footer
        ]
        .spacing(style::SPACE_LG)
        .padding(style::SPACE_XL)
        .width(Length::Fill);

        iced::widget::scrollable(page).height(Length::Fill).into()
    }
}
