use serde::{Deserialize, Serialize};

/// Parsed elements extracted from an anime filename.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Elements {
    /// The anime title.
    pub title: Option<String>,
    /// Episode number (as string to handle "01", "12.5", "S2", etc.).
    pub episode: Option<String>,
    /// Episode number parsed as u32 when possible.
    pub episode_number: Option<u32>,
    /// Release group name (e.g., "SubsPlease").
    pub release_group: Option<String>,
    /// Video resolution (e.g., "1080p", "720p").
    pub resolution: Option<String>,
    /// Video codec (e.g., "x264", "HEVC").
    pub video_codec: Option<String>,
    /// Audio codec (e.g., "FLAC", "AAC").
    pub audio_codec: Option<String>,
    /// Season number.
    pub season: Option<String>,
    /// File checksum (e.g., "ABCD1234").
    pub checksum: Option<String>,
    /// Source (e.g., "BD", "WEB", "TV").
    pub source: Option<String>,
    /// Year of release.
    pub year: Option<u32>,
}
