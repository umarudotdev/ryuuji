# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is Kurozumi?

Kurozumi is a desktop anime tracker that automatically detects media players, parses filenames, and maintains a local watch library with optional service sync (MyAnimeList). Built with Rust + Iced GUI.

## Build & Development Commands

```bash
cargo build --release                        # Build workspace
cargo run --package kurozumi-gui --release   # Run GUI app
cargo test --workspace                       # Run all tests (~30 unit tests, in-memory SQLite)
cargo test -p kurozumi-core                  # Test a single crate
cargo test -p kurozumi-core recognition      # Run tests matching a name
cargo fmt --all                              # Format
cargo clippy --workspace                     # Lint
```

Logging: `RUST_LOG=kurozumi_core=debug,kurozumi_api=trace cargo run --package kurozumi-gui`

## Workspace Crates & Dependency Flow

```
kurozumi-gui  →  kurozumi-core  →  kurozumi-detect  (platform media player detection)
     ↓                ↓
kurozumi-api     kurozumi-parse  (anime filename tokenizer + parser)
```

- **kurozumi-core** — Domain models (`Anime`, `LibraryEntry`, `WatchStatus`), SQLite storage, config, recognition cache with 3-pass fuzzy matching, orchestrator
- **kurozumi-detect** — Platform-specific player detection: MPRIS D-Bus on Linux, Win32 window enumeration on Windows (`src/platform/`)
- **kurozumi-parse** — 6-pass filename parser with keyword tables (via `phf` compile-time hashes). Handles fansub naming conventions: `[Group] Title - 05 (1080p) [CRC32].mkv`
- **kurozumi-api** — `AnimeService` trait + MAL OAuth2 PKCE client. AniList/Kitsu are stubs
- **kurozumi-gui** — Iced 0.14 app with 4 screens (Now Playing, Library, Search, Settings), actor-pattern DB handle, theme system

## Key Architecture Patterns

**Detection → Recognition Pipeline:** Every N seconds, `detect_players()` → `parse(filename)` → `process_detection()` (orchestrator) → 3-pass title matching (exact → normalized → fuzzy via Skim at 60% threshold) → SQLite update. This is the core data flow.

**Actor-based DB Access** (`kurozumi-gui/src/db.rs`): `DbHandle` wraps a Tokio MPSC channel to run all SQLite ops on a dedicated thread, keeping I/O off the Iced render thread.

**Recognition Cache** (`kurozumi-core/src/recognition.rs`): In-memory index with 4-level lookup (query cache → exact → normalized → fuzzy). Bounded LRU cache (64 entries). Auto-populates on first call; invalidate when DB changes.

**Screen Architecture** (`kurozumi-gui/src/screen/`): Each screen has its own state struct, message enum, and `update()` method. Screens return `Action` enums that the app router in `app.rs` dispatches.

**Theme System** (`kurozumi-gui/src/theme/`): Embedded dark/light TOML themes in `assets/themes/`. User themes from `~/.config/kurozumi/themes/`. System appearance auto-detection polls every 5 seconds.

## Conventions

- Error types: `<CrateName>Error` with `thiserror`. Central `KurozumiError` in `kurozumi-core/error.rs` wraps all subsystems
- Config: TOML at `~/.config/kurozumi/kurozumi.toml` with compile-time embedded defaults from `config/default.toml`
- Database: SQLite with WAL + foreign keys. Schema in `migrations/001_initial.sql`, auto-migrated on first run
- Icons: Lucide icon font (not Unicode symbols)
- UI font: Geist Sans/Mono (variable, embedded)
- Platform dispatch: `kurozumi-detect/src/platform/mod.rs` uses `#[cfg(target_os)]` to select Linux/Windows impl

## MAL Integration Details

OAuth2 PKCE with plain method; localhost redirect listener on port 19742. User must register their own MAL client ID (stored in config as `services.mal.client_id`). Cursor-based pagination for list import. Token refresh is automatic.

## Stubs / Not Yet Implemented

AniList GraphQL client, Kitsu JSON:API client, Discord Rich Presence, system tray integration, service sync orchestration (push/pull), conflict resolution, cover image display, notifications. Dependencies for AniList (`graphql_client`) are already in Cargo.toml.

## Docs

- `docs/prd.md` — Full product requirements, feature status, architecture overview
- `docs/decisions/` — ADRs:
  - `0001` — MAL uses manual OAuth2 PKCE (no oauth2 crate)
  - `0002` — MAL config uses owned `client_id` String
  - `0003` — MAL API quirks require form-encoded PATCH
