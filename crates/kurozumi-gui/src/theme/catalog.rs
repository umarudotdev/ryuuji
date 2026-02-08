//! Style functions parameterized by ColorScheme.
//!
//! Each function returns a closure suitable for Iced's `.style()` method,
//! capturing the needed color tokens from a `ColorScheme`.

use iced::widget::{button, container, text_input};
use iced::{Background, Border, Color, Shadow, Theme, Vector};
use iced_aw::style::{
    context_menu as aw_context_menu, number_input as aw_number_input, status::Status as AwStatus,
};

use crate::style;

use super::ColorScheme;

/// A card container: surface background, rounded corners, subtle border.
pub fn card(cs: &ColorScheme) -> impl Fn(&Theme) -> container::Style {
    let bg = cs.surface_container;
    let border_color = cs.outline_variant;
    move |_theme| container::Style {
        text_color: None,
        background: Some(Background::Color(bg)),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: style::RADIUS_LG.into(),
        },
        ..Default::default()
    }
}

/// Status bar container style.
pub fn status_bar(cs: &ColorScheme) -> impl Fn(&Theme) -> container::Style {
    let text = cs.on_surface_variant;
    let bg = cs.surface_container_lowest;
    move |_theme| container::Style {
        text_color: Some(text),
        background: Some(Background::Color(bg)),
        ..Default::default()
    }
}

/// Navigation rail background.
pub fn nav_rail_bg(cs: &ColorScheme) -> impl Fn(&Theme) -> container::Style {
    let bg = cs.surface_container_low;
    move |_theme| container::Style {
        text_color: None,
        background: Some(Background::Color(bg)),
        ..Default::default()
    }
}

/// Navigation rail item — icon+label with pill indicator when active.
pub fn nav_rail_item(
    active: bool,
    cs: &ColorScheme,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    let primary_container = cs.primary_container;
    let on_primary_container = cs.on_primary_container;
    let surface_bright = cs.surface_bright;
    let on_surface = cs.on_surface;
    let on_surface_variant = cs.on_surface_variant;

    move |_theme, status| {
        if active {
            button::Style {
                background: Some(Background::Color(primary_container)),
                text_color: on_primary_container,
                border: Border {
                    radius: style::RADIUS_XL.into(),
                    ..Border::default()
                },
                ..Default::default()
            }
        } else {
            match status {
                button::Status::Hovered => button::Style {
                    background: Some(Background::Color(surface_bright)),
                    text_color: on_surface,
                    border: Border {
                        radius: style::RADIUS_XL.into(),
                        ..Border::default()
                    },
                    ..Default::default()
                },
                _ => button::Style {
                    background: None,
                    text_color: on_surface_variant,
                    border: Border {
                        radius: style::RADIUS_XL.into(),
                        ..Border::default()
                    },
                    ..Default::default()
                },
            }
        }
    }
}

/// Filter chip — MD3 style: outlined when unselected, tonal fill when selected.
pub fn filter_chip(
    selected: bool,
    cs: &ColorScheme,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    let secondary_container = cs.secondary_container;
    let on_secondary_container = cs.on_secondary_container;
    let outline_variant = cs.outline_variant;
    let surface_bright = cs.surface_bright;
    let on_surface = cs.on_surface;
    let on_surface_variant = cs.on_surface_variant;

    move |_theme, status| {
        if selected {
            button::Style {
                background: Some(Background::Color(secondary_container)),
                text_color: on_secondary_container,
                border: Border {
                    radius: style::CHIP_RADIUS.into(),
                    ..Border::default()
                },
                ..Default::default()
            }
        } else {
            let (bg, tc) = match status {
                button::Status::Hovered => (Some(Background::Color(surface_bright)), on_surface),
                _ => (None, on_surface_variant),
            };
            button::Style {
                background: bg,
                text_color: tc,
                border: Border {
                    color: outline_variant,
                    width: 1.0,
                    radius: style::CHIP_RADIUS.into(),
                },
                ..Default::default()
            }
        }
    }
}

/// List item button — card-like with selection highlight.
pub fn list_item(
    selected: bool,
    cs: &ColorScheme,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    let surface_container_high = cs.surface_container_high;
    let surface_container = cs.surface_container;
    let outline_variant = cs.outline_variant;
    let primary = cs.primary;
    let on_surface = cs.on_surface;

    move |_theme, status| {
        let (bg, border_color) = if selected {
            (Some(Background::Color(surface_container_high)), primary)
        } else {
            match status {
                button::Status::Hovered => {
                    (Some(Background::Color(surface_container)), outline_variant)
                }
                _ => (None, Color::TRANSPARENT),
            }
        };

        button::Style {
            background: bg,
            text_color: on_surface,
            border: Border {
                color: border_color,
                width: if selected { 1.0 } else { 0.0 },
                radius: style::RADIUS_MD.into(),
            },
            ..Default::default()
        }
    }
}

