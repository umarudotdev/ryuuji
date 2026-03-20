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

/// Build a recognition + library-update debug event pair.
fn recognition_and_update(
    query: &str,
    anime_title: &str,
    episode: u32,
    outcome: UpdateKind,
) -> Vec<DebugEvent> {
    vec![
        DebugEvent::RecognitionResult {
            query: query.to_string(),
            match_level: MatchLevel::Exact,
            anime_title: Some(anime_title.to_string()),
        },
        DebugEvent::LibraryUpdate {
            anime_title: anime_title.to_string(),
            episode,
            outcome,
        },
    ]
}

/// Convert a `DomainEvent` into debug events for the event log.
///
/// Returns a list because some domain events produce multiple debug entries
/// (e.g., a recognition result + a library update).
pub fn debug_events_from_domain(
    event: &crate::events::DomainEvent,
    query: &str,
) -> Vec<DebugEvent> {
    use crate::events::DomainEvent;

    match event {
        DomainEvent::EpisodeUpdated {
            anime_title,
            new_episode,
            ..
        } => recognition_and_update(query, anime_title, *new_episode, UpdateKind::Updated),
        DomainEvent::AddedToLibrary {
            anime_title,
            initial_episode,
            ..
        } => recognition_and_update(query, anime_title, *initial_episode, UpdateKind::Added),
        DomainEvent::AlreadyCurrent {
            anime_title,
            episode,
            ..
        } => recognition_and_update(query, anime_title, *episode, UpdateKind::AlreadyCurrent),
        DomainEvent::Unrecognized { raw_title, .. } => vec![
            DebugEvent::RecognitionResult {
                query: query.to_string(),
                match_level: MatchLevel::NoMatch,
                anime_title: None,
            },
            DebugEvent::Unrecognized {
                raw_title: raw_title.clone(),
            },
        ],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{ChangeSource, DomainEvent};
    use crate::models::WatchStatus;
    use chrono::Utc;

    #[test]
    fn episode_updated_produces_recognition_and_update() {
        let event = DomainEvent::EpisodeUpdated {
            anime_id: 1,
            anime_title: "Frieren".into(),
            old_episode: 3,
            new_episode: 5,
            source: ChangeSource::Detection,
            timestamp: Utc::now(),
        };
        let debug = debug_events_from_domain(&event, "Frieren");
        assert_eq!(debug.len(), 2);
        assert!(matches!(
            &debug[0],
            DebugEvent::RecognitionResult {
                match_level: MatchLevel::Exact,
                ..
            }
        ));
        assert!(matches!(
            &debug[1],
            DebugEvent::LibraryUpdate {
                outcome: UpdateKind::Updated,
                episode: 5,
                ..
            }
        ));
    }

    #[test]
    fn added_to_library_produces_recognition_and_added() {
        let event = DomainEvent::AddedToLibrary {
            anime_id: 1,
            anime_title: "Frieren".into(),
            initial_status: WatchStatus::Watching,
            initial_episode: 1,
            source: ChangeSource::Detection,
            timestamp: Utc::now(),
        };
        let debug = debug_events_from_domain(&event, "Frieren");
        assert_eq!(debug.len(), 2);
        assert!(matches!(
            &debug[1],
            DebugEvent::LibraryUpdate {
                outcome: UpdateKind::Added,
                episode: 1,
                ..
            }
        ));
    }

    #[test]
    fn already_current_produces_recognition_and_already_current() {
        let event = DomainEvent::AlreadyCurrent {
            anime_id: 1,
            anime_title: "Frieren".into(),
            episode: 5,
            timestamp: Utc::now(),
        };
        let debug = debug_events_from_domain(&event, "Frieren");
        assert_eq!(debug.len(), 2);
        assert!(matches!(
            &debug[1],
            DebugEvent::LibraryUpdate {
                outcome: UpdateKind::AlreadyCurrent,
                ..
            }
        ));
    }

    #[test]
    fn unrecognized_produces_no_match_and_unrecognized() {
        let event = DomainEvent::Unrecognized {
            raw_title: "Unknown.mkv".into(),
            timestamp: Utc::now(),
        };
        let debug = debug_events_from_domain(&event, "Unknown");
        assert_eq!(debug.len(), 2);
        assert!(matches!(
            &debug[0],
            DebugEvent::RecognitionResult {
                match_level: MatchLevel::NoMatch,
                anime_title: None,
                ..
            }
        ));
        assert!(matches!(&debug[1], DebugEvent::Unrecognized { .. }));
    }

    #[test]
    fn non_detection_events_produce_empty() {
        let cases = vec![
            DomainEvent::StatusChanged {
                anime_id: 1,
                anime_title: "Frieren".into(),
                old_status: WatchStatus::Watching,
                new_status: WatchStatus::Completed,
                source: ChangeSource::Manual,
                timestamp: Utc::now(),
            },
            DomainEvent::ScoreUpdated {
                anime_id: 1,
                anime_title: "Frieren".into(),
                old_score: None,
                new_score: Some(9.0),
                source: ChangeSource::Manual,
                timestamp: Utc::now(),
            },
            DomainEvent::NothingPlaying {
                timestamp: Utc::now(),
            },
            DomainEvent::EntryDeleted {
                anime_id: 1,
                anime_title: "Frieren".into(),
                source: ChangeSource::Manual,
                timestamp: Utc::now(),
            },
        ];
        for event in &cases {
            assert!(
                debug_events_from_domain(event, "q").is_empty(),
                "Expected empty for {event:?}"
            );
        }
    }
}
