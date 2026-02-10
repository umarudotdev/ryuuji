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

    /// Get anime details by service-specific ID.
    fn get_anime(
        &self,
        anime_id: u64,
    ) -> impl Future<Output = Result<AnimeSearchResult, Self::Error>> + Send;

    /// Add an anime to the authenticated user's list with the given status.
    ///
    /// Status values: "watching", "plan_to_watch", "completed", "on_hold", "dropped".
    /// Each service maps these to its own format internally.
    /// Services with upsert semantics (MAL, AniList) will update if already present.
    fn add_library_entry(
        &self,
        anime_id: u64,
        status: &str,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Delete an anime from the authenticated user's list.
    ///
    /// Treat "not found" as success (entry already gone = desired state).
    fn delete_library_entry(
        &self,
        anime_id: u64,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Browse anime by season and year with pagination.
    fn browse_season(
        &self,
        season: AnimeSeason,
        year: u32,
        page: u32,
    ) -> impl Future<Output = Result<SeasonPage, Self::Error>> + Send;
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

    /// Convert to MAL API season string (lowercase).
    pub fn to_mal_str(self) -> &'static str {
        match self {
            Self::Winter => "winter",
            Self::Spring => "spring",
            Self::Summer => "summer",
            Self::Fall => "fall",
        }
    }

    /// Convert to Kitsu API season string (lowercase).
    pub fn to_kitsu_str(self) -> &'static str {
        match self {
            Self::Winter => "winter",
            Self::Spring => "spring",
            Self::Summer => "summer",
            Self::Fall => "fall",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anime_season_to_anilist_str() {
        assert_eq!(AnimeSeason::Winter.to_anilist_str(), "WINTER");
        assert_eq!(AnimeSeason::Spring.to_anilist_str(), "SPRING");
        assert_eq!(AnimeSeason::Summer.to_anilist_str(), "SUMMER");
        assert_eq!(AnimeSeason::Fall.to_anilist_str(), "FALL");
    }

    #[test]
    fn test_anime_season_to_mal_str() {
        assert_eq!(AnimeSeason::Winter.to_mal_str(), "winter");
        assert_eq!(AnimeSeason::Spring.to_mal_str(), "spring");
        assert_eq!(AnimeSeason::Summer.to_mal_str(), "summer");
        assert_eq!(AnimeSeason::Fall.to_mal_str(), "fall");
    }

    #[test]
    fn test_anime_season_to_kitsu_str() {
        assert_eq!(AnimeSeason::Winter.to_kitsu_str(), "winter");
        assert_eq!(AnimeSeason::Spring.to_kitsu_str(), "spring");
        assert_eq!(AnimeSeason::Summer.to_kitsu_str(), "summer");
        assert_eq!(AnimeSeason::Fall.to_kitsu_str(), "fall");
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
