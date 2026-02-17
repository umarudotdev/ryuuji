# Debug Dashboard Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add an in-app debug dashboard that surfaces the full detection pipeline state via a typed event ring buffer and cache statistics.

**Architecture:** An `EventLog` (bounded `VecDeque<DebugEvent>`) lives in app state behind `Arc<Mutex<>>`. The `detect_and_parse()` function and DB actor push typed events as the pipeline runs. A new Debug screen reads snapshots of this buffer on each detection tick. `CacheStats` counters are added to `RecognitionCache` and exposed via a new `DbCommand`.

**Tech Stack:** Rust, Iced 0.14, chrono, std::sync (Arc, Mutex), tokio::sync (mpsc, oneshot)

---

## Task 1: Create `debug_log` module — data types

**Files:**
- Create: `crates/ryuuji-core/src/debug_log.rs`
- Modify: `crates/ryuuji-core/src/lib.rs`

**Step 1: Create `debug_log.rs` with `DebugEvent`, `EventLog`, `CacheStats`**

```rust
// crates/ryuuji-core/src/debug_log.rs

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};

/// Maximum number of events retained in the ring buffer.
const EVENT_LOG_CAPACITY: usize = 200;

/// A typed event from the detection/recognition pipeline.
#[derive(Debug, Clone)]
pub enum DebugEvent {
    DetectionTick {
        players_found: u32,
    },
    PlayerDetected {
        player_name: String,
        file_path: Option<String>,
        is_browser: bool,
        media_title: Option<String>,
    },
    StreamMatched {
        service_name: String,
        extracted_title: String,
    },
    StreamNotMatched {
        player_name: String,
    },
    Parsed {
        raw_title: String,
        title: Option<String>,
        episode: Option<u32>,
        group: Option<String>,
        resolution: Option<String>,
    },
    RecognitionResult {
        query: String,
        match_level: MatchLevel,
        anime_title: Option<String>,
    },
    EpisodeRedirect {
        from_title: String,
        from_ep: u32,
        to_title: String,
        to_ep: u32,
    },
    LibraryUpdate {
        anime_title: String,
        episode: u32,
        outcome: UpdateKind,
    },
    Unrecognized {
        raw_title: String,
    },
    Error {
        source: String,
        message: String,
    },
}

/// How a title was matched in the recognition cache.
#[derive(Debug, Clone)]
pub enum MatchLevel {
    Exact,
    Normalized,
    Fuzzy(f64),
    LruHit,
    NoMatch,
}

/// What happened to the library entry.
#[derive(Debug, Clone)]
pub enum UpdateKind {
    Updated,
    AlreadyCurrent,
    Added,
}

/// A timestamped event entry.
pub type EventEntry = (DateTime<Utc>, DebugEvent);

/// Bounded ring buffer of debug events.
#[derive(Debug)]
pub struct EventLog {
    entries: VecDeque<EventEntry>,
}

impl Default for EventLog {
    fn default() -> Self {
        Self::new()
    }
}

impl EventLog {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::with_capacity(EVENT_LOG_CAPACITY),
        }
    }

    /// Push a new event, evicting the oldest if at capacity.
    pub fn push(&mut self, event: DebugEvent) {
        if self.entries.len() >= EVENT_LOG_CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back((Utc::now(), event));
    }

    /// Return a snapshot of all entries (newest last).
    pub fn snapshot(&self) -> Vec<EventEntry> {
        self.entries.iter().cloned().collect()
    }
}

/// Thread-safe handle to the event log.
pub type SharedEventLog = Arc<Mutex<EventLog>>;

/// Create a new shared event log.
pub fn shared_event_log() -> SharedEventLog {
    Arc::new(Mutex::new(EventLog::new()))
}

/// Recognition cache hit/miss counters.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub entries_indexed: usize,
    pub lru_size: usize,
    pub hits_exact: u64,
    pub hits_normalized: u64,
    pub hits_fuzzy: u64,
    pub hits_lru: u64,
    pub misses: u64,
}
```