/// Primary action button (Save, Confirm, etc.).
pub fn primary_button(cs: &ColorScheme) -> impl Fn(&Theme, button::Status) -> button::Style {
    let primary = cs.primary;
    let primary_hover = cs.primary_hover;
    let primary_dim = cs.primary_dim;
    let on_primary = cs.on_primary;

    move |_theme, status| {
        let bg = match status {
            button::Status::Hovered => primary_hover,
            button::Status::Pressed => primary_dim,
            _ => primary,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: on_primary,
            border: Border {
                radius: style::RADIUS_MD.into(),
                ..Border::default()
            },
            ..Default::default()
        }
    }
}

/// Danger action button (Delete confirmation, etc.).
pub fn danger_button(cs: &ColorScheme) -> impl Fn(&Theme, button::Status) -> button::Style {
    let error = cs.error;
    let error_hover = cs.error_hover;
    let error_pressed = cs.error_pressed;
    let on_error = cs.on_error;

    move |_theme, status| {
        let bg = match status {
            button::Status::Hovered => error_hover,
            button::Status::Pressed => error_pressed,
            _ => error,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: on_error,
            border: Border {
                radius: style::RADIUS_MD.into(),
                ..Border::default()
            },
            ..Default::default()
        }
    }
}

/// Ghost / outlined button — transparent bg, border outline.
pub fn ghost_button(cs: &ColorScheme) -> impl Fn(&Theme, button::Status) -> button::Style {
    let surface_bright = cs.surface_bright;
    let on_surface = cs.on_surface;
    let on_surface_variant = cs.on_surface_variant;
    let outline_variant = cs.outline_variant;

    move |_theme, status| {
        let (bg, text_color) = match status {
            button::Status::Hovered => (Some(Background::Color(surface_bright)), on_surface),
            _ => (None, on_surface_variant),
        };
        button::Style {
            background: bg,
            text_color,
            border: Border {
                color: outline_variant,
                width: 1.0,
                radius: style::RADIUS_MD.into(),
            },
            ..Default::default()
        }
    }
}

/// Custom text input styling that adapts to theme.
pub fn text_input_style(
    cs: &ColorScheme,
) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    let primary = cs.primary;
    let outline = cs.outline;
    let outline_variant = cs.outline_variant;
    let surface_container_low = cs.surface_container_low;
    let on_surface_variant = cs.on_surface_variant;
    let on_surface = cs.on_surface;

    move |_theme, status| {
        let border_color = match status {
            text_input::Status::Focused { .. } => primary,
            text_input::Status::Hovered => outline,
            _ => outline_variant,
        };
        text_input::Style {
            background: Background::Color(surface_container_low),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: style::RADIUS_SM.into(),
            },
            icon: on_surface_variant,
            placeholder: outline,
            value: on_surface,
            selection: primary,
        }
    }
}

/// Cover art placeholder container.
pub fn cover_placeholder(cs: &ColorScheme) -> impl Fn(&Theme) -> container::Style {
    let bg = cs.surface_container_high;
    let border_color = cs.outline_variant;
    move |_theme| container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: style::RADIUS_LG.into(),
        },
        ..Default::default()
    }
}

/// Dialog container — elevated card for modals.
pub fn dialog_container(cs: &ColorScheme) -> impl Fn(&Theme) -> container::Style {
    let bg = cs.surface_container_high;
    let border_color = cs.outline_variant;
    move |_theme| container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: style::RADIUS_XL.into(),
        },
        shadow: Shadow {
            color: Color {
                a: 0.3,
                ..Color::BLACK
            },
            offset: Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        },
        ..Default::default()
    }
}

/// Status color bar at top of grid cards.
pub fn status_bar_accent(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(color)),
        ..Default::default()
    }
}

/// Get the status color for a watch status.
pub fn status_color(cs: &ColorScheme, status: kurozumi_core::models::WatchStatus) -> Color {
    use kurozumi_core::models::WatchStatus;
    match status {
        WatchStatus::Watching => cs.status_watching,
        WatchStatus::Completed => cs.status_completed,
        WatchStatus::OnHold => cs.status_on_hold,
        WatchStatus::Dropped => cs.status_dropped,
        WatchStatus::PlanToWatch => cs.status_plan,
    }
}

// ── iced_aw widget styles ───────────────────────────────────────────

/// iced_aw ContextMenu style: transparent backdrop (menu container is styled separately).
pub fn aw_context_menu_style(
    _cs: &ColorScheme,
) -> impl Fn(&Theme, AwStatus) -> aw_context_menu::Style + 'static {
    move |_theme, _status| aw_context_menu::Style {
        background: Background::Color(Color::TRANSPARENT),
    }
}

/// iced_aw NumberInput button style.
pub fn aw_number_input_style(
    cs: &ColorScheme,
) -> impl Fn(&Theme, AwStatus) -> aw_number_input::Style + 'static {
    let primary = cs.primary;
    let on_primary = cs.on_primary;
    let surface_container_high = cs.surface_container_high;
    let on_surface = cs.on_surface;
    move |_theme, status| {
        let (bg, icon) = match status {
            AwStatus::Hovered => (Some(Background::Color(primary)), on_primary),
            AwStatus::Disabled => (
                Some(Background::Color(surface_container_high)),
                Color {
                    a: 0.5,
                    ..on_surface
                },
            ),
            _ => (Some(Background::Color(surface_container_high)), on_surface),
        };
        aw_number_input::Style {
            button_background: bg,
            icon_color: icon,
        }
    }
}
