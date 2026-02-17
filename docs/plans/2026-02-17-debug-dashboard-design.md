# Debug Dashboard Design

## Problem

Ryuuji's detection pipeline is a black box. Users see the final result (matched anime or "Nothing playing") but have no visibility into _why_ something matched or failed. Developers debugging detection issues must rely on `RUST_LOG` terminal output. A visual debugger would surface the full pipeline state inside the app.

## Approach

In-process event ring buffer. An `EventLog` (bounded `VecDeque<DebugEvent>`) lives in app state behind `Arc<Mutex<>>`. The detection function and DB actor push typed events as the pipeline runs. A new Debug screen reads snapshots of this buffer.

### Why not a tracing subscriber tap?

Tracing events are stringly-typed — field names are strings, values are `dyn Value`. Parsing structured data back out of tracing spans is brittle and loses the rich typing (e.g., `MatchResult::Fuzzy` becomes `{matched: "true", score: "72"}`). Typed `DebugEvent` enums give the UI much better rendering.

## Data Model

### DebugEvent (in ryuuji-core)

```
DebugEvent:
  DetectionTick { players_found: u32, timestamp }
  PlayerDetected { player_name, file_path, is_browser, media_title }
  StreamMatched { service_name, extracted_title }
  StreamNotMatched { player_name }
  Parsed { raw_title, title, episode, group, resolution }
  RecognitionResult { query, match_level: Exact|Normalized|Fuzzy(score)|LruHit|NoMatch, anime_title }
  EpisodeRedirect { from_title, from_ep, to_title, to_ep }
  LibraryUpdate { anime_title, episode, outcome: Updated|AlreadyCurrent|Added }
  Unrecognized { raw_title }
  Error { source, message }
```

### EventLog

`VecDeque<(DateTime<Utc>, DebugEvent)>` capped at 200 entries. Wrapped in `Arc<Mutex<EventLog>>`. Push is O(1) amortized.

### CacheStats

```
CacheStats {
  entries_indexed: usize,
  lru_size: usize,
  hits_exact: u64,
  hits_normalized: u64,
  hits_fuzzy: u64,
  hits_lru: u64,
  misses: u64,
}
```

Counters live in `RecognitionCache`, incremented in `recognize()`, reset on `invalidate()`.

## Screen Layout

### Top: Current State (~30% height)

- **Detection status**: Player name + what's playing, or "Nothing detected"
- **Last parse result**: Raw title -> parsed title + episode + group (one-liner)
- **Last match result**: Query -> matched anime (with fuzzy score if applicable), or "No match"
- **Cache stats**: Entries indexed, LRU size, hit counts by level

### Bottom: Event History (~70% height)

- Reverse-chronological scrollable list
- Each row: timestamp (HH:MM:SS), colored icon by event type, one-line summary
- Color coding: green = match/update, yellow = fuzzy/unrecognized, red = error, gray = tick/info
- **Verbose toggle** (top-right): expands events to show all fields. Default off.

### Nav Rail

`icon_activity` (pulse icon), labeled "Debug", placed in the bottom group next to Stats and Settings.

## Plumbing

### Event producers

1. **`detect_and_parse()` async function**: Gets `Arc<Mutex<EventLog>>` clone. Pushes `DetectionTick`, `PlayerDetected`, `StreamMatched`/`StreamNotMatched`, `Parsed`.

2. **DB actor thread** (`db.rs`): Gets `Arc<Mutex<EventLog>>` clone at construction. After calling `orchestrator::process_detection()`, inspects the `UpdateOutcome` and pushes `RecognitionResult`, `EpisodeRedirect`, `LibraryUpdate`, `Unrecognized`, or `Error`.

The orchestrator itself is unchanged — the DB actor observes its output.

### Event consumer

On each `DetectionTick`, the app copies the EventLog snapshot into the Debug screen's local state via `debug_screen.refresh(&event_log)`. No lock held during rendering.

### Cache stats

New `DbCommand::GetCacheStats` with oneshot reply, same pattern as all other DbHandle methods. Debug screen requests this on refresh.

### Cost when Debug screen isn't open

Events are still pushed (~nanosecond Mutex lock + VecDeque push). Ring buffer caps at 200 entries (~50KB). Negligible.

## Files

### New

| File | Contents |
|---|---|
| `crates/ryuuji-core/src/debug_log.rs` | `DebugEvent`, `EventLog`, `CacheStats` |
| `crates/ryuuji-gui/src/screen/debug.rs` | Debug screen state, Message, update(), view() |

### Modified

| File | Change |
|---|---|
| `crates/ryuuji-core/src/lib.rs` | `pub mod debug_log` |
| `crates/ryuuji-core/src/recognition.rs` | `stats: CacheStats` field, counter increments, `stats()` + `last_match_detail()` getters |
| `crates/ryuuji-gui/src/screen.rs` | `pub mod debug`, `Debug` variant in `Page` |
| `crates/ryuuji-gui/src/app.rs` | Create EventLog, pass Arc to detect_and_parse + DbHandle, push events in detect_and_parse, add debug screen state, wire nav + routing + refresh |
| `crates/ryuuji-gui/src/db.rs` | Accept EventLog Arc, push events after ProcessDetection, add GetCacheStats command |

### Unchanged

Orchestrator, parser, storage, API clients, all other screens.

## Build Order

1. `ryuuji-core`: debug_log module + recognition.rs changes
2. `ryuuji-gui`: db.rs plumbing -> app.rs wiring -> screen/debug.rs