**Step 2: Register the module in `lib.rs`**

Add `pub mod debug_log;` to `crates/ryuuji-core/src/lib.rs` (after the existing `pub mod config;` line).

**Step 3: Verify it compiles**

Run: `cargo check -p ryuuji-core`
Expected: success (no errors)

**Step 4: Commit**

```bash
git add crates/ryuuji-core/src/debug_log.rs crates/ryuuji-core/src/lib.rs
git commit -m "feat(core): add debug_log module with DebugEvent, EventLog, CacheStats"
```

---

## Task 2: Add `CacheStats` to `RecognitionCache`

**Files:**
- Modify: `crates/ryuuji-core/src/recognition.rs`

**Step 1: Write tests for cache stats**

Add to the existing `#[cfg(test)] mod tests` block in `recognition.rs`:

```rust
#[test]
fn test_cache_stats_exact() {
    let storage = Storage::open_memory().unwrap();
    insert_frieren(&storage);

    let mut cache = RecognitionCache::new();
    cache.recognize("Sousou no Frieren", &storage);
    let stats = cache.stats();
    assert_eq!(stats.hits_exact, 1);
    assert_eq!(stats.misses, 0);
}

#[test]
fn test_cache_stats_lru() {
    let storage = Storage::open_memory().unwrap();
    insert_frieren(&storage);

    let mut cache = RecognitionCache::new();
    // First call: exact hit.
    cache.recognize("Sousou no Frieren", &storage);
    // Second call: LRU cache hit.
    cache.recognize("Sousou no Frieren", &storage);
    let stats = cache.stats();
    assert_eq!(stats.hits_exact, 1);
    assert_eq!(stats.hits_lru, 1);
}

#[test]
fn test_cache_stats_reset_on_invalidate() {
    let storage = Storage::open_memory().unwrap();
    insert_frieren(&storage);

    let mut cache = RecognitionCache::new();
    cache.recognize("Sousou no Frieren", &storage);
    assert_eq!(cache.stats().hits_exact, 1);

    cache.invalidate();
    let stats = cache.stats();
    assert_eq!(stats.hits_exact, 0);
    assert_eq!(stats.entries_indexed, 0);
}

#[test]
fn test_cache_stats_miss() {
    let storage = Storage::open_memory().unwrap();
    insert_frieren(&storage);

    let mut cache = RecognitionCache::new();
    cache.recognize("Totally Unknown Anime Title", &storage);
    let stats = cache.stats();
    assert_eq!(stats.misses, 1);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p ryuuji-core cache_stats`
Expected: FAIL — `stats()` method does not exist

**Step 3: Add `stats` field and counter increments to `RecognitionCache`**

In `RecognitionCache` struct, add:
```rust
use crate::debug_log::CacheStats;
```

Add field to struct:
```rust
stats: CacheStats,
```

Initialize in `new()`:
```rust
stats: CacheStats::default(),
```

In `populate()`, after building indices, update:
```rust
self.stats.entries_indexed = self.entries.len();
```

In `invalidate()`, add:
```rust
self.stats = CacheStats::default();
```

In `recognize()`, increment counters at each match level:
- After query cache hit (line ~111): `self.stats.hits_lru += 1;`
- After exact index hit (line ~119): `self.stats.hits_exact += 1;`
- After normalized index hit (line ~129): `self.stats.hits_normalized += 1;`
- After fuzzy scan returns `Fuzzy` (line ~139): `self.stats.hits_fuzzy += 1;`
- After fuzzy scan returns `NoMatch` (line ~147): `self.stats.misses += 1;`
- Update LRU size: `self.stats.lru_size = self.query_cache.len();` (before returning from `recognize()`, or after each `query_cache_insert`)

Add getter:
```rust
/// Return current cache statistics.
pub fn stats(&self) -> CacheStats {
    self.stats.clone()
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p ryuuji-core cache_stats`
Expected: PASS (all 4 new tests)

