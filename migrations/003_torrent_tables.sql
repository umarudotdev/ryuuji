-- Torrent system tables

CREATE TABLE IF NOT EXISTS torrent_feed (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    url TEXT NOT NULL UNIQUE,
    enabled INTEGER NOT NULL DEFAULT 1,
    last_checked TEXT
);

CREATE TABLE IF NOT EXISTS torrent_filter (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    priority INTEGER NOT NULL DEFAULT 0,
    match_mode TEXT NOT NULL DEFAULT 'all',
    action TEXT NOT NULL DEFAULT 'discard',
    conditions TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE IF NOT EXISTS torrent_archive (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item_guid TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    action TEXT NOT NULL DEFAULT 'downloaded',
    archived_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_torrent_archive_guid ON torrent_archive(item_guid);

-- Default sources
INSERT OR IGNORE INTO torrent_feed (name, url) VALUES
    ('Nyaa.si (All)', 'https://nyaa.si/?page=rss&c=1_2'),
    ('Nyaa.si (Trusted)', 'https://nyaa.si/?page=rss&c=1_2&f=2');
