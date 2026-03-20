//! Pure policy functions extracted from the detection pipeline.
//!
//! Each function encapsulates a single business rule and is independently
//! testable without storage or I/O. The orchestrator chains these policies
//! into the full detection flow.

use chrono::{DateTime, Utc};

use crate::events::{ChangeSource, DomainEvent};
use crate::models::{DetectedMedia, LibraryEntry, WatchStatus};

/// Media info extracted from a detection result.
#[derive(Debug, Clone)]
pub struct ExtractedMedia {
    pub title: String,
    pub episode: u32,
    pub raw_title: String,
}

/// Decision about whether/how to update episode progress.
#[derive(Debug, Clone)]
pub enum ProgressDecision {
    /// Episode is ahead of current progress — update it.
    Update {
        anime_id: i64,
        anime_title: String,
        new_episode: u32,
        old_episode: u32,
    },
    /// Already at or past this episode — no update needed.
    AlreadyCurrent {
        anime_id: i64,
        anime_title: String,
        episode: u32,
    },
    /// Auto-update is disabled — suppress the update.
    Suppressed {
        anime_id: i64,
        anime_title: String,
        episode: u32,
    },
}

/// Extract title and episode from a detection result.
///
/// Returns `None` if either title or episode is missing.
pub fn extract_media(detected: &DetectedMedia) -> Option<ExtractedMedia> {
    let title = detected.anime_title.as_ref()?;
    let episode = detected.episode?;
    Some(ExtractedMedia {
        title: title.clone(),
        episode,
        raw_title: detected.raw_title.clone(),
    })
}

/// Evaluate whether episode progress should be updated.
///
/// Pure function: takes the current library state and returns a decision
/// with no side effects.
pub fn evaluate_progress(
    entry: &LibraryEntry,
    anime_id: i64,
    anime_title: &str,
    detected_episode: u32,
    auto_update: bool,
) -> ProgressDecision {
    if detected_episode > entry.watched_episodes {
        if auto_update {
            ProgressDecision::Update {
                anime_id,
                anime_title: anime_title.to_string(),
                new_episode: detected_episode,
                old_episode: entry.watched_episodes,
            }
        } else {
            ProgressDecision::Suppressed {
                anime_id,
                anime_title: anime_title.to_string(),
                episode: detected_episode,
            }
        }
    } else {
        ProgressDecision::AlreadyCurrent {
            anime_id,
            anime_title: anime_title.to_string(),
            episode: detected_episode,
        }
    }
}

/// Build a new library entry and domain event for a first-time detection.
///
/// Pure function: constructs the entry struct and event without persisting.
pub fn create_initial_entry(
    anime_id: i64,
    anime_title: &str,
    episode: u32,
    now: DateTime<Utc>,
) -> (LibraryEntry, DomainEvent) {
    let entry = LibraryEntry {
        id: 0,
        anime_id,
        status: WatchStatus::Watching,
        watched_episodes: episode,
        score: None,
        updated_at: now,
        start_date: None,
        finish_date: None,
        notes: None,
        rewatching: false,
        rewatch_count: 0,
    };
    let event = DomainEvent::AddedToLibrary {
        anime_id,
        anime_title: anime_title.to_string(),
        initial_status: WatchStatus::Watching,
        initial_episode: episode,
        source: ChangeSource::Detection,
        timestamp: now,
    };
    (entry, event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DetectedMedia;

    fn detected(title: Option<&str>, episode: Option<u32>) -> DetectedMedia {
        DetectedMedia {
            player_name: "mpv".into(),
            anime_title: title.map(Into::into),
            episode,
            release_group: None,
            resolution: None,
            raw_title: title.unwrap_or("unknown").to_string(),
            service_name: None,
        }
    }

    fn library_entry(anime_id: i64, watched_episodes: u32) -> LibraryEntry {
        LibraryEntry {
            id: 1,
            anime_id,
            status: WatchStatus::Watching,
            watched_episodes,
            score: None,
            updated_at: Utc::now(),
            start_date: None,
            finish_date: None,
            notes: None,
            rewatching: false,
            rewatch_count: 0,
        }
    }

    // ── extract_media tests ──────────────────────────────────────

    #[test]
    fn extract_media_returns_some_when_both_present() {
        let d = detected(Some("Frieren"), Some(5));
        let m = extract_media(&d).unwrap();
        assert_eq!(m.title, "Frieren");
        assert_eq!(m.episode, 5);
    }

    #[test]
    fn extract_media_returns_none_when_title_missing() {
        let d = detected(None, Some(5));
        assert!(extract_media(&d).is_none());
    }

    #[test]
    fn extract_media_returns_none_when_episode_missing() {
        let d = detected(Some("Frieren"), None);
        assert!(extract_media(&d).is_none());
    }

    // ── evaluate_progress tests ──────────────────────────────────

    #[test]
    fn progress_update_when_ahead() {
        let entry = library_entry(1, 3);
        match evaluate_progress(&entry, 1, "Frieren", 5, true) {
            ProgressDecision::Update {
                new_episode,
                old_episode,
                ..
            } => {
                assert_eq!(new_episode, 5);
                assert_eq!(old_episode, 3);
            }
            other => panic!("Expected Update, got {other:?}"),
        }
    }

    #[test]
    fn progress_already_current_when_same() {
        let entry = library_entry(1, 5);
        assert!(matches!(
            evaluate_progress(&entry, 1, "Frieren", 5, true),
            ProgressDecision::AlreadyCurrent { episode: 5, .. }
        ));
    }

    #[test]
    fn progress_already_current_when_behind() {
        let entry = library_entry(1, 10);
        assert!(matches!(
            evaluate_progress(&entry, 1, "Frieren", 5, true),
            ProgressDecision::AlreadyCurrent { episode: 5, .. }
        ));
    }

    #[test]
    fn progress_suppressed_when_auto_update_disabled() {
        let entry = library_entry(1, 3);
        assert!(matches!(
            evaluate_progress(&entry, 1, "Frieren", 5, false),
            ProgressDecision::Suppressed { episode: 5, .. }
        ));
    }

    // ── create_initial_entry tests ───────────────────────────────

    #[test]
    fn initial_entry_has_watching_status() {
        let now = Utc::now();
        let (entry, event) = create_initial_entry(42, "Frieren", 3, now);
        assert_eq!(entry.status, WatchStatus::Watching);
        assert_eq!(entry.watched_episodes, 3);
        assert_eq!(entry.anime_id, 42);
        assert!(matches!(
            event,
            DomainEvent::AddedToLibrary {
                initial_episode: 3,
                ..
            }
        ));
    }
}