Run: `cargo test -p ryuuji-core recognition`
Expected: PASS (all existing + new tests)

**Step 5: Commit**

```bash
git add crates/ryuuji-core/src/recognition.rs
git commit -m "feat(core): add CacheStats counters to RecognitionCache"
```

---

## Task 3: Wire `EventLog` into DB actor

**Files:**
- Modify: `crates/ryuuji-gui/src/db.rs`

**Step 1: Add `GetCacheStats` command and `EventLog` threading**

Add imports at top of `db.rs`:
```rust
use std::sync::{Arc, Mutex};
use ryuuji_core::debug_log::{self, CacheStats, DebugEvent, EventLog, SharedEventLog};
```

Add to `DbCommand` enum:
```rust
GetCacheStats {
    reply: oneshot::Sender<CacheStats>,
},
```

Change `DbHandle::open()` to accept and pass the event log:
```rust
pub fn open(path: &Path, event_log: SharedEventLog) -> Option<Self> {
```

In the `std::thread::Builder::new()` spawn closure, capture `event_log`:
```rust
.spawn(move || actor_loop(storage, rx, event_log))
```

Change `actor_loop` signature:
```rust
fn actor_loop(storage: Storage, mut rx: mpsc::UnboundedReceiver<DbCommand>, event_log: SharedEventLog) {
```

Add handler for `GetCacheStats` in the `match cmd` block:
```rust
DbCommand::GetCacheStats { reply } => {
    let _ = reply.send(cache.stats());
}
```

After the `ProcessDetection` handler's `orchestrator::process_detection()` call, push events to the event log based on the result:
```rust
DbCommand::ProcessDetection {
    detected,
    config,
    reply,
} => {
    let result = orchestrator::process_detection(
        &detected,
        &storage,
        &config,
        &mut cache,
        Some(&relations),
    );

    // Push debug events based on outcome.
    if let Ok(ref log) = event_log.lock() {
        // (Events will be pushed in the next step after the lock pattern is established)
    }
    if let Ok(ref outcome) = &result {
        let mut log = event_log.lock().unwrap_or_else(|e| e.into_inner());
        match outcome {
            UpdateOutcome::Updated { anime_title, episode, .. } => {
                log.push(DebugEvent::LibraryUpdate {
                    anime_title: anime_title.clone(),
                    episode: *episode,
                    outcome: debug_log::UpdateKind::Updated,
                });
            }
            UpdateOutcome::AlreadyCurrent { anime_title, episode, .. } => {
                log.push(DebugEvent::LibraryUpdate {
                    anime_title: anime_title.clone(),
                    episode: *episode,
                    outcome: debug_log::UpdateKind::AlreadyCurrent,
                });
            }
            UpdateOutcome::AddedToLibrary { anime_title, episode, .. } => {
                log.push(DebugEvent::LibraryUpdate {
                    anime_title: anime_title.clone(),
                    episode: *episode,
                    outcome: debug_log::UpdateKind::Added,
                });
            }
            UpdateOutcome::Unrecognized { raw_title } => {
                log.push(DebugEvent::Unrecognized {
                    raw_title: raw_title.clone(),
                });
            }
        }
    }
    if let Err(ref e) = &result {
        let mut log = event_log.lock().unwrap_or_else(|e| e.into_inner());
        log.push(DebugEvent::Error {
            source: "orchestrator".into(),
            message: e.to_string(),
        });
    }

    if let Ok(UpdateOutcome::AddedToLibrary { .. }) = &result {
        cache.invalidate();
    }
    let _ = reply.send(result);
}
```

Add `DbHandle` method:
```rust
pub async fn get_cache_stats(&self) -> CacheStats {
    let (reply, rx) = oneshot::channel();
    let _ = self.tx.send(DbCommand::GetCacheStats { reply });
    rx.await.unwrap_or_default()
}
```

**Step 2: Verify it compiles**

Run: `cargo check -p ryuuji-gui`
Expected: FAIL — `DbHandle::open()` call sites in `app.rs` need the new parameter. That's expected; we fix it in Task 4.

