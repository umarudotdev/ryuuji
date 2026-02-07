//! Style functions parameterized by ColorScheme.
//!
//! Each function returns a closure suitable for Iced's `.style()` method,
//! capturing the needed color tokens from a `ColorScheme`.

use iced::overlay::menu;
use iced::widget::{button, container, pick_list, progress_bar, scrollable, text_input, toggler};
use iced::{Background, Border, Color, Shadow, Theme, Vector};
use iced_aw::style::{context_menu as aw_context_menu, status::Status as AwStatus};

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

/// Transparent icon button — no border, subtle hover.
pub fn icon_button(cs: &ColorScheme) -> impl Fn(&Theme, button::Status) -> button::Style {
    let surface_bright = cs.surface_bright;

    move |_theme, status| {
        let bg = match status {
            button::Status::Hovered => Some(Background::Color(surface_bright)),
            _ => None,
        };
        button::Style {
            background: bg,
            text_color: Color::TRANSPARENT,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: style::RADIUS_FULL.into(),
            },
            ..Default::default()
        }
    }
}

/// Settings sidebar navigation item — shows active state with primary-tinted bg.
pub fn settings_nav_item(
    cs: &ColorScheme,
    is_active: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    let primary = cs.primary;
    let surface_container_low = cs.surface_container_low;
    let surface_bright = cs.surface_bright;

    move |_theme, status| {
        let bg = if is_active {
            Some(Background::Color(Color { a: 0.12, ..primary }))
        } else {
            match status {
                button::Status::Hovered => Some(Background::Color(surface_bright)),
                _ => Some(Background::Color(surface_container_low)),
            }
        };

        let border = if is_active {
            Border {
                color: primary,
                width: 0.0,
                radius: style::RADIUS_MD.into(),
            }
        } else {
            Border {
                radius: style::RADIUS_MD.into(),
                ..Border::default()
            }
        };

        button::Style {
            background: bg,
            text_color: Color::TRANSPARENT, // Text color set via child widget
            border,
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
                radius: style::RADIUS_MD.into(),
            },
            icon: on_surface_variant,
            placeholder: outline,
            value: on_surface,
            selection: primary,
        }
    }
}

/// Borderless text input for use inside a composite search bar container.
pub fn text_input_borderless(
    cs: &ColorScheme,
) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    let on_surface = cs.on_surface;
    let on_surface_variant = cs.on_surface_variant;
    let outline = cs.outline;
    let primary = cs.primary;

    move |_theme, _status| text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        icon: on_surface_variant,
        placeholder: outline,
        value: on_surface,
        selection: primary,
    }
}

/// Composite search bar container — pill-shaped with subtle border.
pub fn search_bar(cs: &ColorScheme) -> impl Fn(&Theme) -> container::Style {
    let bg = cs.surface_container_low;
    let border_color = cs.outline_variant;
    move |_theme| container::Style {
        text_color: None,
        background: Some(Background::Color(bg)),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: style::RADIUS_FULL.into(),
        },
        ..Default::default()
    }
}

/// MD3-style toggler: primary track when on, outline track when off.
pub fn toggler_style(cs: &ColorScheme) -> impl Fn(&Theme, toggler::Status) -> toggler::Style {
    let primary = cs.primary;
    let primary_hover = cs.primary_hover;
    let on_primary = cs.on_primary;
    let outline = cs.outline;
    let outline_variant = cs.outline_variant;
    let surface_bright = cs.surface_bright;
    let on_surface = cs.on_surface;

    move |_theme, status| match status {
        toggler::Status::Active { is_toggled } | toggler::Status::Disabled { is_toggled } => {
            let disabled = matches!(status, toggler::Status::Disabled { .. });
            let alpha = if disabled { 0.38 } else { 1.0 };
            let (track, knob) = if is_toggled {
                (primary, on_primary)
            } else {
                (outline_variant, outline)
            };
            toggler::Style {
                background: Background::Color(Color { a: alpha, ..track }),
                foreground: Background::Color(Color { a: alpha, ..knob }),
                background_border_width: 1.0,
                background_border_color: Color {
                    a: alpha,
                    ..outline_variant
                },
                foreground_border_width: 0.0,
                foreground_border_color: Color::TRANSPARENT,
                text_color: Some(on_surface),
                border_radius: None,
                padding_ratio: 0.25,
            }
        }
        toggler::Status::Hovered { is_toggled } => {
            let (track, knob) = if is_toggled {
                (primary_hover, on_primary)
            } else {
                (surface_bright, on_surface)
            };
            toggler::Style {
                background: Background::Color(track),
                foreground: Background::Color(knob),
                background_border_width: 1.0,
                background_border_color: outline_variant,
                foreground_border_width: 0.0,
                foreground_border_color: Color::TRANSPARENT,
                text_color: Some(on_surface),
                border_radius: None,
                padding_ratio: 0.25,
            }
        }
    }
}

