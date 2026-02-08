//! Material Design 3 theme — warm pink accent with tonal surfaces.
//!
//! Built on the MD3 tonal surface system: layered warm-tinted neutrals
//! create depth, while a pink primary accent drives interactivity.
//! Supports both dark and light themes via `ColorScheme`.

use iced::widget::{button, container, text_input};
use iced::{color, border::Radius, Background, Border, Color, Shadow, Theme, Vector};

use crate::style;

// ── Theme mode ──────────────────────────────────────────────────────

/// Light or dark theme selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

impl ThemeMode {
    pub const ALL: &[ThemeMode] = &[Self::Dark, Self::Light];
}

impl std::fmt::Display for ThemeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dark => write!(f, "Dark"),
            Self::Light => write!(f, "Light"),
        }
    }
}

// ── Color scheme ────────────────────────────────────────────────────

/// All semantic color tokens for the application.
///
/// Mirrors MD3's tonal surface hierarchy plus custom status colors.
/// Construct via `ColorScheme::dark()` or `ColorScheme::light()`.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ColorScheme {
    // Surfaces (7 levels, low → high elevation)
    pub surface_container_lowest: Color,
    pub surface: Color,
    pub surface_container_low: Color,
    pub surface_container: Color,
    pub surface_container_high: Color,
    pub surface_container_highest: Color,
    pub surface_bright: Color,

    // Text hierarchy
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub outline_variant: Color,

    // Primary accent (warm pink)
    pub primary: Color,
    pub primary_hover: Color,
    pub primary_dim: Color,
    pub on_primary: Color,
    pub primary_container: Color,
    pub on_primary_container: Color,

    // Secondary
    pub secondary_container: Color,
    pub on_secondary_container: Color,

    // Tertiary (warm gold)
    pub tertiary: Color,
    pub on_tertiary: Color,

    // Error
    pub error: Color,
    pub error_hover: Color,
    pub error_pressed: Color,
    pub on_error: Color,

    // Status colors (watch status)
    pub status_watching: Color,
    pub status_completed: Color,
    pub status_on_hold: Color,
    pub status_dropped: Color,
    pub status_plan: Color,

    // Inverse / scrim
    pub inverse_surface: Color,
    pub inverse_on_surface: Color,
    pub scrim: Color,
    pub modal_backdrop: Color,
}

impl ColorScheme {
    /// Dark theme — warm-tinted neutrals with pink accent.
    pub fn dark() -> Self {
        Self {
            // Surfaces
            surface_container_lowest: color!(0x140D10),
            surface: color!(0x1A1114),
            surface_container_low: color!(0x221B1E),
            surface_container: color!(0x262025),
            surface_container_high: color!(0x312A2F),
            surface_container_highest: color!(0x3C343A),
            surface_bright: color!(0x413437),

            // Text
            on_surface: color!(0xEFE0E3),
            on_surface_variant: color!(0xD6C2C6),
            outline: color!(0x9E8E92),
            outline_variant: color!(0x51454A),

            // Primary
            primary: color!(0xFFB1C1),
            primary_hover: color!(0xFFC4D0),
            primary_dim: color!(0xE89AAB),
            on_primary: color!(0x5E1127),
            primary_container: color!(0x7A2E3E),
            on_primary_container: color!(0xFFD9E0),

            // Secondary
            secondary_container: color!(0x524347),
            on_secondary_container: color!(0xD6C2C6),

            // Tertiary
            tertiary: color!(0xE8C68F),
            on_tertiary: color!(0x3F2E04),

            // Error
            error: color!(0xFFB4AB),
            error_hover: color!(0xCC3030),
            error_pressed: color!(0xAA2020),
            on_error: Color::WHITE,

            // Status
            status_watching: color!(0xFFB1C1), // primary pink
            status_completed: color!(0x4AC78B),
            status_on_hold: color!(0xE8C68F), // tertiary gold
            status_dropped: color!(0xFFB4AB), // error
            status_plan: color!(0xD6C2C6),    // on_surface_variant

            // Inverse / scrim
            inverse_surface: color!(0xEFE0E3),
            inverse_on_surface: color!(0x1A1114),
            scrim: Color::BLACK,
            modal_backdrop: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.75,
            },
        }
    }

    /// Light theme — warm whites with deeper pink accent.
    pub fn light() -> Self {
        Self {
            // Surfaces (warm whites)
            surface_container_lowest: color!(0xFFFFFF),
            surface: color!(0xFFF8F8),
            surface_container_low: color!(0xFFF0F1),
            surface_container: color!(0xFBE8EA),
            surface_container_high: color!(0xF5E2E5),
            surface_container_highest: color!(0xEFDCDF),
            surface_bright: color!(0xE8D5D8),

            // Text (dark on light)
            on_surface: color!(0x211A1C),
            on_surface_variant: color!(0x534348),
            outline: color!(0x857076),
            outline_variant: color!(0xD6C2C6),

            // Primary (deeper pink for contrast on light bg)
            primary: color!(0x8E4456),
            primary_hover: color!(0x7A3A4A),
            primary_dim: color!(0xA25568),
            on_primary: Color::WHITE,
            primary_container: color!(0xFFD9E0),
            on_primary_container: color!(0x3B0716),

            // Secondary
            secondary_container: color!(0xF5DEE1),
            on_secondary_container: color!(0x2B1D21),

            // Tertiary
            tertiary: color!(0x7D5800),
            on_tertiary: Color::WHITE,

            // Error
            error: color!(0xBA1A1A),
            error_hover: color!(0x9C1414),
            error_pressed: color!(0x7E0E0E),
            on_error: Color::WHITE,

            // Status
            status_watching: color!(0x8E4456),
            status_completed: color!(0x1B6E42),
            status_on_hold: color!(0x7D5800),
            status_dropped: color!(0xBA1A1A),
            status_plan: color!(0x534348),

            // Inverse / scrim
            inverse_surface: color!(0x362E31),
            inverse_on_surface: color!(0xFBEEF0),
            scrim: Color::BLACK,
            modal_backdrop: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.55,
            },
        }
    }

    /// Get the color scheme for a given theme mode.
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self::dark(),
            ThemeMode::Light => Self::light(),
        }
    }
}

