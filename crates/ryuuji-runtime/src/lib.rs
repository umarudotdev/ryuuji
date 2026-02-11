mod db;

use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use ryuuji_api::traits::{AnimeService, LibraryEntryUpdate};
use ryuuji_core::config::AppConfig;
use ryuuji_core::models::{Anime, AnimeIds, AnimeTitle, DetectedMedia, LibraryEntry, WatchStatus};
use ryuuji_core::orchestrator::UpdateOutcome;
use ryuuji_core::storage::LibraryRow;

pub use db::DbHandle;

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("config error: {0}")]
    Config(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("api error: {0}")]
    Api(String),
    #[error("not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DetectionStateDto {
    pub detected: Option<DetectedMedia>,
    pub outcome: Option<String>,
    pub status_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LibraryPatchDto {
    pub episode: Option<u32>,
    pub status: Option<WatchStatus>,
    pub score: Option<f32>,
    pub start_date: Option<String>,
    pub finish_date: Option<String>,
    pub notes: Option<String>,
    pub rewatching: Option<bool>,
    pub rewatch_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceLoginDto {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

pub struct Runtime {
    db: DbHandle,
    config: Arc<RwLock<AppConfig>>,
    detection_state: Arc<RwLock<DetectionStateDto>>,
}

impl Runtime {
    pub fn new() -> Result<Self, RuntimeError> {
        let config = AppConfig::load().map_err(|e| RuntimeError::Config(e.to_string()))?;
        let db_path =
            AppConfig::ensure_db_path().map_err(|e| RuntimeError::Config(e.to_string()))?;
        let db = DbHandle::open(&db_path)
            .ok_or_else(|| RuntimeError::Database("failed to open database".into()))?;

        Ok(Self {
            db,
            config: Arc::new(RwLock::new(config)),
            detection_state: Arc::new(RwLock::new(DetectionStateDto {
                status_message: "Ready".into(),
                ..Default::default()
            })),
        })
    }

    pub fn db_handle(&self) -> DbHandle {
        self.db.clone()
    }

    pub async fn get_config(&self) -> AppConfig {
        self.config.read().await.clone()
    }

    pub async fn update_config(&self, new_config: AppConfig) -> Result<(), RuntimeError> {
        new_config
            .save()
            .map_err(|e| RuntimeError::Config(e.to_string()))?;
        *self.config.write().await = new_config;
        Ok(())
    }

    pub async fn get_detection_state(&self) -> DetectionStateDto {
        self.detection_state.read().await.clone()
    }

    pub async fn run_detection_tick(&self) -> Result<DetectionStateDto, RuntimeError> {
        let detected = detect_and_parse().await;

        if detected.is_none() {
            let mut state = self.detection_state.write().await;
            state.detected = None;
            return Ok(state.clone());
        }

        let detected_media = detected.expect("checked is_some");
        let cfg = self.get_config().await;

        let outcome = self
            .db
            .process_detection(detected_media.clone(), cfg.clone())
            .await
            .map_err(|e| RuntimeError::Database(e.to_string()))?;

        if let UpdateOutcome::Updated {
            anime_id, episode, ..
        }
        | UpdateOutcome::AddedToLibrary {
            anime_id, episode, ..
        } = outcome
        {
            let _ = self
                .sync_push_update(
                    anime_id,
                    LibraryEntryUpdate {
                        episode: Some(episode),
                        ..Default::default()
                    },
                )
                .await;
        }

        let status_message = outcome_to_status(&outcome);
        let mut state = self.detection_state.write().await;
        state.detected = Some(detected_media);
        state.outcome = Some(format!("{outcome:?}"));
        state.status_message = status_message;

        Ok(state.clone())
    }

    pub async fn get_library(
        &self,
        status: Option<WatchStatus>,
        query: Option<String>,
    ) -> Result<Vec<LibraryRow>, RuntimeError> {
        let mut rows = if let Some(status) = status {
            self.db
                .get_library_by_status(status)
                .await
                .map_err(|e| RuntimeError::Database(e.to_string()))?
        } else {
            self.db
                .get_all_library()
                .await
                .map_err(|e| RuntimeError::Database(e.to_string()))?
        };

        if let Some(query) = query {
            let needle = query.to_lowercase();
            rows.retain(|r| {
                r.anime.title.preferred().to_lowercase().contains(&needle)
                    || r.anime
                        .title
                        .english
                        .as_deref()
                        .unwrap_or_default()
                        .to_lowercase()
                        .contains(&needle)
                    || r.anime
                        .title
                        .native
                        .as_deref()
                        .unwrap_or_default()
                        .to_lowercase()
                        .contains(&needle)
            });
        }

        Ok(rows)
    }

    pub async fn patch_library_entry(
        &self,
        anime_id: i64,
        patch: LibraryPatchDto,
    ) -> Result<(), RuntimeError> {
        let start_date = patch.start_date.clone();
        let finish_date = patch.finish_date.clone();
        let notes = patch.notes.clone();

        if let Some(ep) = patch.episode {
            self.db
                .update_episode_count(anime_id, ep)
                .await
                .map_err(|e| RuntimeError::Database(e.to_string()))?;
        }
        if let Some(status) = patch.status {
            self.db
                .update_library_status(anime_id, status)
                .await
                .map_err(|e| RuntimeError::Database(e.to_string()))?;
        }
        if let Some(score) = patch.score {
            self.db
                .update_library_score(anime_id, score)
                .await
                .map_err(|e| RuntimeError::Database(e.to_string()))?;
        }
        if start_date.is_some() || finish_date.is_some() {
            self.db
                .update_library_dates(anime_id, start_date.clone(), finish_date.clone())
                .await
                .map_err(|e| RuntimeError::Database(e.to_string()))?;
        }
        if notes.is_some() {
            self.db
                .update_library_notes(anime_id, notes.clone())
                .await
                .map_err(|e| RuntimeError::Database(e.to_string()))?;
        }
        if patch.rewatching.is_some() || patch.rewatch_count.is_some() {
            self.db
                .update_library_rewatch(
                    anime_id,
                    patch.rewatching.unwrap_or(false),
                    patch.rewatch_count.unwrap_or(0),
                )
                .await
                .map_err(|e| RuntimeError::Database(e.to_string()))?;
        }

        let update = LibraryEntryUpdate {
            episode: patch.episode,
            status: patch.status.map(|s| s.as_db_str().to_string()),
            score: patch.score,
            start_date,
            finish_date,
            notes,
            rewatching: patch.rewatching,
            rewatch_count: patch.rewatch_count,
        };
        let _ = self.sync_push_update(anime_id, update).await;

        Ok(())
    }

    pub async fn delete_library_entry(&self, anime_id: i64) -> Result<(), RuntimeError> {
        let _ = self.sync_delete(anime_id).await;
        self.db
            .delete_library_entry(anime_id)
            .await
            .map_err(|e| RuntimeError::Database(e.to_string()))
    }

    pub async fn service_auth_state(&self, service: &str) -> Result<bool, RuntimeError> {
        let token = self
            .db
            .get_service_token(service)
            .await
            .map_err(|e| RuntimeError::Database(e.to_string()))?;
        Ok(token.is_some())
    }

    pub async fn service_login(
        &self,
        service: &str,
        input: ServiceLoginDto,
    ) -> Result<(), RuntimeError> {
        match service {
            "anilist" => {
                let cfg = self.get_config().await;
                let client_id = input
                    .client_id
                    .or_else(|| cfg.services.anilist.client_id.clone())
                    .ok_or_else(|| RuntimeError::Config("AniList client_id required".into()))?;
                let client_secret = input
                    .client_secret
                    .or_else(|| cfg.services.anilist.client_secret.clone())
                    .ok_or_else(|| RuntimeError::Config("AniList client_secret required".into()))?;

                let token = ryuuji_api::anilist::auth::authorize(&client_id, &client_secret)
                    .await
                    .map_err(|e| RuntimeError::Api(e.to_string()))?;

                self.db
                    .save_service_token("anilist", token.access_token, None, None)
                    .await
                    .map_err(|e| RuntimeError::Database(e.to_string()))?;
            }
            "kitsu" => {
                let username = input
                    .username
                    .ok_or_else(|| RuntimeError::Config("Kitsu username required".into()))?;
                let password = input
                    .password
                    .ok_or_else(|| RuntimeError::Config("Kitsu password required".into()))?;

                let token = ryuuji_api::kitsu::auth::authenticate(&username, &password)
                    .await
                    .map_err(|e| RuntimeError::Api(e.to_string()))?;

                let expires_at = token
                    .expires_in
                    .map(|secs| (Utc::now() + chrono::Duration::seconds(secs as i64)).to_rfc3339());

                self.db
                    .save_service_token(
                        "kitsu",
                        token.access_token,
                        token.refresh_token,
                        expires_at,
                    )
                    .await
                    .map_err(|e| RuntimeError::Database(e.to_string()))?;
            }
            _ => {
                let cfg = self.get_config().await;
                let client_id = input
                    .client_id
                    .or_else(|| cfg.services.mal.client_id.clone())
                    .ok_or_else(|| RuntimeError::Config("MAL client_id required".into()))?;

                let token = ryuuji_api::mal::auth::authorize(&client_id)
                    .await
                    .map_err(|e| RuntimeError::Api(e.to_string()))?;

                let expires_at = token
                    .expires_in
                    .map(|secs| (Utc::now() + chrono::Duration::seconds(secs as i64)).to_rfc3339());

                self.db
                    .save_service_token("mal", token.access_token, token.refresh_token, expires_at)
                    .await
                    .map_err(|e| RuntimeError::Database(e.to_string()))?;
            }
        }

        Ok(())
    }

    pub async fn service_import(&self, service: &str) -> Result<usize, RuntimeError> {
        let token = self
            .db
            .get_service_token(service)
            .await
            .map_err(|e| RuntimeError::Database(e.to_string()))?
            .ok_or_else(|| RuntimeError::Api(format!("not logged in to {service}")))?;

        match service {
            "anilist" => {
                let client = ryuuji_api::anilist::AniListClient::new(token);
                let entries = client
                    .get_user_list_full()
                    .await
                    .map_err(|e| RuntimeError::Api(e.to_string()))?;
                let batch: Vec<(Anime, Option<LibraryEntry>)> = entries
                    .into_iter()
                    .map(|entry| {
                        let media = &entry.media;
                        let anime = Anime {
                            id: 0,
                            ids: AnimeIds {
                                anilist: Some(media.id),
                                kitsu: None,
                                mal: None,
                            },
                            title: AnimeTitle {
                                romaji: media.title.as_ref().and_then(|t| t.romaji.clone()),
                                english: media.title.as_ref().and_then(|t| t.english.clone()),
                                native: media.title.as_ref().and_then(|t| t.native.clone()),
                            },
                            synonyms: media.synonyms.clone().unwrap_or_default(),
                            episodes: media.episodes,
                            cover_url: media.cover_image.as_ref().and_then(|x| x.large.clone()),
                            season: media.season.as_deref().map(capitalize_first),
                            year: media.season_year,
                            synopsis: media.description.clone(),
                            genres: media.genres.clone().unwrap_or_default(),
                            media_type: media.format.clone(),
                            airing_status: media.status.clone(),
                            mean_score: media.mean_score.map(|s| s as f32 / 10.0),
                            studios: media
                                .studios
                                .as_ref()
                                .and_then(|x| x.nodes.as_ref())
                                .map(|nodes| nodes.iter().map(|n| n.name.clone()).collect())
                                .unwrap_or_default(),
                            source: media.source.clone(),
                            rating: None,
                            start_date: media.start_date.as_ref().and_then(|d| d.to_string_opt()),
                            end_date: media.end_date.as_ref().and_then(|d| d.to_string_opt()),
                        };

                        let status = entry
                            .status
                            .as_deref()
                            .and_then(map_anilist_status)
                            .unwrap_or(WatchStatus::Watching);

                        let library_entry = LibraryEntry {
                            id: 0,
                            anime_id: 0,
                            status,
                            watched_episodes: entry.progress,
                            score: entry.score.map(|s| s as f32 / 10.0),
                            updated_at: Utc::now(),
                            start_date: entry.started_at.as_ref().and_then(|d| d.to_string_opt()),
                            finish_date: entry
                                .completed_at
                                .as_ref()
                                .and_then(|d| d.to_string_opt()),
                            notes: entry.notes.clone(),
                            rewatching: entry.repeat.unwrap_or(0) > 0,
                            rewatch_count: entry.repeat.unwrap_or(0),
                        };

                        (anime, Some(library_entry))
                    })
                    .collect();

                self.db
                    .service_import_batch("anilist", batch)
                    .await
                    .map_err(|e| RuntimeError::Database(e.to_string()))
            }
            "kitsu" => {
                let client = ryuuji_api::kitsu::KitsuClient::new(token);
                let entries = client
                    .get_user_list_full()
                    .await
                    .map_err(|e| RuntimeError::Api(e.to_string()))?;

                let batch: Vec<(Anime, Option<LibraryEntry>)> = entries
                    .into_iter()
                    .map(|item| {
                        let anime = Anime {
                            id: 0,
                            ids: AnimeIds {
                                anilist: None,
                                kitsu: Some(item.anime_id),
                                mal: None,
                            },
                            title: AnimeTitle {
                                romaji: item.anime.canonical_title.clone(),
                                english: item.anime.titles.as_ref().and_then(|t| t.en.clone()),
                                native: item.anime.titles.as_ref().and_then(|t| t.ja_jp.clone()),
                            },
                            synonyms: vec![],
                            episodes: item.anime.episode_count,
                            cover_url: item
                                .anime
                                .poster_image
                                .as_ref()
                                .and_then(|x| x.large.clone().or_else(|| x.medium.clone())),
                            season: None,
                            year: item
                                .anime
                                .start_date
                                .as_deref()
                                .and_then(|d| d.split('-').next())
                                .and_then(|y| y.parse::<u32>().ok()),
                            synopsis: item.anime.synopsis.clone(),
                            genres: vec![],
                            media_type: item.anime.subtype.clone(),
                            airing_status: item.anime.status.clone(),
                            mean_score: item
                                .anime
                                .average_rating
                                .as_ref()
                                .and_then(|s| s.parse::<f32>().ok())
                                .map(|v| v / 10.0),
                            studios: vec![],
                            source: None,
                            rating: None,
                            start_date: item.anime.start_date.clone(),
                            end_date: item.anime.end_date.clone(),
                        };

                        let status = item
                            .entry
                            .status
                            .as_deref()
                            .and_then(map_kitsu_status)
                            .unwrap_or(WatchStatus::Watching);

                        let library_entry = LibraryEntry {
                            id: 0,
                            anime_id: 0,
                            status,
                            watched_episodes: item.entry.progress.unwrap_or(0),
                            score: item.entry.rating_twenty.map(|r| r as f32 / 2.0),
                            updated_at: Utc::now(),
                            start_date: item.entry.started_at.clone().map(strip_datetime_date),
                            finish_date: item.entry.finished_at.clone().map(strip_datetime_date),
                            notes: item.entry.notes.clone(),
                            rewatching: item.entry.reconsuming.unwrap_or(false),
                            rewatch_count: item.entry.reconsume_count.unwrap_or(0),
                        };

                        (anime, Some(library_entry))
                    })
                    .collect();

                self.db
                    .service_import_batch("kitsu", batch)
                    .await
                    .map_err(|e| RuntimeError::Database(e.to_string()))
            }
            _ => {
                let client_id = self
                    .get_config()
                    .await
                    .services
                    .mal
                    .client_id
                    .unwrap_or_default();
                let client = ryuuji_api::mal::MalClient::new(client_id, token);
                let items = client
                    .get_user_list_full()
                    .await
                    .map_err(|e| RuntimeError::Api(e.to_string()))?;

                let batch: Vec<(Anime, Option<LibraryEntry>)> = items
                    .into_iter()
                    .map(|item| {
                        let alt = &item.node.alternative_titles;
                        let season = item
                            .node
                            .start_season
                            .as_ref()
                            .map(|s| capitalize_first(&s.season));
                        let year = item.node.start_season.as_ref().map(|s| s.year);

                        let anime = Anime {
                            id: 0,
                            ids: AnimeIds {
                                anilist: None,
                                kitsu: None,
                                mal: Some(item.node.id),
                            },
                            title: AnimeTitle {
                                romaji: Some(item.node.title.clone()),
                                english: alt.as_ref().and_then(|a| a.en.clone()),
                                native: alt.as_ref().and_then(|a| a.ja.clone()),
                            },
                            synonyms: alt
                                .as_ref()
                                .and_then(|a| a.synonyms.clone())
                                .unwrap_or_default(),
                            episodes: item.node.num_episodes,
                            cover_url: item
                                .node
                                .main_picture
                                .as_ref()
                                .and_then(|p| p.medium.clone()),
                            season,
                            year,
                            synopsis: item.node.synopsis.clone(),
                            genres: item
                                .node
                                .genres
                                .as_ref()
                                .map(|g| g.iter().map(|x| x.name.clone()).collect())
                                .unwrap_or_default(),
                            media_type: item.node.media_type.clone(),
                            airing_status: item.node.status.clone(),
                            mean_score: item.node.mean,
                            studios: item
                                .node
                                .studios
                                .as_ref()
                                .map(|s| s.iter().map(|x| x.name.clone()).collect())
                                .unwrap_or_default(),
                            source: item.node.source.clone(),
                            rating: item.node.rating.clone(),
                            start_date: item.node.start_date.clone(),
                            end_date: item.node.end_date.clone(),
                        };

                        let status = item
                            .list_status
                            .status
                            .as_deref()
                            .and_then(WatchStatus::from_db_str)
                            .unwrap_or(WatchStatus::Watching);

                        let library_entry = LibraryEntry {
                            id: 0,
                            anime_id: 0,
                            status,
                            watched_episodes: item.list_status.num_episodes_watched.unwrap_or(0),
                            score: item.list_status.score.map(|s| s as f32),
                            updated_at: Utc::now(),
                            start_date: item.list_status.start_date.clone(),
                            finish_date: item.list_status.finish_date.clone(),
                            notes: item.list_status.comments.clone(),
                            rewatching: item.list_status.is_rewatching.unwrap_or(false),
                            rewatch_count: item.list_status.num_times_rewatched.unwrap_or(0),
                        };

                        (anime, Some(library_entry))
                    })
                    .collect();

                self.db
                    .service_import_batch("mal", batch)
                    .await
                    .map_err(|e| RuntimeError::Database(e.to_string()))
            }
        }
    }

    pub async fn search_remote(
        &self,
        query: String,
    ) -> Result<Vec<ryuuji_api::traits::AnimeSearchResult>, RuntimeError> {
        let config = self.get_config().await;
        let primary = config.services.primary;
        let token = self
            .db
            .get_service_token(&primary)
            .await
            .map_err(|e| RuntimeError::Database(e.to_string()))?
            .ok_or_else(|| RuntimeError::Api(format!("No {primary} token found")))?;

        match primary.as_str() {
            "anilist" => {
                let client = ryuuji_api::anilist::AniListClient::new(token);
                client
                    .search_anime(&query)
                    .await
                    .map_err(|e| RuntimeError::Api(e.to_string()))
            }
            "kitsu" => {
                let client = ryuuji_api::kitsu::KitsuClient::new(token);
                client
                    .search_anime(&query)
                    .await
                    .map_err(|e| RuntimeError::Api(e.to_string()))
            }
            _ => {
                let client_id = config.services.mal.client_id.unwrap_or_default();
                let client = ryuuji_api::mal::MalClient::new(client_id, token);
                client
                    .search_anime(&query)
                    .await
                    .map_err(|e| RuntimeError::Api(e.to_string()))
            }
        }
    }

    pub async fn add_remote_search_result_to_library(
        &self,
        result: ryuuji_api::traits::AnimeSearchResult,
    ) -> Result<i64, RuntimeError> {
        let cfg = self.get_config().await;
        let primary = cfg.services.primary;

        let anime = Anime {
            id: 0,
            ids: match primary.as_str() {
                "anilist" => AnimeIds {
                    anilist: Some(result.service_id),
                    kitsu: None,
                    mal: None,
                },
                "kitsu" => AnimeIds {
                    anilist: None,
                    kitsu: Some(result.service_id),
                    mal: None,
                },
                _ => AnimeIds {
                    anilist: None,
                    kitsu: None,
                    mal: Some(result.service_id),
                },
            },
            title: AnimeTitle {
                romaji: Some(result.title.clone()),
                english: result.title_english.clone(),
                native: None,
            },
            synonyms: vec![],
            episodes: result.episodes,
            cover_url: result.cover_url.clone(),
            season: result.season.clone(),
            year: result.year,
            synopsis: result.synopsis.clone(),
            genres: result.genres.clone(),
            media_type: result.media_type.clone(),
            airing_status: result.status.clone(),
            mean_score: result.mean_score,
            studios: vec![],
            source: None,
            rating: None,
            start_date: None,
            end_date: None,
        };

        let entry = LibraryEntry {
            id: 0,
            anime_id: 0,
            status: WatchStatus::Watching,
            watched_episodes: 0,
            score: None,
            updated_at: Utc::now(),
            start_date: None,
            finish_date: None,
            notes: None,
            rewatching: false,
            rewatch_count: 0,
        };

        self.db
            .service_import_batch(&primary, vec![(anime, Some(entry))])
            .await
            .map_err(|e| RuntimeError::Database(e.to_string()))?;

        let rows = self
            .db
            .get_all_library()
            .await
            .map_err(|e| RuntimeError::Database(e.to_string()))?;

        let added = rows
            .into_iter()
            .find(|r| r.anime.title.preferred() == result.title)
            .ok_or_else(|| RuntimeError::NotFound("added anime row not found".into()))?;

        let token = self
            .db
            .get_service_token(&primary)
            .await
            .map_err(|e| RuntimeError::Database(e.to_string()))?;
        if let Some(token) = token {
            match primary.as_str() {
                "anilist" => {
                    let client = ryuuji_api::anilist::AniListClient::new(token);
                    let _ = client
                        .add_library_entry(result.service_id, "watching")
                        .await;
                }
                "kitsu" => {
                    let client = ryuuji_api::kitsu::KitsuClient::new(token);
                    let _ = client
                        .add_library_entry(result.service_id, "watching")
                        .await;
                }
                _ => {
                    let client_id = self
                        .get_config()
                        .await
                        .services
                        .mal
                        .client_id
                        .unwrap_or_default();
                    let client = ryuuji_api::mal::MalClient::new(client_id, token);
                    let _ = client
                        .add_library_entry(result.service_id, "watching")
                        .await;
                }
            }
        }

        Ok(added.anime.id)
    }

    async fn sync_push_update(
        &self,
        anime_id: i64,
        update: LibraryEntryUpdate,
    ) -> Result<(), RuntimeError> {
        let cfg = self.get_config().await;
        let primary = cfg.services.primary;

        let row = self
            .db
            .get_library_row(anime_id)
            .await
            .map_err(|e| RuntimeError::Database(e.to_string()))?
            .ok_or_else(|| RuntimeError::NotFound("Anime not found in library".into()))?;

        let token = self
            .db
            .get_service_token(&primary)
            .await
            .map_err(|e| RuntimeError::Database(e.to_string()))?
            .ok_or_else(|| RuntimeError::Api(format!("No {primary} token found")))?;

        match primary.as_str() {
            "anilist" => {
                let service_id = row
                    .anime
                    .ids
                    .anilist
                    .ok_or_else(|| RuntimeError::NotFound("No AniList ID".into()))?;
                let client = ryuuji_api::anilist::AniListClient::new(token);
                client
                    .update_library_entry(service_id, update)
                    .await
                    .map_err(|e| RuntimeError::Api(e.to_string()))
            }
            "kitsu" => {
                let service_id = row
                    .anime
                    .ids
                    .kitsu
                    .ok_or_else(|| RuntimeError::NotFound("No Kitsu ID".into()))?;
                let client = ryuuji_api::kitsu::KitsuClient::new(token);
                client
                    .update_library_entry(service_id, update)
                    .await
                    .map_err(|e| RuntimeError::Api(e.to_string()))
            }
            _ => {
                let service_id = row
                    .anime
                    .ids
                    .mal
                    .ok_or_else(|| RuntimeError::NotFound("No MAL ID".into()))?;
                let client_id = cfg.services.mal.client_id.unwrap_or_default();
                let client = ryuuji_api::mal::MalClient::new(client_id, token);
                client
                    .update_library_entry(service_id, update)
                    .await
                    .map_err(|e| RuntimeError::Api(e.to_string()))
            }
        }
    }

    async fn sync_delete(&self, anime_id: i64) -> Result<(), RuntimeError> {
        let cfg = self.get_config().await;
        let primary = cfg.services.primary;
        let row = self
            .db
            .get_library_row(anime_id)
            .await
            .map_err(|e| RuntimeError::Database(e.to_string()))?
            .ok_or_else(|| RuntimeError::NotFound("Anime not found in library".into()))?;

        let Some(token) = self
            .db
            .get_service_token(&primary)
            .await
            .map_err(|e| RuntimeError::Database(e.to_string()))?
        else {
            return Ok(());
        };

        let service_id = match primary.as_str() {
            "anilist" => row.anime.ids.anilist,
            "kitsu" => row.anime.ids.kitsu,
            _ => row.anime.ids.mal,
        };

        let Some(service_id) = service_id else {
            return Ok(());
        };

        match primary.as_str() {
            "anilist" => {
                let client = ryuuji_api::anilist::AniListClient::new(token);
                client
                    .delete_library_entry(service_id)
                    .await
                    .map_err(|e| RuntimeError::Api(e.to_string()))
            }
            "kitsu" => {
                let client = ryuuji_api::kitsu::KitsuClient::new(token);
                client
                    .delete_library_entry(service_id)
                    .await
                    .map_err(|e| RuntimeError::Api(e.to_string()))
            }
            _ => {
                let client_id = cfg.services.mal.client_id.unwrap_or_default();
                let client = ryuuji_api::mal::MalClient::new(client_id, token);
                client
                    .delete_library_entry(service_id)
                    .await
                    .map_err(|e| RuntimeError::Api(e.to_string()))
            }
        }
    }
}

fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(first) => first.to_uppercase().to_string() + c.as_str(),
        None => String::new(),
    }
}