/// Cover art placeholder container.
pub fn cover_placeholder(cs: &ColorScheme, radius: f32) -> impl Fn(&Theme) -> container::Style {
    let bg = cs.surface_container_high;
    let border_color = cs.outline_variant;
    move |_theme| container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: radius.into(),
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

/// Anime card container: card background with a thin left accent border for status color.
pub fn anime_card_style(
    cs: &ColorScheme,
    status_color: Color,
) -> impl Fn(&Theme) -> container::Style {
    let bg = cs.surface_container;
    let border_color = cs.outline_variant;
    move |_theme| container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: style::RADIUS_MD.into(),
        },
        shadow: Shadow {
            color: Color {
                a: 0.08,
                ..status_color
            },
            offset: Vector::new(0.0, 2.0),
            blur_radius: 6.0,
        },
        ..Default::default()
    }
}

/// Anime card button: transparent with hover elevation effect.
pub fn anime_card_button(cs: &ColorScheme) -> impl Fn(&Theme, button::Status) -> button::Style {
    let surface_container_high = cs.surface_container_high;
    move |_theme, status| {
        let bg = match status {
            button::Status::Hovered => Some(Background::Color(Color {
                a: 0.08,
                ..surface_container_high
            })),
            _ => None,
        };
        button::Style {
            background: bg,
            text_color: Color::TRANSPARENT,
            border: Border {
                radius: style::RADIUS_MD.into(),
                ..Border::default()
            },
            ..Default::default()
        }
    }
}

/// Status color bar at top of grid cards.
pub fn status_bar_accent(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(color)),
        border: Border {
            radius: style::RADIUS_FULL.into(),
            ..Border::default()
        },
        ..Default::default()
    }
}

/// Subtle outlined badge for watch status labels.
pub fn status_badge(cs: &ColorScheme, status_color: Color) -> impl Fn(&Theme) -> container::Style {
    let bg = Color {
        a: 0.1,
        ..status_color
    };
    let border_color = Color {
        a: 0.3,
        ..status_color
    };
    let _ = cs;
    move |_theme| container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: style::RADIUS_SM.into(),
        },
        ..Default::default()
    }
}

/// Get the status color for a watch status.
pub fn status_color(cs: &ColorScheme, status: ryuuji_core::models::WatchStatus) -> Color {
    use ryuuji_core::models::WatchStatus;
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

// ── New styled components ─────────────────────────────────────────

/// Progress bar track (surface_container_high) with primary-colored fill.
pub fn episode_progress(cs: &ColorScheme) -> impl Fn(&Theme) -> progress_bar::Style {
    let primary = cs.primary;
    let track = cs.surface_container_high;
    move |_theme| progress_bar::Style {
        background: Background::Color(track),
        bar: Background::Color(primary),
        border: Border {
            radius: style::RADIUS_FULL.into(),
            ..Border::default()
        },
    }
}

/// Metadata badge (genre/studio pill): tonal surface with outline border.
pub fn metadata_badge(cs: &ColorScheme) -> impl Fn(&Theme) -> container::Style {
    let bg = cs.surface_container_high;
    let border_color = cs.outline_variant;
    move |_theme| container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: style::RADIUS_FULL.into(),
        },
        ..Default::default()
    }
}

