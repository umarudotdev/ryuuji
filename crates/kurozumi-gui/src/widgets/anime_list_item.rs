use iced::widget::{button, column, container, row, rule, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};
use iced_aw::ContextMenu;

use kurozumi_core::models::WatchStatus;
use kurozumi_core::storage::LibraryRow;

use crate::cover_cache::CoverCache;
use crate::screen::ContextAction;
use crate::style;
use crate::theme::{self, ColorScheme};
use crate::widgets;

/// A single anime list item with cover thumbnail, metadata, and right-click context menu.
///
/// Generic over the caller's `Message` type via closure callbacks, so both
/// Library and Search screens can reuse the same rendering.
pub fn anime_list_item<'a, Message: Clone + 'static>(
    cs: &'a ColorScheme,
    lib_row: &'a LibraryRow,
    selected: Option<i64>,
    covers: &'a CoverCache,
    on_select: impl Fn(i64) -> Message + 'a + Clone,
    on_context: impl Fn(i64, ContextAction) -> Message + 'a + Clone,
) -> Element<'a, Message> {
    let anime = &lib_row.anime;
    let title = anime.title.preferred();
    let progress = match anime.episodes {
        Some(total) => format!("{} / {}", lib_row.entry.watched_episodes, total),
        None => format!("{}", lib_row.entry.watched_episodes),
    };

    let is_selected = selected == Some(anime.id);
    let anime_id = anime.id;
    let status_col = theme::status_color(cs, lib_row.entry.status);

    // Left status accent bar
    let status_bar = container(text("").size(1))
        .width(Length::Fixed(3.0))
        .height(Length::Fill)
        .style(theme::status_bar_accent(status_col));

    // Cover thumbnail
    let thumb = widgets::rounded_cover(
        cs,
        covers,
        anime_id,
        style::THUMB_WIDTH,
        style::THUMB_HEIGHT,
        style::RADIUS_SM,
    );

    // Title + metadata column
    let mut info_col = column![text(title)
        .size(style::TEXT_BASE)
        .font(style::FONT_HEADING)
        .line_height(style::LINE_HEIGHT_NORMAL)
        .wrapping(iced::widget::text::Wrapping::None)]
    .spacing(style::SPACE_XXS)
    .clip(true);

    // Meta line: media type · year · genres
    let mut meta_parts: Vec<String> = Vec::new();
    if let Some(mt) = &anime.media_type {
        meta_parts.push(crate::format::media_type(mt));
    }
    if let Some(year) = anime.year {
        meta_parts.push(year.to_string());
    }
    let genre_str: String = anime
        .genres
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if !genre_str.is_empty() {
        meta_parts.push(genre_str);
    }
    if !meta_parts.is_empty() {
        info_col = info_col.push(
            text(meta_parts.join("  \u{00B7}  "))
                .size(style::TEXT_XS)
                .color(cs.outline)
                .line_height(style::LINE_HEIGHT_LOOSE),
        );
    }

    // Right-side: status badge + episode progress
    let status_label = lib_row.entry.status.to_string();
    let right_col = column![
        container(
            text(status_label)
                .size(style::TEXT_XS)
                .color(status_col)
                .line_height(style::LINE_HEIGHT_NORMAL),
        )
        .style(theme::status_badge(cs, status_col))
        .padding([style::SPACE_XXS, style::SPACE_SM]),
        text(progress)
            .size(style::TEXT_XS)
            .color(cs.on_surface_variant)
            .line_height(style::LINE_HEIGHT_LOOSE),
    ]
    .spacing(style::SPACE_XXS)
    .align_x(Alignment::End);

    let content = row![status_bar, thumb, info_col.width(Length::Fill), right_col,]
        .spacing(style::SPACE_SM)
        .align_y(Alignment::Center);

    let on_select_clone = on_select.clone();
    let base = button(content)
        .width(Length::Fill)
        .padding([style::SPACE_XS, style::SPACE_MD])
        .on_press(on_select_clone(anime_id))
        .style(theme::list_item(is_selected, cs));

    // Context menu
    let primary = cs.primary;
    let on_primary = cs.on_primary;
    let on_surface = cs.on_surface;
    let error = cs.error;
    let on_error = cs.on_error;
    let menu_bg = cs.surface_container;
    let menu_border = cs.outline_variant;

    let on_ctx = on_context.clone();

    let menu_item = move |label: &'a str, msg: Message| -> Element<'a, Message> {
        button(
            text(label)
                .size(style::TEXT_SM)
                .line_height(style::LINE_HEIGHT_LOOSE),
        )
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

    ContextMenu::new(base, move || {
        let on_ctx = on_ctx.clone();
        container(
            column![
                menu_item(
                    "Watching",
                    on_ctx(anime_id, ContextAction::ChangeStatus(WatchStatus::Watching)),
                ),
                menu_item(
                    "Completed",
                    on_ctx(
                        anime_id,
                        ContextAction::ChangeStatus(WatchStatus::Completed)
                    ),
                ),
                menu_item(
                    "On Hold",
                    on_ctx(anime_id, ContextAction::ChangeStatus(WatchStatus::OnHold)),
                ),
                menu_item(
                    "Dropped",
                    on_ctx(anime_id, ContextAction::ChangeStatus(WatchStatus::Dropped)),
                ),
                menu_item(
                    "Plan to Watch",
                    on_ctx(
                        anime_id,
                        ContextAction::ChangeStatus(WatchStatus::PlanToWatch)
                    ),
                ),
                rule::horizontal(1),
                button(
                    text("Delete")
                        .size(style::TEXT_SM)
                        .line_height(style::LINE_HEIGHT_LOOSE),
                )
                .width(Length::Fill)
                .padding([style::SPACE_XS, style::SPACE_MD])
                .on_press(on_ctx(anime_id, ContextAction::Delete))
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
                color: Color {
                    a: 0.2,
                    ..Color::BLACK
                },
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            ..Default::default()
        })
        .padding(style::SPACE_XS)
        .into()
    })
    .style(theme::aw_context_menu_style(cs))
    .into()
}
