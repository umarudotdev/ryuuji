# Kurozumi — Product Requirements Document

## Overview

Kurozumi is a cross-platform desktop application that automatically detects anime playback in media players, tracks watch progress locally, and synchronizes with external anime tracking services. It targets users who watch anime via local media files (fansubs, Blu-ray rips) and want their tracking services updated without manual entry.

## Problem

Anime viewers who watch local files must manually update their episode progress on services like MyAnimeList, AniList, or Kitsu. This is tedious and easy to forget, leading to stale lists. Existing tools are either Windows-only, abandoned, or lack robust filename parsing for the diverse fansub naming conventions.

## Users

- Anime viewers who watch locally downloaded files via mpv, VLC, MPC-HC, or PotPlayer
- Users with accounts on MyAnimeList, AniList, or Kitsu who want automatic progress sync
- Linux and Windows desktop users

## Architecture

Cargo workspace with five crates:

| Crate | Role |
|-------|------|
| `kurozumi-core` | Models, storage (SQLite), config (TOML), orchestrator, recognition cache (4-level fuzzy matcher) |
| `kurozumi-detect` | Platform-specific media player detection (MPRIS on Linux, Win32 on Windows) |
| `kurozumi-parse` | Anime filename tokenizer + multi-pass parser with compile-time keyword tables |
| `kurozumi-api` | Service clients behind `AnimeService` trait (MAL implemented, AniList/Kitsu stubs) |
| `kurozumi-gui` | Iced 0.14 desktop UI — Now Playing, Library, Search, Settings pages; iced_aw widgets, Lucide icons, Geist font |

### Data flow

```
Detection tick (every N seconds)
  -> detect_players()          [kurozumi-detect, platform-specific]
  -> parse(filename)           [kurozumi-parse, tokenizer + parser]
  -> DetectedMedia             [kurozumi-core/models]
  -> process_detection()       [kurozumi-core/orchestrator]
     -> recognize()            [kurozumi-core/recognition, 4-level: query cache/exact/normalized/fuzzy]
     -> upsert library entry   [kurozumi-core/storage, SQLite]
     -> record watch history
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
- Linux: MPRIS D-Bus queries; Windows: Win32 window enumeration
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
- Iced 0.14 desktop app with sidebar navigation, dark/light theme system
- Embedded default themes (dark/light TOML); user-provided themes from `~/.config/kurozumi/themes/`; system appearance auto-detection
- Geist Sans/Mono variable fonts; Lucide icon font; iced_aw widgets (cards, number inputs, context menus)
- Shared detail panel widget for anime info + episode controls
- Now Playing page: detected title, episode, player, quality, status message
- Library page: tabbed by status (Watching/Completed/On Hold/Dropped/Plan to Watch), 60/40 list+detail split, episode adjustment buttons
- Search page: MAL anime search with results list and detail view
- Settings page: general, library, MAL service, and appearance configuration

**MyAnimeList Integration**
- OAuth2 PKCE authentication (plain method, localhost redirect listener on port 19742)
- Anime search via MAL API v2
- User list import with cursor-based pagination
- Episode progress update (form-encoded PATCH)
- Token refresh flow

### Stubs / Planned

- AniList GraphQL client (graphql_client dependency present)
- Kitsu JSON:API client
- Discord Rich Presence
- System tray integration
- Service sync orchestration (push local changes to remote, pull remote changes)
- Conflict resolution
- Cover image display
- Notifications

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

Unit tests in: orchestrator (workflow), storage (CRUD), recognition cache (4-level matching), config (roundtrip), parser (filename formats), tokenizer (edge cases), MAL types (JSON deserialization). All tests use in-memory SQLite for speed. 37 tests across the workspace.

## License

GPL-3.0
