use iced::widget::{button, column, container, pick_list, row, rule, scrollable, text, text_input};
use iced::{Alignment, Background, Border, Color, Element, Length, Task, Theme};

use kurozumi_core::models::WatchStatus;
use kurozumi_core::storage::LibraryRow;

use crate::app;
use crate::db::DbHandle;
use crate::screen::{Action, ModalKind, Page};
use crate::style;
use crate::theme::{self, ColorScheme};
use crate::widgets::context_menu::context_menu;

/// Actions available in the search context menu (mirrors library).
#[derive(Debug, Clone)]
pub enum ContextAction {
    ChangeStatus(WatchStatus),
    Delete,
}

/// Search screen state.
pub struct Search {
    query: String,
    all_entries: Vec<LibraryRow>,
    filtered_indices: Vec<usize>,
    loaded: bool,
    selected_anime: Option<i64>,
    score_input: String,
}

/// Messages handled by the Search screen.
#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(String),
    EntriesLoaded(Result<Vec<LibraryRow>, String>),
    AnimeSelected(i64),
    EpisodeIncrement(i64),
    EpisodeDecrement(i64),
    StatusChanged(i64, WatchStatus),
    ScoreInputChanged(String),
    ScoreSubmitted(i64),
    ContextAction(i64, ContextAction),
    ConfirmDelete(i64),
    CancelModal,
    DbOperationDone(Result<(), String>),
}

