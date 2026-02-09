//! Semantic color tokens for the application.
//!
//! Mirrors MD3's tonal surface hierarchy plus custom status colors.
//! Now serde-deserializable from TOML theme files via hex color strings.

use iced::Color;
use serde::Deserialize;

// ── Hex color serde ─────────────────────────────────────────────────

#[allow(dead_code)]
mod hex_color {
    use iced::Color;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let r = (color.r * 255.0) as u8;
        let g = (color.g * 255.0) as u8;
        let b = (color.b * 255.0) as u8;
        if (color.a - 1.0).abs() < f32::EPSILON {
            serializer.serialize_str(&format!("#{r:02X}{g:02X}{b:02X}"))
        } else {
            let a = (color.a * 255.0) as u8;
            serializer.serialize_str(&format!("#{r:02X}{g:02X}{b:02X}{a:02X}"))
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        parse_hex_color(&s).map_err(serde::de::Error::custom)
    }

    fn parse_hex_color(s: &str) -> Result<Color, String> {
        let hex = s.strip_prefix('#').unwrap_or(s);
        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|e| e.to_string())?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|e| e.to_string())?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|e| e.to_string())?;
                Ok(Color::from_rgb8(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|e| e.to_string())?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|e| e.to_string())?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|e| e.to_string())?;
                let a = u8::from_str_radix(&hex[6..8], 16).map_err(|e| e.to_string())?;
                Ok(Color::from_rgba8(r, g, b, a as f32 / 255.0))
            }
            _ => Err(format!("invalid hex color: {s}")),
        }
    }
}

// ── TOML intermediate structs ──────────────────────────────────────

/// Raw TOML theme file structure.
///
/// Each file defines one theme with both dark and light variants.
#[derive(Debug, Deserialize)]
pub struct ThemeFile {
    pub meta: ThemeMeta,
    pub dark: ThemeVariant,
    pub light: ThemeVariant,
}

#[derive(Debug, Deserialize)]
pub struct ThemeMeta {
    pub name: String,
}

/// A single appearance variant (dark or light) within a theme.
#[derive(Debug, Deserialize)]
pub struct ThemeVariant {
    pub surface: SurfaceColors,
    pub text: TextColors,
    pub primary: PrimaryColors,
    pub secondary: SecondaryColors,
    pub tertiary: TertiaryColors,
    pub error: ErrorColors,
    pub status: StatusColors,
    pub inverse: InverseColors,
}

#[derive(Debug, Deserialize)]
pub struct SurfaceColors {
    #[serde(with = "hex_color")]
    pub container_lowest: Color,
    #[serde(with = "hex_color")]
    pub base: Color,
    #[serde(with = "hex_color")]
    pub container_low: Color,
    #[serde(with = "hex_color")]
    pub container: Color,
    #[serde(with = "hex_color")]
    pub container_high: Color,
    #[serde(with = "hex_color")]
    pub container_highest: Color,
    #[serde(with = "hex_color")]
    pub bright: Color,
}

#[derive(Debug, Deserialize)]
pub struct TextColors {
    #[serde(with = "hex_color")]
    pub on_surface: Color,
    #[serde(with = "hex_color")]
    pub on_surface_variant: Color,
    #[serde(with = "hex_color")]
    pub outline: Color,
    #[serde(with = "hex_color")]
    pub outline_variant: Color,
}

#[derive(Debug, Deserialize)]
pub struct PrimaryColors {
    #[serde(with = "hex_color")]
    pub base: Color,
    #[serde(with = "hex_color")]
    pub hover: Color,
    #[serde(with = "hex_color")]
    pub dim: Color,
    #[serde(with = "hex_color")]
    pub on_primary: Color,
    #[serde(with = "hex_color")]
    pub container: Color,
    #[serde(with = "hex_color")]
    pub on_container: Color,
}

#[derive(Debug, Deserialize)]
pub struct SecondaryColors {
    #[serde(with = "hex_color")]
    pub container: Color,
    #[serde(with = "hex_color")]
    pub on_container: Color,
}

#[derive(Debug, Deserialize)]
pub struct TertiaryColors {
    #[serde(with = "hex_color")]
    pub base: Color,
    #[serde(with = "hex_color")]
    pub on_tertiary: Color,
}

#[derive(Debug, Deserialize)]
pub struct ErrorColors {
    #[serde(with = "hex_color")]
    pub base: Color,
    #[serde(with = "hex_color")]
    pub hover: Color,
    #[serde(with = "hex_color")]
    pub pressed: Color,
    #[serde(with = "hex_color")]
    pub on_error: Color,
}

#[derive(Debug, Deserialize)]
pub struct StatusColors {
    #[serde(with = "hex_color")]
    pub watching: Color,
    #[serde(with = "hex_color")]
    pub completed: Color,
    #[serde(with = "hex_color")]
    pub on_hold: Color,
    #[serde(with = "hex_color")]
    pub dropped: Color,
    #[serde(with = "hex_color")]
    pub plan: Color,
}

#[derive(Debug, Deserialize)]
pub struct InverseColors {
    #[serde(with = "hex_color")]
    pub surface: Color,
    #[serde(with = "hex_color")]
    pub on_surface: Color,
    #[serde(with = "hex_color")]
    pub scrim: Color,
    #[serde(with = "hex_color")]
    pub modal_backdrop: Color,
}

// Re-export ThemeMode from core so there's a single source of truth.
pub use ryuuji_core::config::ThemeMode;

// ── ColorScheme ────────────────────────────────────────────────────

/// All semantic color tokens for the application.
///
/// Mirrors MD3's tonal surface hierarchy plus custom status colors.
/// Can be constructed from a `ThemeFile` (TOML) or from the built-in
/// `dark()` / `light()` constructors.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ColorScheme {
    // Surfaces (7 levels, low -> high elevation)
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
    /// Build a ColorScheme from a single theme variant.
    pub fn from_variant(v: &ThemeVariant) -> Self {
        Self {
            surface_container_lowest: v.surface.container_lowest,
            surface: v.surface.base,
            surface_container_low: v.surface.container_low,
            surface_container: v.surface.container,
            surface_container_high: v.surface.container_high,
            surface_container_highest: v.surface.container_highest,
            surface_bright: v.surface.bright,

            on_surface: v.text.on_surface,
            on_surface_variant: v.text.on_surface_variant,
            outline: v.text.outline,
            outline_variant: v.text.outline_variant,

            primary: v.primary.base,
            primary_hover: v.primary.hover,
            primary_dim: v.primary.dim,
            on_primary: v.primary.on_primary,
            primary_container: v.primary.container,
            on_primary_container: v.primary.on_container,

            secondary_container: v.secondary.container,
            on_secondary_container: v.secondary.on_container,

            tertiary: v.tertiary.base,
            on_tertiary: v.tertiary.on_tertiary,

            error: v.error.base,
            error_hover: v.error.hover,
            error_pressed: v.error.pressed,
            on_error: v.error.on_error,

            status_watching: v.status.watching,
            status_completed: v.status.completed,
            status_on_hold: v.status.on_hold,
            status_dropped: v.status.dropped,
            status_plan: v.status.plan,

            inverse_surface: v.inverse.surface,
            inverse_on_surface: v.inverse.on_surface,
            scrim: v.inverse.scrim,
            modal_backdrop: v.inverse.modal_backdrop,
        }
    }
}
