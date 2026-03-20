use chrono::Utc;
use tracing::{debug, info, warn};

use crate::config::AppConfig;
use crate::error::RyuujiError;
use crate::events::{ChangeSource, DomainEvent};
use crate::matcher::MatchResult;
use crate::models::DetectedMedia;
use crate::policy::{self, ProgressDecision};
use crate::recognition::RecognitionCache;
use crate::relations::RelationDatabase;
use crate::repository::{AnimeRepository, LibraryRepository, WatchHistoryRepository};

/// Outcome of processing a detection event.
#[derive(Debug, Clone)]
pub enum UpdateOutcome {
    /// Episode progress was updated.
    Updated {
        anime_id: i64,
        anime_title: String,
        episode: u32,
    },
    /// Already at this episode or beyond — no update needed.
    AlreadyCurrent {
        anime_id: i64,
        anime_title: String,
        episode: u32,
    },
    /// Anime was recognized but no library entry exists yet — created one.
    AddedToLibrary {
        anime_id: i64,
        anime_title: String,
        episode: u32,
    },
    /// Could not match the detected title to any known anime.
    Unrecognized { raw_title: String },
    /// Nothing is currently playing.
    NothingPlaying,
}

/// Process a detection result: match against library, update progress.
///
/// If a `relations` database is provided, episode numbers may be remapped
/// to handle cross-season continuous numbering (e.g., episode 26 → S2E1).
#[tracing::instrument(
    name = "process_detection",
    skip(repo, config, cache, relations),
    fields(
        raw_title = %detected.raw_title,
        player = %detected.player_name,
    )
)]
pub fn process_detection(
    detected: &DetectedMedia,
    repo: &(impl AnimeRepository + LibraryRepository + WatchHistoryRepository),
    config: &AppConfig,
    cache: &mut RecognitionCache,
    relations: Option<&RelationDatabase>,
) -> Result<(UpdateOutcome, Vec<DomainEvent>), RyuujiError> {
    let now = Utc::now();

    // 1. Extract title + episode from detection result.
    let media = match policy::extract_media(detected) {
        Some(m) => m,
        None => {
            debug!(raw = %detected.raw_title, "Missing title or episode");
            let event = DomainEvent::Unrecognized {
                raw_title: detected.raw_title.clone(),
                timestamp: now,
            };
            return Ok((
                UpdateOutcome::Unrecognized {
                    raw_title: detected.raw_title.clone(),
                },
                vec![event],
            ));
        }
    };

    // 2. Match against known anime via recognition cache.
    let match_result = cache.recognize(&media.title, repo);
    let anime = match match_result {
        MatchResult::Matched(anime) | MatchResult::Fuzzy(anime, _) => anime,
        MatchResult::NoMatch => {
            warn!(title = %media.title, "No match found in local library");
            let event = DomainEvent::Unrecognized {
                raw_title: detected.raw_title.clone(),
                timestamp: now,
            };
            return Ok((
                UpdateOutcome::Unrecognized {
                    raw_title: detected.raw_title.clone(),
                },
                vec![event],
            ));
        }
    };

    // 3. Resolve episode relation redirects (cross-season mapping).
    let mut target_anime_id = anime.id;
    let mut target_episode = media.episode;
    let mut anime_title = anime.title.preferred().to_string();

    if let Some(relations) = relations {
        if let Some(mal_id) = anime.ids.mal {
            if let Some(redirect) = relations.redirect_mal(mal_id, media.episode) {
                if let Some(dest_mal) = redirect.dest_mal {
                    if let Ok(Some(dest_anime)) = repo.get_anime_by_mal_id(dest_mal) {
                        debug!(
                            from_title = %anime_title,
                            from_ep = media.episode,
                            to_title = %dest_anime.title.preferred(),
                            to_ep = redirect.dest_episode,
                            "Episode relation redirect"
                        );
                        target_anime_id = dest_anime.id;
                        target_episode = redirect.dest_episode;
                        anime_title = dest_anime.title.preferred().to_string();
                    }
                }
            }
        }
    }

    // 4. Decide on progress update and execute.
    match repo.get_library_entry_for_anime(target_anime_id)? {
        Some(entry) => {
            let decision = policy::evaluate_progress(
                &entry,
                target_anime_id,
                &anime_title,
                target_episode,
                config.library.auto_update,
            );

            match decision {
                ProgressDecision::Update {
                    anime_id,
                    anime_title,
                    new_episode,
                    old_episode,
                } => {
                    repo.update_episode_count(anime_id, new_episode)?;
                    repo.record_watch(anime_id, new_episode)?;
                    info!(title = %anime_title, episode = new_episode, "Updated progress");
                    let event = DomainEvent::EpisodeUpdated {
                        anime_id,
                        anime_title: anime_title.clone(),
                        old_episode,
                        new_episode,
                        source: ChangeSource::Detection,
                        timestamp: now,
                    };
                    Ok((
                        UpdateOutcome::Updated {
                            anime_id,
                            anime_title,
                            episode: new_episode,
                        },
                        vec![event],
                    ))
                }
                ProgressDecision::AlreadyCurrent {
                    anime_id,
                    anime_title,
                    episode,
                }
                | ProgressDecision::Suppressed {
                    anime_id,
                    anime_title,
                    episode,
                } => {
                    debug!(title = %anime_title, episode, "No update");
                    let event = DomainEvent::AlreadyCurrent {
                        anime_id,
                        anime_title: anime_title.clone(),
                        episode,
                        timestamp: now,
                    };
                    Ok((
                        UpdateOutcome::AlreadyCurrent {
                            anime_id,
                            anime_title,
                            episode,
                        },
                        vec![event],
                    ))
                }
            }
        }
        None => {
            // No library entry — auto-add as Watching.
            let (entry, event) =
                policy::create_initial_entry(target_anime_id, &anime_title, target_episode, now);
            repo.upsert_library_entry(&entry)?;
            repo.record_watch(target_anime_id, target_episode)?;
            info!(title = %anime_title, episode = target_episode, "Added to library");
            Ok((
                UpdateOutcome::AddedToLibrary {
                    anime_id: target_anime_id,
                    anime_title,
                    episode: target_episode,
                },
                vec![event],
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Anime, AnimeIds, AnimeTitle, WatchStatus};
    use crate::storage::Storage;

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
                synopsis: None,
                genres: vec![],
                media_type: None,
                airing_status: None,
                mean_score: None,
                studios: vec![],
                source: None,
                rating: None,
                start_date: None,
                end_date: None,
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
            service_name: None,
        }
    }

    #[test]
    fn test_adds_to_library_on_first_detection() {
        let (storage, config, mut cache) = setup();
        insert_frieren(&storage);

        let (outcome, events) = process_detection(
            &detected("Sousou no Frieren", 1),
            &storage,
            &config,
            &mut cache,
            None,
        )
        .unwrap();
        match outcome {
            UpdateOutcome::AddedToLibrary { episode, .. } => assert_eq!(episode, 1),
            other => panic!("Expected AddedToLibrary, got {other:?}"),
        }
        assert!(matches!(events[0], DomainEvent::AddedToLibrary { .. }));

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
            None,
        )
        .unwrap();

        // Second detection with higher episode updates.
        let (outcome, events) = process_detection(
            &detected("Sousou no Frieren", 5),
            &storage,
            &config,
            &mut cache,
            None,
        )
        .unwrap();
        match outcome {
            UpdateOutcome::Updated { episode, .. } => assert_eq!(episode, 5),
            other => panic!("Expected Updated, got {other:?}"),
        }
        assert!(matches!(
            events[0],
            DomainEvent::EpisodeUpdated {
                old_episode: 3,
                new_episode: 5,
                ..
            }
        ));

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
            None,
        )
        .unwrap();

        // Same episode again.
        let (outcome, _events) = process_detection(
            &detected("Sousou no Frieren", 5),
            &storage,
            &config,
            &mut cache,
            None,
        )
        .unwrap();
        assert!(matches!(outcome, UpdateOutcome::AlreadyCurrent { .. }));
    }

    #[test]
    fn test_unrecognized() {
        let (storage, config, mut cache) = setup();
        // DB is empty, so nothing matches.
        let (outcome, events) = process_detection(
            &detected("Unknown Anime", 1),
            &storage,
            &config,
            &mut cache,
            None,
        )
        .unwrap();
        assert!(matches!(outcome, UpdateOutcome::Unrecognized { .. }));
        assert!(matches!(events[0], DomainEvent::Unrecognized { .. }));
    }

    #[test]
    fn test_suppressed_when_auto_update_disabled() {
        let storage = Storage::open_memory().unwrap();
        let mut config = AppConfig::default();
        config.library.auto_update = false;
        let mut cache = RecognitionCache::new();

        insert_frieren(&storage);

        // First detection adds to library (auto-add always works).
        process_detection(
            &detected("Sousou no Frieren", 1),
            &storage,
            &config,
            &mut cache,
            None,
        )
        .unwrap();

        // Higher episode — but auto_update is disabled, so suppressed.
        let (outcome, _events) = process_detection(
            &detected("Sousou no Frieren", 5),
            &storage,
            &config,
            &mut cache,
            None,
        )
        .unwrap();
        assert!(matches!(outcome, UpdateOutcome::AlreadyCurrent { .. }));

        // Episode count should NOT have been updated.
        let entry = storage.get_library_entry_for_anime(1).unwrap().unwrap();
        assert_eq!(entry.watched_episodes, 1);
    }

    #[test]
    fn test_episode_redirect_via_relations() {
        let storage = Storage::open_memory().unwrap();
        let config = AppConfig::default();
        let mut cache = RecognitionCache::new();

        // Insert source anime (continuous numbering) with MAL ID.
        let source_id = storage
            .insert_anime(&Anime {
                id: 0,
                ids: AnimeIds {
                    mal: Some(16498),
                    ..Default::default()
                },
                title: AnimeTitle {
                    romaji: Some("Shingeki no Kyojin".into()),
                    english: Some("Attack on Titan".into()),
                    native: None,
                },
                synonyms: vec![],
                episodes: Some(25),
                cover_url: None,
                season: None,
                year: None,
                synopsis: None,
                genres: vec![],
                media_type: None,
                airing_status: None,
                mean_score: None,
                studios: vec![],
                source: None,
                rating: None,
                start_date: None,
                end_date: None,
            })
            .unwrap();

        // Insert dest anime (season 2) with its own MAL ID.
        let dest_id = storage
            .insert_anime(&Anime {
                id: 0,
                ids: AnimeIds {
                    mal: Some(25777),
                    ..Default::default()
                },
                title: AnimeTitle {
                    romaji: Some("Shingeki no Kyojin Season 2".into()),
                    english: Some("Attack on Titan Season 2".into()),
                    native: None,
                },
                synonyms: vec![],
                episodes: Some(12),
                cover_url: None,
                season: None,
                year: None,
                synopsis: None,
                genres: vec![],
                media_type: None,
                airing_status: None,
                mean_score: None,
                studios: vec![],
                source: None,
                rating: None,
                start_date: None,
                end_date: None,
            })
            .unwrap();

        // Build a minimal relation database with a redirect rule.
        use crate::relations::{EpisodeRange, RelationDatabase, RelationRule};
        let mut relations = RelationDatabase::new();
        relations.by_mal.insert(
            16498,
            vec![RelationRule {
                source_mal: Some(16498),
                source_kitsu: None,
                source_anilist: None,
                source_episodes: EpisodeRange { start: 26, end: 37 },
                dest_mal: Some(25777),
                dest_kitsu: None,
                dest_anilist: None,
                dest_episodes: EpisodeRange { start: 1, end: 12 },
            }],
        );

        // Detect "episode 26" of source → should redirect to dest episode 1.
        let (outcome, _events) = process_detection(
            &detected("Shingeki no Kyojin", 26),
            &storage,
            &config,
            &mut cache,
            Some(&relations),
        )
        .unwrap();

        match outcome {
            UpdateOutcome::AddedToLibrary {
                anime_id, episode, ..
            } => {
                assert_eq!(anime_id, dest_id);
                assert_eq!(episode, 1);
            }
            other => panic!("Expected AddedToLibrary for dest anime, got {other:?}"),
        }

        // Source anime should NOT have a library entry.
        assert!(storage
            .get_library_entry_for_anime(source_id)
            .unwrap()
            .is_none());
        // Dest anime should have episode 1.
        let dest_entry = storage
            .get_library_entry_for_anime(dest_id)
            .unwrap()
            .unwrap();
        assert_eq!(dest_entry.watched_episodes, 1);
    }

    #[test]
    fn test_missing_episode_is_unrecognized() {
        let (storage, config, mut cache) = setup();
        insert_frieren(&storage);

        let d = DetectedMedia {
            player_name: "mpv".into(),
            anime_title: Some("Sousou no Frieren".into()),
            episode: None,
            release_group: None,
            resolution: None,
            raw_title: "Sousou no Frieren.mkv".into(),
            service_name: None,
        };
        let (outcome, _) = process_detection(&d, &storage, &config, &mut cache, None).unwrap();
        assert!(matches!(outcome, UpdateOutcome::Unrecognized { .. }));
    }
}