impl Search {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            all_entries: Vec::new(),
            filtered_indices: Vec::new(),
            loaded: false,
            selected_anime: None,
            score_input: String::new(),
        }
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

    /// Rebuild `filtered_indices` from the current query.
    fn refilter(&mut self) {
        let q = self.query.to_lowercase();
        if q.is_empty() {
            self.filtered_indices = (0..self.all_entries.len()).collect();
        } else {
            self.filtered_indices = self
                .all_entries
                .iter()
                .enumerate()
                .filter(|(_, row)| matches_query(row, &q))
                .map(|(i, _)| i)
                .collect();
        }
    }

    /// Handle a search message, returning an Action for the app router.
    pub fn update(&mut self, msg: Message, db: Option<&DbHandle>) -> Action {
        match msg {
            Message::QueryChanged(new_query) => {
                self.query = new_query;
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
                Action::None
            }
            Message::EntriesLoaded(result) => {
                if let Ok(mut entries) = result {
                    entries.sort_by(|a, b| {
                        a.anime
                            .title
                            .preferred()
                            .to_lowercase()
                            .cmp(&b.anime.title.preferred().to_lowercase())
                    });
                    self.all_entries = entries;
                    self.loaded = true;
                    self.refilter();
                }
                Action::None
            }
            Message::AnimeSelected(id) => {
                self.selected_anime = Some(id);
                if let Some(row) = self.all_entries.iter().find(|r| r.anime.id == id) {
                    self.score_input = row
                        .entry
                        .score
                        .map(|s| format!("{s:.0}"))
                        .unwrap_or_default();
                }
                Action::None
            }
            Message::EpisodeIncrement(anime_id) => {
                if let Some(db) = db {
                    if let Some(entry) = self.all_entries.iter().find(|r| r.anime.id == anime_id) {
                        let new_ep = entry.entry.watched_episodes + 1;
                        let db = db.clone();
                        return Action::RunTask(Task::perform(
                            async move {
                                let _ = db.update_episode_count(anime_id, new_ep).await;
                                let _ = db.record_watch(anime_id, new_ep).await;
                            },
                            |_| app::Message::Search(Message::DbOperationDone(Ok(()))),
                        ));
                    }
                }
                Action::None
            }
            Message::EpisodeDecrement(anime_id) => {
                if let Some(db) = db {
                    if let Some(entry) = self.all_entries.iter().find(|r| r.anime.id == anime_id) {
                        if entry.entry.watched_episodes > 0 {
                            let new_ep = entry.entry.watched_episodes - 1;
                            let db = db.clone();
                            return Action::RunTask(Task::perform(
                                async move {
                                    let _ = db.update_episode_count(anime_id, new_ep).await;
                                },
                                |_| app::Message::Search(Message::DbOperationDone(Ok(()))),
                            ));
                        }
                    }
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
            Message::ScoreInputChanged(val) => {
                self.score_input = val;
                Action::None
            }
            Message::ScoreSubmitted(anime_id) => {
                if let Some(db) = db {
                    if let Ok(score) = self.score_input.parse::<f32>() {
                        let score = score.clamp(0.0, 10.0);
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
        }
    }

    pub fn view<'a>(&'a self, cs: &'a ColorScheme) -> Element<'a, Message> {
        let search_icon = lucide_icons::iced::icon_search()
            .size(style::TEXT_BASE)
            .color(cs.on_surface_variant);

        let search_input = text_input("Search library...", &self.query)
            .on_input(Message::QueryChanged)
            .size(style::TEXT_BASE)
            .padding([style::SPACE_SM, style::SPACE_MD])
            .width(Length::Fill)
            .style(theme::text_input_style(cs));

        let header = row![search_icon, search_input]
            .spacing(style::SPACE_SM)
            .align_y(Alignment::Center)
            .padding([style::SPACE_SM, style::SPACE_LG]);

        let result_count = if self.query.is_empty() {
            format!(
                "{} {}",
                self.all_entries.len(),
                if self.all_entries.len() == 1 {
                    "entry"
                } else {
                    "entries"
                }
            )
        } else {
            format!(
                "{} {}",
                self.filtered_indices.len(),
                if self.filtered_indices.len() == 1 {
                    "result"
                } else {
                    "results"
                }
            )
        };

        let count_row = container(text(result_count).size(style::TEXT_XS).color(cs.outline))
            .padding([0.0, style::SPACE_LG]);

        let list: Element<'_, Message> = if !self.loaded {
            container(
                text("Loading...")
                    .size(style::TEXT_SM)
                    .color(cs.on_surface_variant),
            )
            .padding(style::SPACE_3XL)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .into()
        } else if self.filtered_indices.is_empty() {
            let msg = if self.query.is_empty() {
                "Your library is empty."
            } else {
                "No matching anime found."
            };
            container(text(msg).size(style::TEXT_SM).color(cs.on_surface_variant))
                .padding(style::SPACE_3XL)
                .width(Length::Fill)
                .center_x(Length::Fill)
                .into()
        } else {
            let items: Vec<Element<'a, Message>> = self
                .filtered_indices
                .iter()
                .map(|&i| search_list_item(cs, &self.all_entries[i], self.selected_anime))
                .collect();

            scrollable(
                column(items)
                    .spacing(style::SPACE_XXS)
                    .padding([style::SPACE_XS, style::SPACE_LG]),
            )
            .height(Length::Fill)
            .into()
        };

        let content = column![header, count_row, rule::horizontal(1), list]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill);

        if let Some(anime_id) = self.selected_anime {
            if let Some(lib_row) = self.all_entries.iter().find(|r| r.anime.id == anime_id) {
                let detail = search_detail(cs, lib_row, &self.score_input);
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

/// A single search result list item with context menu.
fn search_list_item<'a>(
    cs: &'a ColorScheme,
    lib_row: &'a LibraryRow,
    selected: Option<i64>,
) -> Element<'a, Message> {
    let title = lib_row.anime.title.preferred();
    let progress = match lib_row.anime.episodes {
        Some(total) => format!("{} / {}", lib_row.entry.watched_episodes, total),
        None => format!("{}", lib_row.entry.watched_episodes),
    };

    let is_selected = selected == Some(lib_row.anime.id);
    let anime_id = lib_row.anime.id;
    let status_col = theme::status_color(cs, lib_row.entry.status);

    let status_label = text(lib_row.entry.status.as_str())
        .size(style::TEXT_XS)
        .color(cs.on_surface_variant);

    let status_bar = container(text("").size(1))
        .width(Length::Fixed(3.0))
        .height(Length::Fill)
        .style(theme::status_bar_accent(status_col));

    let content = row![
        status_bar,
        text(title).size(style::TEXT_BASE).width(Length::Fill),
        status_label,
        text(progress)
            .size(style::TEXT_SM)
            .color(cs.on_surface_variant),
    ]
    .spacing(style::SPACE_SM)
    .align_y(Alignment::Center);

    let base = button(content)
        .width(Length::Fill)
        .padding([style::SPACE_SM, style::SPACE_MD])
        .on_press(Message::AnimeSelected(anime_id))
        .style(theme::list_item(is_selected, cs));

    let primary = cs.primary;
    let on_primary = cs.on_primary;
    let on_surface = cs.on_surface;
    let error = cs.error;
    let on_error = cs.on_error;
    let menu_bg = cs.surface_container_high;
    let menu_border = cs.outline;
    let menu_item = move |label: &'a str, msg: Message| -> Element<'a, Message> {
        button(text(label).size(style::TEXT_SM))
            .width(Length::Fill)
            .padding([style::SPACE_XS, style::SPACE_MD])
            .on_press(msg)
            .style(move |_theme: &Theme, status| {
                let (bg, tc) = match status {
                    button::Status::Hovered => (Some(Background::Color(primary)), on_primary),
                    _ => (None, on_surface),
                };
                button::Style {
                    background: bg,
                    text_color: tc,
                    border: Border {
                        radius: style::RADIUS_SM.into(),
                        ..Border::default()
                    },
                    ..Default::default()
                }
            })
            .into()
    };

    context_menu(base, move || {
        container(
            column![
                menu_item(
                    "Watching",
                    Message::ContextAction(
                        anime_id,
                        ContextAction::ChangeStatus(WatchStatus::Watching),
                    ),
                ),
                menu_item(
                    "Completed",
                    Message::ContextAction(
                        anime_id,
                        ContextAction::ChangeStatus(WatchStatus::Completed),
                    ),
                ),
                menu_item(
                    "On Hold",
                    Message::ContextAction(
                        anime_id,
                        ContextAction::ChangeStatus(WatchStatus::OnHold),
                    ),
                ),
                menu_item(
                    "Dropped",
                    Message::ContextAction(
                        anime_id,
                        ContextAction::ChangeStatus(WatchStatus::Dropped),
                    ),
                ),
                menu_item(
                    "Plan to Watch",
                    Message::ContextAction(
                        anime_id,
                        ContextAction::ChangeStatus(WatchStatus::PlanToWatch),
                    ),
                ),
                rule::horizontal(1),
                button(text("Delete").size(style::TEXT_SM))
                    .width(Length::Fill)
                    .padding([style::SPACE_XS, style::SPACE_MD])
                    .on_press(Message::ContextAction(anime_id, ContextAction::Delete))
                    .style(move |_theme: &Theme, status| {
                        let (bg, tc) = match status {
                            button::Status::Hovered => (Some(Background::Color(error)), on_error),
                            _ => (None, error),
                        };
                        button::Style {
                            background: bg,
                            text_color: tc,
                            border: Border {
                                radius: style::RADIUS_SM.into(),
                                ..Border::default()
                            },
                            ..Default::default()
                        }
                    }),
            ]
            .spacing(style::SPACE_XXS)
            .width(Length::Fixed(160.0)),
        )
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(menu_bg)),
            border: Border {
                color: menu_border,
                width: 1.0,
                radius: style::RADIUS_MD.into(),
            },
            shadow: iced::Shadow {
                color: Color::BLACK,
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 16.0,
            },
            ..Default::default()
        })
        .padding(style::SPACE_XS)
        .into()
    })
}

/// Detail panel for the selected anime (mirrors library detail).
fn search_detail<'a>(
    cs: &ColorScheme,
    lib_row: &'a LibraryRow,
    score_input: &str,
) -> Element<'a, Message> {
    let anime = &lib_row.anime;
    let entry = &lib_row.entry;

    let cover = container(
        text("\u{1F3AC}")
            .size(style::TEXT_3XL)
            .color(cs.outline)
            .center(),
    )
    .width(Length::Fixed(style::COVER_WIDTH))
    .height(Length::Fixed(style::COVER_HEIGHT))
    .center_x(Length::Fixed(style::COVER_WIDTH))
    .center_y(Length::Fixed(style::COVER_HEIGHT))
    .style(theme::cover_placeholder(cs));

    let mut title_section =
        column![text(anime.title.preferred()).size(style::TEXT_XL),].spacing(style::SPACE_XS);

    if let Some(english) = &anime.title.english {
        if Some(english.as_str()) != anime.title.romaji.as_deref() {
            title_section = title_section.push(
                text(english.as_str())
                    .size(style::TEXT_SM)
                    .color(cs.on_surface_variant),
            );
        }
    }

    let mut meta_parts: Vec<String> = Vec::new();
    if let Some(season) = &anime.season {
        meta_parts.push(season.clone());
    }
    if let Some(year) = anime.year {
        meta_parts.push(year.to_string());
    }
    if !meta_parts.is_empty() {
        title_section = title_section.push(
            text(meta_parts.join(" "))
                .size(style::TEXT_SM)
                .color(cs.outline),
        );
    }

    let anime_id = anime.id;
    let status_card = container(
        column![
            text("Status")
                .size(style::TEXT_XS)
                .color(cs.on_surface_variant),
            pick_list(WatchStatus::ALL, Some(entry.status), move |s| {
                Message::StatusChanged(anime_id, s)
            })
            .text_size(style::TEXT_SM)
            .padding([style::SPACE_XS, style::SPACE_SM]),
            text("Score")
                .size(style::TEXT_XS)
                .color(cs.on_surface_variant),
            row![text_input("0-10", score_input)
                .on_input(Message::ScoreInputChanged)
                .on_submit(Message::ScoreSubmitted(anime_id))
                .size(style::TEXT_SM)
                .padding([style::SPACE_XS, style::SPACE_SM])
                .width(Length::Fixed(80.0))
                .style(theme::text_input_style(cs)),]
            .spacing(style::SPACE_SM)
            .align_y(Alignment::Center),
        ]
        .spacing(style::SPACE_SM),
    )
    .style(theme::card(cs))
    .padding(style::SPACE_LG)
    .width(Length::Fill);

    let ep_text = match anime.episodes {
        Some(total) => format!("Episode {} / {}", entry.watched_episodes, total),
        None => format!("Episode {}", entry.watched_episodes),
    };

    let progress_card = container(
        column![
            text(ep_text).size(style::TEXT_BASE),
            row![
                button(lucide_icons::iced::icon_minus().size(style::TEXT_SM))
                    .on_press(Message::EpisodeDecrement(anime_id))
                    .style(theme::control_button(cs))
                    .padding([style::SPACE_XS, style::SPACE_LG]),
                button(lucide_icons::iced::icon_plus().size(style::TEXT_SM))
                    .on_press(Message::EpisodeIncrement(anime_id))
                    .style(theme::control_button(cs))
                    .padding([style::SPACE_XS, style::SPACE_LG]),
            ]
            .spacing(style::SPACE_SM),
        ]
        .spacing(style::SPACE_SM),
    )
    .style(theme::card(cs))
    .padding(style::SPACE_LG)
    .width(Length::Fill);

    let detail = column![
        row![cover, title_section]
            .spacing(style::SPACE_LG)
            .align_y(Alignment::Start),
        status_card,
        progress_card,
    ]
    .spacing(style::SPACE_LG)
    .padding(style::SPACE_LG);

    scrollable(detail).height(Length::Fill).into()
}
