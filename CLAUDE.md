# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is Ryuuji?

Ryuuji is a desktop anime tracker that automatically detects media players (and streaming services in browsers), parses filenames, and maintains a local watch library with service sync (MyAnimeList, AniList, Kitsu). Built with Rust + Iced GUI.

## Build & Development Commands

```bash
cargo build --release                        # Build workspace
cargo run --package ryuuji-gui --release   # Run GUI app
cargo test --workspace                       # Run all tests (~167 unit tests, in-memory SQLite)
cargo test -p ryuuji-core                  # Test a single crate
cargo test -p ryuuji-core recognition      # Run tests matching a name
cargo fmt --all                              # Format
cargo clippy --workspace                     # Lint
```

Logging: `RUST_LOG=ryuuji_core=debug,ryuuji_api=trace cargo run --package ryuuji-gui`

## Workspace Crates & Dependency Flow

```
ryuuji-gui  →  ryuuji-core  →  ryuuji-detect  (platform media player detection)
     ↓                ↓
ryuuji-api     ryuuji-parse  (anime filename tokenizer + parser)
                     ↑
               ryuuji-wasm   (WASM wrapper for browser usage)
```

- **ryuuji-core** — Domain models (`Anime`, `LibraryEntry`, `WatchStatus`), SQLite storage, config, recognition cache with 4-level fuzzy matching, orchestrator
- **ryuuji-detect** — Platform-specific player detection (MPRIS D-Bus on Linux, Win32 on Windows) + streaming service detection for browsers via data-driven `streams.toml`
- **ryuuji-parse** — 6-pass filename parser with keyword tables (via `phf` compile-time hashes). Handles fansub naming conventions: `[Group] Title - 05 (1080p) [CRC32].mkv`
- **ryuuji-api** — `AnimeService` trait + clients for MAL (OAuth2 PKCE), AniList (GraphQL), Kitsu (JSON:API). Season browse API for AniList.
- **ryuuji-gui** — Iced 0.14 app with 8 screens (Now Playing, Library, History, Search, Seasons, Torrents, Stats, Settings), actor-pattern DB handle, theme system
- **ryuuji-wasm** — Thin WASM wrapper around `ryuuji-parse` exposing `parse_filename()` → JSON for browser usage

## Key Architecture Patterns

**Detection → Recognition Pipeline:** Every N seconds, `detect_players()` → if browser, `detect_stream()` to extract title from streaming service; else extract basename from file path → `parse(title)` → `process_detection()` (orchestrator) → 4-level title matching (LRU cache → exact → normalized → fuzzy via Skim at 60% threshold) → SQLite update. This is the core data flow. The orchestrator also checks the embedded anime-relations database (`anime-relations.txt`) for cross-season episode redirects (e.g., continuous episode numbering → per-season numbering).

**GUI Message Flow (unidirectional):** Screen `Message` → screen `update()` → `Action` enum → app `handle_action()` → state change. Screens never mutate app state directly. Key `Action` variants: `NavigateTo(Page)`, `RefreshLibrary`, `ShowModal(ModalKind)`, `RunTask(Task<app::Message>)`. Async work (DB ops, network) goes through `Action::RunTask` which hands an `iced::Task` to the runtime — screens themselves are synchronous.

**Actor-based DB Access** (`ryuuji-gui/src/db.rs`): `DbHandle` wraps a Tokio MPSC channel to a dedicated OS thread (not a tokio task) that owns `Storage`, `RecognitionCache`, and `RelationDatabase`. Uses `blocking_recv()`. Operations use oneshot channels for responses. Cache is invalidated after `AddedToLibrary` outcomes and bulk MAL imports.

**Recognition Cache** (`ryuuji-core/src/recognition.rs`): 4-level waterfall lookup: (1) LRU query cache (64 entries) → (2) exact HashMap → (3) normalized HashMap (lowercase/trim/ascii-fold) → (4) fuzzy via `SkimMatcherV2` at 60% threshold. Auto-populates indices from DB on first call; full invalidation clears all levels and forces repopulation.

**Subscriptions** (`ryuuji-gui/src/subscription.rs`): 3–5 Iced subscriptions composed via `subscription::subscriptions()`. Always active: (1) detection tick at configurable interval, (2) window resize/move events for geometry persistence, (3) global keyboard shortcuts. Conditional: (4) OS appearance polling every 5s (only when `ThemeMode::System`), (5) torrent refresh tick (only when torrents enabled).

