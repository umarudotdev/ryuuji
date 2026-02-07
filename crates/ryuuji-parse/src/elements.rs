use serde::{Deserialize, Serialize};

/// Parsed elements extracted from an anime filename.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Elements {
    /// The anime title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Episode number (as string to handle "01", "12.5", "S2", etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode: Option<String>,
    /// Episode number parsed as u32 when possible.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_number: Option<u32>,
    /// Release group name (e.g., "SubsPlease").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_group: Option<String>,
    /// Video resolution (e.g., "1080p", "720p").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    /// Video codec (e.g., "x264", "HEVC").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_codec: Option<String>,
    /// Audio codec (e.g., "FLAC", "AAC").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_codec: Option<String>,
    /// Season number as string (e.g., "2", "01").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season: Option<String>,
    /// Season number parsed as u32 when possible.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season_number: Option<u32>,
    /// File checksum (e.g., "ABCD1234").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    /// Source (e.g., "BD", "WEB", "TV").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Year of release.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<u32>,
    /// Episode title (text after episode number).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_title: Option<String>,
    /// Part identifier (e.g., "Part 2").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
    /// Part number parsed as u32.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_number: Option<u32>,
    /// Volume identifier (e.g., "Vol.3").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<String>,
    /// Volume number parsed as u32.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_number: Option<u32>,
    /// Release version (e.g., "v2").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_version: Option<String>,
    /// Release info terms (e.g., "Remastered", "Uncensored").
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub release_info: Vec<String>,
    /// Anime type (e.g., "OVA", "Special", "Movie").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anime_type: Option<String>,
    /// Language tags found.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub language: Vec<String>,
    /// Subtitle-related tags.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subtitles: Vec<String>,
    /// Video-related terms (HDR, 10bit, etc.).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub video_term: Vec<String>,
    /// Audio-related terms (Dual Audio, channels, etc.).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub audio_term: Vec<String>,
    /// File extension (e.g., "mkv").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_extension: Option<String>,
    /// Streaming source (e.g., "CR", "AMZN").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming_source: Option<String>,
}