Run: `cargo check -p ryuuji-core`
Expected: success

**Step 3: Commit**

```bash
git add crates/ryuuji-gui/src/db.rs
git commit -m "feat(gui): wire EventLog into DB actor, add GetCacheStats command"
```

---

## Task 4: Wire `EventLog` into app state and `detect_and_parse()`

**Files:**
- Modify: `crates/ryuuji-gui/src/app.rs`

**Step 1: Add `event_log` to app state**

Add import:
```rust
use ryuuji_core::debug_log::{self, DebugEvent, SharedEventLog};
```

Add field to `Ryuuji` struct:
```rust
event_log: SharedEventLog,
```

In `Default::default()`, create the shared log and pass it to `DbHandle::open()`:
```rust
let event_log = debug_log::shared_event_log();
let db = match AppConfig::ensure_db_path() {
    Ok(path) => DbHandle::open(&path, event_log.clone()),
    // ...
};
```

Add `event_log` to struct initialization:
```rust
event_log,
```

**Step 2: Pass `event_log` to `detect_and_parse()`**

Change the function signature:
```rust
async fn detect_and_parse(event_log: SharedEventLog) -> Option<DetectedMedia> {
```

At the call site (`Message::DetectionTick`), pass the clone:
```rust
Message::DetectionTick => {
    let log = self.event_log.clone();
    Task::perform(detect_and_parse(log), Message::DetectionResult)
}
```

Inside `detect_and_parse()`, push events after each stage:

After player detection:
```rust
{
    let mut log = event_log.lock().unwrap_or_else(|e| e.into_inner());
    log.push(DebugEvent::DetectionTick {
        players_found: players.len() as u32,
    });
}
```

After selecting a player:
```rust
{
    let mut log = event_log.lock().unwrap_or_else(|e| e.into_inner());
    log.push(DebugEvent::PlayerDetected {
        player_name: player.player_name.clone(),
        file_path: player.file_path.clone(),
        is_browser: player.is_browser,
        media_title: player.media_title.clone(),
    });
}
```

After stream match/no-match:
```rust
// On match:
log.push(DebugEvent::StreamMatched {
    service_name: m.service_name.clone(),
    extracted_title: m.extracted_title.clone(),
});
// On no match:
log.push(DebugEvent::StreamNotMatched {
    player_name: player.player_name.clone(),
});
```

After parsing:
```rust
{
    let mut log = event_log.lock().unwrap_or_else(|e| e.into_inner());
    log.push(DebugEvent::Parsed {
        raw_title: raw_title.clone(),
        title: parsed.title.clone(),
        episode: parsed.episode_number,
        group: parsed.release_group.clone(),
        resolution: parsed.resolution.clone(),
    });
}
```

**Step 3: Verify it compiles**

Run: `cargo check -p ryuuji-gui`
Expected: success (warnings about unused `debug` screen are OK — we add it next)

**Step 4: Run existing tests**

Run: `cargo test --workspace`
Expected: PASS (all existing tests)

**Step 5: Commit**

```bash
git add crates/ryuuji-gui/src/app.rs
git commit -m "feat(gui): wire SharedEventLog into app state and detect_and_parse"
```

---

## Task 5: Add Debug screen — module registration and Page variant

**Files:**
- Modify: `crates/ryuuji-gui/src/screen.rs`
- Create: `crates/ryuuji-gui/src/screen/debug.rs` (stub)

**Step 1: Register `debug` module and add `Page::Debug`**

In `crates/ryuuji-gui/src/screen.rs`, add `pub mod debug;` at the top with the other module declarations.

Add `Debug` variant to the `Page` enum:
```rust
pub enum Page {
    #[default]
    NowPlaying,
    Library,
    History,
    Search,
    Seasons,
    Torrents,
    Stats,
    Debug,
    Settings,
}
```

**Step 2: Create stub `debug.rs`**

