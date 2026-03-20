//! Repository trait definitions (ports).
//!
//! These traits abstract storage operations so that domain logic depends on
//! interfaces rather than a concrete SQLite implementation. The types returned
//! by these traits (`LibraryRow`, `HistoryRow`, etc.) form the port contract.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::error::RyuujiError;
use crate::models::{Anime, AvailableEpisode, AvailableEpisodeSummary, LibraryEntry, WatchStatus};
use crate::torrent::{TorrentFeed, TorrentFilter};

/// A library entry joined with its anime data for display.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LibraryRow {
    pub entry: LibraryEntry,
    pub anime: Anime,
}

/// Aggregate statistics about the user's library.
#[derive(Debug, Clone)]
pub struct LibraryStatistics {
    pub total_entries: usize,
    pub by_status: HashMap<WatchStatus, usize>,
    pub total_episodes_watched: u32,
    pub total_rewatch_episodes: u32,
    pub total_watch_time_minutes: u64,
    pub mean_score: Option<f32>,
    pub score_distribution: Vec<(u8, usize)>,
    pub top_genres: Vec<(String, usize)>,
}

/// A watch history record (raw, without anime data).
#[derive(Debug, Clone)]
pub struct WatchHistoryRow {
    pub anime_id: i64,
    pub episode: u32,
    pub watched_at: DateTime<Utc>,
}

/// A watch history record joined with anime data for display.
#[derive(Debug, Clone)]
pub struct HistoryRow {
    pub anime: Anime,
    pub episode: u32,
    pub watched_at: DateTime<Utc>,
}

/// Token record: (access_token, refresh_token, expires_at).
pub type TokenRecord = (String, Option<String>, Option<String>);

pub trait AnimeRepository {
    fn get_anime(&self, id: i64) -> Result<Option<Anime>, RyuujiError>;
    fn all_anime(&self) -> Result<Vec<Anime>, RyuujiError>;
    fn get_anime_by_mal_id(&self, mal_id: u64) -> Result<Option<Anime>, RyuujiError>;
    fn get_anime_by_anilist_id(&self, id: u64) -> Result<Option<Anime>, RyuujiError>;
    fn get_anime_by_kitsu_id(&self, id: u64) -> Result<Option<Anime>, RyuujiError>;
    fn insert_anime(&self, anime: &Anime) -> Result<i64, RyuujiError>;
    fn upsert_anime_by_mal_id(&self, anime: &Anime) -> Result<i64, RyuujiError>;
    fn upsert_anime_by_anilist_id(&self, anime: &Anime) -> Result<i64, RyuujiError>;
    fn upsert_anime_by_kitsu_id(&self, anime: &Anime) -> Result<i64, RyuujiError>;
    fn search_anime(&self, query: &str) -> Result<Vec<Anime>, RyuujiError>;
}

pub trait LibraryRepository {
    fn get_library_entry_for_anime(
        &self,
        anime_id: i64,
    ) -> Result<Option<LibraryEntry>, RyuujiError>;
    fn upsert_library_entry(&self, entry: &LibraryEntry) -> Result<i64, RyuujiError>;
    fn update_episode_count(&self, anime_id: i64, episodes: u32) -> Result<(), RyuujiError>;
    fn update_library_status(&self, anime_id: i64, status: WatchStatus) -> Result<(), RyuujiError>;
    fn update_library_score(&self, anime_id: i64, score: f32) -> Result<(), RyuujiError>;
    fn update_library_dates(
        &self,
        anime_id: i64,
        start: Option<&str>,
        finish: Option<&str>,
    ) -> Result<(), RyuujiError>;
    fn update_library_notes(&self, anime_id: i64, notes: Option<&str>) -> Result<(), RyuujiError>;
    fn update_library_rewatch(
        &self,
        anime_id: i64,
        rewatching: bool,
        count: u32,
    ) -> Result<(), RyuujiError>;
    fn delete_library_entry(&self, anime_id: i64) -> Result<(), RyuujiError>;
    fn get_library_by_status(&self, status: WatchStatus) -> Result<Vec<LibraryRow>, RyuujiError>;
    fn get_all_library(&self) -> Result<Vec<LibraryRow>, RyuujiError>;
    fn get_library_statistics(&self) -> Result<LibraryStatistics, RyuujiError>;
}

pub trait WatchHistoryRepository {
    fn record_watch(&self, anime_id: i64, episode: u32) -> Result<(), RyuujiError>;
    fn get_watch_history(&self, limit: u32) -> Result<Vec<HistoryRow>, RyuujiError>;
}

pub trait TokenRepository {
    fn save_token(
        &self,
        service: &str,
        token: &str,
        refresh: Option<&str>,
        expires_at: Option<&str>,
    ) -> Result<(), RyuujiError>;
    fn get_token(&self, service: &str) -> Result<Option<String>, RyuujiError>;
    fn get_token_full(&self, service: &str) -> Result<Option<TokenRecord>, RyuujiError>;
}

pub trait TorrentRepository {
    fn get_torrent_feeds(&self) -> Result<Vec<TorrentFeed>, RyuujiError>;
    fn upsert_torrent_feed(&self, feed: &TorrentFeed) -> Result<i64, RyuujiError>;
    fn delete_torrent_feed(&self, id: i64) -> Result<(), RyuujiError>;
    fn get_torrent_filters(&self) -> Result<Vec<TorrentFilter>, RyuujiError>;
    fn upsert_torrent_filter(&self, filter: &TorrentFilter) -> Result<i64, RyuujiError>;
    fn delete_torrent_filter(&self, id: i64) -> Result<(), RyuujiError>;
    fn is_torrent_archived(&self, guid: &str) -> Result<bool, RyuujiError>;
    fn archive_torrent(&self, guid: &str, title: &str, action: &str) -> Result<(), RyuujiError>;
    fn clear_torrent_archive(&self) -> Result<(), RyuujiError>;
}

pub trait EpisodeFileRepository {
    fn upsert_available_episode(&self, ep: &AvailableEpisode) -> Result<(), RyuujiError>;
    fn get_available_episode_summaries(&self) -> Result<Vec<AvailableEpisodeSummary>, RyuujiError>;
    fn is_file_indexed(
        &self,
        file_path: &str,
        file_size: u64,
        file_modified: &str,
    ) -> Result<bool, RyuujiError>;
    fn clear_available_episodes(&self) -> Result<(), RyuujiError>;
}

/// Convenience supertrait for the detection pipeline.
pub trait DetectionStore: AnimeRepository + LibraryRepository + WatchHistoryRepository {}
impl<T: AnimeRepository + LibraryRepository + WatchHistoryRepository> DetectionStore for T {}
