CREATE TABLE IF NOT EXISTS available_episode (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    anime_id INTEGER NOT NULL REFERENCES anime(id) ON DELETE CASCADE,
    episode INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    file_modified TEXT NOT NULL,
    release_group TEXT,
    resolution TEXT,
    indexed_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(anime_id, episode, file_path)
);