fn strip_datetime_date(dt: String) -> String {
    dt.split('T').next().unwrap_or(&dt).to_string()
}

fn map_anilist_status(s: &str) -> Option<WatchStatus> {
    match s {
        "CURRENT" => Some(WatchStatus::Watching),
        "COMPLETED" => Some(WatchStatus::Completed),
        "PAUSED" => Some(WatchStatus::OnHold),
        "DROPPED" => Some(WatchStatus::Dropped),
        "PLANNING" => Some(WatchStatus::PlanToWatch),
        _ => None,
    }
}

fn map_kitsu_status(s: &str) -> Option<WatchStatus> {
    match s {
        "current" => Some(WatchStatus::Watching),
        "completed" => Some(WatchStatus::Completed),
        "on_hold" => Some(WatchStatus::OnHold),
        "dropped" => Some(WatchStatus::Dropped),
        "planned" => Some(WatchStatus::PlanToWatch),
        _ => None,
    }
}

fn outcome_to_status(outcome: &UpdateOutcome) -> String {
    match outcome {
        UpdateOutcome::Updated {
            anime_title,
            episode,
            ..
        } => format!("Updated {anime_title} to ep {episode}"),
        UpdateOutcome::AddedToLibrary {
            anime_title,
            episode,
            ..
        } => {
            format!("Added {anime_title} (ep {episode}) to library")
        }
        UpdateOutcome::AlreadyCurrent { anime_title, .. } => {
            format!("Already current: {anime_title}")
        }
        UpdateOutcome::Unrecognized { raw_title } => {
            format!("Unrecognized: {raw_title}")
        }
        UpdateOutcome::NothingPlaying => "Nothing playing".into(),
    }
}

