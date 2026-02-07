-- kurozumi initial schema

CREATE TABLE IF NOT EXISTS anime (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    anilist_id  INTEGER,
    kitsu_id    INTEGER,
    mal_id      INTEGER,
    title_romaji   TEXT,
    title_english  TEXT,
    title_native   TEXT,
    synonyms       TEXT,  -- JSON array
    episodes       INTEGER,
    cover_url      TEXT,
    season         TEXT,
    year           INTEGER,
    created_at     TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at     TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS library_entry (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    anime_id         INTEGER NOT NULL UNIQUE REFERENCES anime(id),
    status           TEXT NOT NULL DEFAULT 'watching',
    watched_episodes INTEGER NOT NULL DEFAULT 0,
    score            REAL,
    updated_at       TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS watch_history (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    anime_id   INTEGER NOT NULL REFERENCES anime(id),
    episode    INTEGER NOT NULL,
    watched_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS auth_tokens (
    service    TEXT PRIMARY KEY,
    token      TEXT NOT NULL,
    refresh    TEXT,
    expires_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_anime_anilist ON anime(anilist_id);
CREATE INDEX IF NOT EXISTS idx_anime_mal ON anime(mal_id);
CREATE INDEX IF NOT EXISTS idx_library_anime ON library_entry(anime_id);
CREATE INDEX IF NOT EXISTS idx_history_anime ON watch_history(anime_id);
