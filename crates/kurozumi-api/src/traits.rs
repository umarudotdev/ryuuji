//! Trait definitions for anime tracking services.
//!
//! All service clients (AniList, Kitsu, MAL) implement these traits,
//! allowing the orchestrator and UI to be service-agnostic.

use std::future::Future;

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
