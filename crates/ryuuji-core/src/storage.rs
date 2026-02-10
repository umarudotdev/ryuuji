use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::RyuujiError;
use crate::models::{Anime, AnimeIds, AnimeTitle, LibraryEntry, WatchStatus};
use crate::torrent::filter::{FilterAction, MatchMode, TorrentFilter};
use crate::torrent::models::TorrentFeed;

const SCHEMA_V1: &str = include_str!("../../../migrations/001_initial.sql");
const SCHEMA_V2: &str = include_str!("../../../migrations/002_add_anime_metadata.sql");
const SCHEMA_V3: &str = include_str!("../../../migrations/003_torrent_tables.sql");
const SCHEMA_V4: &str = include_str!("../../../migrations/004_add_library_fields.sql");

/// Token record: (access_token, refresh_token, expires_at).
pub type TokenRecord = (String, Option<String>, Option<String>);

/// SQLite-backed storage for the ryuuji library.
pub struct Storage {
    conn: Connection,
}

/// A library entry joined with its anime data for display.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LibraryRow {
    pub entry: LibraryEntry,
    pub anime: Anime,
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

impl Storage {
    /// Open (or create) the database at the given path and run migrations.
    pub fn open(path: &Path) -> Result<Self, RyuujiError> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        run_migrations(&conn)?;
        Ok(Self { conn })
    }

    /// Open an in-memory database (for tests).
    pub fn open_memory() -> Result<Self, RyuujiError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        run_migrations(&conn)?;
        Ok(Self { conn })
    }

    // ── Anime CRUD ──────────────────────────────────────────────

    /// Insert a new anime, returning its auto-generated ID.
    pub fn insert_anime(&self, anime: &Anime) -> Result<i64, RyuujiError> {
        let synonyms_json = serde_json::to_string(&anime.synonyms).unwrap_or_default();
        let genres_json = serde_json::to_string(&anime.genres).unwrap_or_default();
        let studios_json = serde_json::to_string(&anime.studios).unwrap_or_default();
        self.conn.execute(
            "INSERT INTO anime (anilist_id, kitsu_id, mal_id, title_romaji, title_english,
             title_native, synonyms, episodes, cover_url, season, year,
             synopsis, genres, media_type, airing_status, mean_score,
             studios, source, rating, start_date, end_date)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                     ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                anime.ids.anilist.map(|v| v as i64),
                anime.ids.kitsu.map(|v| v as i64),
                anime.ids.mal.map(|v| v as i64),
                anime.title.romaji,
                anime.title.english,
                anime.title.native,
                synonyms_json,
                anime.episodes,
                anime.cover_url,
                anime.season,
                anime.year,
                anime.synopsis,
                genres_json,
                anime.media_type,
                anime.airing_status,
                anime.mean_score,
                studios_json,
                anime.source,
                anime.rating,
                anime.start_date,
                anime.end_date,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Get an anime by its local database ID.
    pub fn get_anime(&self, id: i64) -> Result<Option<Anime>, RyuujiError> {
        self.conn
            .query_row(
                "SELECT id, anilist_id, kitsu_id, mal_id, title_romaji, title_english,
                 title_native, synonyms, episodes, cover_url, season, year,
                 synopsis, genres, media_type, airing_status, mean_score,
                 studios, source, rating, start_date, end_date
                 FROM anime WHERE id = ?1",
                params![id],
                |row| Ok(row_to_anime(row)),
            )
            .optional()
            .map_err(Into::into)
    }

    /// Search anime by title substring (case-insensitive).
    pub fn search_anime(&self, query: &str) -> Result<Vec<Anime>, RyuujiError> {
        let pattern = format!("%{query}%");
        let mut stmt = self.conn.prepare(
            "SELECT id, anilist_id, kitsu_id, mal_id, title_romaji, title_english,
             title_native, synonyms, episodes, cover_url, season, year,
             synopsis, genres, media_type, airing_status, mean_score,
             studios, source, rating, start_date, end_date
             FROM anime
             WHERE title_romaji LIKE ?1 OR title_english LIKE ?1 OR title_native LIKE ?1
                   OR synonyms LIKE ?1",
        )?;
        let rows = stmt
            .query_map(params![pattern], |row| Ok(row_to_anime(row)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// Get all anime in the database.
    pub fn all_anime(&self) -> Result<Vec<Anime>, RyuujiError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, anilist_id, kitsu_id, mal_id, title_romaji, title_english,
             title_native, synonyms, episodes, cover_url, season, year,
             synopsis, genres, media_type, airing_status, mean_score,
             studios, source, rating, start_date, end_date
             FROM anime ORDER BY title_romaji",
        )?;
        let rows = stmt
            .query_map([], |row| Ok(row_to_anime(row)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// Get an anime by its MAL ID.
    pub fn get_anime_by_mal_id(&self, mal_id: u64) -> Result<Option<Anime>, RyuujiError> {
        self.conn
            .query_row(
                "SELECT id, anilist_id, kitsu_id, mal_id, title_romaji, title_english,
                 title_native, synonyms, episodes, cover_url, season, year,
                 synopsis, genres, media_type, airing_status, mean_score,
                 studios, source, rating, start_date, end_date
                 FROM anime WHERE mal_id = ?1",
                params![mal_id as i64],
                |row| Ok(row_to_anime(row)),
            )
            .optional()
            .map_err(Into::into)
    }

    /// Insert or update an anime keyed by MAL ID.
    ///
    /// If an anime with the same `mal_id` already exists, update its titles,
    /// synonyms, episodes, and cover. Otherwise insert a new row.
    /// Returns the local database ID.
    pub fn upsert_anime_by_mal_id(&self, anime: &Anime) -> Result<i64, RyuujiError> {
        let mal_id = anime
            .ids
            .mal
            .expect("upsert_anime_by_mal_id requires a MAL ID");

        if let Some(existing) = self.get_anime_by_mal_id(mal_id)? {
            let synonyms_json = serde_json::to_string(&anime.synonyms).unwrap_or_default();
            let genres_json = serde_json::to_string(&anime.genres).unwrap_or_default();
            let studios_json = serde_json::to_string(&anime.studios).unwrap_or_default();
            self.conn.execute(
                "UPDATE anime SET
                    title_romaji = ?1, title_english = ?2, title_native = ?3,
                    synonyms = ?4, episodes = ?5, cover_url = ?6,
                    season = ?7, year = ?8,
                    synopsis = ?9, genres = ?10, media_type = ?11,
                    airing_status = ?12, mean_score = ?13, studios = ?14,
                    source = ?15, rating = ?16, start_date = ?17, end_date = ?18
                 WHERE id = ?19",
                params![
                    anime.title.romaji,
                    anime.title.english,
                    anime.title.native,
                    synonyms_json,
                    anime.episodes,
                    anime.cover_url,
                    anime.season,
                    anime.year,
                    anime.synopsis,
                    genres_json,
                    anime.media_type,
                    anime.airing_status,
                    anime.mean_score,
                    studios_json,
                    anime.source,
                    anime.rating,
                    anime.start_date,
                    anime.end_date,
                    existing.id,
                ],
            )?;
            Ok(existing.id)
        } else {
            self.insert_anime(anime)
        }
    }

    /// Get an anime by its AniList ID.
    pub fn get_anime_by_anilist_id(&self, anilist_id: u64) -> Result<Option<Anime>, RyuujiError> {
        self.conn
            .query_row(
                "SELECT id, anilist_id, kitsu_id, mal_id, title_romaji, title_english,
                 title_native, synonyms, episodes, cover_url, season, year,
                 synopsis, genres, media_type, airing_status, mean_score,
                 studios, source, rating, start_date, end_date
                 FROM anime WHERE anilist_id = ?1",
                params![anilist_id as i64],
                |row| Ok(row_to_anime(row)),
            )
            .optional()
            .map_err(Into::into)
    }

    /// Insert or update an anime keyed by AniList ID.
    pub fn upsert_anime_by_anilist_id(&self, anime: &Anime) -> Result<i64, RyuujiError> {
        let anilist_id = anime
            .ids
            .anilist
            .expect("upsert_anime_by_anilist_id requires an AniList ID");

        if let Some(existing) = self.get_anime_by_anilist_id(anilist_id)? {
            let synonyms_json = serde_json::to_string(&anime.synonyms).unwrap_or_default();
            let genres_json = serde_json::to_string(&anime.genres).unwrap_or_default();
            let studios_json = serde_json::to_string(&anime.studios).unwrap_or_default();
            self.conn.execute(
                "UPDATE anime SET
                    title_romaji = ?1, title_english = ?2, title_native = ?3,
                    synonyms = ?4, episodes = ?5, cover_url = ?6,
                    season = ?7, year = ?8,
                    synopsis = ?9, genres = ?10, media_type = ?11,
                    airing_status = ?12, mean_score = ?13, studios = ?14,
                    source = ?15, rating = ?16, start_date = ?17, end_date = ?18
                 WHERE id = ?19",
                params![
                    anime.title.romaji,
                    anime.title.english,
                    anime.title.native,
                    synonyms_json,
                    anime.episodes,
                    anime.cover_url,
                    anime.season,
                    anime.year,
                    anime.synopsis,
                    genres_json,
                    anime.media_type,
                    anime.airing_status,
                    anime.mean_score,
                    studios_json,
                    anime.source,
                    anime.rating,
                    anime.start_date,
                    anime.end_date,
                    existing.id,
                ],
            )?;
            Ok(existing.id)
        } else {
            self.insert_anime(anime)
        }
    }

    /// Get an anime by its Kitsu ID.
    pub fn get_anime_by_kitsu_id(&self, kitsu_id: u64) -> Result<Option<Anime>, RyuujiError> {
        self.conn
            .query_row(
                "SELECT id, anilist_id, kitsu_id, mal_id, title_romaji, title_english,
                 title_native, synonyms, episodes, cover_url, season, year,
                 synopsis, genres, media_type, airing_status, mean_score,
                 studios, source, rating, start_date, end_date
                 FROM anime WHERE kitsu_id = ?1",
                params![kitsu_id as i64],
                |row| Ok(row_to_anime(row)),
            )
            .optional()
            .map_err(Into::into)
    }

    /// Insert or update an anime keyed by Kitsu ID.
    pub fn upsert_anime_by_kitsu_id(&self, anime: &Anime) -> Result<i64, RyuujiError> {
        let kitsu_id = anime
            .ids
            .kitsu
            .expect("upsert_anime_by_kitsu_id requires a Kitsu ID");

        if let Some(existing) = self.get_anime_by_kitsu_id(kitsu_id)? {
            let synonyms_json = serde_json::to_string(&anime.synonyms).unwrap_or_default();
            let genres_json = serde_json::to_string(&anime.genres).unwrap_or_default();
            let studios_json = serde_json::to_string(&anime.studios).unwrap_or_default();
            self.conn.execute(
                "UPDATE anime SET
                    title_romaji = ?1, title_english = ?2, title_native = ?3,
                    synonyms = ?4, episodes = ?5, cover_url = ?6,
                    season = ?7, year = ?8,
                    synopsis = ?9, genres = ?10, media_type = ?11,
                    airing_status = ?12, mean_score = ?13, studios = ?14,
                    source = ?15, rating = ?16, start_date = ?17, end_date = ?18
                 WHERE id = ?19",
                params![
                    anime.title.romaji,
                    anime.title.english,
                    anime.title.native,
                    synonyms_json,
                    anime.episodes,
                    anime.cover_url,
                    anime.season,
                    anime.year,
                    anime.synopsis,
                    genres_json,
                    anime.media_type,
                    anime.airing_status,
                    anime.mean_score,
                    studios_json,
                    anime.source,
                    anime.rating,
                    anime.start_date,
                    anime.end_date,
                    existing.id,
                ],
            )?;
            Ok(existing.id)
        } else {
            self.insert_anime(anime)
        }
    }

    // ── Library Entry CRUD ──────────────────────────────────────

    /// Insert or update a library entry.
    pub fn upsert_library_entry(&self, entry: &LibraryEntry) -> Result<i64, RyuujiError> {
        self.conn.execute(
            "INSERT INTO library_entry (anime_id, status, watched_episodes, score, updated_at,
             start_date, finish_date, notes, rewatching, rewatch_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(anime_id) DO UPDATE SET
               status = excluded.status,
               watched_episodes = excluded.watched_episodes,
               score = excluded.score,
               updated_at = excluded.updated_at,
               start_date = excluded.start_date,
               finish_date = excluded.finish_date,
               notes = excluded.notes,
               rewatching = excluded.rewatching,
               rewatch_count = excluded.rewatch_count",
            params![
                entry.anime_id,
                entry.status.as_db_str(),
                entry.watched_episodes,
                entry.score,
                entry.updated_at.to_rfc3339(),
                entry.start_date,
                entry.finish_date,
                entry.notes,
                entry.rewatching as i32,
                entry.rewatch_count,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Get all library entries for a given watch status, joined with anime data.
    pub fn get_library_by_status(
        &self,
        status: WatchStatus,
    ) -> Result<Vec<LibraryRow>, RyuujiError> {
        let mut stmt = self.conn.prepare(
            "SELECT le.id, le.anime_id, le.status, le.watched_episodes, le.score, le.updated_at,
                    le.start_date, le.finish_date, le.notes, le.rewatching, le.rewatch_count,
                    a.id, a.anilist_id, a.kitsu_id, a.mal_id, a.title_romaji, a.title_english,
                    a.title_native, a.synonyms, a.episodes, a.cover_url, a.season, a.year,
                    a.synopsis, a.genres, a.media_type, a.airing_status, a.mean_score,
                    a.studios, a.source, a.rating, a.start_date, a.end_date
             FROM library_entry le
             JOIN anime a ON le.anime_id = a.id
             WHERE le.status = ?1
             ORDER BY a.title_romaji",
        )?;
        let rows = stmt
            .query_map(params![status.as_db_str()], |row| {
                Ok(LibraryRow {
                    entry: row_to_library_entry(row, 0),
                    anime: row_to_anime_at(row, 11),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// Get all library entries joined with anime data.
    pub fn get_all_library(&self) -> Result<Vec<LibraryRow>, RyuujiError> {
        let mut stmt = self.conn.prepare(
            "SELECT le.id, le.anime_id, le.status, le.watched_episodes, le.score, le.updated_at,
                    le.start_date, le.finish_date, le.notes, le.rewatching, le.rewatch_count,
                    a.id, a.anilist_id, a.kitsu_id, a.mal_id, a.title_romaji, a.title_english,
                    a.title_native, a.synonyms, a.episodes, a.cover_url, a.season, a.year,
                    a.synopsis, a.genres, a.media_type, a.airing_status, a.mean_score,
                    a.studios, a.source, a.rating, a.start_date, a.end_date
             FROM library_entry le
             JOIN anime a ON le.anime_id = a.id
             ORDER BY le.updated_at DESC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(LibraryRow {
                    entry: row_to_library_entry(row, 0),
                    anime: row_to_anime_at(row, 11),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// Get library entry for a specific anime.
    pub fn get_library_entry_for_anime(
        &self,
        anime_id: i64,
    ) -> Result<Option<LibraryEntry>, RyuujiError> {
        self.conn
            .query_row(
                "SELECT id, anime_id, status, watched_episodes, score, updated_at,
                        start_date, finish_date, notes, rewatching, rewatch_count
                 FROM library_entry WHERE anime_id = ?1",
                params![anime_id],
                |row| Ok(row_to_library_entry(row, 0)),
            )
            .optional()
            .map_err(Into::into)
    }

    /// Update just the episode count for a library entry.
    pub fn update_episode_count(&self, anime_id: i64, episodes: u32) -> Result<(), RyuujiError> {
        self.conn.execute(
            "UPDATE library_entry SET watched_episodes = ?1, updated_at = ?2
             WHERE anime_id = ?3",
            params![episodes, Utc::now().to_rfc3339(), anime_id],
        )?;
        Ok(())
    }

    /// Update just the status for a library entry.
    pub fn update_library_status(
        &self,
        anime_id: i64,
        status: WatchStatus,
    ) -> Result<(), RyuujiError> {
        self.conn.execute(
            "UPDATE library_entry SET status = ?1, updated_at = ?2
             WHERE anime_id = ?3",
            params![status.as_db_str(), Utc::now().to_rfc3339(), anime_id],
        )?;
        Ok(())
    }

    /// Update just the score for a library entry.
    pub fn update_library_score(&self, anime_id: i64, score: f32) -> Result<(), RyuujiError> {
        self.conn.execute(
            "UPDATE library_entry SET score = ?1, updated_at = ?2
             WHERE anime_id = ?3",
            params![score, Utc::now().to_rfc3339(), anime_id],
        )?;
        Ok(())
    }

    /// Update start/finish dates for a library entry.
    pub fn update_library_dates(
        &self,
        anime_id: i64,
        start_date: Option<&str>,
        finish_date: Option<&str>,
    ) -> Result<(), RyuujiError> {
        self.conn.execute(
            "UPDATE library_entry SET start_date = ?1, finish_date = ?2, updated_at = ?3
             WHERE anime_id = ?4",
            params![start_date, finish_date, Utc::now().to_rfc3339(), anime_id],
        )?;
        Ok(())
    }

    /// Update notes for a library entry.
    pub fn update_library_notes(
        &self,
        anime_id: i64,
        notes: Option<&str>,
    ) -> Result<(), RyuujiError> {
        self.conn.execute(
            "UPDATE library_entry SET notes = ?1, updated_at = ?2
             WHERE anime_id = ?3",
            params![notes, Utc::now().to_rfc3339(), anime_id],
        )?;
        Ok(())
    }

    /// Update rewatching flag and rewatch count for a library entry.
    pub fn update_library_rewatch(
        &self,
        anime_id: i64,
        rewatching: bool,
        rewatch_count: u32,
    ) -> Result<(), RyuujiError> {
        self.conn.execute(
            "UPDATE library_entry SET rewatching = ?1, rewatch_count = ?2, updated_at = ?3
             WHERE anime_id = ?4",
            params![
                rewatching as i32,
                rewatch_count,
                Utc::now().to_rfc3339(),
                anime_id
            ],
        )?;
        Ok(())
    }

    /// Delete a library entry by anime ID.
    pub fn delete_library_entry(&self, anime_id: i64) -> Result<(), RyuujiError> {
        self.conn.execute(
            "DELETE FROM library_entry WHERE anime_id = ?1",
            params![anime_id],
        )?;
        Ok(())
    }

    // ── Watch History ───────────────────────────────────────────

    /// Record an episode watch.
    pub fn record_watch(&self, anime_id: i64, episode: u32) -> Result<(), RyuujiError> {
        self.conn.execute(
            "INSERT INTO watch_history (anime_id, episode) VALUES (?1, ?2)",
            params![anime_id, episode],
        )?;
        Ok(())
    }

    /// Get recent watch history.
    pub fn recent_history(&self, limit: u32) -> Result<Vec<WatchHistoryRow>, RyuujiError> {
        let mut stmt = self.conn.prepare(
            "SELECT anime_id, episode, watched_at FROM watch_history
             ORDER BY watched_at DESC, id DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit], |row| {
                let watched_at_str: String = row.get(2)?;
                let watched_at = parse_datetime(&watched_at_str);
                Ok(WatchHistoryRow {
                    anime_id: row.get(0)?,
                    episode: row.get(1)?,
                    watched_at,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// Get recent watch history joined with anime data.
    pub fn get_watch_history(&self, limit: u32) -> Result<Vec<HistoryRow>, RyuujiError> {
        let mut stmt = self.conn.prepare(
            "SELECT a.id, a.anilist_id, a.kitsu_id, a.mal_id, a.title_romaji, a.title_english,
                    a.title_native, a.synonyms, a.episodes, a.cover_url, a.season, a.year,
                    a.synopsis, a.genres, a.media_type, a.airing_status, a.mean_score,
                    a.studios, a.source, a.rating, a.start_date, a.end_date,
                    wh.episode, wh.watched_at
             FROM watch_history wh
             JOIN anime a ON wh.anime_id = a.id
             ORDER BY wh.watched_at DESC, wh.id DESC
             LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit], |row| {
                let watched_at_str: String = row.get(23)?;
                Ok(HistoryRow {
                    anime: row_to_anime_at(row, 0),
                    episode: row.get(22)?,
                    watched_at: parse_datetime(&watched_at_str),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    // ── Auth Tokens ─────────────────────────────────────────────

    /// Store an auth token for a service.
    pub fn save_token(
        &self,
        service: &str,
        token: &str,
        refresh: Option<&str>,
        expires_at: Option<&str>,
    ) -> Result<(), RyuujiError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO auth_tokens (service, token, refresh, expires_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![service, token, refresh, expires_at],
        )?;
        Ok(())
    }

    /// Get the token for a service.
    pub fn get_token(&self, service: &str) -> Result<Option<String>, RyuujiError> {
        self.conn
            .query_row(
                "SELECT token FROM auth_tokens WHERE service = ?1",
                params![service],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    /// Get the full token record for a service (token, refresh_token, expires_at).
    pub fn get_token_full(&self, service: &str) -> Result<Option<TokenRecord>, RyuujiError> {
        self.conn
            .query_row(
                "SELECT token, refresh, expires_at FROM auth_tokens WHERE service = ?1",
                params![service],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(Into::into)
    }

    // ── Torrent Feeds ────────────────────────────────────────────

    /// Get all torrent feed sources.
    pub fn get_torrent_feeds(&self) -> Result<Vec<TorrentFeed>, RyuujiError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, url, enabled, last_checked FROM torrent_feed ORDER BY name",
        )?;
        let rows = stmt
            .query_map([], |row| {
                let last_checked_str: Option<String> = row.get(4)?;
                Ok(TorrentFeed {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    url: row.get(2)?,
                    enabled: row.get::<_, i32>(3)? != 0,
                    last_checked: last_checked_str.map(|s| parse_datetime(&s)),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// Insert or update a torrent feed. Returns the row ID.
    pub fn upsert_torrent_feed(&self, feed: &TorrentFeed) -> Result<i64, RyuujiError> {
        if feed.id > 0 {
            self.conn.execute(
                "UPDATE torrent_feed SET name = ?1, url = ?2, enabled = ?3, last_checked = ?4
                 WHERE id = ?5",
                params![
                    feed.name,
                    feed.url,
                    feed.enabled as i32,
                    feed.last_checked.map(|d| d.to_rfc3339()),
                    feed.id,
                ],
            )?;
            Ok(feed.id)
        } else {
            self.conn.execute(
                "INSERT INTO torrent_feed (name, url, enabled, last_checked)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    feed.name,
                    feed.url,
                    feed.enabled as i32,
                    feed.last_checked.map(|d| d.to_rfc3339()),
                ],
            )?;
            Ok(self.conn.last_insert_rowid())
        }
    }

    /// Delete a torrent feed by ID.
    pub fn delete_torrent_feed(&self, id: i64) -> Result<(), RyuujiError> {
        self.conn
            .execute("DELETE FROM torrent_feed WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ── Torrent Filters ──────────────────────────────────────────

    /// Get all torrent filters.
    pub fn get_torrent_filters(&self) -> Result<Vec<TorrentFilter>, RyuujiError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, enabled, priority, match_mode, action, conditions
             FROM torrent_filter ORDER BY priority, name",
        )?;
        let rows = stmt
            .query_map([], |row| {
                let conditions_json: String = row.get(6)?;
                let match_mode_str: String = row.get(4)?;
                let action_str: String = row.get(5)?;
                Ok(TorrentFilter {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    enabled: row.get::<_, i32>(2)? != 0,
                    priority: row.get(3)?,
                    match_mode: match match_mode_str.as_str() {
                        "any" => MatchMode::Any,
                        _ => MatchMode::All,
                    },
                    action: match action_str.as_str() {
                        "select" => FilterAction::Select,
                        "prefer" => FilterAction::Prefer,
                        _ => FilterAction::Discard,
                    },
                    conditions: serde_json::from_str(&conditions_json).unwrap_or_default(),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// Insert or update a torrent filter. Returns the row ID.
    pub fn upsert_torrent_filter(&self, filter: &TorrentFilter) -> Result<i64, RyuujiError> {
        let conditions_json = serde_json::to_string(&filter.conditions).unwrap_or_default();
        let match_mode_str = match filter.match_mode {
            MatchMode::All => "all",
            MatchMode::Any => "any",
        };
        let action_str = match filter.action {
            FilterAction::Discard => "discard",
            FilterAction::Select => "select",
            FilterAction::Prefer => "prefer",
        };

        if filter.id > 0 {
            self.conn.execute(
                "UPDATE torrent_filter SET name = ?1, enabled = ?2, priority = ?3,
                 match_mode = ?4, action = ?5, conditions = ?6 WHERE id = ?7",
                params![
                    filter.name,
                    filter.enabled as i32,
                    filter.priority,
                    match_mode_str,
                    action_str,
                    conditions_json,
                    filter.id,
                ],
            )?;
            Ok(filter.id)
        } else {
            self.conn.execute(
                "INSERT INTO torrent_filter (name, enabled, priority, match_mode, action, conditions)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    filter.name,
                    filter.enabled as i32,
                    filter.priority,
                    match_mode_str,
                    action_str,
                    conditions_json,
                ],
            )?;
            Ok(self.conn.last_insert_rowid())
        }
    }

    /// Delete a torrent filter by ID.
    pub fn delete_torrent_filter(&self, id: i64) -> Result<(), RyuujiError> {
        self.conn
            .execute("DELETE FROM torrent_filter WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ── Torrent Archive ──────────────────────────────────────────

    /// Check if a torrent GUID is in the archive.
    pub fn is_torrent_archived(&self, guid: &str) -> Result<bool, RyuujiError> {
        let count: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM torrent_archive WHERE item_guid = ?1",
                params![guid],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(count > 0)
    }

    /// Add a torrent to the archive (prevents re-download).
    pub fn archive_torrent(
        &self,
        guid: &str,
        title: &str,
        action: &str,
    ) -> Result<(), RyuujiError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO torrent_archive (item_guid, title, action)
             VALUES (?1, ?2, ?3)",
            params![guid, title, action],
        )?;
        Ok(())
    }

    /// Clear the entire torrent archive.
    pub fn clear_torrent_archive(&self) -> Result<(), RyuujiError> {
        self.conn.execute("DELETE FROM torrent_archive", [])?;
        Ok(())
    }
}

// ── Migrations ──────────────────────────────────────────────────

/// Run schema migrations using `PRAGMA user_version` for version tracking.
fn run_migrations(conn: &Connection) -> Result<(), RyuujiError> {
    let version: i32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap_or(0);

    if version < 1 {
        // Detect if V1 was already applied (old code didn't set user_version).
        let table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='anime'",
                [],
                |row| row.get::<_, i32>(0),
            )
            .unwrap_or(0)
            > 0;
        if !table_exists {
            conn.execute_batch(SCHEMA_V1)?;
        }
        conn.pragma_update(None, "user_version", 1)?;
    }
    if version < 2 {
        conn.execute_batch(SCHEMA_V2)?;
        conn.pragma_update(None, "user_version", 2)?;
    }
    if version < 3 {
        conn.execute_batch(SCHEMA_V3)?;
        conn.pragma_update(None, "user_version", 3)?;
    }
    if version < 4 {
        conn.execute_batch(SCHEMA_V4)?;
        conn.pragma_update(None, "user_version", 4)?;
    }
    Ok(())
}

// ── Helpers ─────────────────────────────────────────────────────

/// Parse a datetime string from SQLite (either RFC 3339 or SQLite's `datetime('now')` format).
fn parse_datetime(s: &str) -> DateTime<Utc> {
    // Try RFC 3339 first (what we write via `.to_rfc3339()`).
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return dt.with_timezone(&Utc);
    }
    // SQLite's datetime('now') produces "YYYY-MM-DD HH:MM:SS".
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return naive.and_utc();
    }
    DateTime::default()
}

// ── Row mapping helpers ─────────────────────────────────────────

fn row_to_anime(row: &rusqlite::Row<'_>) -> Anime {
    row_to_anime_at(row, 0)
}

fn row_to_anime_at(row: &rusqlite::Row<'_>, off: usize) -> Anime {
    let synonyms_str: String = row.get(off + 7).unwrap_or_default();
    let synonyms: Vec<String> = serde_json::from_str(&synonyms_str).unwrap_or_default();

    let genres_str: String = row.get(off + 13).unwrap_or_default();
    let genres: Vec<String> = serde_json::from_str(&genres_str).unwrap_or_default();

    let studios_str: String = row.get(off + 17).unwrap_or_default();
    let studios: Vec<String> = serde_json::from_str(&studios_str).unwrap_or_default();

    Anime {
        id: row.get(off).unwrap_or(0),
        ids: AnimeIds {
            anilist: row
                .get::<_, Option<i64>>(off + 1)
                .unwrap_or(None)
                .map(|v| v as u64),
            kitsu: row
                .get::<_, Option<i64>>(off + 2)
                .unwrap_or(None)
                .map(|v| v as u64),
            mal: row
                .get::<_, Option<i64>>(off + 3)
                .unwrap_or(None)
                .map(|v| v as u64),
        },
        title: AnimeTitle {
            romaji: row.get(off + 4).unwrap_or(None),
            english: row.get(off + 5).unwrap_or(None),
            native: row.get(off + 6).unwrap_or(None),
        },
        synonyms,
        episodes: row.get(off + 8).unwrap_or(None),
        cover_url: row.get(off + 9).unwrap_or(None),
        season: row.get(off + 10).unwrap_or(None),
        year: row.get(off + 11).unwrap_or(None),
        synopsis: row.get(off + 12).unwrap_or(None),
        genres,
        media_type: row.get(off + 14).unwrap_or(None),
        airing_status: row.get(off + 15).unwrap_or(None),
        mean_score: row.get(off + 16).unwrap_or(None),
        studios,
        source: row.get(off + 18).unwrap_or(None),
        rating: row.get(off + 19).unwrap_or(None),
        start_date: row.get(off + 20).unwrap_or(None),
        end_date: row.get(off + 21).unwrap_or(None),
    }
}

fn row_to_library_entry(row: &rusqlite::Row<'_>, off: usize) -> LibraryEntry {
    let status_str: String = row.get(off + 2).unwrap_or_default();
    let updated_str: String = row.get(off + 5).unwrap_or_default();

    LibraryEntry {
        id: row.get(off).unwrap_or(0),
        anime_id: row.get(off + 1).unwrap_or(0),
        status: WatchStatus::from_db_str(&status_str).unwrap_or(WatchStatus::Watching),
        watched_episodes: row.get(off + 3).unwrap_or(0),
        score: row.get(off + 4).unwrap_or(None),
        updated_at: parse_datetime(&updated_str),
        start_date: row.get(off + 6).unwrap_or(None),
        finish_date: row.get(off + 7).unwrap_or(None),
        notes: row.get(off + 8).unwrap_or(None),
        rewatching: row.get::<_, i32>(off + 9).unwrap_or(0) != 0,
        rewatch_count: row.get(off + 10).unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_anime() -> Anime {
        Anime {
            id: 0,
            ids: AnimeIds {
                anilist: Some(154587),
                kitsu: None,
                mal: Some(52991),
            },
            title: AnimeTitle {
                romaji: Some("Sousou no Frieren".into()),
                english: Some("Frieren: Beyond Journey's End".into()),
                native: Some("葬送のフリーレン".into()),
            },
            synonyms: vec!["Frieren".into()],
            episodes: Some(28),
            cover_url: None,
            season: Some("Fall".into()),
            year: Some(2023),
            synopsis: None,
            genres: vec![],
            media_type: None,
            airing_status: None,
            mean_score: None,
            studios: vec![],
            source: None,
            rating: None,
            start_date: None,
            end_date: None,
        }
    }

    #[test]
    fn test_insert_and_get_anime() {
        let db = Storage::open_memory().unwrap();
        let anime = test_anime();
        let id = db.insert_anime(&anime).unwrap();
        assert!(id > 0);

        let fetched = db.get_anime(id).unwrap().unwrap();
        assert_eq!(fetched.title.romaji.as_deref(), Some("Sousou no Frieren"));
        assert_eq!(fetched.ids.anilist, Some(154587));
        assert_eq!(fetched.episodes, Some(28));
    }

    #[test]
    fn test_search_anime() {
        let db = Storage::open_memory().unwrap();
        db.insert_anime(&test_anime()).unwrap();

        let results = db.search_anime("Frieren").unwrap();
        assert_eq!(results.len(), 1);

        let results = db.search_anime("nonexistent").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_library_entry_crud() {
        let db = Storage::open_memory().unwrap();
        let anime_id = db.insert_anime(&test_anime()).unwrap();

        let entry = LibraryEntry {
            id: 0,
            anime_id,
            status: WatchStatus::Watching,
            watched_episodes: 5,
            score: None,
            updated_at: Utc::now(),
            start_date: None,
            finish_date: None,
            notes: None,
            rewatching: false,
            rewatch_count: 0,
        };
        db.upsert_library_entry(&entry).unwrap();

        let rows = db.get_library_by_status(WatchStatus::Watching).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].entry.watched_episodes, 5);
        assert_eq!(rows[0].anime.title.preferred(), "Sousou no Frieren");

        // Update episode count.
        db.update_episode_count(anime_id, 10).unwrap();
        let updated = db.get_library_entry_for_anime(anime_id).unwrap().unwrap();
        assert_eq!(updated.watched_episodes, 10);
    }

    #[test]
    fn test_watch_history() {
        let db = Storage::open_memory().unwrap();
        let anime_id = db.insert_anime(&test_anime()).unwrap();

        db.record_watch(anime_id, 1).unwrap();
        db.record_watch(anime_id, 2).unwrap();

        let history = db.recent_history(10).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].episode, 2); // most recent first
    }

    #[test]
    fn test_auth_tokens() {
        let db = Storage::open_memory().unwrap();

        db.save_token("anilist", "abc123", None, None).unwrap();
        let token = db.get_token("anilist").unwrap();
        assert_eq!(token.as_deref(), Some("abc123"));

        // Overwrite.
        db.save_token("anilist", "xyz789", Some("refresh_tok"), None)
            .unwrap();
        let token = db.get_token("anilist").unwrap();
        assert_eq!(token.as_deref(), Some("xyz789"));
    }
}