/// Pick list trigger: themed surface background with outline border.
pub fn pick_list_style(cs: &ColorScheme) -> impl Fn(&Theme, pick_list::Status) -> pick_list::Style {
    let primary = cs.primary;
    let outline = cs.outline;
    let outline_variant = cs.outline_variant;
    let surface_container_low = cs.surface_container_low;
    let on_surface = cs.on_surface;
    let on_surface_variant = cs.on_surface_variant;

    move |_theme, status| {
        let (border_color, handle_color) = match status {
            pick_list::Status::Opened { .. } => (primary, primary),
            pick_list::Status::Hovered => (outline, on_surface),
            _ => (outline_variant, on_surface_variant),
        };
        pick_list::Style {
            text_color: on_surface,
            placeholder_color: on_surface_variant,
            handle_color,
            background: Background::Color(surface_container_low),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: style::RADIUS_MD.into(),
            },
        }
    }
}

/// Pick list dropdown menu: themed background with primary selection highlight.
pub fn pick_list_menu_style(cs: &ColorScheme) -> impl Fn(&Theme) -> menu::Style {
    let surface_container = cs.surface_container;
    let outline_variant = cs.outline_variant;
    let on_surface = cs.on_surface;
    let primary = cs.primary;
    let on_primary = cs.on_primary;

    move |_theme| menu::Style {
        background: Background::Color(surface_container),
        border: Border {
            color: outline_variant,
            width: 1.0,
            radius: style::RADIUS_MD.into(),
        },
        text_color: on_surface,
        selected_text_color: on_primary,
        selected_background: Background::Color(primary),
        shadow: Shadow {
            color: Color {
                a: 0.2,
                ..Color::BLACK
            },
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
    }
}

/// Stepper +/- button style with per-corner radius for pill-group layout.
/// Stepper +/- button — rounded, subtle background, clear hover/press states.
pub fn stepper_button(cs: &ColorScheme) -> impl Fn(&Theme, button::Status) -> button::Style {
    let surface_container_high = cs.surface_container_high;
    let primary_container = cs.primary_container;
    let on_surface = cs.on_surface;
    let outline_variant = cs.outline_variant;

    move |_theme, status| {
        let bg = match status {
            button::Status::Pressed => primary_container,
            button::Status::Hovered => surface_container_high,
            _ => Color::TRANSPARENT,
        };
        let opacity = match status {
            button::Status::Disabled => 0.38,
            _ => 1.0,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: Color {
                a: opacity,
                ..on_surface
            },
            border: Border {
                color: outline_variant,
                width: 1.0,
                radius: style::RADIUS_MD.into(),
            },
            ..Default::default()
        }
    }
}

/// Tooltip container — surface container with subtle border.
pub fn tooltip_style(cs: &ColorScheme) -> impl Fn(&Theme) -> container::Style {
    let surface_container = cs.surface_container;
    let outline_variant = cs.outline_variant;

    move |_theme| container::Style {
        background: Some(Background::Color(surface_container)),
        border: Border {
            color: outline_variant,
            width: 1.0,
            radius: style::RADIUS_SM.into(),
        },
        text_color: None,
        ..Default::default()
    }
}

/// Fluent Design overlay scrollbar: thin transparent rail, pill scroller
/// that becomes more visible on hover/drag.
pub fn overlay_scrollbar(
    cs: &ColorScheme,
) -> impl Fn(&Theme, scrollable::Status) -> scrollable::Style {
    let on_surface = cs.on_surface;
    let primary = cs.primary;

    move |_theme, status| {
        let (scroller_color, scroller_alpha) = match status {
            scrollable::Status::Dragged { .. } => (primary, 0.7),
            scrollable::Status::Hovered {
                is_vertical_scrollbar_hovered: true,
                ..
            } => (on_surface, 0.5),
            scrollable::Status::Hovered { .. } => (on_surface, 0.25),
            _ => (on_surface, 0.15),
        };

        let rail = scrollable::Rail {
            background: None,
            border: Border::default(),
            scroller: scrollable::Scroller {
                background: Background::Color(Color {
                    a: scroller_alpha,
                    ..scroller_color
                }),
                border: Border {
                    radius: style::RADIUS_FULL.into(),
                    ..Border::default()
                },
            },
        };

        scrollable::Style {
            container: container::Style::default(),
            vertical_rail: rail,
            horizontal_rail: rail,
            gap: None,
            auto_scroll: scrollable::AutoScroll {
                background: Background::Color(Color::TRANSPARENT),
                border: Border::default(),
                shadow: Shadow::default(),
                icon: on_surface,
            },
        }
    }
}
