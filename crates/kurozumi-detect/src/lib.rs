pub mod platform;
pub mod players;

use serde::{Deserialize, Serialize};

/// Information about a detected media player and what it's playing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    /// Name of the media player (e.g., "mpv", "VLC").
    pub player_name: String,
    /// The media title as reported by the player.
    pub media_title: Option<String>,
    /// The file path currently being played, if available.
    pub file_path: Option<String>,
}

/// Detect what's currently playing across all supported media players.
///
/// Returns a list of all detected active players with media info.
pub fn detect_players() -> Vec<PlayerInfo> {
    platform::detect()
}
