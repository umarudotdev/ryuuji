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
