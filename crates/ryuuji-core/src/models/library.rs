use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::events::{ChangeSource, DomainEvent};

/// User's watch status for a library entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WatchStatus {
    Watching,
    Completed,
    OnHold,
    Dropped,
    PlanToWatch,
}

impl WatchStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Watching => "Watching",
            Self::Completed => "Completed",
            Self::OnHold => "On Hold",
            Self::Dropped => "Dropped",
            Self::PlanToWatch => "Plan to Watch",
        }
    }

    /// Database string representation (lowercase, no spaces).
    pub fn as_db_str(&self) -> &'static str {
        match self {
            Self::Watching => "watching",
            Self::Completed => "completed",
            Self::OnHold => "on_hold",
            Self::Dropped => "dropped",
            Self::PlanToWatch => "plan_to_watch",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "watching" => Some(Self::Watching),
            "completed" => Some(Self::Completed),
            "on_hold" => Some(Self::OnHold),
            "dropped" => Some(Self::Dropped),
            "plan_to_watch" => Some(Self::PlanToWatch),
            _ => None,
        }
    }

    pub const ALL: &[WatchStatus] = &[
        Self::Watching,
        Self::Completed,
        Self::OnHold,
        Self::Dropped,
        Self::PlanToWatch,
    ];

    /// Returns the set of statuses this status can transition to.
    pub fn valid_transitions(&self) -> &'static [WatchStatus] {
        match self {
            Self::PlanToWatch => &[Self::Watching],
            Self::Watching => &[Self::Completed, Self::OnHold, Self::Dropped],
            Self::OnHold => &[Self::Watching, Self::Dropped],
            Self::Completed => &[Self::Watching],
            Self::Dropped => &[Self::Watching, Self::PlanToWatch],
        }
    }

    /// Check if transitioning to `target` is valid.
    pub fn can_transition_to(&self, target: WatchStatus) -> bool {
        self.valid_transitions().contains(&target)
    }
}

/// Error for invalid status transitions.
#[derive(Debug, Clone, thiserror::Error)]
#[error("cannot transition from {from} to {to}")]
pub struct InvalidTransition {
    pub from: WatchStatus,
    pub to: WatchStatus,
}

impl std::fmt::Display for WatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An episode file found on disk by the folder scanner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableEpisode {
    pub id: i64,
    pub anime_id: i64,
    pub episode: u32,
    pub file_path: String,
    pub file_size: u64,
    pub file_modified: String,
    pub release_group: Option<String>,
    pub resolution: Option<String>,
}

/// Summary of available episodes for a library entry (for display).
#[derive(Debug, Clone, Default)]
pub struct AvailableEpisodeSummary {
    pub anime_id: i64,
    pub count: u32,
}

/// A user's library entry linking to an anime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryEntry {
    pub id: i64,
    pub anime_id: i64,
    pub status: WatchStatus,
    pub watched_episodes: u32,
    pub score: Option<f32>,
    pub updated_at: DateTime<Utc>,
    pub start_date: Option<String>,
    pub finish_date: Option<String>,
    pub notes: Option<String>,
    pub rewatching: bool,
    pub rewatch_count: u32,
}

impl LibraryEntry {
    /// Transition to a new status, validating the transition is allowed.
    ///
    /// Also handles auto-setting dates:
    /// - PlanToWatch → Watching: set `start_date` if not set
    /// - * → Completed: set `finish_date` if not set
    pub fn transition_status(
        &mut self,
        new_status: WatchStatus,
        anime_title: &str,
        source: ChangeSource,
    ) -> Result<DomainEvent, InvalidTransition> {
        if !self.status.can_transition_to(new_status) {
            return Err(InvalidTransition {
                from: self.status,
                to: new_status,
            });
        }

        let old_status = self.status;
        let now = Utc::now();

        if new_status == WatchStatus::Watching && self.start_date.is_none() {
            self.start_date = Some(now.format("%Y-%m-%d").to_string());
        }

        if new_status == WatchStatus::Completed && self.finish_date.is_none() {
            self.finish_date = Some(now.format("%Y-%m-%d").to_string());
        }

        self.status = new_status;
        self.updated_at = now;

        Ok(DomainEvent::StatusChanged {
            anime_id: self.anime_id,
            anime_title: anime_title.to_string(),
            old_status,
            new_status,
            source,
            timestamp: now,
        })
    }

