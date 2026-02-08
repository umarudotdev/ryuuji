use iced::widget::{button, column, container, pick_list, row, text, text_input, toggler};
use iced::{Alignment, Element, Length};

use kurozumi_core::config::AppConfig;

use crate::app::{Message, SettingsMsg};
use crate::style;
use crate::theme::{self, ColorScheme, ThemeMode};

/// Transient settings form state (separate from AppConfig until Save).
#[derive(Debug, Clone)]
pub struct SettingsState {
    pub interval_input: String,
    pub auto_update: bool,
    pub confirm_update: bool,
    pub mal_enabled: bool,
    pub mal_client_id: String,
    pub theme_mode: ThemeMode,
    pub status_message: String,
}

impl SettingsState {
    /// Initialize form state from the current config.
    pub fn from_config(config: &AppConfig) -> Self {
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
            theme_mode: ThemeMode::default(),
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
    }
}

/// Render the Settings page.
pub fn view<'a>(cs: &ColorScheme, state: &'a SettingsState) -> Element<'a, Message> {
    let heading = text("Settings").size(style::TEXT_2XL);

    // Appearance card — theme toggle.
    let appearance_card = settings_card(
        cs,
        "Appearance",
        column![
            row![
                text("Theme").size(style::TEXT_BASE).width(Length::Fill),
                pick_list(ThemeMode::ALL, Some(state.theme_mode), |m| {
                    Message::Settings(SettingsMsg::ThemeModeChanged(m))
                })
                .text_size(style::TEXT_SM)
                .padding([style::SPACE_XS, style::SPACE_SM]),
            ]
            .align_y(Alignment::Center)
            .spacing(style::SPACE_MD),
        ],
    );

    // Detection card.
    let detection_card = settings_card(
        cs,
        "Detection",
        column![
            row![
                text("Detection interval (seconds)")
                    .size(style::TEXT_BASE)
                    .width(Length::Fill),
                text_input("5", &state.interval_input)
                    .on_input(|v| Message::Settings(SettingsMsg::IntervalChanged(v)))
                    .size(style::TEXT_SM)
                    .padding([style::SPACE_XS, style::SPACE_SM])
                    .width(Length::Fixed(80.0))
                    .style(theme::text_input_style(cs)),
            ]
            .align_y(Alignment::Center)
            .spacing(style::SPACE_MD),
        ],
    );

    // Library card.
    let library_card = settings_card(
        cs,
        "Library",
        column![
            toggler(state.auto_update)
                .label("Auto-update progress from detected playback")
                .text_size(style::TEXT_BASE)
                .on_toggle(|v| Message::Settings(SettingsMsg::AutoUpdateToggled(v)))
                .spacing(style::SPACE_SM),
            toggler(state.confirm_update)
                .label("Confirm before updating")
                .text_size(style::TEXT_BASE)
                .on_toggle(|v| Message::Settings(SettingsMsg::ConfirmUpdateToggled(v)))
                .spacing(style::SPACE_SM),
        ]
        .spacing(style::SPACE_MD),
    );

    // MAL card.
    let mut mal_content = column![
        toggler(state.mal_enabled)
            .label("Enable MAL sync")
            .text_size(style::TEXT_BASE)
            .on_toggle(|v| Message::Settings(SettingsMsg::MalEnabledToggled(v)))
            .spacing(style::SPACE_SM),
    ]
    .spacing(style::SPACE_MD);

    if state.mal_enabled {
        mal_content = mal_content.push(
            row![
                text("Client ID").size(style::TEXT_BASE).width(Length::Fill),
                text_input("your-client-id", &state.mal_client_id)
                    .on_input(|v| Message::Settings(SettingsMsg::MalClientIdChanged(v)))
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

    // Save button + status.
    let mut footer = row![
        button(text("Save Settings").size(style::TEXT_SM))
            .padding([style::SPACE_SM, style::SPACE_XL])
            .on_press(Message::Settings(SettingsMsg::Save))
            .style(theme::primary_button(cs)),
    ]
    .spacing(style::SPACE_MD)
    .align_y(Alignment::Center);

    if !state.status_message.is_empty() {
        let color = if state.status_message.starts_with("Save failed") {
            cs.error
        } else {
            cs.status_completed
        };
        footer = footer.push(
            text(&state.status_message).size(style::TEXT_SM).color(color),
        );
    }

    let page = column![heading, appearance_card, detection_card, library_card, mal_card, footer]
        .spacing(style::SPACE_LG)
        .padding(style::SPACE_XL)
        .width(Length::Fill);

    iced::widget::scrollable(page).height(Length::Fill).into()
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
