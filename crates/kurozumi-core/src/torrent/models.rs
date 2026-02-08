use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A configured RSS feed source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorrentFeed {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub last_checked: Option<DateTime<Utc>>,
}

/// A single torrent parsed from an RSS feed.
#[derive(Debug, Clone)]
pub struct TorrentItem {
    /// Unique ID (from RSS `<guid>` or hash of link).
    pub guid: String,
    /// Raw torrent name from feed.
    pub title: String,
    /// `.torrent` download URL.
    pub link: Option<String>,
    /// Magnet URI.
    pub magnet_link: Option<String>,
    pub description: Option<String>,
    pub size: Option<String>,
    pub seeders: Option<u32>,
    pub leechers: Option<u32>,
    pub downloads: Option<u32>,
    pub pub_date: Option<DateTime<Utc>>,
    pub info_link: Option<String>,
    // Populated by parser/matcher:
    pub anime_id: Option<i64>,
    pub anime_title: Option<String>,
    pub episode: Option<u32>,
    pub release_group: Option<String>,
    pub resolution: Option<String>,
    /// Filter evaluation result.
    pub filter_state: FilterState,
}

/// Result of filter evaluation on a torrent item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterState {
    #[default]
    None,
    Discarded,
    Selected,
    Preferred,
}
