use iced::widget::{
    button, column, container, pick_list, row, rule, scrollable, text, text_input,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use kurozumi_core::models::WatchStatus;
use kurozumi_core::storage::LibraryRow;

use crate::app::{ContextAction, LibraryMsg, LibrarySort, LibraryViewMode, Message};
use crate::style;
use crate::theme::{self, ColorScheme};
use crate::widgets::context_menu::context_menu;

/// Render the Library page.
pub fn view<'a>(
    cs: &'a ColorScheme,
    active_tab: WatchStatus,
    entries: &'a [LibraryRow],
    selected_anime_id: Option<i64>,
    sort: LibrarySort,
    score_input: &str,
    view_mode: LibraryViewMode,
) -> Element<'a, Message> {
    // Header: filter chips on left, count + view toggle + sort picker on right.
    let count_text = format!(
        "{} {}",
        entries.len(),
        if entries.len() == 1 { "entry" } else { "entries" }
    );

    let view_icon = match view_mode {
        LibraryViewMode::Grid => lucide_icons::iced::icon_list(),
        LibraryViewMode::List => lucide_icons::iced::icon_layout_grid(),
    };

    let header = row![
        chip_bar(cs, active_tab),
        text(count_text)
            .size(style::TEXT_XS)
            .color(cs.outline)
            .width(Length::Fill),
        button(view_icon.size(style::TEXT_BASE))
            .padding([style::SPACE_XS, style::SPACE_SM])
            .on_press(Message::Library(LibraryMsg::ViewModeToggled))
            .style(theme::ghost_button(cs)),
        pick_list(LibrarySort::ALL, Some(sort), |s| {
            Message::Library(LibraryMsg::SortChanged(s))
        })
        .text_size(style::TEXT_SM)
        .padding([style::SPACE_XS, style::SPACE_SM]),
    ]
    .spacing(style::SPACE_SM)
    .align_y(Alignment::Center)
    .padding([style::SPACE_SM, style::SPACE_LG]);

    // Anime list / grid.
    let list: Element<'_, Message> = if entries.is_empty() {
        container(
            column![
                text("No anime in this list.")
                    .size(style::TEXT_SM)
                    .color(cs.on_surface_variant),
            ]
            .align_x(Alignment::Center),
        )
        .padding(style::SPACE_3XL)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
    } else {
        match view_mode {
            LibraryViewMode::List => {
                let items: Vec<Element<'a, Message>> = entries
                    .iter()
                    .map(|r| anime_list_item(cs, r, selected_anime_id))
                    .collect();

                scrollable(
                    column(items)
                        .spacing(style::SPACE_XXS)
                        .padding([style::SPACE_XS, style::SPACE_LG]),
                )
                .height(Length::Fill)
                .into()
            }
            LibraryViewMode::Grid => {
                let mut cards: Vec<Element<'a, Message>> = entries
                    .iter()
                    .map(|r| grid_card(cs, r, selected_anime_id))
                    .collect();

                // Build rows of GRID_COLUMNS cards each.
                let mut grid_rows: Vec<Element<'a, Message>> = Vec::new();
                let mut drain = cards.drain(..);
                loop {
                    let mut grid_row = row![].spacing(style::SPACE_MD);
                    let mut count = 0;
                    for _ in 0..style::GRID_COLUMNS {
                        if let Some(card) = drain.next() {
                            grid_row = grid_row.push(card);
                            count += 1;
                        }
                    }
                    if count == 0 {
                        break;
                    }
                    // Pad incomplete rows
                    for _ in count..style::GRID_COLUMNS {
                        grid_row = grid_row.push(
                            container(text(""))
                                .width(Length::Fixed(style::GRID_CARD_WIDTH)),
                        );
                    }
                    grid_rows.push(grid_row.into());
                }

                scrollable(
                    column(grid_rows)
                        .spacing(style::SPACE_MD)
                        .padding([style::SPACE_SM, style::SPACE_LG]),
                )
                .height(Length::Fill)
                .into()
            }
        }
    };

    let content = column![header, rule::horizontal(1), list]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

    // If an anime is selected, show detail panel on the right.
    if let Some(anime_id) = selected_anime_id {
        if let Some(lib_row) = entries.iter().find(|r| r.anime.id == anime_id) {
            let detail = anime_detail(cs, lib_row, score_input);
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

/// Filter chip bar for watch status filtering (MD3 filter chips).
fn chip_bar(cs: &ColorScheme, active: WatchStatus) -> Element<'static, Message> {
    let chips: Vec<Element<'_, Message>> = WatchStatus::ALL
        .iter()
        .map(|&status| {
            let is_selected = status == active;
            let base_label = match status {
                WatchStatus::PlanToWatch => "Plan".to_string(),
                other => other.as_str().to_string(),
            };
            let label = if is_selected {
                format!("\u{2713} {base_label}")
            } else {
                base_label
            };

            button(
                text(label)
                    .size(style::TEXT_XS)
                    .center(),
            )
            .height(Length::Fixed(style::CHIP_HEIGHT))
            .padding([style::SPACE_XS, style::SPACE_MD])
            .on_press(Message::Library(LibraryMsg::TabChanged(status)))
            .style(theme::filter_chip(is_selected, cs))
            .into()
        })
        .collect();

    row(chips).spacing(style::SPACE_XS).into()
}

/// A single anime grid card — cover placeholder + title + progress.
fn grid_card<'a>(
    cs: &ColorScheme,
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

    // Status color bar at top (3px)
    let status_bar = container(text("").size(1))
        .width(Length::Fill)
        .height(Length::Fixed(3.0))
        .style(theme::status_bar_accent(status_col));

    // Cover placeholder
    let cover = container(
        text("\u{1F3AC}")
            .size(style::TEXT_3XL)
            .color(cs.outline)
            .center(),
    )
    .width(Length::Fill)
    .height(Length::Fixed(style::GRID_COVER_HEIGHT))
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .style(theme::grid_cover_placeholder(cs));

    // Info section
    let info = column![
        text(title)
            .size(style::TEXT_SM)
            .width(Length::Fill),
        text(progress)
            .size(style::TEXT_XS)
            .color(cs.on_surface_variant),
    ]
    .spacing(style::SPACE_XXS)
    .padding([style::SPACE_SM, style::SPACE_SM]);

    let card_content = column![status_bar, cover, info];

    button(card_content)
        .width(Length::Fixed(style::GRID_CARD_WIDTH))
        .padding(0)
        .on_press(Message::Library(LibraryMsg::AnimeSelected(anime_id)))
        .style(theme::grid_card(is_selected, cs))
        .into()
}

