//! Persist and restore window size and position across sessions.
//!
//! Saves a small JSON file to `~/.local/share/ryuuji/window.json`
//! (or platform equivalent via `directories` crate).

use iced::{Point, Size};
use serde::{Deserialize, Serialize};

const FILE_NAME: &str = "window.json";

/// Persisted window geometry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub width: f32,
    pub height: f32,
    pub x: f32,
    pub y: f32,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            width: 960.0,
            height: 640.0,
            x: -1.0,
            y: -1.0,
        }
    }
}

impl WindowState {
    /// Convert to an iced `Size`.
    pub fn size(&self) -> Size {
        Size::new(self.width.max(400.0), self.height.max(300.0))
    }

    /// Convert to an iced window `Position`, if we have a valid saved position.
    pub fn position(&self) -> Option<Point> {
        if self.x >= 0.0 && self.y >= 0.0 {
            Some(Point::new(self.x, self.y))
        } else {
            None
        }
    }

    /// Load from disk, returning default if file doesn't exist or is invalid.
    pub fn load() -> Self {
        state_path()
            .and_then(|path| std::fs::read_to_string(&path).ok())
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    /// Save to disk. Errors are logged but not propagated.
    pub fn save(&self) {
        if let Some(path) = state_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match serde_json::to_string_pretty(self) {
                Ok(json) => {
                    if let Err(e) = std::fs::write(&path, json) {
                        tracing::warn!("Failed to save window state: {e}");
                    }
                }
                Err(e) => tracing::warn!("Failed to serialize window state: {e}"),
            }
        }
    }
}

/// Path to the window state JSON file.
fn state_path() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("", "", "ryuuji").map(|dirs| dirs.data_dir().join(FILE_NAME))
}
