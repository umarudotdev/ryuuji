use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::error::KurozumiError;

const DEFAULT_CONFIG: &str = include_str!("../../../config/default.toml");

/// Top-level application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub library: LibraryConfig,
    pub services: ServicesConfig,
    pub discord: DiscordConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub detection_interval: u64,
    pub close_to_tray: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryConfig {
    pub auto_update: bool,
    pub confirm_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesConfig {
    pub primary: String,
    pub anilist: ServiceToggle,
    pub kitsu: ServiceToggle,
    pub mal: MalConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceToggle {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MalConfig {
    pub enabled: bool,
    pub client_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub enabled: bool,
}

impl AppConfig {
    /// Load config: user file (if exists) merged over built-in defaults.
    pub fn load() -> Result<Self, KurozumiError> {
        let defaults: AppConfig =
            toml::from_str(DEFAULT_CONFIG).map_err(|e| KurozumiError::Config(e.to_string()))?;

        let user_path = Self::config_path();
        if user_path.exists() {
            let user_str =
                std::fs::read_to_string(&user_path).map_err(|e| KurozumiError::Config(e.to_string()))?;
            let user: AppConfig =
                toml::from_str(&user_str).map_err(|e| KurozumiError::Config(e.to_string()))?;
            Ok(user)
        } else {
            Ok(defaults)
        }
    }

    /// Save current config to the user config file.
    pub fn save(&self) -> Result<(), KurozumiError> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| KurozumiError::Config(e.to_string()))?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Path to user config file (XDG on Linux, AppData on Windows).
    pub fn config_path() -> PathBuf {
        Self::project_dirs()
            .map(|d| d.config_dir().join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    }

    /// Path to the database file.
    pub fn db_path() -> PathBuf {
        Self::project_dirs()
            .map(|d| d.data_dir().join("kurozumi.db"))
            .unwrap_or_else(|| PathBuf::from("kurozumi.db"))
    }

    /// Ensure the data directory exists and return the DB path.
    pub fn ensure_db_path() -> Result<PathBuf, KurozumiError> {
        let path = Self::db_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(path)
    }

    fn project_dirs() -> Option<ProjectDirs> {
        ProjectDirs::from("", "", "kurozumi")
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        toml::from_str(DEFAULT_CONFIG).expect("built-in default config is valid TOML")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_parses() {
        let config = AppConfig::default();
        assert_eq!(config.general.detection_interval, 5);
        assert!(config.library.auto_update);
        assert!(config.services.anilist.enabled);
        assert!(!config.discord.enabled);
    }

    #[test]
    fn test_roundtrip() {
        let config = AppConfig::default();
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: AppConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.general.detection_interval, config.general.detection_interval);
    }
}
