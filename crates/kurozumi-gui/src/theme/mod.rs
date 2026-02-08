//! Material Design 3 theme — warm pink accent with tonal surfaces.
//!
//! Supports both embedded default themes and user-provided TOML theme files
//! discovered from `~/.config/kurozumi/themes/`.

mod catalog;
mod colors;

// Re-export everything so `crate::theme::*` paths remain unchanged.
pub use catalog::*;
pub use colors::*;

use iced::Theme;

/// Embedded default theme TOML sources.
pub(crate) const DEFAULT_DARK_TOML: &str =
    include_str!("../../assets/themes/default-dark.toml");
pub(crate) const DEFAULT_LIGHT_TOML: &str =
    include_str!("../../assets/themes/default-light.toml");

/// A fully loaded theme with metadata and computed colors.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct KurozumiTheme {
    pub name: String,
    pub kind: ThemeMode,
    pub colors: ColorScheme,
}

impl KurozumiTheme {
    /// Load a theme from a TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self, String> {
        let file: ThemeFile =
            toml::from_str(toml_str).map_err(|e| format!("theme parse error: {e}"))?;
        let kind = match file.meta.kind.as_str() {
            "dark" => ThemeMode::Dark,
            "light" => ThemeMode::Light,
            other => return Err(format!("unknown theme kind: {other}")),
        };
        let colors = ColorScheme::from_theme_file(&file);
        Ok(Self {
            name: file.meta.name.clone(),
            kind,
            colors,
        })
    }

    /// Load the embedded default dark theme.
    pub fn default_dark() -> Self {
        Self::from_toml(DEFAULT_DARK_TOML).expect("embedded dark theme is valid TOML")
    }

    /// Load the embedded default light theme.
    pub fn default_light() -> Self {
        Self::from_toml(DEFAULT_LIGHT_TOML).expect("embedded light theme is valid TOML")
    }

    /// Get the default theme for a given mode.
    ///
    /// For `System`, detects the OS preference at call time.
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self::default_dark(),
            ThemeMode::Light => Self::default_light(),
            ThemeMode::System => {
                match dark_light::detect() {
                    Ok(dark_light::Mode::Light) => Self::default_light(),
                    _ => Self::default_dark(),
                }
            }
        }
    }

    /// Build the iced Theme from this theme's colors.
    pub fn iced_theme(&self) -> Theme {
        build_theme(&self.colors)
    }
}

/// Discover all available themes: embedded defaults + user themes from disk.
pub fn available_themes() -> Vec<KurozumiTheme> {
    let mut themes = vec![
        KurozumiTheme::default_dark(),
        KurozumiTheme::default_light(),
    ];

    // Scan user themes directory.
    if let Some(user_themes) = user_themes_dir() {
        if let Ok(entries) = std::fs::read_dir(&user_themes) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "toml") {
                    match std::fs::read_to_string(&path) {
                        Ok(content) => match KurozumiTheme::from_toml(&content) {
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
pub fn find_theme(name: &str) -> Option<KurozumiTheme> {
    available_themes().into_iter().find(|t| t.name == name)
}

/// Path to the user themes directory.
fn user_themes_dir() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("", "", "kurozumi")
        .map(|dirs| dirs.config_dir().join("themes"))
}

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