```rust
// crates/ryuuji-gui/src/screen/debug.rs

//! Debug dashboard screen.
//!
//! Displays the detection pipeline state: current detection status,
//! last parse/match results, cache statistics, and a scrollable
//! event history log.

use iced::widget::{button, column, container, row, scrollable, text, toggler, Space};
use iced::{Element, Length};

use chrono::{DateTime, Utc};

use ryuuji_core::debug_log::{CacheStats, DebugEvent, EventEntry, SharedEventLog};

use crate::app;
use crate::db::DbHandle;
use crate::screen::Action;
use crate::style;
use crate::theme::{self, ColorScheme};

/// Debug screen state.
pub struct Debug {
    /// Snapshot of the event log (newest last).
    events: Vec<EventEntry>,
    /// Cache statistics from the recognition cache.
    cache_stats: Option<CacheStats>,
    /// Whether verbose mode is on (expands all event fields).
    verbose: bool,
}

/// Messages handled by the Debug screen.
#[derive(Debug, Clone)]
pub enum Message {
    /// New event log snapshot received.
    EventsRefreshed(Vec<EventEntry>),
    /// Cache stats loaded from DB actor.
    CacheStatsLoaded(CacheStats),
    /// Toggle verbose display.
    ToggleVerbose(bool),
}

impl Debug {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            cache_stats: None,
            verbose: false,
        }
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::EventsRefreshed(events) => {
                self.events = events;
                Action::None
            }
            Message::CacheStatsLoaded(stats) => {
                self.cache_stats = Some(stats);
                Action::None
            }
            Message::ToggleVerbose(on) => {
                self.verbose = on;
                Action::None
            }
        }
    }

    /// Refresh the debug screen from the shared event log and request cache stats.
    pub fn refresh(
        &mut self,
        event_log: &SharedEventLog,
        db: Option<&DbHandle>,
    ) -> Action {
        // Snapshot the event log (brief lock).
        let snapshot = event_log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .snapshot();
        self.events = snapshot;

        // Request cache stats from the DB actor.
        if let Some(db) = db {
            let db = db.clone();
            return Action::RunTask(iced::Task::perform(
                async move { db.get_cache_stats().await },
                |stats| app::Message::Debug(Message::CacheStatsLoaded(stats)),
            ));
        }
        Action::None
    }

    pub fn view<'a>(&'a self, cs: &ColorScheme) -> Element<'a, Message> {
        let header_row = row![
            text("Debug").size(style::TEXT_XL).font(style::FONT_HEADING),
            Space::new(Length::Fill, Length::Shrink),
            text("Verbose").size(style::TEXT_SM).color(cs.on_surface_variant),
            toggler(self.verbose)
                .on_toggle(Message::ToggleVerbose)
                .size(16.0),
        ]
        .spacing(style::SPACE_SM)
        .align_y(iced::Alignment::Center);

        // ── Current State section ──────────────────────────────
        let state_section = self.current_state_view(cs);

        // ── Event History section ──────────────────────────────
        let history_section = self.event_history_view(cs);

        let page = column![header_row, state_section, history_section]
            .spacing(style::SPACE_LG)
            .padding(style::SPACE_XL)
            .width(Length::Fill)
            .height(Length::Fill);

        page.into()
    }

    /// Render the "current state" snapshot panel.
    fn current_state_view<'a>(&'a self, cs: &ColorScheme) -> Element<'a, Message> {
        let last_detection = self
            .events
            .iter()
            .rev()
            .find_map(|(_, e)| match e {
                DebugEvent::PlayerDetected {
                    player_name,
                    media_title,
                    ..
                } => Some(format!(
                    "{} — {}",
                    player_name,
                    media_title.as_deref().unwrap_or("(no title)")
                )),
                _ => None,
            })
            .unwrap_or_else(|| "Nothing detected".into());

        let last_parse = self
            .events
            .iter()
            .rev()
            .find_map(|(_, e)| match e {
                DebugEvent::Parsed {
                    raw_title,
                    title,
                    episode,
                    group,
                    ..
                } => {
                    let title_str = title.as_deref().unwrap_or("?");
                    let ep_str = episode
                        .map(|e| format!(" ep {e}"))
                        .unwrap_or_default();
                    let group_str = group
                        .as_ref()
                        .map(|g| format!(" [{g}]"))
                        .unwrap_or_default();
                    Some(format!("{raw_title} -> {title_str}{ep_str}{group_str}"))
                }
                _ => None,
            })
            .unwrap_or_else(|| "—".into());

        let last_match = self
            .events
            .iter()
            .rev()
            .find_map(|(_, e)| match e {
                DebugEvent::RecognitionResult {
                    query,
                    match_level,
                    anime_title,
                } => {
                    let level_str = format!("{match_level:?}");
                    let title_str = anime_title.as_deref().unwrap_or("—");
                    Some(format!("\"{query}\" -> {title_str} ({level_str})"))
                }
                _ => None,
            })
            .unwrap_or_else(|| "—".into());

        let cache_line = if let Some(ref stats) = self.cache_stats {
            format!(
                "Indexed: {} | LRU: {} | Hits: exact {} / norm {} / fuzzy {} / lru {} | Miss: {}",
                stats.entries_indexed,
                stats.lru_size,
                stats.hits_exact,
                stats.hits_normalized,
                stats.hits_fuzzy,
                stats.hits_lru,
                stats.misses,
            )
        } else {
            "Cache stats loading...".into()
        };

        let card = column![
            self.label_value(cs, "Detection", &last_detection),
            self.label_value(cs, "Last parse", &last_parse),
            self.label_value(cs, "Last match", &last_match),
            self.label_value(cs, "Cache", &cache_line),
        ]
        .spacing(style::SPACE_SM)
        .padding(style::SPACE_LG)
        .width(Length::Fill);

        container(card)
            .style(theme::card_container(cs))
            .width(Length::Fill)
            .into()
    }

    /// Render a label: value pair.
    fn label_value<'a>(
        &self,
        cs: &ColorScheme,
        label: &str,
        value: &str,
    ) -> Element<'a, Message> {
        row![
            text(format!("{label}:"))
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant)
                .width(Length::Fixed(80.0)),
            text(value)
                .size(style::TEXT_SM)
                .color(cs.on_surface),
        ]
        .spacing(style::SPACE_SM)
        .into()
    }

    /// Render the scrollable event history.
    fn event_history_view<'a>(&'a self, cs: &ColorScheme) -> Element<'a, Message> {
        let mut event_column = column![].spacing(style::SPACE_XXS);

        // Reverse chronological — newest first.
        for (timestamp, event) in self.events.iter().rev() {
            let time_str = timestamp.format("%H:%M:%S").to_string();
            let (summary, color) = self.event_summary(cs, event);

            let row_content = if self.verbose {
                let detail = format!("{event:?}");
                column![
                    row![
                        text(&time_str)
                            .size(style::TEXT_XS)
                            .color(cs.on_surface_variant)
                            .width(Length::Fixed(64.0)),
                        text(&summary).size(style::TEXT_SM).color(color),
                    ]
                    .spacing(style::SPACE_SM),
                    text(&detail)
                        .size(style::TEXT_XS)
                        .color(cs.on_surface_variant),
                ]
                .spacing(style::SPACE_XXS)
            } else {
                column![row![
                    text(&time_str)
                        .size(style::TEXT_XS)
                        .color(cs.on_surface_variant)
                        .width(Length::Fixed(64.0)),
                    text(&summary).size(style::TEXT_SM).color(color),
                ]
                .spacing(style::SPACE_SM)]
            };

            event_column = event_column.push(row_content);
        }

        let history = if self.events.is_empty() {
            column![
                text("No events yet. Detection events will appear here as the pipeline runs.")
                    .size(style::TEXT_SM)
                    .color(cs.on_surface_variant),
            ]
        } else {
            event_column
        };

        scrollable(
            container(history)
                .padding(style::SPACE_LG)
                .width(Length::Fill),
        )
        .height(Length::Fill)
        .into()
    }

    /// Return a one-line summary and color for an event.
    fn event_summary(&self, cs: &ColorScheme, event: &DebugEvent) -> (String, iced::Color) {
        match event {
            DebugEvent::DetectionTick { players_found } => (
                format!("Tick — {players_found} player(s)"),
                cs.on_surface_variant,
            ),
            DebugEvent::PlayerDetected {
                player_name,
                is_browser,
                ..
            } => {
                let kind = if *is_browser { "browser" } else { "player" };
                (format!("Detected {kind}: {player_name}"), cs.on_surface)
            }
            DebugEvent::StreamMatched {
                service_name,
                extracted_title,
            } => (
                format!("Stream: {service_name} — \"{extracted_title}\""),
                cs.primary,
            ),
            DebugEvent::StreamNotMatched { player_name } => (
                format!("No stream match for {player_name}"),
                cs.on_surface_variant,
            ),
            DebugEvent::Parsed {
                title, episode, ..
            } => {
                let t = title.as_deref().unwrap_or("?");
                let ep = episode.map(|e| format!(" ep {e}")).unwrap_or_default();
                (format!("Parsed: {t}{ep}"), cs.on_surface)
            }
            DebugEvent::RecognitionResult {
                match_level,
                anime_title,
                ..
            } => {
                let title = anime_title.as_deref().unwrap_or("—");
                match match_level {
                    ryuuji_core::debug_log::MatchLevel::NoMatch => {
                        (format!("No match"), cs.error)
                    }
                    ryuuji_core::debug_log::MatchLevel::Fuzzy(score) => (
                        format!("Fuzzy: {title} ({:.0}%)", score * 100.0),
                        cs.tertiary,
                    ),
                    level => (format!("{level:?}: {title}"), cs.primary),
                }
            }
            DebugEvent::EpisodeRedirect {
                from_title,
                from_ep,
                to_title,
                to_ep,
            } => (
                format!("Redirect: {from_title} ep {from_ep} -> {to_title} ep {to_ep}"),
                cs.on_surface,
            ),
            DebugEvent::LibraryUpdate {
                anime_title,
                episode,
                outcome,
            } => {
                let verb = match outcome {
                    ryuuji_core::debug_log::UpdateKind::Updated => "Updated",
                    ryuuji_core::debug_log::UpdateKind::AlreadyCurrent => "Current",
                    ryuuji_core::debug_log::UpdateKind::Added => "Added",
                };
                (
                    format!("{verb}: {anime_title} ep {episode}"),
                    cs.primary,
                )
            }
            DebugEvent::Unrecognized { raw_title } => (
                format!("Unrecognized: \"{raw_title}\""),
                cs.tertiary,
            ),
            DebugEvent::Error { source, message } => (
                format!("Error ({source}): {message}"),
                cs.error,
            ),
        }
    }
}
```

