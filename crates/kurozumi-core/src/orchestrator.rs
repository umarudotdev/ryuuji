use chrono::Utc;
use tracing::{debug, info, warn};

use crate::config::AppConfig;
use crate::error::KurozumiError;
use crate::matcher::MatchResult;
use crate::models::{DetectedMedia, LibraryEntry, WatchStatus};
use crate::recognition::RecognitionCache;
use crate::storage::Storage;

/// Outcome of processing a detection event.
#[derive(Debug, Clone)]
pub enum UpdateOutcome {
    /// Episode progress was updated.
    Updated { anime_title: String, episode: u32 },
    /// Already at this episode or beyond — no update needed.
    AlreadyCurrent { anime_title: String, episode: u32 },
    /// Anime was recognized but no library entry exists yet — created one.
    AddedToLibrary { anime_title: String, episode: u32 },
    /// Could not match the detected title to any known anime.
    Unrecognized { raw_title: String },
    /// Nothing is currently playing.
    NothingPlaying,
}

/// Process a detection result: match against library, update progress.
pub fn process_detection(
    detected: &DetectedMedia,
    storage: &Storage,
    config: &AppConfig,
    cache: &mut RecognitionCache,
) -> Result<UpdateOutcome, KurozumiError> {
    let title = match &detected.anime_title {
        Some(t) => t,
        None => {
            return Ok(UpdateOutcome::Unrecognized {
                raw_title: detected.raw_title.clone(),
            });
        }
    };

    let episode = match detected.episode {
        Some(ep) => ep,
        None => {
            debug!(title = %title, "No episode number detected, skipping update");
            return Ok(UpdateOutcome::Unrecognized {
                raw_title: detected.raw_title.clone(),
            });
        }
    };

    // Try to match against all known anime using the recognition cache.
    let match_result = cache.recognize(title, storage);

    match match_result {
        MatchResult::Matched(anime) | MatchResult::Fuzzy(anime, _) => {
            let anime_title = anime.title.preferred().to_string();

            // Check existing library entry.
            match storage.get_library_entry_for_anime(anime.id)? {
                Some(entry) => {
                    if episode > entry.watched_episodes {
                        if config.library.auto_update {
                            storage.update_episode_count(anime.id, episode)?;
                            storage.record_watch(anime.id, episode)?;
                            info!(title = %anime_title, episode, "Updated progress");
                            Ok(UpdateOutcome::Updated {
                                anime_title,
                                episode,
                            })
                        } else {
                            debug!(title = %anime_title, episode, "Auto-update disabled");
                            Ok(UpdateOutcome::AlreadyCurrent {
                                anime_title,
                                episode,
                            })
                        }
                    } else {
                        debug!(
                            title = %anime_title,
                            current = entry.watched_episodes,
                            detected = episode,
                            "Already at or past this episode"
                        );
                        Ok(UpdateOutcome::AlreadyCurrent {
                            anime_title,
                            episode,
                        })
                    }
                }
                None => {
                    // No library entry — auto-add as Watching.
                    let entry = LibraryEntry {
                        id: 0,
                        anime_id: anime.id,
                        status: WatchStatus::Watching,
                        watched_episodes: episode,
                        score: None,
                        updated_at: Utc::now(),
                    };
                    storage.upsert_library_entry(&entry)?;
                    storage.record_watch(anime.id, episode)?;
                    info!(title = %anime_title, episode, "Added to library");
                    Ok(UpdateOutcome::AddedToLibrary {
                        anime_title,
                        episode,
                    })
                }
            }
        }
        MatchResult::NoMatch => {
            warn!(title = %title, "No match found in local library");
            Ok(UpdateOutcome::Unrecognized {
                raw_title: detected.raw_title.clone(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Anime, AnimeIds, AnimeTitle};

    fn setup() -> (Storage, AppConfig, RecognitionCache) {
        let storage = Storage::open_memory().unwrap();
        let config = AppConfig::default();
        let cache = RecognitionCache::new();
        (storage, config, cache)
    }

    fn insert_frieren(storage: &Storage) -> i64 {
        storage
            .insert_anime(&Anime {
                id: 0,
                ids: AnimeIds::default(),
                title: AnimeTitle {
                    romaji: Some("Sousou no Frieren".into()),
                    english: Some("Frieren: Beyond Journey's End".into()),
                    native: None,
                },
                synonyms: vec!["Frieren".into()],
                episodes: Some(28),
                cover_url: None,
                season: None,
                year: None,
            })
            .unwrap()
    }

    fn detected(title: &str, episode: u32) -> DetectedMedia {
        DetectedMedia {
            player_name: "mpv".into(),
            anime_title: Some(title.into()),
            episode: Some(episode),
            release_group: None,
            resolution: None,
            raw_title: format!("[Group] {title} - {episode:02} [1080p].mkv"),
        }
    }

    #[test]
    fn test_adds_to_library_on_first_detection() {
        let (storage, config, mut cache) = setup();
        insert_frieren(&storage);

        let result = process_detection(
            &detected("Sousou no Frieren", 1),
            &storage,
            &config,
            &mut cache,
        );
        match result.unwrap() {
            UpdateOutcome::AddedToLibrary { episode, .. } => assert_eq!(episode, 1),
            other => panic!("Expected AddedToLibrary, got {other:?}"),
        }

        let entry = storage.get_library_entry_for_anime(1).unwrap().unwrap();
        assert_eq!(entry.watched_episodes, 1);
        assert_eq!(entry.status, WatchStatus::Watching);
    }

    #[test]
    fn test_updates_progress() {
        let (storage, config, mut cache) = setup();
        let anime_id = insert_frieren(&storage);

        // First detection creates entry.
        process_detection(
            &detected("Sousou no Frieren", 3),
            &storage,
            &config,
            &mut cache,
        )
        .unwrap();

        // Second detection with higher episode updates.
        let result = process_detection(
            &detected("Sousou no Frieren", 5),
            &storage,
            &config,
            &mut cache,
        );
        match result.unwrap() {
            UpdateOutcome::Updated { episode, .. } => assert_eq!(episode, 5),
            other => panic!("Expected Updated, got {other:?}"),
        }

        let entry = storage
            .get_library_entry_for_anime(anime_id)
            .unwrap()
            .unwrap();
        assert_eq!(entry.watched_episodes, 5);
    }

    #[test]
    fn test_already_current() {
        let (storage, config, mut cache) = setup();
        insert_frieren(&storage);

        process_detection(
            &detected("Sousou no Frieren", 5),
            &storage,
            &config,
            &mut cache,
        )
        .unwrap();

        // Same episode again.
        let result = process_detection(
            &detected("Sousou no Frieren", 5),
            &storage,
            &config,
            &mut cache,
        );
        assert!(matches!(
            result.unwrap(),
            UpdateOutcome::AlreadyCurrent { .. }
        ));
    }

    #[test]
    fn test_unrecognized() {
        let (storage, config, mut cache) = setup();
        // DB is empty, so nothing matches.
        let result =
            process_detection(&detected("Unknown Anime", 1), &storage, &config, &mut cache);
        assert!(matches!(
            result.unwrap(),
            UpdateOutcome::Unrecognized { .. }
        ));
    }
}
