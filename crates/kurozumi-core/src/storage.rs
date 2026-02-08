use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::KurozumiError;
use crate::models::{Anime, AnimeIds, AnimeTitle, LibraryEntry, WatchStatus};

const SCHEMA: &str = include_str!("../../../migrations/001_initial.sql");

/// Token record: (access_token, refresh_token, expires_at).
pub type TokenRecord = (String, Option<String>, Option<String>);

/// SQLite-backed storage for the kurozumi library.
pub struct Storage {
    conn: Connection,
}

/// A library entry joined with its anime data for display.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LibraryRow {
    pub entry: LibraryEntry,
    pub anime: Anime,
}

/// A watch history record.
#[derive(Debug, Clone)]
pub struct WatchHistoryRow {
    pub anime_id: i64,
    pub episode: u32,
    pub watched_at: DateTime<Utc>,
}

impl Storage {
    /// Open (or create) the database at the given path and run migrations.
    pub fn open(path: &Path) -> Result<Self, KurozumiError> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    /// Open an in-memory database (for tests).
    pub fn open_memory() -> Result<Self, KurozumiError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    // ── Anime CRUD ──────────────────────────────────────────────

    /// Insert a new anime, returning its auto-generated ID.
    pub fn insert_anime(&self, anime: &Anime) -> Result<i64, KurozumiError> {
        let synonyms_json = serde_json::to_string(&anime.synonyms).unwrap_or_default();
        self.conn.execute(
            "INSERT INTO anime (anilist_id, kitsu_id, mal_id, title_romaji, title_english,
             title_native, synonyms, episodes, cover_url, season, year)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
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
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Get an anime by its local database ID.
    pub fn get_anime(&self, id: i64) -> Result<Option<Anime>, KurozumiError> {
        self.conn
            .query_row(
                "SELECT id, anilist_id, kitsu_id, mal_id, title_romaji, title_english,
                 title_native, synonyms, episodes, cover_url, season, year
                 FROM anime WHERE id = ?1",
                params![id],
                |row| Ok(row_to_anime(row)),
            )
            .optional()
            .map_err(Into::into)
    }

    /// Search anime by title substring (case-insensitive).
    pub fn search_anime(&self, query: &str) -> Result<Vec<Anime>, KurozumiError> {
        let pattern = format!("%{query}%");
        let mut stmt = self.conn.prepare(
            "SELECT id, anilist_id, kitsu_id, mal_id, title_romaji, title_english,
             title_native, synonyms, episodes, cover_url, season, year
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
    pub fn all_anime(&self) -> Result<Vec<Anime>, KurozumiError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, anilist_id, kitsu_id, mal_id, title_romaji, title_english,
             title_native, synonyms, episodes, cover_url, season, year
             FROM anime ORDER BY title_romaji",
        )?;
        let rows = stmt
            .query_map([], |row| Ok(row_to_anime(row)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// Get an anime by its MAL ID.
    pub fn get_anime_by_mal_id(&self, mal_id: u64) -> Result<Option<Anime>, KurozumiError> {
        self.conn
            .query_row(
                "SELECT id, anilist_id, kitsu_id, mal_id, title_romaji, title_english,
                 title_native, synonyms, episodes, cover_url, season, year
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
    pub fn upsert_anime_by_mal_id(&self, anime: &Anime) -> Result<i64, KurozumiError> {
        let mal_id = anime
            .ids
            .mal
            .expect("upsert_anime_by_mal_id requires a MAL ID");

        if let Some(existing) = self.get_anime_by_mal_id(mal_id)? {
            let synonyms_json = serde_json::to_string(&anime.synonyms).unwrap_or_default();
            self.conn.execute(
                "UPDATE anime SET
                    title_romaji = ?1, title_english = ?2, title_native = ?3,
                    synonyms = ?4, episodes = ?5, cover_url = ?6
                 WHERE id = ?7",
                params![
                    anime.title.romaji,
                    anime.title.english,
                    anime.title.native,
                    synonyms_json,
                    anime.episodes,
                    anime.cover_url,
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
    pub fn upsert_library_entry(&self, entry: &LibraryEntry) -> Result<i64, KurozumiError> {
        self.conn.execute(
            "INSERT INTO library_entry (anime_id, status, watched_episodes, score, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(anime_id) DO UPDATE SET
               status = excluded.status,
               watched_episodes = excluded.watched_episodes,
               score = excluded.score,
               updated_at = excluded.updated_at",
            params![
                entry.anime_id,
                entry.status.as_db_str(),
                entry.watched_episodes,
                entry.score,
                entry.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Get all library entries for a given watch status, joined with anime data.
    pub fn get_library_by_status(
        &self,
        status: WatchStatus,
    ) -> Result<Vec<LibraryRow>, KurozumiError> {
        let mut stmt = self.conn.prepare(
            "SELECT le.id, le.anime_id, le.status, le.watched_episodes, le.score, le.updated_at,
                    a.id, a.anilist_id, a.kitsu_id, a.mal_id, a.title_romaji, a.title_english,
                    a.title_native, a.synonyms, a.episodes, a.cover_url, a.season, a.year
             FROM library_entry le
             JOIN anime a ON le.anime_id = a.id
             WHERE le.status = ?1
             ORDER BY a.title_romaji",
        )?;
        let rows = stmt
            .query_map(params![status.as_db_str()], |row| {
                Ok(LibraryRow {
                    entry: row_to_library_entry(row, 0),
                    anime: row_to_anime_at(row, 6),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// Get all library entries joined with anime data.
    pub fn get_all_library(&self) -> Result<Vec<LibraryRow>, KurozumiError> {
        let mut stmt = self.conn.prepare(
            "SELECT le.id, le.anime_id, le.status, le.watched_episodes, le.score, le.updated_at,
                    a.id, a.anilist_id, a.kitsu_id, a.mal_id, a.title_romaji, a.title_english,
                    a.title_native, a.synonyms, a.episodes, a.cover_url, a.season, a.year
             FROM library_entry le
             JOIN anime a ON le.anime_id = a.id
             ORDER BY le.updated_at DESC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(LibraryRow {
                    entry: row_to_library_entry(row, 0),
                    anime: row_to_anime_at(row, 6),
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
    ) -> Result<Option<LibraryEntry>, KurozumiError> {
        self.conn
            .query_row(
                "SELECT id, anime_id, status, watched_episodes, score, updated_at
                 FROM library_entry WHERE anime_id = ?1",
                params![anime_id],
                |row| Ok(row_to_library_entry(row, 0)),
            )
            .optional()
            .map_err(Into::into)
    }

    /// Update just the episode count for a library entry.
    pub fn update_episode_count(&self, anime_id: i64, episodes: u32) -> Result<(), KurozumiError> {
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
    ) -> Result<(), KurozumiError> {
        self.conn.execute(
            "UPDATE library_entry SET status = ?1, updated_at = ?2
             WHERE anime_id = ?3",
            params![status.as_db_str(), Utc::now().to_rfc3339(), anime_id],
        )?;
        Ok(())
    }

    /// Update just the score for a library entry.
    pub fn update_library_score(&self, anime_id: i64, score: f32) -> Result<(), KurozumiError> {
        self.conn.execute(
            "UPDATE library_entry SET score = ?1, updated_at = ?2
             WHERE anime_id = ?3",
            params![score, Utc::now().to_rfc3339(), anime_id],
        )?;
        Ok(())
    }

    /// Delete a library entry by anime ID.
    pub fn delete_library_entry(&self, anime_id: i64) -> Result<(), KurozumiError> {
        self.conn.execute(
            "DELETE FROM library_entry WHERE anime_id = ?1",
            params![anime_id],
        )?;
        Ok(())
    }

    // ── Watch History ───────────────────────────────────────────

    /// Record an episode watch.
    pub fn record_watch(&self, anime_id: i64, episode: u32) -> Result<(), KurozumiError> {
        self.conn.execute(
            "INSERT INTO watch_history (anime_id, episode) VALUES (?1, ?2)",
            params![anime_id, episode],
        )?;
        Ok(())
    }

    /// Get recent watch history.
    pub fn recent_history(&self, limit: u32) -> Result<Vec<WatchHistoryRow>, KurozumiError> {
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

    // ── Auth Tokens ─────────────────────────────────────────────

    /// Store an auth token for a service.
    pub fn save_token(
        &self,
        service: &str,
        token: &str,
        refresh: Option<&str>,
        expires_at: Option<&str>,
    ) -> Result<(), KurozumiError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO auth_tokens (service, token, refresh, expires_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![service, token, refresh, expires_at],
        )?;
        Ok(())
    }

    /// Get the token for a service.
    pub fn get_token(&self, service: &str) -> Result<Option<String>, KurozumiError> {
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
    pub fn get_token_full(&self, service: &str) -> Result<Option<TokenRecord>, KurozumiError> {
        self.conn
            .query_row(
                "SELECT token, refresh, expires_at FROM auth_tokens WHERE service = ?1",
                params![service],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(Into::into)
    }
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
