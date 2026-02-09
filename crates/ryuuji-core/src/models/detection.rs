use serde::{Deserialize, Serialize};

/// Result of detecting and parsing media playback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedMedia {
    /// The media player that's playing.
    pub player_name: String,
    /// Parsed anime title from the filename/title.
    pub anime_title: Option<String>,
    /// Parsed episode number.
    pub episode: Option<u32>,
    /// Release group (e.g., "SubsPlease").
    pub release_group: Option<String>,
    /// Video resolution (e.g., "1080p").
    pub resolution: Option<String>,
    /// Raw title string before parsing.
    pub raw_title: String,
    /// Streaming service name (e.g., "Crunchyroll"), if detected via browser.
    pub service_name: Option<String>,
}
