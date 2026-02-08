use iced::widget::{button, column, container, row, text};
use iced::window;
use iced::{Alignment, Element, Length, Subscription, Task, Theme};

use kurozumi_core::config::AppConfig;
use kurozumi_core::models::DetectedMedia;
use kurozumi_core::orchestrator::UpdateOutcome;

use crate::db::DbHandle;
use crate::screen::{library, now_playing, settings, Action, ModalKind, Page};
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
    settings: settings::Settings,
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
            settings: settings_screen,
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
    DetectionTick,
    DetectionResult(Option<DetectedMedia>),
    DetectionProcessed(Result<UpdateOutcome, String>),
    AppearanceChanged(ThemeMode),
    WindowEvent(window::Event),
    NowPlaying(now_playing::Message),
    Library(library::Message),
    Settings(settings::Message),
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
                    let action = self.library.refresh_task(self.db.as_ref());
                    return self.handle_action(action);
                }
                Task::none()
            }
            Message::DetectionTick => Task::perform(detect_and_parse(), Message::DetectionResult),
            Message::DetectionResult(media) => {
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
                match result {
                    Ok(outcome) => {
                        self.status_message = match &outcome {
                            UpdateOutcome::Updated {
                                anime_title,
                                episode,
                            } => {
                                format!("Updated {anime_title} to ep {episode}")
                            }
                            UpdateOutcome::AddedToLibrary {
                                anime_title,
                                episode,
                            } => {
                                format!("Added {anime_title} (ep {episode}) to library")
                            }
                            UpdateOutcome::AlreadyCurrent { .. } => self.status_message.clone(),
                            UpdateOutcome::Unrecognized { raw_title } => {
                                format!("Unrecognized: {raw_title}")
                            }
                            UpdateOutcome::NothingPlaying => self.status_message.clone(),
                        };
                        self.now_playing.last_outcome = Some(outcome);
                    }
                    Err(e) => {
                        self.status_message = format!("Error: {e}");
                    }
                }
                if self.page == Page::Library {
                    let action = self.library.refresh_task(self.db.as_ref());
                    return self.handle_action(action);
                }
                Task::none()
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
            Message::NowPlaying(_msg) => {
                // NowPlaying has no interactive messages yet.
                Task::none()
            }
            Message::Library(msg) => {
                let action = self.library.update(msg, self.db.as_ref());
                self.handle_action(action)
            }
            Message::Settings(msg) => {
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

    pub fn view(&self) -> Element<'_, Message> {
        let cs = &self.current_theme.colors;
        let nav = self.nav_rail(cs);

        let page_content: Element<'_, Message> = match self.page {
            Page::NowPlaying => self
                .now_playing
                .view(cs, &self.status_message)
                .map(Message::NowPlaying),
            Page::Library => self.library.view(cs).map(Message::Library),
            Page::Search => container(
                column![
                    text("Search")
                        .size(style::TEXT_XL)
                        .color(cs.on_surface_variant),
                    text("Coming soon").size(style::TEXT_SM).color(cs.outline),
                ]
                .spacing(style::SPACE_SM),
            )
            .padding(style::SPACE_XL)
            .into(),
            Page::Settings => self.settings.view(cs).map(Message::Settings),
        };

        let status_bar = container(text(&self.status_message).size(style::TEXT_XS))
            .style(theme::status_bar(cs))
            .width(Length::Fill)
            .height(Length::Fixed(style::STATUS_BAR_HEIGHT))
            .padding([4.0, style::SPACE_MD]);

        let main: Element<'_, Message> =
            column![row![nav, page_content].height(Length::Fill), status_bar,].into();

        // Wrap in modal if one is active.
        if let Some(modal_kind) = &self.modal_state {
            let modal_content = self.build_modal_content(cs, modal_kind);
            crate::widgets::modal(
                main,
                modal_content,
                Message::Library(library::Message::CancelModal),
            )
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
            ModalKind::ConfirmDelete { anime_id, title } => {
                let anime_id = *anime_id;
                container(
                    column![
                        text("Remove from library?").size(style::TEXT_LG),
                        text(title.as_str())
                            .size(style::TEXT_SM)
                            .color(cs.on_surface_variant),
                        text("This action cannot be undone.")
                            .size(style::TEXT_XS)
                            .color(cs.outline),
                        row![
                            button(text("Cancel").size(style::TEXT_SM))
                                .padding([style::SPACE_SM, style::SPACE_XL])
                                .on_press(Message::Library(library::Message::CancelModal))
                                .style(theme::ghost_button(cs)),
                            button(text("Delete").size(style::TEXT_SM))
                                .padding([style::SPACE_SM, style::SPACE_XL])
                                .on_press(Message::Library(library::Message::ConfirmDelete(
                                    anime_id
                                )))
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
            container(text("K").size(style::TEXT_XL).color(cs.primary),)
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
