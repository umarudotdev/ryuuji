# Ryuuji

Desktop anime tracker that works while you watch.

Ryuuji automatically detects your media player or streaming service, parses what you're watching, and keeps your anime list in sync — no manual updates needed.

## Features

- **Auto-detection** — 20+ media players on Linux and Windows, plus streaming services (Crunchyroll, Netflix, Jellyfin, Plex, Hidive, Bilibili) in browsers
- **Fuzzy recognition** — 4-level matching pipeline (exact → normalized → fuzzy) handles fansub naming conventions like `[SubGroup] Title - 05v2 (1080p) [ABCD1234].mkv`
- **Multi-service sync** — Push progress to MyAnimeList, AniList, or Kitsu automatically after detection
- **Local-first** — SQLite database with full watch history, works offline, services are optional
- **7-screen GUI** — Now Playing, Library, History, Search, Season Charts, Torrents, Settings
- **Themeable** — Dark/light themes with custom TOML theme support and system appearance detection

## Building

```
cargo build --release
cargo run --package ryuuji-gui --release
```

### Tauri App (Parallel)

Ryuuji now includes a parallel Tauri app scaffold with a shared runtime crate.

```
cargo run --package ryuuji-tauri
```

Tauri frontend source lives in `apps/ryuuji-tauri/ui` (Leptos CSR). During migration, the desktop app loads static assets from `apps/ryuuji-tauri/dist`.

Requires Rust 2021 edition. Platform-specific dependencies:

- **Linux**: D-Bus development libraries (`libdbus-1-dev` on Debian/Ubuntu, `dbus` on Arch)
- **Windows**: No extra dependencies

## Usage

1. Launch Ryuuji
2. (Optional) Connect a tracking service in Settings → Services
3. Start watching anime in any supported player or streaming service
4. Ryuuji detects playback, recognizes the anime, and updates your progress

Detection runs every few seconds (configurable). Config lives at `~/.config/ryuuji/ryuuji.toml`.

## Architecture

```
ryuuji-gui  →  ryuuji-core  →  ryuuji-detect
     ↓              ↓
ryuuji-api     ryuuji-parse
```

| Crate | Role |
|-------|------|
| `ryuuji-core` | Models, SQLite storage, config, orchestrator, 4-level recognition cache |
| `ryuuji-detect` | Platform-specific player detection (MPRIS on Linux, Win32 on Windows) |
| `ryuuji-parse` | Anime filename tokenizer + multi-pass parser with compile-time keyword tables |
| `ryuuji-api` | `AnimeService` trait + clients for MAL, AniList, Kitsu |
| `ryuuji-gui` | Iced 0.14 desktop app with theming, Lucide icons, Geist fonts |
| `ryuuji-runtime` | Shared app runtime (DB actor, detection tick, sync orchestration) for desktop frontends |
| `ryuuji-tauri` | Tauri v2 backend app using `ryuuji-runtime` with webview frontend integration |

### Supported players

mpv, VLC, MPC-HC, MPC-BE, PotPlayer, Kodi, Celluloid, SMPlayer, Haruna, KMPlayer, GOM Player, QMPlay2, and more. Player definitions are data-driven via `players.toml` — add new players through config, not code.

### Supported streaming services

Crunchyroll, Netflix, Jellyfin, Plex, Hidive, Bilibili (detected via browser tab URL/title patterns in `streams.toml`).

### Supported tracking services

MyAnimeList (OAuth2 PKCE), AniList (GraphQL + OAuth2), Kitsu (JSON:API + password grant).

## Testing

```
cargo test --workspace
```

~116 unit tests across the workspace, all using in-memory SQLite.

## License

GPL-3.0
