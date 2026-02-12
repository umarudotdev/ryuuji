<p align="center">
  <a href="https://ryuuji.umaru.dev">
    <img src="website/static/logo-mark.svg" width="80" height="80" alt="Ryuuji">
  </a>
</p>

<h3 align="center">Ryuuji</h3>
<p align="center">Your anime list, on autopilot.<br>Just press play — Ryuuji keeps MyAnimeList, AniList, or Kitsu in sync for you.</p>

<p align="center">
  <a href="https://ryuuji.umaru.dev">Website</a> &nbsp;·&nbsp;
  <a href="https://github.com/umarudotdev/ryuuji/releases">Download</a> &nbsp;·&nbsp;
  <a href="https://ryuuji.umaru.dev/overview">Overview</a>
</p>

---

## How it works

Ryuuji runs a detection → recognition → sync pipeline in the background while you watch:

1. **Detect** — Finds your active media player (MPRIS/D-Bus on Linux, Win32 on Windows) or streaming service in a browser tab
2. **Parse** — Extracts the anime title and episode from the filename or page title, handling fansub conventions like `[SubGroup] Title - 05v2 (1080p) [CRC32].mkv`
3. **Match** — 4-level recognition cascade (exact → normalized → fuzzy at 60% threshold) identifies the anime against your local library
4. **Sync** — Updates your local SQLite database and pushes progress to your connected tracking service

No browser extensions, no manual input. Just press play.

## Features

- **20+ media players** — mpv, VLC, MPC-HC, MPC-BE, PotPlayer, Kodi, Celluloid, SMPlayer, Haruna, and more
- **Streaming services** — Crunchyroll, Netflix, Jellyfin, Plex, Hidive, Bilibili (detected via browser tab)
- **Multi-service sync** — MyAnimeList (OAuth2 PKCE), AniList (GraphQL), Kitsu (JSON:API)
- **Local-first** — SQLite with full watch history, works offline, services are optional
- **Data-driven** — Player and streaming definitions live in TOML config files, not code
- **Themeable** — Dark/light themes with custom TOML themes and system appearance detection
- **Cross-platform** — Linux and Windows

## Building

Requires Rust 2021 edition.

```
cargo build --release
cargo run --package ryuuji-gui --release
```

Platform dependencies:

| Platform | Requirement |
|----------|-------------|
| Linux | D-Bus dev libraries (`libdbus-1-dev` on Debian/Ubuntu, `dbus` on Arch) |
| Windows | None |

## Architecture

```
ryuuji-gui  →  ryuuji-core  →  ryuuji-detect
     ↓              ↓
ryuuji-api     ryuuji-parse
```

| Crate | Purpose |
|-------|---------|
| `ryuuji-core` | Domain models, SQLite storage, config, orchestrator, 4-level recognition cache |
| `ryuuji-detect` | Platform-specific player detection + streaming service detection |
| `ryuuji-parse` | Anime filename tokenizer and multi-pass parser with `phf` keyword tables |
| `ryuuji-api` | `AnimeService` trait + clients for MAL, AniList, Kitsu |
| `ryuuji-gui` | Iced 0.14 desktop app — 7 screens, actor-pattern DB, theme system |

## Testing

```
cargo test --workspace
```

~116 unit tests across the workspace, all using in-memory SQLite.

## Contributing

See [`CONTRIBUTING.md`](https://github.com/umarudotdev/ryuuji/blob/main/CONTRIBUTING.md) for guidelines. Architecture decisions are documented in [`docs/decisions/`](docs/decisions/).

## License

MIT