**Keyboard Shortcuts** (`ryuuji-gui/src/keyboard.rs`): Global shortcuts via Iced subscription — F5 refresh, Ctrl+F search, Ctrl+C copy title, Ctrl+Up/Down episode increment/decrement, Ctrl+1–9/0 scoring, Escape dismiss. Shortcuts produce app-level `Message` variants.

**Screen Architecture** (`ryuuji-gui/src/screen/`): Each screen has its own state struct, message enum, and `update()` method. Screens return `Action` enums that the app router in `app.rs` dispatches.

**Modal Pattern:** App-level `modal_state: Option<ModalKind>` wraps the main view. Screens request modals via `Action::ShowModal(...)`, confirmation routes back to originating screen. Add new dialog types by extending `ModalKind` enum.

**Theme System** (`ryuuji-gui/src/theme/`): 3-tier theme discovery — embedded TOML themes (`assets/themes/`), user themes from `~/.config/ryuuji/themes/`, and OS system appearance. Config stores both `appearance.theme` (name) and `appearance.mode` (Dark/Light/System). `ColorScheme` has 30+ semantic tokens; `build_theme()` maps these to Iced's 6-color `Palette`.

**Design Tokens** (`ryuuji-gui/src/style.rs`): All spacing/typography/layout uses compile-time constants on a 4px grid (`SPACE_XXS` through `SPACE_3XL`). Layout constants: `NAV_RAIL_WIDTH`, `STATUS_BAR_HEIGHT`, `COVER_WIDTH`/`COVER_HEIGHT`. No magic numbers in view code.

**Player Database** (`ryuuji-detect/src/player_db.rs`): Player definitions are data-driven via embedded `data/players.toml`. Each player specifies `executables`, `mpris_identities`, `window_classes`, `title_patterns` (regex capture groups), and `is_browser` flag. Adding new player support is config, not code. User overrides via `merge_user()`.

**Stream Database** (`ryuuji-detect/src/stream.rs`): Streaming service definitions via embedded `data/streams.toml`. Each service specifies `url_patterns` (regex matched against MPRIS URL) and `title_pattern` (regex to extract anime title from window/tab title). Supports Crunchyroll, Netflix, Jellyfin, Plex, Hidive, Bilibili. User overrides via `merge_user()`.

## Conventions

- Error types: `<CrateName>Error` with `thiserror`. Central `RyuujiError` in `ryuuji-core/error.rs` wraps all subsystems
- Config: TOML at `~/.config/ryuuji/ryuuji.toml` with compile-time embedded defaults from `config/default.toml`
- Database: SQLite with WAL + foreign keys. Schema in `migrations/`. Auto-migrated on first run. All library queries return `LibraryRow` (pre-joined `Anime` + `LibraryEntry`) to avoid N+1 patterns
- Workspace deps: All shared dependencies declared in root `Cargo.toml` `[workspace.dependencies]`, crates use `dep = { workspace = true }`
- Icons: Lucide icon font (not Unicode symbols)
- UI font: Geist Sans/Mono (variable, embedded as bytes via `include_bytes!`)
- Platform dispatch: `ryuuji-detect/src/platform/mod.rs` uses `#[cfg(target_os)]` to select Linux/Windows impl
- Testing: All tests use `Storage::open_memory()` for in-memory SQLite. No integration test directory — all tests are unit tests within module files

## Service Integration Details

**MAL:** OAuth2 PKCE with plain method; localhost redirect listener on port 19742. User must register their own MAL client ID (stored in config as `services.mal.client_id`). Cursor-based pagination for list import. Token refresh is automatic.

**AniList:** OAuth2 Authorization Code grant; localhost redirect listener. Raw `reqwest` POST to `https://graphql.anilist.co` with Bearer token. Supports search, user list import, progress update, and season browse (paginated). Tokens don't expire.

**Kitsu:** Resource Owner Password Credentials grant (username + password → token). JSON:API responses with `{ data, included }` structure. Supports search, user list import, progress update.

**Auto-sync:** After detection updates episode progress, if a primary service is authenticated, the app auto-pushes the progress update to that service.

## Stubs / Not Yet Implemented

System tray integration, conflict resolution, notifications, HTTP webhook sharing. Discord Rich Presence has a module (`ryuuji-gui/src/discord.rs`) but is not yet fully integrated.

## Docs

- `docs/prd.md` — Full product requirements, feature status, architecture overview
- `docs/decisions/` — ADRs:
  - `0001` — MAL uses manual OAuth2 PKCE (no oauth2 crate)
  - `0002` — MAL config uses owned `client_id` String
  - `0003` — MAL API quirks require form-encoded PATCH
