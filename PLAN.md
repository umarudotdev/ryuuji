# Taiga Feature Parity — Implementation Plan

## Scope

6 feature areas, organized into 4 phases. Each phase is independently shippable.

---

## Phase 1: AniList + Kitsu Service Clients ✅

**Goal:** Complete the `AnimeService` trait implementations for AniList and Kitsu, enabling search, import, and progress sync on all three services.

### 1A. AniList GraphQL Client (`ryuuji-api/src/anilist/`)

**Files to create/modify:**
- `ryuuji-api/src/anilist/client.rs` — `AniListClient` implementing `AnimeService`
- `ryuuji-api/src/anilist/auth.rs` — OAuth2 Authorization Code flow (AniList uses code grant with PKCE)
- `ryuuji-api/src/anilist/types.rs` — GraphQL response types
- `ryuuji-api/src/anilist/queries.rs` — GraphQL query strings as `const &str`
- `ryuuji-api/src/anilist/error.rs` — `AniListError` enum
- `ryuuji-api/src/anilist/mod.rs` — re-exports

**Auth flow:**
- AniList OAuth2 Authorization Code grant
- Redirect to `https://anilist.co/api/v2/oauth/authorize?client_id=X&redirect_uri=http://localhost:19742&response_type=code`
- Exchange code at `https://anilist.co/api/v2/oauth/token`
- Reuse localhost listener pattern from MAL (port 19742 or pick another)
- Store tokens in `auth_tokens` table with service = "anilist"

**GraphQL queries needed:**
1. **Search:** `query ($search: String) { Page { media(search: $search, type: ANIME) { id title { romaji english native } episodes coverImage { large } meanScore season seasonYear genres studios { nodes { name } } format status description } } }`
2. **User list:** `query ($userId: Int) { MediaListCollection(userId: $userId, type: ANIME) { lists { entries { mediaId progress score(format: POINT_10_DECIMAL) status media { ... } } } } }`
3. **Update progress:** `mutation ($mediaId: Int, $progress: Int) { SaveMediaListEntry(mediaId: $mediaId, progress: $progress) { id progress } }`
4. **Season browse:** `query ($season: MediaSeason, $seasonYear: Int, $page: Int) { Page(page: $page, perPage: 50) { pageInfo { hasNextPage } media(season: $season, seasonYear: $seasonYear, type: ANIME, sort: POPULARITY_DESC) { ... } } }`
5. **Viewer ID:** `query { Viewer { id name } }` (needed to fetch user list)

**Implementation notes:**
- Use `graphql_client` crate (already in Cargo.toml) OR raw `reqwest` POST with query strings (simpler, avoids codegen). Recommend raw POST since queries are small and static.
- All requests POST to `https://graphql.anilist.co` with `Authorization: Bearer {token}` header
- Rate limit: 90 requests/minute — add simple delay if needed
- Map AniList `MediaFormat` (TV, OVA, MOVIE, etc.) to existing `media_type` field
- Map AniList `MediaStatus` to `airing_status`

### 1B. Kitsu JSON:API Client (`ryuuji-api/src/kitsu/`)

**Files to create/modify:**
- `ryuuji-api/src/kitsu/client.rs` — `KitsuClient` implementing `AnimeService`
- `ryuuji-api/src/kitsu/auth.rs` — OAuth2 Resource Owner Password Grant (Kitsu uses username/password, no browser redirect)
- `ryuuji-api/src/kitsu/types.rs` — JSON:API response types
- `ryuuji-api/src/kitsu/error.rs` — `KitsuError` enum
- `ryuuji-api/src/kitsu/mod.rs` — re-exports

**Auth flow:**
- Kitsu uses Resource Owner Password Credentials grant (username + password → token)
- POST `https://kitsu.app/api/oauth/token` with `grant_type=password&username=X&password=Y`
- No browser redirect needed — direct credential exchange
- Store tokens in `auth_tokens` table with service = "kitsu"

**API calls:**
1. **Search:** `GET https://kitsu.app/api/edge/anime?filter[text]=query&page[limit]=10&include=genres,studios`
2. **User list:** `GET https://kitsu.app/api/edge/users/{id}/library-entries?filter[kind]=anime&include=anime&page[limit]=20` (paginated)
3. **Update progress:** `PATCH https://kitsu.app/api/edge/library-entries/{id}` with JSON:API body `{ data: { attributes: { progress: N } } }`
4. **User self:** `GET https://kitsu.app/api/edge/users?filter[self]=true` (get user ID from token)

