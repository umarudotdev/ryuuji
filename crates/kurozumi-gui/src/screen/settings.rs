use iced::widget::{button, column, container, pick_list, row, text, text_input, toggler};
use iced::{Alignment, Element, Length};

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
}

impl Settings {
    /// Initialize form state from the current config.
    pub fn from_config(config: &AppConfig) -> Self {
        let theme_names: Vec<String> = available_themes()
            .iter()
            .map(|t| t.name.clone())
            .collect();

        Self {
            interval_input: config.general.detection_interval.to_string(),
            auto_update: config.library.auto_update,
            confirm_update: config.library.confirm_update,
            mal_enabled: config.services.mal.enabled,
            mal_client_id: config
                .services
                .mal
                .client_id
                .clone()
                .unwrap_or_default(),
            selected_theme: config.appearance.theme.clone(),
            selected_mode: config.appearance.mode,
            available_theme_names: theme_names,
            status_message: String::new(),
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
        }
    }

    pub fn view<'a>(&'a self, cs: &ColorScheme) -> Element<'a, Message> {
        let heading = text("Settings").size(style::TEXT_2XL);

        let appearance_card = settings_card(
            cs,
            "Appearance",
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
        );

        let detection_card = settings_card(
            cs,
            "Detection",
            column![
                row![
                    text("Detection interval (seconds)")
                        .size(style::TEXT_BASE)
                        .width(Length::Fill),
                    text_input("5", &self.interval_input)
                        .on_input(|v| Message::IntervalChanged(v))
                        .size(style::TEXT_SM)
                        .padding([style::SPACE_XS, style::SPACE_SM])
                        .width(Length::Fixed(80.0))
                        .style(theme::text_input_style(cs)),
                ]
                .align_y(Alignment::Center)
                .spacing(style::SPACE_MD),
            ],
        );

        let library_card = settings_card(
            cs,
            "Library",
            column![
                toggler(self.auto_update)
                    .label("Auto-update progress from detected playback")
                    .text_size(style::TEXT_BASE)
                    .on_toggle(|v| Message::AutoUpdateToggled(v))
                    .spacing(style::SPACE_SM),
                toggler(self.confirm_update)
                    .label("Confirm before updating")
                    .text_size(style::TEXT_BASE)
                    .on_toggle(|v| Message::ConfirmUpdateToggled(v))
                    .spacing(style::SPACE_SM),
            ]
            .spacing(style::SPACE_MD),
        );

        let mut mal_content = column![
            toggler(self.mal_enabled)
                .label("Enable MAL sync")
                .text_size(style::TEXT_BASE)
                .on_toggle(|v| Message::MalEnabledToggled(v))
                .spacing(style::SPACE_SM),
        ]
        .spacing(style::SPACE_MD);

        if self.mal_enabled {
            mal_content = mal_content.push(
                row![
                    text("Client ID").size(style::TEXT_BASE).width(Length::Fill),
                    text_input("your-client-id", &self.mal_client_id)
                        .on_input(|v| Message::MalClientIdChanged(v))
                        .size(style::TEXT_SM)
                        .padding([style::SPACE_XS, style::SPACE_SM])
                        .width(Length::Fixed(240.0))
                        .style(theme::text_input_style(cs)),
                ]
                .align_y(Alignment::Center)
                .spacing(style::SPACE_MD),
            );
        }

        let mal_card = settings_card(cs, "MyAnimeList", mal_content);

        let mut footer = row![
            button(text("Save Settings").size(style::TEXT_SM))
                .padding([style::SPACE_SM, style::SPACE_XL])
                .on_press(Message::Save)
                .style(theme::primary_button(cs)),
        ]
        .spacing(style::SPACE_MD)
        .align_y(Alignment::Center);

        if !self.status_message.is_empty() {
            let color = if self.status_message.starts_with("Save failed") {
                cs.error
            } else {
                cs.status_completed
            };
            footer = footer.push(
                text(&self.status_message).size(style::TEXT_SM).color(color),
            );
        }

        let page = column![heading, appearance_card, detection_card, library_card, mal_card, footer]
            .spacing(style::SPACE_LG)
            .padding(style::SPACE_XL)
            .width(Length::Fill);

        iced::widget::scrollable(page).height(Length::Fill).into()
    }
}

/// Helper: wrap content in a labeled card container.
fn settings_card<'a>(
    cs: &ColorScheme,
    label: &'a str,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        column![
            text(label)
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant),
            content.into(),
        ]
        .spacing(style::SPACE_MD),
    )
    .style(theme::card(cs))
    .padding(style::SPACE_LG)
    .width(Length::Fill)
    .into()
}