// ── Theme constructor ───────────────────────────────────────────────

/// Build the iced Theme from a ColorScheme.
pub fn build_theme(cs: &ColorScheme) -> Theme {
    use iced::theme::Palette;

    Theme::custom(
        "Kurozumi",
        Palette {
            background: cs.surface,
            text: cs.on_surface,
            primary: cs.primary,
            success: cs.status_completed,
            warning: cs.tertiary,
            danger: cs.error,
        },
    )
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

// ── Style functions (parameterized by ColorScheme) ──────────────────

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
                button::Status::Hovered => (
                    Some(Background::Color(surface_bright)),
                    on_surface,
                ),
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
            (
                Some(Background::Color(surface_container_high)),
                primary,
            )
        } else {
            match status {
                button::Status::Hovered => (
                    Some(Background::Color(surface_container)),
                    outline_variant,
                ),
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

/// Grid card button — elevated card with selection border.
pub fn grid_card(
    selected: bool,
    cs: &ColorScheme,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    let surface_container = cs.surface_container;
    let surface_container_high = cs.surface_container_high;
    let outline_variant = cs.outline_variant;
    let primary = cs.primary;
    let on_surface = cs.on_surface;

    move |_theme, status| {
        let (bg, border_color, shadow) = if selected {
            (
                surface_container_high,
                primary,
                Shadow {
                    color: Color { a: 0.2, ..Color::BLACK },
                    offset: Vector::new(0.0, 2.0),
                    blur_radius: 8.0,
                },
            )
        } else {
            match status {
                button::Status::Hovered => (
                    surface_container_high,
                    outline_variant,
                    Shadow {
                        color: Color { a: 0.15, ..Color::BLACK },
                        offset: Vector::new(0.0, 2.0),
                        blur_radius: 6.0,
                    },
                ),
                _ => (
                    surface_container,
                    outline_variant,
                    Shadow {
                        color: Color { a: 0.1, ..Color::BLACK },
                        offset: Vector::new(0.0, 1.0),
                        blur_radius: 4.0,
                    },
                ),
            }
        };

        button::Style {
            background: Some(Background::Color(bg)),
            text_color: on_surface,
            border: Border {
                color: border_color,
                width: if selected { 2.0 } else { 1.0 },
                radius: style::RADIUS_LG.into(),
            },
            shadow,
            ..Default::default()
        }
    }
}

/// Score / episode small control button (filled tonal).
pub fn control_button(cs: &ColorScheme) -> impl Fn(&Theme, button::Status) -> button::Style {
    let primary = cs.primary;
    let primary_dim = cs.primary_dim;
    let on_primary = cs.on_primary;
    let surface_container_high = cs.surface_container_high;
    let on_surface = cs.on_surface;

    move |_theme, status| {
        let (bg, text_color) = match status {
            button::Status::Hovered => (
                Some(Background::Color(primary)),
                on_primary,
            ),
            button::Status::Pressed => (
                Some(Background::Color(primary_dim)),
                on_primary,
            ),
            _ => (
                Some(Background::Color(surface_container_high)),
                on_surface,
            ),
        };
        button::Style {
            background: bg,
            text_color,
            border: Border {
                radius: style::RADIUS_SM.into(),
                ..Border::default()
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
            button::Status::Hovered => (
                Some(Background::Color(surface_bright)),
                on_surface,
            ),
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
pub fn text_input_style(cs: &ColorScheme) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
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
            color: Color { a: 0.3, ..Color::BLACK },
            offset: Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        },
        ..Default::default()
    }
}

/// Grid cover placeholder — slightly different proportions for grid cards.
pub fn grid_cover_placeholder(cs: &ColorScheme) -> impl Fn(&Theme) -> container::Style {
    let bg = cs.surface_container_high;
    let outline_variant = cs.outline_variant;
    move |_theme| container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: outline_variant,
            width: 0.0,
            radius: Radius {
                top_left: style::RADIUS_LG,
                top_right: style::RADIUS_LG,
                bottom_right: 0.0,
                bottom_left: 0.0,
            },
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

