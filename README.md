<p align="center">
  <a href="https://ryuuji.umaru.dev">
    <img src="assets/logo-icon.svg" width="80" height="80" alt="Ryuuji">
  </a>
</p>

<h3 align="center">Ryuuji</h3>
<p align="center">Your anime list, on autopilot.<br>Just press play — Ryuuji keeps MyAnimeList, AniList, or Kitsu in sync for you.</p>

<p align="center">
  <a href="https://github.com/umarudotdev/ryuuji/actions/workflows/ci.yml"><img src="https://github.com/umarudotdev/ryuuji/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/umarudotdev/ryuuji/releases/latest"><img src="https://img.shields.io/github/v/release/umarudotdev/ryuuji" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/umarudotdev/ryuuji" alt="License"></a>
</p>

<p align="center">
  <a href="https://ryuuji.umaru.dev">Website</a> &nbsp;·&nbsp;
  <a href="https://github.com/umarudotdev/ryuuji/releases/latest">Download</a> &nbsp;·&nbsp;
  <a href="https://ryuuji.umaru.dev/overview">Overview</a>
</p>

<!-- TODO: Add a screenshot of the app here -->
<!-- <p align="center"><img src="docs/screenshot.png" width="800" alt="Ryuuji screenshot"></p> -->

---

Ryuuji runs in the background while you watch. It detects your media player or streaming service, figures out what you're watching from the filename or page title, and automatically updates your tracking service. No browser extensions, no manual input.

## Install

Download the latest release from [GitHub Releases](https://github.com/umarudotdev/ryuuji/releases/latest):

| Platform | Options |
|----------|---------|
| **Linux** | [`.AppImage`](https://github.com/umarudotdev/ryuuji/releases/latest) (portable) &nbsp;·&nbsp; [`.deb`](https://github.com/umarudotdev/ryuuji/releases/latest) (Debian/Ubuntu) |
| **Windows** | [`.exe` installer](https://github.com/umarudotdev/ryuuji/releases/latest) &nbsp;·&nbsp; [portable `.zip`](https://github.com/umarudotdev/ryuuji/releases/latest) |

<details>
<summary><strong>Windows (Scoop)</strong></summary>

```
scoop bucket add ryuuji https://github.com/umarudotdev/ryuuji
scoop install ryuuji
```

</details>

<details>
<summary><strong>Build from source</strong></summary>

Requires Rust 2021 edition. Linux needs D-Bus dev libraries (`libdbus-1-dev` on Debian/Ubuntu, `dbus` on Arch).

```
cargo build --release
cargo run --package ryuuji-gui --release
```

</details>

## Features

- **20+ media players** — mpv, VLC, MPC-HC, MPC-BE, PotPlayer, Kodi, Celluloid, SMPlayer, Haruna, and more
- **Streaming services** — Crunchyroll, Netflix, Jellyfin, Plex, Hidive, Bilibili via browser tab detection
- **Multi-service sync** — MyAnimeList, AniList, and Kitsu
- **Local-first** — SQLite library with full watch history, works offline, services are optional
- **Themeable** — dark/light themes, custom TOML theme files, system appearance detection
- **Cross-platform** — Linux and Windows

## How it works

1. **Detect** — Finds your active media player (MPRIS on Linux, Win32 on Windows) or streaming service in a browser tab
2. **Parse** — Extracts anime title and episode from the filename, handling fansub conventions like `[Group] Title - 05v2 (1080p) [CRC32].mkv`
3. **Match** — 4-level recognition cascade (exact → normalized → fuzzy) identifies the anime in your library
4. **Sync** — Updates your local database and pushes progress to your connected service

## Development

```
ryuuji-gui  →  ryuuji-core  →  ryuuji-detect
     ↓              ↓
ryuuji-api     ryuuji-parse
```

| Crate | Purpose |
|-------|---------|
| `ryuuji-core` | Domain models, SQLite storage, config, recognition cache, orchestrator |
| `ryuuji-detect` | Platform-specific player detection + streaming service detection |
| `ryuuji-parse` | Anime filename tokenizer and multi-pass parser |
| `ryuuji-api` | `AnimeService` trait + MAL, AniList, Kitsu clients |
| `ryuuji-gui` | Iced 0.14 desktop app — 8 screens, actor-pattern DB, theme system |

```
cargo test --workspace    # all tests use in-memory SQLite
cargo clippy --workspace
cargo fmt --all
```

See [`CONTRIBUTING.md`](https://github.com/umarudotdev/ryuuji/blob/master/CONTRIBUTING.md) for guidelines and [`docs/decisions/`](docs/decisions/) for architecture decisions.

## License

MIT
