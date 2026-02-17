//! Material Design 3 theme — warm pink accent with tonal surfaces.
//!
//! Each theme is a single TOML file containing both dark and light variants.
//! Supports embedded defaults and user-provided themes from `~/.config/ryuuji/themes/`.

mod catalog;
mod colors;

// Re-export everything so `crate::theme::*` paths remain unchanged.
pub use catalog::*;
pub use colors::*;

use iced::Theme;

/// Embedded default theme TOML source (contains both dark and light).
pub(crate) const DEFAULT_THEME_TOML: &str = include_str!("../assets/themes/default.toml");

/// Embedded Tokyo Night theme.
pub(crate) const TOKYO_NIGHT_THEME_TOML: &str = include_str!("../assets/themes/tokyo-night.toml");

/// Embedded MyAnimeList theme (accurate dark mode colors).
pub(crate) const MAL_THEME_TOML: &str = include_str!("../assets/themes/myanimelist.toml");

/// Embedded MyAnimeList Blue theme (navy-tinted variant).
pub(crate) const MAL_BLUE_THEME_TOML: &str = include_str!("../assets/themes/myanimelist-blue.toml");

/// Embedded Rosé Pine theme.
pub(crate) const ROSE_PINE_THEME_TOML: &str = include_str!("../assets/themes/rose-pine.toml");

/// Embedded Osaka Jade theme.
pub(crate) const OSAKA_JADE_THEME_TOML: &str = include_str!("../assets/themes/osaka-jade.toml");

/// Embedded Monokai Ristretto theme.
pub(crate) const MONOKAI_RISTRETTO_THEME_TOML: &str =
    include_str!("../assets/themes/monokai-ristretto.toml");

/// A fully loaded theme with both appearance variants.
#[derive(Debug, Clone)]
pub struct RyuujiTheme {
    pub name: String,
    pub dark: ColorScheme,
    pub light: ColorScheme,
}

impl RyuujiTheme {
    /// Load a theme from a TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self, String> {
        let file: ThemeFile =
            toml::from_str(toml_str).map_err(|e| format!("theme parse error: {e}"))?;
        Ok(Self {
            name: file.meta.name.clone(),
            dark: ColorScheme::from_variant(&file.dark),
            light: ColorScheme::from_variant(&file.light),
        })
    }

    /// Load the embedded default theme.
    pub fn default_theme() -> Self {
        Self::from_toml(DEFAULT_THEME_TOML).expect("embedded default theme is valid TOML")
    }

    /// Get the color scheme for a resolved mode (Dark or Light).
    pub fn colors(&self, mode: ThemeMode) -> &ColorScheme {
        match mode {
            ThemeMode::Light => &self.light,
            // Dark is the fallback for both Dark and System.
            _ => &self.dark,
        }
    }

    /// Build the iced Theme for a given mode.
    pub fn iced_theme(&self, mode: ThemeMode) -> Theme {
        build_theme(self.colors(mode))
    }
}

/// Resolve `ThemeMode::System` to a concrete Dark or Light.
pub fn resolve_mode(mode: ThemeMode) -> ThemeMode {
    match mode {
        ThemeMode::System => match dark_light::detect() {
            Ok(dark_light::Mode::Light) => ThemeMode::Light,
            _ => ThemeMode::Dark,
        },
        other => other,
    }
}

/// Discover all available themes: embedded default + user themes from disk.
pub fn available_themes() -> Vec<RyuujiTheme> {
    let mut themes = vec![
        RyuujiTheme::default_theme(),
        RyuujiTheme::from_toml(TOKYO_NIGHT_THEME_TOML)
            .expect("embedded Tokyo Night theme is valid TOML"),
        RyuujiTheme::from_toml(MAL_THEME_TOML).expect("embedded MyAnimeList theme is valid TOML"),
        RyuujiTheme::from_toml(MAL_BLUE_THEME_TOML)
            .expect("embedded MyAnimeList Blue theme is valid TOML"),
        RyuujiTheme::from_toml(ROSE_PINE_THEME_TOML)
            .expect("embedded Rosé Pine theme is valid TOML"),
        RyuujiTheme::from_toml(OSAKA_JADE_THEME_TOML)
            .expect("embedded Osaka Jade theme is valid TOML"),
        RyuujiTheme::from_toml(MONOKAI_RISTRETTO_THEME_TOML)
            .expect("embedded Monokai Ristretto theme is valid TOML"),
    ];

    // Scan user themes directory.
    if let Some(user_themes) = user_themes_dir() {
        if let Ok(entries) = std::fs::read_dir(&user_themes) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "toml") {
                    match std::fs::read_to_string(&path) {
                        Ok(content) => match RyuujiTheme::from_toml(&content) {
                            Ok(theme) => themes.push(theme),
                            Err(e) => {
                                tracing::warn!("Skipping theme {}: {e}", path.display());
                            }
                        },
                        Err(e) => {
                            tracing::warn!("Cannot read {}: {e}", path.display());
                        }
                    }
                }
            }
        }
    }

    themes
}

/// Find a theme by name from the available themes.
pub fn find_theme(name: &str) -> Option<RyuujiTheme> {
    available_themes().into_iter().find(|t| t.name == name)
}

/// Path to the user themes directory.
fn user_themes_dir() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("", "", "ryuuji").map(|dirs| dirs.config_dir().join("themes"))
}

/// Build the iced Theme from a ColorScheme.
pub fn build_theme(cs: &ColorScheme) -> Theme {
    use iced::theme::Palette;

    Theme::custom(
        "Ryuuji",
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
