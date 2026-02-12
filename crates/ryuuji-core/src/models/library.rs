use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// User's watch status for a library entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WatchStatus {
    Watching,
    Completed,
    OnHold,
    Dropped,
    PlanToWatch,
}

impl WatchStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Watching => "Watching",
            Self::Completed => "Completed",
            Self::OnHold => "On Hold",
            Self::Dropped => "Dropped",
            Self::PlanToWatch => "Plan to Watch",
        }
    }

    /// Database string representation (lowercase, no spaces).
    pub fn as_db_str(&self) -> &'static str {
        match self {
            Self::Watching => "watching",
            Self::Completed => "completed",
            Self::OnHold => "on_hold",
            Self::Dropped => "dropped",
            Self::PlanToWatch => "plan_to_watch",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "watching" => Some(Self::Watching),
            "completed" => Some(Self::Completed),
            "on_hold" => Some(Self::OnHold),
            "dropped" => Some(Self::Dropped),
            "plan_to_watch" => Some(Self::PlanToWatch),
            _ => None,
        }
    }

    pub const ALL: &[WatchStatus] = &[
        Self::Watching,
        Self::Completed,
        Self::OnHold,
        Self::Dropped,
        Self::PlanToWatch,
    ];
}

impl std::fmt::Display for WatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An episode file found on disk by the folder scanner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableEpisode {
    pub id: i64,
    pub anime_id: i64,
    pub episode: u32,
    pub file_path: String,
    pub file_size: u64,
    pub file_modified: String,
    pub release_group: Option<String>,
    pub resolution: Option<String>,
}

/// Summary of available episodes for a library entry (for display).
#[derive(Debug, Clone, Default)]
pub struct AvailableEpisodeSummary {
    pub anime_id: i64,
    pub count: u32,
}

/// A user's library entry linking to an anime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryEntry {
    pub id: i64,
    pub anime_id: i64,
    pub status: WatchStatus,
    pub watched_episodes: u32,
    pub score: Option<f32>,
    pub updated_at: DateTime<Utc>,
    pub start_date: Option<String>,
    pub finish_date: Option<String>,
    pub notes: Option<String>,
    pub rewatching: bool,
    pub rewatch_count: u32,
}