**Step 3: Verify it compiles**

Run: `cargo check -p ryuuji-gui`
Expected: may still fail if `app.rs` doesn't import/use `debug` screen yet. Compilation errors from `app.rs` are expected and fixed in Task 6.

**Step 4: Commit**

```bash
git add crates/ryuuji-gui/src/screen/debug.rs crates/ryuuji-gui/src/screen.rs
git commit -m "feat(gui): add Debug screen module with event history and cache stats view"
```

---

## Task 6: Wire Debug screen into app router and nav rail

**Files:**
- Modify: `crates/ryuuji-gui/src/app.rs`

**Step 1: Add debug screen state and message routing**

In the `use crate::screen::{...}` import, add `debug`:
```rust
use crate::screen::{
    debug, history, library, now_playing, search, seasons, settings, stats, torrents, Action,
    ContextAction, ModalKind, Page,
};
```

Add field to `Ryuuji` struct:
```rust
debug: debug::Debug,
```

Initialize in `Default::default()`:
```rust
debug: debug::Debug::new(),
```

Add `Message::Debug` variant to the `Message` enum:
```rust
Debug(debug::Message),
```

Add handler in `update()`:
```rust
Message::Debug(msg) => {
    let action = self.debug.update(msg);
    self.handle_action(action)
}
```

