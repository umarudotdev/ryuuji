pub mod platform;
pub mod player_db;
pub mod stream;

use serde::{Deserialize, Serialize};

pub use player_db::{PlayerDatabase, PlayerDef};
pub use stream::{StreamDatabase, StreamDef, StreamMatch};

/// Information about a detected media player and what it's playing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    /// Name of the media player (e.g., "mpv", "VLC").
    pub player_name: String,
    /// The media title as reported by the player.
    pub media_title: Option<String>,
    /// The file path currently being played, if available.
    pub file_path: Option<String>,
    /// Whether this player is a web browser.
    pub is_browser: bool,
}

/// Detect what's currently playing across all supported media players.
///
/// Uses the embedded player database.
pub fn detect_players() -> Vec<PlayerInfo> {
    let db = PlayerDatabase::embedded();
    detect_players_with_db(&db)
}

/// Detect what's currently playing, using a custom player database.
pub fn detect_players_with_db(db: &PlayerDatabase) -> Vec<PlayerInfo> {
    platform::detect(db)
}