/// A single anime list item — card-style button wrapped in a context menu.
fn anime_list_item<'a>(
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

    // Thin vertical status color bar
    let status_bar = container(text("").size(1))
        .width(Length::Fixed(3.0))
        .height(Length::Fill)
        .style(theme::status_bar_accent(status_col));

    let content = row![
        status_bar,
        text(title).size(style::TEXT_BASE).width(Length::Fill),
        text(progress).size(style::TEXT_SM).color(cs.on_surface_variant),
    ]
    .spacing(style::SPACE_SM)
    .align_y(Alignment::Center);

    let base = button(content)
        .width(Length::Fill)
        .padding([style::SPACE_SM, style::SPACE_MD])
        .on_press(Message::Library(LibraryMsg::AnimeSelected(anime_id)))
        .style(theme::list_item(is_selected, cs));

    // Capture colors for menu closures (avoids borrowing cs inside the lambda).
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
                    Message::Library(LibraryMsg::ContextAction(
                        anime_id,
                        ContextAction::ChangeStatus(WatchStatus::Watching),
                    )),
                ),
                menu_item(
                    "Completed",
                    Message::Library(LibraryMsg::ContextAction(
                        anime_id,
                        ContextAction::ChangeStatus(WatchStatus::Completed),
                    )),
                ),
                menu_item(
                    "On Hold",
                    Message::Library(LibraryMsg::ContextAction(
                        anime_id,
                        ContextAction::ChangeStatus(WatchStatus::OnHold),
                    )),
                ),
                menu_item(
                    "Dropped",
                    Message::Library(LibraryMsg::ContextAction(
                        anime_id,
                        ContextAction::ChangeStatus(WatchStatus::Dropped),
                    )),
                ),
                menu_item(
                    "Plan to Watch",
                    Message::Library(LibraryMsg::ContextAction(
                        anime_id,
                        ContextAction::ChangeStatus(WatchStatus::PlanToWatch),
                    )),
                ),
                rule::horizontal(1),
                button(text("Delete").size(style::TEXT_SM))
                    .width(Length::Fill)
                    .padding([style::SPACE_XS, style::SPACE_MD])
                    .on_press(Message::Library(LibraryMsg::ContextAction(
                        anime_id,
                        ContextAction::Delete,
                    )))
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

