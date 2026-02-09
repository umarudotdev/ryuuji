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
    /// Season number as string (e.g., "2", "01").
    pub season: Option<String>,
    /// Season number parsed as u32 when possible.
    pub season_number: Option<u32>,
    /// File checksum (e.g., "ABCD1234").
    pub checksum: Option<String>,
    /// Source (e.g., "BD", "WEB", "TV").
    pub source: Option<String>,
    /// Year of release.
    pub year: Option<u32>,
    /// Episode title (text after episode number).
    pub episode_title: Option<String>,
    /// Part identifier (e.g., "Part 2").
    pub part: Option<String>,
    /// Part number parsed as u32.
    pub part_number: Option<u32>,
    /// Volume identifier (e.g., "Vol.3").
    pub volume: Option<String>,
    /// Volume number parsed as u32.
    pub volume_number: Option<u32>,
    /// Release version (e.g., "v2").
    pub release_version: Option<String>,
    /// Release info terms (e.g., "Remastered", "Uncensored").
    pub release_info: Vec<String>,
    /// Anime type (e.g., "OVA", "Special", "Movie").
    pub anime_type: Option<String>,
    /// Language tags found.
    pub language: Vec<String>,
    /// Subtitle-related tags.
    pub subtitles: Vec<String>,
    /// Video-related terms (HDR, 10bit, etc.).
    pub video_term: Vec<String>,
    /// Audio-related terms (Dual Audio, channels, etc.).
    pub audio_term: Vec<String>,
    /// File extension (e.g., "mkv").
    pub file_extension: Option<String>,
    /// Streaming source (e.g., "CR", "AMZN").
    pub streaming_source: Option<String>,
}