Add navigation handler for `Page::Debug` in the `Message::NavigateTo` match:
```rust
if page == Page::Debug {
    let action = self.debug.refresh(&self.event_log, self.db.as_ref());
    return self.handle_action(action);
}
```

In the `DetectionResult` handler, after processing, refresh the debug screen if currently on that page:
After the existing `self.now_playing.detected = media.clone();` line, add:
```rust
// Refresh debug screen snapshot on every detection tick.
if self.page == Page::Debug {
    let action = self.debug.refresh(&self.event_log, self.db.as_ref());
    let debug_task = self.handle_action(action);
    // ... combine with existing task via Task::batch if needed
}
```

**Step 2: Add Debug to nav rail**

In `nav_rail()`, add the Debug item to the bottom group (between Stats and Settings):
```rust
column![
    nav_item(icons::icon_chart_bar(), "Stats", Page::Stats),
    nav_item(icons::icon_activity(), "Debug", Page::Debug),
    nav_item(icons::icon_settings(), "Settings", Page::Settings),
]
```

**Step 3: Add Debug screen to view routing**

In the `view()` method's page routing match, add:
```rust
Page::Debug => self.debug.view(cs).map(Message::Debug),
```

**Step 4: Verify it compiles**

Run: `cargo check -p ryuuji-gui`
Expected: success

