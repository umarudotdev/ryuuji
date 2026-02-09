//! Trait definitions for anime tracking services.
//!
//! All service clients (AniList, Kitsu, MAL) implement these traits,
//! allowing the orchestrator and UI to be service-agnostic.

use std::future::Future;

use chrono::Datelike;

/// A unified anime tracking service interface.
pub trait AnimeService: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Authenticate with the service. Returns an access token.
    fn authenticate(&self) -> impl Future<Output = Result<String, Self::Error>> + Send;

    /// Search for anime by title.
    fn search_anime(
        &self,
        query: &str,
    ) -> impl Future<Output = Result<Vec<AnimeSearchResult>, Self::Error>> + Send;

    /// Get the authenticated user's anime list.
    fn get_user_list(&self)
        -> impl Future<Output = Result<Vec<UserListEntry>, Self::Error>> + Send;

    /// Update progress for an anime.
    fn update_progress(
        &self,
        anime_id: u64,
        episode: u32,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

/// A search result from any anime service.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnimeSearchResult {
    pub service_id: u64,
    pub title: String,
    pub title_english: Option<String>,
    pub episodes: Option<u32>,
    pub cover_url: Option<String>,
    pub media_type: Option<String>,
    pub status: Option<String>,
    pub synopsis: Option<String>,
    pub genres: Vec<String>,
    pub mean_score: Option<f32>,
    pub season: Option<String>,
    pub year: Option<u32>,
}

/// An entry from a user's anime list.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserListEntry {
    pub service_id: u64,
    pub title: String,
    pub watched_episodes: u32,
    pub total_episodes: Option<u32>,
    pub status: String,
    pub score: Option<f32>,
}

/// A page of season browse results.
#[derive(Debug, Clone)]
pub struct SeasonPage {
    pub items: Vec<AnimeSearchResult>,
    pub has_next: bool,
}

/// Anime season (quarter of the year).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimeSeason {
    Winter,
    Spring,
    Summer,
    Fall,
}

impl AnimeSeason {
    pub const ALL: &[AnimeSeason] = &[Self::Winter, Self::Spring, Self::Summer, Self::Fall];

    /// Convert to AniList GraphQL `MediaSeason` enum value.
    pub fn to_anilist_str(self) -> &'static str {
        match self {
            Self::Winter => "WINTER",
            Self::Spring => "SPRING",
            Self::Summer => "SUMMER",
            Self::Fall => "FALL",
        }
    }

    /// Determine the current anime season from the current month.
    pub fn current() -> Self {
        let month = chrono::Utc::now().month();
        match month {
            1..=3 => Self::Winter,
            4..=6 => Self::Spring,
            7..=9 => Self::Summer,
            _ => Self::Fall,
        }
    }
}

impl std::fmt::Display for AnimeSeason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Winter => write!(f, "Winter"),
            Self::Spring => write!(f, "Spring"),
            Self::Summer => write!(f, "Summer"),
            Self::Fall => write!(f, "Fall"),
        }
    }
}