async fn detect_and_parse() -> Option<DetectedMedia> {
    let players = ryuuji_detect::detect_players();
    let player = players.into_iter().next()?;

    if player.is_browser {
        let stream_db = ryuuji_detect::StreamDatabase::embedded();
        let stream_match = ryuuji_detect::stream::detect_stream(&player, &stream_db)?;
        let raw_title = stream_match.extracted_title;
        let parsed = ryuuji_parse::parse(&raw_title);

        return Some(DetectedMedia {
            player_name: player.player_name,
            anime_title: parsed.title,
            episode: parsed.episode_number,
            release_group: parsed.release_group,
            resolution: parsed.resolution,
            raw_title,
            service_name: Some(stream_match.service_name),
        });
    }

    let raw_title = player
        .file_path
        .as_deref()
        .and_then(|p| {
            std::path::Path::new(p)
                .file_name()
                .and_then(|f| f.to_str())
                .map(|s| s.to_string())
        })
        .or_else(|| player.media_title.clone())?;

    let parsed = ryuuji_parse::parse(&raw_title);

    Some(DetectedMedia {
        player_name: player.player_name,
        anime_title: parsed.title,
        episode: parsed.episode_number,
        release_group: parsed.release_group,
        resolution: parsed.resolution,
        raw_title,
        service_name: None,
    })
}

#[allow(dead_code)]
fn _config_path() -> PathBuf {
    AppConfig::config_path()
}
