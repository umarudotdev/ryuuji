# Kurozumi — Product Requirements Document

## Overview

Kurozumi is a cross-platform desktop application that automatically detects anime playback in media players, tracks watch progress locally, and synchronizes with external anime tracking services. It targets users who watch anime via local media files (fansubs, Blu-ray rips) and want their tracking services updated without manual entry.

## Problem

Anime viewers who watch local files must manually update their episode progress on services like MyAnimeList, AniList, or Kitsu. This is tedious and easy to forget, leading to stale lists. Existing tools are either Windows-only, abandoned, or lack robust filename parsing for the diverse fansub naming conventions.

## Users

- Anime viewers who watch locally downloaded files via mpv, VLC, MPC-HC, or PotPlayer
- Anime viewers who watch via streaming services (Crunchyroll, Netflix, etc.) in browsers
- Users with accounts on MyAnimeList, AniList, or Kitsu who want automatic progress sync
- Linux and Windows desktop users

## Architecture

Cargo workspace with five crates:

| Crate | Role |
|-------|------|
| `kurozumi-core` | Models, storage (SQLite), config (TOML), orchestrator, recognition cache (4-level fuzzy matcher) |
| `kurozumi-detect` | Platform-specific media player detection (MPRIS on Linux, Win32 on Windows) |
| `kurozumi-parse` | Anime filename tokenizer + multi-pass parser with compile-time keyword tables |
| `kurozumi-api` | Service clients behind `AnimeService` trait: MAL (OAuth2 PKCE), AniList (GraphQL), Kitsu (JSON:API) |
| `kurozumi-gui` | Iced 0.14 desktop UI — 7 screens (Now Playing, Library, History, Search, Seasons, Torrents, Settings); iced_aw widgets, Lucide icons, Geist font |

### Data flow

```
Detection tick (every N seconds)
  -> detect_players()          [kurozumi-detect, platform-specific]
  -> if browser:
       detect_stream()         [kurozumi-detect/stream, URL/title pattern matching]
       -> extracted title
     else:
       extract basename        [from file_path or media_title]
  -> parse(title)              [kurozumi-parse, tokenizer + parser]
  -> DetectedMedia             [kurozumi-core/models]
  -> process_detection()       [kurozumi-core/orchestrator]
     -> recognize()            [kurozumi-core/recognition, 4-level: query cache/exact/normalized/fuzzy]
     -> upsert library entry   [kurozumi-core/storage, SQLite]
     -> record watch history
     -> auto-push to service   [if primary service authenticated]
  -> UpdateOutcome             [displayed in GUI]
```

### Storage

SQLite with WAL mode. Four tables:

- **anime** — metadata with cross-service IDs (anilist_id, kitsu_id, mal_id), titles (romaji/english/native), synonyms, episode count, cover URL, season, year
- **library_entry** — per-anime watch status, episode progress, score, last updated
- **watch_history** — timestamped per-episode log
- **auth_tokens** — OAuth tokens per service (token, refresh, expires_at)

### Configuration

TOML config with built-in defaults (`config/default.toml`), overridden by user file at XDG/AppData paths:

- `general.detection_interval` — polling interval in seconds (default 5)
- `general.close_to_tray` — minimize to tray on close
- `library.auto_update` — auto-increment progress on detection
- `library.confirm_update` — prompt before updating
- `services.primary` — which service to sync with
- `services.{anilist,kitsu}.enabled` — toggle
- `services.mal.enabled` / `services.mal.client_id` — MAL requires user-registered client ID
- `discord.enabled` — Rich Presence toggle

## Features

### Implemented

**Detection & Parsing**
- Media player detection: mpv, VLC, MPC-HC/BE, PotPlayer
- Browser detection: Firefox, Chrome, Edge, Brave (with `is_browser` flag in player DB)
- Streaming service detection: Crunchyroll, Netflix, Jellyfin, Plex, Hidive, Bilibili (data-driven via `streams.toml`)
- Linux: MPRIS D-Bus queries (URL available from metadata); Windows: Win32 window enumeration (title-based fallback)
- Filename parser handles fansub conventions: `[Group] Title - 05 (1080p) [CRC32].mkv`
- Extracts: title, episode number, release group, resolution, codec, checksum
- Handles edge cases: version suffixes (v2), decimal episodes (12.5), ranges (01-03), season prefixes (S2)

**Local Tracking**
- Recognition cache with 4-level lookup: query cache (LRU, 64 entries) → exact index → normalized index → fuzzy fallback (Skim, 60% threshold)
- Auto-populates on first recognition call; invalidated when DB changes
- Automatic library addition on first detection
- Episode progress auto-increment (configurable)
- Watch history log with timestamps
- Manual +/- episode adjustment in GUI

**GUI**
- Iced 0.14 desktop app with sidebar navigation (7 screens), dark/light theme system
- Embedded default themes (dark/light TOML); user-provided themes from `~/.config/kurozumi/themes/`; system appearance auto-detection
- Geist Sans/Mono variable fonts; Lucide icon font; iced_aw widgets (cards, number inputs, context menus, wrap)
- Shared detail panel widget for anime info + episode controls
- Now Playing: detected title, episode, player name (with streaming service name for browsers), quality, status message
- Library: tabbed by status (Watching/Completed/On Hold/Dropped/Plan to Watch), 60/40 list+detail split, episode adjustment
- History: timestamped watch log with detail panel, filtering
- Search: local library search + online search via primary service, add-to-library flow
- Seasons: browse anime by season/year via AniList API, genre filter, sort (popularity/score/title), detail panel with add-to-library
- Torrents: RSS feed management, torrent filters, auto-check
- Settings: general, library, service configuration (MAL/AniList/Kitsu), appearance, torrent, library export

**Service Integration**
- MyAnimeList: OAuth2 PKCE authentication, anime search, user list import (cursor-based pagination), episode progress update, token refresh
- AniList: OAuth2 Authorization Code grant, GraphQL API, anime search, user list import, episode progress update, season browse (paginated)
- Kitsu: Resource Owner Password grant, JSON:API, anime search, user list import, episode progress update
- Multi-service support: configurable primary service, search delegates to primary, auto-push progress on detection
- Cover image cache: disk-backed with network fetch, negative IDs for online results

### Stubs / Planned

- Discord Rich Presence
- System tray integration
- Conflict resolution (currently last-write-wins)
- Desktop notifications
- HTTP webhook sharing

## Technology

| Concern | Choice |
|---------|--------|
| Language | Rust, edition 2021 |
| Async | Tokio |
| GUI | Iced 0.14, iced_aw 0.13 |
| HTTP | reqwest with rustls |
| Database | SQLite via rusqlite (bundled) |
| Config | TOML |
| Logging | tracing + tracing-subscriber |
| Matching | fuzzy-matcher (Skim) |
| Keywords | phf compile-time hash maps |
| Errors | thiserror |
| Icons | Lucide 0.563 with Iced integration |
| Font | Geist Sans/Mono (variable, embedded) |
| Theme detection | dark-light 2 |

Platform-specific: `mpris` (Linux D-Bus), `windows` (Win32 FFI).

## Testing

Unit tests in: orchestrator (workflow), storage (CRUD), recognition cache (4-level matching), config (roundtrip), parser (filename formats), tokenizer (edge cases), MAL/AniList/Kitsu types (JSON deserialization), player DB (matching, browser flag), stream detection (URL/title matching, stream extraction). All tests use in-memory SQLite for speed. 116 tests across the workspace.

## License

GPL-3.0