**Implementation notes:**
- JSON:API responses have `{ data: [], included: [] }` structure — `included` has related resources
- Pagination via `links.next` URL
- Content-Type must be `application/vnd.api+json`

### 1C. GUI Integration for Multi-Service

**Files to modify:**
- `ryuuji-gui/src/screen/settings.rs` — Add AniList and Kitsu settings sections (mirroring MAL pattern)
- `ryuuji-gui/src/screen/search.rs` — Search delegates to `config.services.primary` service
- `ryuuji-gui/src/app.rs` — Add `spawn_anilist_*` and `spawn_kitsu_*` task methods, or generalize with service dispatch
- `ryuuji-gui/src/db.rs` — Generalize token storage: `save_service_token(service, ...)` / `get_service_token(service, ...)`
- `ryuuji-core/src/config.rs` — Add `AniListConfig { enabled, client_id }` (AniList needs client_id + client_secret registered at https://anilist.co/settings/developer)

**Settings UI additions:**
- AniList section: Client ID input, Login button (opens browser), Import button, status text
- Kitsu section: Username + Password inputs, Login button (direct auth), Import button, status text

**Search delegation:**
- Instead of hardcoded MAL search, dispatch based on `config.services.primary`:
  - `"anilist"` → AniListClient search
  - `"kitsu"` → KitsuClient search
  - `"mal"` → MalClient search (current)

### 1D. Bidirectional Sync Foundation

**New files:**
- `ryuuji-core/src/sync.rs` — `SyncEngine` with push/pull logic
- `ryuuji-gui/src/screen/settings.rs` — "Sync Now" button per service

**Sync strategy:**
- **Pull:** Import from service → upsert local (existing pattern, extended to all services)
- **Push:** After local episode update → call `service.update_progress(service_id, episode)` if service is enabled
- **Auto-push on detection:** When orchestrator updates episode count AND config has a primary service enabled, queue a push task
- **Conflict resolution:** Last-write-wins by `updated_at` timestamp (simplest, same as Taiga)

**Integration point in `app.rs`:**
```
Message::DetectionProcessed(Ok(UpdateOutcome::Updated { anime_id, episode, .. })) => {
    // Existing: update UI
    // NEW: if auto_sync enabled, spawn push task for primary service
    let push_task = self.spawn_sync_push(anime_id, episode);
    Task::batch([existing_task, push_task])
}
```

---

## Phase 2: Streaming Service Detection ✅

**Goal:** Detect anime from web browsers watching streaming services (Crunchyroll, Netflix, Jellyfin, Plex, etc.)

### 2A. Stream Provider Database (`ryuuji-detect/data/streams.toml`)

**New file** — data-driven stream definitions (same pattern as `players.toml`):
```toml
[[stream]]
name = "Crunchyroll"
domains = ["crunchyroll.com", "beta.crunchyroll.com"]
url_pattern = "crunchyroll\\.com/(?:watch|[a-z]{2}/watch)/([A-Z0-9]+)"
title_pattern = "^(.+?)\\s*-\\s*(?:Watch on Crunchyroll|Crunchyroll)$"
enabled = true

[[stream]]
name = "Netflix"
domains = ["netflix.com"]
url_pattern = "netflix\\.com/watch/([0-9]+)"
title_pattern = "^(.+?)\\s*\\|\\s*Netflix$"
enabled = true

[[stream]]
name = "Jellyfin"
domains = []  # Any domain (self-hosted)
url_pattern = "/web/index\\.html#/video"
title_pattern = "^(.+?)\\s*-\\s*Jellyfin$"
enabled = true

[[stream]]
name = "Plex"
domains = ["app.plex.tv", "localhost:32400"]
url_pattern = "plex\\.tv/desktop|localhost:32400/web"
title_pattern = "^▶\\s*(.+?)$"
enabled = true
```

Additional services: Bilibili, Hidive, Disney+, Hulu, Amazon Prime Video, Funimation (legacy), YouTube.

### 2B. Browser Detection in `players.toml`

**Modify** `ryuuji-detect/data/players.toml` — add browser entries:
```toml
[[player]]
name = "Firefox"
executables = ["firefox", "firefox-esr"]
mpris_identities = ["firefox", "Firefox"]
window_classes = ["MozillaWindowClass", "Navigator"]
title_patterns = []  # Title extraction handled by stream engine
enabled = true
is_browser = true

[[player]]
name = "Chrome"
executables = ["google-chrome", "google-chrome-stable", "chromium", "chromium-browser"]
mpris_identities = ["chrome", "chromium", "Chrome", "Chromium"]
window_classes = ["Google-chrome", "Chromium", "Chrome_WidgetWin_1"]
title_patterns = []
enabled = true
is_browser = true

[[player]]
name = "Edge"
executables = ["microsoft-edge", "msedge"]
mpris_identities = ["msedge", "Microsoft Edge"]
window_classes = ["Chrome_WidgetWin_1"]  # Edge uses Chrome's class
title_patterns = []
enabled = true
is_browser = true

[[player]]
name = "Brave"
executables = ["brave", "brave-browser"]
mpris_identities = ["brave", "Brave"]
window_classes = ["Brave-browser"]
title_patterns = []
enabled = true
is_browser = true
```

### 2C. Stream Detection Engine (`ryuuji-detect/src/stream.rs`)

**New file:**
- `StreamDatabase` — loads `streams.toml`, matches URLs against stream patterns
- `StreamInfo { service_name, anime_title, episode }` — extracted from URL + page title
- `detect_stream(player_info: &PlayerInfo, stream_db: &StreamDatabase) -> Option<StreamInfo>`

**Logic:**
1. If `PlayerInfo` came from a browser (`is_browser` flag or matched browser player)
2. Check `file_path` (which is URL on Linux MPRIS) or `media_title` (window title) against stream URL patterns
3. If URL matches a stream provider, extract title from `media_title` using the stream's `title_pattern`
4. Parse extracted title through `ryuuji-parse` just like file-based detection
5. Return `StreamInfo` with service name and parsed anime data

**Integration in `PlayerInfo`:**
- Add `is_browser: bool` field to `PlayerInfo`
- On Linux MPRIS: `file_path` contains the URL directly from metadata
- On Windows: URL not directly available from window title — need to extract from title only

### 2D. Pipeline Integration

**Modify** `ryuuji-gui/src/app.rs` `detect_and_parse()`:
```
Current: detect_players() → first player → extract title from file_path/media_title → parse()
New:     detect_players() → first player →
           if browser: try stream detection (URL + title pattern → service-specific title)
           else: extract title from file_path/media_title
         → parse()
```

The rest of the pipeline (orchestrator, recognition, library update) stays unchanged — it just receives a different parsed title.

---

## Phase 3: Season Charts Screen ✅

**Goal:** New "Seasons" screen for browsing current/past/future anime seasons with cover art, genres, scores.

### 3A. Season Data Fetching (`ryuuji-api/src/anilist/`)

**Add to AniList client:**
- `browse_season(season: Season, year: u32, page: u32) -> Result<SeasonPage, AniListError>`
- `SeasonPage { items: Vec<AnimeSearchResult>, has_next: bool }`
- Uses AniList's `Page(page, perPage: 50) { media(season, seasonYear, type: ANIME, sort: POPULARITY_DESC) }` query
- This provides: title, episodes, cover, genres, score, studios, format, status, description — everything needed

**Season enum:** Reuse or extend existing:
```rust
pub enum AnimeSeason { Winter, Spring, Summer, Fall }
```

### 3B. Seasons Screen (`ryuuji-gui/src/screen/seasons.rs`)

**New screen following established pattern:**

**State:**
```rust
pub struct Seasons {
    season: AnimeSeason,       // Currently displayed season
    year: u32,                 // Currently displayed year
    entries: Vec<AnimeSearchResult>,
    loading: bool,
    error: Option<String>,
    selected: Option<usize>,   // For detail panel
    genre_filter: Option<String>,
    sort: SeasonSort,          // Popularity, Score, Title
}
```

**Messages:**
```rust
pub enum Message {
    SeasonChanged(AnimeSeason),
    YearChanged(i32),  // +1 or -1
    DataLoaded(Result<Vec<AnimeSearchResult>, String>),
    AnimeSelected(usize),
    AddToLibrary(usize),
    AddedToLibrary(Result<(), String>),
    GenreFilterChanged(Option<String>),
    SortChanged(SeasonSort),
}
```

**Layout:**
- Header: Season selector (4 buttons: Winter/Spring/Summer/Fall) + Year stepper (< 2025 >)
- Grid of anime cards (cover image + title + episode count + score badge)
- Right detail panel on selection (reuse existing `DetailPanel` widget)
- Genre filter chips at top
- Sort dropdown (Popularity, Score, Title A-Z)

### 3C. Navigation Integration

- Add `Page::Seasons` to `Page` enum
- Add nav rail item with `icon_calendar()` between Search and Torrents
- Add screen instance to `Ryuuji` struct
- Wire up messages in `app.rs`

---

## Phase 4: Sharing & Social Features

### 4A. Discord Rich Presence (`ryuuji-gui/src/discord.rs`)

**New dependency:** `discord-rich-presence = "1"` in `ryuuji-gui/Cargo.toml`

**New file** — `discord.rs`:
- `DiscordPresence` struct wrapping the IPC client
- `update(anime_title: &str, episode: u32, total: Option<u32>)` — sets activity
- `clear()` — clears activity when nothing is playing

**Activity fields:**
- State: "Watching Episode {N}/{Total}" or "Watching Episode {N}"
- Details: Anime title
- Large image: Cover URL (Discord needs a registered application image key — use AniList cover URL as fallback text)
- Timestamps: Start time of current session

**Integration in `app.rs`:**
```
Message::DetectionProcessed(Ok(UpdateOutcome::Updated { anime_title, episode, .. })) => {
    if config.discord.enabled {
        discord.update(&anime_title, episode, total_episodes);
    }
}
Message::DetectionResult(None) => {
    if config.discord.enabled {
        discord.clear();
    }
}
```

**Lifecycle:**
- Initialize on app startup if `config.discord.enabled`
- Reconnect on config toggle
- Handle connection errors gracefully (Discord not running → log warning, don't crash)

### 4B. HTTP Webhook Sharing (`ryuuji-core/src/sharing/`)

**New module:**
- `ryuuji-core/src/sharing/mod.rs` — `SharingEngine`
- `ryuuji-core/src/sharing/http.rs` — HTTP webhook
- `ryuuji-core/src/sharing/format.rs` — Template formatting

**Template system:**
- Config field: `sharing.http.url`, `sharing.http.body_template`
- Placeholders: `{title}`, `{episode}`, `{total}`, `{score}`, `{status}`, `{service_url}`
- Example: `Currently watching {title} - Episode {episode}/{total}`

**HTTP sharing:**
- POST to configured URL with formatted body
- Triggered on episode update (same hook point as Discord + sync push)

### 4C. Config & Settings

**Modify** `ryuuji-core/src/config.rs`:
```rust
pub struct SharingConfig {
    pub discord: DiscordConfig,     // { enabled: bool }  (already exists)
    pub http: HttpSharingConfig,    // { enabled, url, body_template }
}
```

**Modify** `ryuuji-gui/src/screen/settings.rs`:
- Add "Sharing" section with:
  - Discord Rich Presence toggle (already exists, now functional)
  - HTTP webhook: URL input, body template text area, test button

---

## Dependency Changes Summary

### Root `Cargo.toml` (workspace)
```toml
# New
discord-rich-presence = "1"
```

### `ryuuji-api/Cargo.toml`
No new deps needed — `graphql_client`, `reqwest`, `serde`, `chrono` already present.
(We'll use raw reqwest POST for GraphQL instead of `graphql_client` codegen.)

### `ryuuji-gui/Cargo.toml`
```toml
# New
discord-rich-presence = "1"
```

### `ryuuji-detect/Cargo.toml`
No new deps — `regex`, `serde`, `toml` already present.

---

## Implementation Order

```
Phase 1A: AniList client          (3-4 sessions)
Phase 1B: Kitsu client            (2-3 sessions)
Phase 1C: GUI multi-service       (2 sessions)
Phase 1D: Bidirectional sync      (1-2 sessions)
Phase 2:  Streaming detection     (2-3 sessions)
Phase 3:  Season charts screen    (2-3 sessions)
Phase 4A: Discord Rich Presence   (1 session)
Phase 4B: HTTP sharing            (1 session)
```

Each phase builds on the previous and is independently testable/shippable.

---

## Testing Strategy

- **AniList/Kitsu clients:** Unit tests with mock HTTP responses (same pattern as existing MAL tests if any, or mock reqwest)
- **Stream detection:** Unit tests with sample URLs and window titles for each streaming service
- **Season screen:** Manual testing (requires AniList API access)
- **Discord:** Manual testing (requires Discord running)
- **Sync:** Unit tests for conflict resolution logic, manual for end-to-end

---

## Out of Scope (Deferred)

These Taiga features are intentionally deferred:
- IRC sharing (niche, declining usage)
- System tray integration (platform-specific complexity)
- Desktop notifications (lower priority than core features)
- Local media library scanning (different paradigm from real-time detection)
- App update checker (low priority)
- Drag-and-drop between tabs (UX polish, not feature parity)
