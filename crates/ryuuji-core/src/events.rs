//! Typed domain events produced by state changes.
//!
//! Every meaningful state change in the detection pipeline or library
//! management produces a `DomainEvent`. These events decouple the domain
//! from side-effects like debug logging, service sync, and history tracking.

use chrono::{DateTime, Utc};

use crate::models::WatchStatus;

/// A typed event from a domain state change.
#[derive(Debug, Clone)]
pub enum DomainEvent {
    /// Episode progress was updated.
    EpisodeUpdated {
        anime_id: i64,
        anime_title: String,
        old_episode: u32,
        new_episode: u32,
        source: ChangeSource,
        timestamp: DateTime<Utc>,
    },
    /// Anime was added to the library for the first time.
    AddedToLibrary {
        anime_id: i64,
        anime_title: String,
        initial_status: WatchStatus,
        initial_episode: u32,
        source: ChangeSource,
        timestamp: DateTime<Utc>,
    },
    /// Library entry status changed.
    StatusChanged {
        anime_id: i64,
        anime_title: String,
        old_status: WatchStatus,
        new_status: WatchStatus,
        source: ChangeSource,
        timestamp: DateTime<Utc>,
    },
    /// Score was updated.
    ScoreUpdated {
        anime_id: i64,
        anime_title: String,
        old_score: Option<f32>,
        new_score: Option<f32>,
        source: ChangeSource,
        timestamp: DateTime<Utc>,
    },
    /// Library entry was deleted.
    EntryDeleted {
        anime_id: i64,
        anime_title: String,
        source: ChangeSource,
        timestamp: DateTime<Utc>,
    },
    /// Detected title could not be matched to any known anime.
    Unrecognized {
        raw_title: String,
        timestamp: DateTime<Utc>,
    },
    /// Detected anime/episode matches current progress — no update needed.
    AlreadyCurrent {
        anime_id: i64,
        anime_title: String,
        episode: u32,
        timestamp: DateTime<Utc>,
    },
    /// Nothing is currently playing.
    NothingPlaying { timestamp: DateTime<Utc> },
}

/// What triggered the state change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeSource {
    /// Automatic detection from a media player.
    Detection,
    /// Manual user action in the GUI.
    Manual,
    /// Bulk import from an external service (MAL, AniList, Kitsu).
    ServiceImport,
}