**Step 5: Run all tests**

Run: `cargo test --workspace`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/ryuuji-gui/src/app.rs
git commit -m "feat(gui): wire Debug screen into app router, nav rail, and view dispatch"
```

---

## Task 7: Push recognition events from DB actor

**Files:**
- Modify: `crates/ryuuji-gui/src/db.rs`

The `ProcessDetection` handler already pushes `LibraryUpdate`, `Unrecognized`, and `Error` events (from Task 3). This task adds `RecognitionResult` events by inspecting the recognition cache state before/after the orchestrator call.

**Step 1: Add `RecognitionResult` event to `ProcessDetection` handler**

Before calling `orchestrator::process_detection()`, capture the query:
```rust
let query = detected.anime_title.clone().unwrap_or_default();
```

After the `process_detection()` call, push a `RecognitionResult` event based on the outcome:
```rust
{
    let mut log = event_log.lock().unwrap_or_else(|e| e.into_inner());
    match &result {
        Ok(UpdateOutcome::Updated { anime_title, .. })
        | Ok(UpdateOutcome::AlreadyCurrent { anime_title, .. })
        | Ok(UpdateOutcome::AddedToLibrary { anime_title, .. }) => {
            log.push(DebugEvent::RecognitionResult {
                query: query.clone(),
                match_level: debug_log::MatchLevel::Exact, // Simplified — we know it matched
                anime_title: Some(anime_title.clone()),
            });
        }
        Ok(UpdateOutcome::Unrecognized { .. }) => {
            log.push(DebugEvent::RecognitionResult {
                query: query.clone(),
                match_level: debug_log::MatchLevel::NoMatch,
                anime_title: None,
            });
        }
        Err(_) => {} // Error event already pushed
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo check -p ryuuji-gui`
Expected: success

**Step 3: Commit**

```bash
git add crates/ryuuji-gui/src/db.rs
git commit -m "feat(gui): push RecognitionResult events from DB actor"
```

---

## Task 8: Final verification and formatting

**Step 1: Format**

Run: `cargo fmt --all`

**Step 2: Lint**

Run: `cargo clippy --workspace`
Expected: no new warnings (pre-existing warnings in normalize.rs and scanner.rs are OK)

**Step 3: Test**

Run: `cargo test --workspace`
Expected: all tests pass

**Step 4: Commit any format fixes**

```bash
git add -A
git commit -m "style: format debug dashboard code"
```

(Skip if nothing changed.)