/// Detail panel for the selected anime.
fn anime_detail<'a>(cs: &ColorScheme, lib_row: &'a LibraryRow, score_input: &str) -> Element<'a, Message> {
    let anime = &lib_row.anime;
    let entry = &lib_row.entry;

    // Cover placeholder.
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

    // Title section.
    let mut title_section = column![
        text(anime.title.preferred()).size(style::TEXT_XL),
    ]
    .spacing(style::SPACE_XS);

    if let Some(english) = &anime.title.english {
        if Some(english.as_str()) != anime.title.romaji.as_deref() {
            title_section = title_section
                .push(text(english.as_str()).size(style::TEXT_SM).color(cs.on_surface_variant));
        }
    }

    // Season / year info.
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

    // Status section -- card.
    let anime_id = anime.id;
    let status_card = container(
        column![
            text("Status").size(style::TEXT_XS).color(cs.on_surface_variant),
            pick_list(WatchStatus::ALL, Some(entry.status), move |s| {
                Message::Library(LibraryMsg::StatusChanged(anime_id, s))
            })
            .text_size(style::TEXT_SM)
            .padding([style::SPACE_XS, style::SPACE_SM]),
            text("Score").size(style::TEXT_XS).color(cs.on_surface_variant),
            row![
                text_input("0-10", score_input)
                    .on_input(|v| Message::Library(LibraryMsg::ScoreInputChanged(v)))
                    .on_submit(Message::Library(LibraryMsg::ScoreSubmitted(anime_id)))
                    .size(style::TEXT_SM)
                    .padding([style::SPACE_XS, style::SPACE_SM])
                    .width(Length::Fixed(80.0))
                    .style(theme::text_input_style(cs)),
            ]
            .spacing(style::SPACE_SM)
            .align_y(Alignment::Center),
        ]
        .spacing(style::SPACE_SM),
    )
    .style(theme::card(cs))
    .padding(style::SPACE_LG)
    .width(Length::Fill);

    // Progress section -- card.
    let ep_text = match anime.episodes {
        Some(total) => format!("Episode {} / {}", entry.watched_episodes, total),
        None => format!("Episode {}", entry.watched_episodes),
    };

    let progress_card = container(
        column![
            text(ep_text).size(style::TEXT_BASE),
            row![
                button(text("\u{2212}").size(style::TEXT_SM))
                    .on_press(Message::Library(LibraryMsg::EpisodeDecrement(anime_id)))
                    .style(theme::control_button(cs))
                    .padding([style::SPACE_XS, style::SPACE_LG]),
                button(text("+").size(style::TEXT_SM))
                    .on_press(Message::Library(LibraryMsg::EpisodeIncrement(anime_id)))
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

    // Assemble detail panel.
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