    /// Increment episode progress. Only valid when Watching.
    pub fn increment_episode(
        &mut self,
        new_ep: u32,
        anime_title: &str,
        source: ChangeSource,
    ) -> Result<DomainEvent, InvalidTransition> {
        if self.status != WatchStatus::Watching && self.status != WatchStatus::Completed {
            return Err(InvalidTransition {
                from: self.status,
                to: self.status,
            });
        }

        let old_episode = self.watched_episodes;
        let now = Utc::now();
        self.watched_episodes = new_ep;
        self.updated_at = now;

        Ok(DomainEvent::EpisodeUpdated {
            anime_id: self.anime_id,
            anime_title: anime_title.to_string(),
            old_episode,
            new_episode: new_ep,
            source,
            timestamp: now,
        })
    }

    /// Set the score. Always valid — no transition check needed.
    pub fn set_score(
        &mut self,
        score: Option<f32>,
        anime_title: &str,
        source: ChangeSource,
    ) -> DomainEvent {
        let old_score = self.score;
        let now = Utc::now();
        self.score = score;
        self.updated_at = now;

        DomainEvent::ScoreUpdated {
            anime_id: self.anime_id,
            anime_title: anime_title.to_string(),
            old_score,
            new_score: score,
            source,
            timestamp: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(status: WatchStatus) -> LibraryEntry {
        LibraryEntry {
            id: 1,
            anime_id: 42,
            status,
            watched_episodes: 5,
            score: None,
            updated_at: Utc::now(),
            start_date: None,
            finish_date: None,
            notes: None,
            rewatching: false,
            rewatch_count: 0,
        }
    }

    #[test]
    fn plan_to_watch_can_transition_to_watching() {
        assert!(WatchStatus::PlanToWatch.can_transition_to(WatchStatus::Watching));
    }

    #[test]
    fn watching_can_transition_to_completed() {
        assert!(WatchStatus::Watching.can_transition_to(WatchStatus::Completed));
    }

    #[test]
    fn watching_cannot_transition_to_plan_to_watch() {
        assert!(!WatchStatus::Watching.can_transition_to(WatchStatus::PlanToWatch));
    }

    #[test]
    fn completed_can_rewatch() {
        assert!(WatchStatus::Completed.can_transition_to(WatchStatus::Watching));
    }

    #[test]
    fn dropped_can_return_to_watching() {
        assert!(WatchStatus::Dropped.can_transition_to(WatchStatus::Watching));
    }

    #[test]
    fn dropped_can_return_to_plan_to_watch() {
        assert!(WatchStatus::Dropped.can_transition_to(WatchStatus::PlanToWatch));
    }

    #[test]
    fn valid_transition_updates_status() {
        let mut e = entry(WatchStatus::Watching);
        let event = e
            .transition_status(WatchStatus::Completed, "Frieren", ChangeSource::Manual)
            .unwrap();
        assert_eq!(e.status, WatchStatus::Completed);
        assert!(matches!(event, DomainEvent::StatusChanged { .. }));
    }

    #[test]
    fn invalid_transition_returns_error() {
        let mut e = entry(WatchStatus::Watching);
        let result = e.transition_status(WatchStatus::PlanToWatch, "Frieren", ChangeSource::Manual);
        assert!(result.is_err());
        assert_eq!(e.status, WatchStatus::Watching);
    }

    #[test]
    fn transition_to_watching_sets_start_date() {
        let mut e = entry(WatchStatus::PlanToWatch);
        assert!(e.start_date.is_none());
        e.transition_status(WatchStatus::Watching, "Frieren", ChangeSource::Manual)
            .unwrap();
        assert!(e.start_date.is_some());
    }

    #[test]
    fn transition_to_completed_sets_finish_date() {
        let mut e = entry(WatchStatus::Watching);
        assert!(e.finish_date.is_none());
        e.transition_status(WatchStatus::Completed, "Frieren", ChangeSource::Manual)
            .unwrap();
        assert!(e.finish_date.is_some());
    }

    #[test]
    fn increment_episode_while_watching() {
        let mut e = entry(WatchStatus::Watching);
        let event = e
            .increment_episode(10, "Frieren", ChangeSource::Detection)
            .unwrap();
        assert_eq!(e.watched_episodes, 10);
        assert!(matches!(
            event,
            DomainEvent::EpisodeUpdated {
                old_episode: 5,
                new_episode: 10,
                ..
            }
        ));
    }

    #[test]
    fn increment_episode_while_on_hold_fails() {
        let mut e = entry(WatchStatus::OnHold);
        assert!(e
            .increment_episode(10, "Frieren", ChangeSource::Manual)
            .is_err());
        assert_eq!(e.watched_episodes, 5);
    }

    #[test]
    fn set_score_produces_event() {
        let mut e = entry(WatchStatus::Watching);
        let event = e.set_score(Some(8.5), "Frieren", ChangeSource::Manual);
        assert_eq!(e.score, Some(8.5));
        assert!(matches!(
            event,
            DomainEvent::ScoreUpdated {
                old_score: None,
                new_score: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn clear_score_produces_event() {
        let mut e = entry(WatchStatus::Watching);
        e.score = Some(7.0);
        let event = e.set_score(None, "Frieren", ChangeSource::Manual);
        assert_eq!(e.score, None);
        assert!(matches!(
            event,
            DomainEvent::ScoreUpdated {
                old_score: Some(_),
                new_score: None,
                ..
            }
        ));
    }

    #[test]
    fn all_invalid_transitions_are_rejected() {
        let invalid_pairs = [
            (WatchStatus::PlanToWatch, WatchStatus::Completed),
            (WatchStatus::PlanToWatch, WatchStatus::OnHold),
            (WatchStatus::PlanToWatch, WatchStatus::Dropped),
            (WatchStatus::Watching, WatchStatus::PlanToWatch),
            (WatchStatus::OnHold, WatchStatus::PlanToWatch),
            (WatchStatus::OnHold, WatchStatus::Completed),
            (WatchStatus::Completed, WatchStatus::OnHold),
            (WatchStatus::Completed, WatchStatus::Dropped),
            (WatchStatus::Completed, WatchStatus::PlanToWatch),
            (WatchStatus::Dropped, WatchStatus::OnHold),
            (WatchStatus::Dropped, WatchStatus::Completed),
        ];

        for (from, to) in invalid_pairs {
            assert!(
                !from.can_transition_to(to),
                "{from} should NOT be able to transition to {to}"
            );
            let mut e = entry(from);
            assert!(
                e.transition_status(to, "Test", ChangeSource::Manual)
                    .is_err(),
                "transition_status({from} → {to}) should return Err"
            );
            assert_eq!(
                e.status, from,
                "status should be unchanged after rejected transition"
            );
        }
    }

    #[test]
    fn self_transitions_are_rejected() {
        for &status in WatchStatus::ALL {
            assert!(
                !status.can_transition_to(status),
                "{status} → {status} should be invalid"
            );
        }
    }

    #[test]
    fn all_valid_transitions_are_accepted() {
        let valid_pairs = [
            (WatchStatus::PlanToWatch, WatchStatus::Watching),
            (WatchStatus::Watching, WatchStatus::Completed),
            (WatchStatus::Watching, WatchStatus::OnHold),
            (WatchStatus::Watching, WatchStatus::Dropped),
            (WatchStatus::OnHold, WatchStatus::Watching),
            (WatchStatus::OnHold, WatchStatus::Dropped),
            (WatchStatus::Completed, WatchStatus::Watching),
            (WatchStatus::Dropped, WatchStatus::Watching),
            (WatchStatus::Dropped, WatchStatus::PlanToWatch),
        ];

        for (from, to) in valid_pairs {
            assert!(
                from.can_transition_to(to),
                "{from} should be able to transition to {to}"
            );
            let mut e = entry(from);
            assert!(
                e.transition_status(to, "Test", ChangeSource::Manual)
                    .is_ok(),
                "transition_status({from} → {to}) should succeed"
            );
            assert_eq!(e.status, to);
        }
    }

    #[test]
    fn transition_to_watching_preserves_existing_start_date() {
        let mut e = entry(WatchStatus::PlanToWatch);
        e.start_date = Some("2024-01-15".to_string());
        e.transition_status(WatchStatus::Watching, "Frieren", ChangeSource::Manual)
            .unwrap();
        assert_eq!(e.start_date.as_deref(), Some("2024-01-15"));
    }

    #[test]
    fn transition_to_completed_preserves_existing_finish_date() {
        let mut e = entry(WatchStatus::Watching);
        e.finish_date = Some("2024-06-30".to_string());
        e.transition_status(WatchStatus::Completed, "Frieren", ChangeSource::Manual)
            .unwrap();
        assert_eq!(e.finish_date.as_deref(), Some("2024-06-30"));
    }

    #[test]
    fn increment_episode_while_completed_is_allowed() {
        let mut e = entry(WatchStatus::Completed);
        assert!(e
            .increment_episode(10, "Frieren", ChangeSource::Manual)
            .is_ok());
        assert_eq!(e.watched_episodes, 10);
    }

    #[test]
    fn increment_episode_while_plan_to_watch_fails() {
        let mut e = entry(WatchStatus::PlanToWatch);
        assert!(e
            .increment_episode(1, "Frieren", ChangeSource::Manual)
            .is_err());
        assert_eq!(e.watched_episodes, 5);
    }

    #[test]
    fn increment_episode_while_dropped_fails() {
        let mut e = entry(WatchStatus::Dropped);
        assert!(e
            .increment_episode(1, "Frieren", ChangeSource::Manual)
            .is_err());
        assert_eq!(e.watched_episodes, 5);
    }
}
