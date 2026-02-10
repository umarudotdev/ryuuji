//! Async database actor.
//!
//! Owns the `Storage` on a dedicated thread and exposes async methods
//! that communicate via channels. This keeps all SQLite I/O off the
//! render thread.

use std::path::Path;

use tokio::sync::{mpsc, oneshot};

use ryuuji_core::config::AppConfig;
use ryuuji_core::error::RyuujiError;
use ryuuji_core::models::{Anime, DetectedMedia, LibraryEntry, WatchStatus};
use ryuuji_core::orchestrator::{self, UpdateOutcome};
use ryuuji_core::recognition::RecognitionCache;
use ryuuji_core::relations::RelationDatabase;
use ryuuji_core::storage::{HistoryRow, LibraryRow, Storage};
use ryuuji_core::torrent::{TorrentFeed, TorrentFilter, TorrentItem};

/// Cloneable handle to the DB actor thread.
#[derive(Clone)]
pub struct DbHandle {
    tx: mpsc::UnboundedSender<DbCommand>,
}

/// Commands sent to the actor thread.
#[allow(dead_code)]
enum DbCommand {
    GetLibraryByStatus {
        status: WatchStatus,
        reply: oneshot::Sender<Result<Vec<LibraryRow>, RyuujiError>>,
    },
    GetAllLibrary {
        reply: oneshot::Sender<Result<Vec<LibraryRow>, RyuujiError>>,
    },
    UpdateEpisodeCount {
        anime_id: i64,
        episodes: u32,
        reply: oneshot::Sender<Result<(), RyuujiError>>,
    },
    RecordWatch {
        anime_id: i64,
        episode: u32,
        reply: oneshot::Sender<Result<(), RyuujiError>>,
    },
    UpdateLibraryStatus {
        anime_id: i64,
        status: WatchStatus,
        reply: oneshot::Sender<Result<(), RyuujiError>>,
    },
    UpdateLibraryScore {
        anime_id: i64,
        score: f32,
        reply: oneshot::Sender<Result<(), RyuujiError>>,
    },
    DeleteLibraryEntry {
        anime_id: i64,
        reply: oneshot::Sender<Result<(), RyuujiError>>,
    },
    ProcessDetection {
        detected: DetectedMedia,
        config: Box<AppConfig>,
        reply: oneshot::Sender<Result<UpdateOutcome, RyuujiError>>,
    },
    SaveServiceToken {
        service: String,
        token: String,
        refresh: Option<String>,
        expires_at: Option<String>,
        reply: oneshot::Sender<Result<(), RyuujiError>>,
    },
    GetServiceToken {
        service: String,
        reply: oneshot::Sender<Result<Option<String>, RyuujiError>>,
    },
    ServiceImportBatch {
        service: String,
        entries: Vec<(Anime, Option<LibraryEntry>)>,
        reply: oneshot::Sender<Result<usize, RyuujiError>>,
    },
    GetLibraryRow {
        anime_id: i64,
        reply: oneshot::Sender<Result<Option<LibraryRow>, RyuujiError>>,
    },
    GetWatchHistory {
        limit: u32,
        reply: oneshot::Sender<Result<Vec<HistoryRow>, RyuujiError>>,
    },
    // ── Torrent commands ─────────────────────────────────────────
    GetTorrentFeeds {
        reply: oneshot::Sender<Result<Vec<TorrentFeed>, RyuujiError>>,
    },
    UpsertTorrentFeed {
        feed: TorrentFeed,
        reply: oneshot::Sender<Result<i64, RyuujiError>>,
    },
    DeleteTorrentFeed {
        id: i64,
        reply: oneshot::Sender<Result<(), RyuujiError>>,
    },
    GetTorrentFilters {
        reply: oneshot::Sender<Result<Vec<TorrentFilter>, RyuujiError>>,
    },
    UpsertTorrentFilter {
        filter: TorrentFilter,
        reply: oneshot::Sender<Result<i64, RyuujiError>>,
    },
    DeleteTorrentFilter {
        id: i64,
        reply: oneshot::Sender<Result<(), RyuujiError>>,
    },
    IsTorrentArchived {
        guid: String,
        reply: oneshot::Sender<Result<bool, RyuujiError>>,
    },
    ArchiveTorrent {
        guid: String,
        title: String,
        action: String,
        reply: oneshot::Sender<Result<(), RyuujiError>>,
    },
    ClearTorrentArchive {
        reply: oneshot::Sender<Result<(), RyuujiError>>,
    },
    MatchTorrentItems {
        items: Vec<TorrentItem>,
        reply: oneshot::Sender<Vec<TorrentItem>>,
    },
    UpdateLibraryDates {
        anime_id: i64,
        start_date: Option<String>,
        finish_date: Option<String>,
        reply: oneshot::Sender<Result<(), RyuujiError>>,
    },
    UpdateLibraryNotes {
        anime_id: i64,
        notes: Option<String>,
        reply: oneshot::Sender<Result<(), RyuujiError>>,
    },
    UpdateLibraryRewatch {
        anime_id: i64,
        rewatching: bool,
        rewatch_count: u32,
        reply: oneshot::Sender<Result<(), RyuujiError>>,
    },
}

#[allow(dead_code)]
impl DbHandle {
    /// Spawn the DB actor on a dedicated thread and return a handle.
    ///
    /// Returns `None` if the database cannot be opened.
    pub fn open(path: &Path) -> Option<Self> {
        let storage = Storage::open(path)
            .map_err(|e| tracing::error!("Failed to open database: {e}"))
            .ok()?;

        let (tx, rx) = mpsc::unbounded_channel();

        std::thread::Builder::new()
            .name("db-actor".into())
            .spawn(move || actor_loop(storage, rx))
            .map_err(|e| tracing::error!("Failed to spawn DB thread: {e}"))
            .ok()?;

        Some(Self { tx })
    }

    pub async fn get_library_by_status(
        &self,
        status: WatchStatus,
    ) -> Result<Vec<LibraryRow>, RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self
            .tx
            .send(DbCommand::GetLibraryByStatus { status, reply });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    pub async fn get_all_library(&self) -> Result<Vec<LibraryRow>, RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::GetAllLibrary { reply });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    pub async fn update_episode_count(
        &self,
        anime_id: i64,
        episodes: u32,
    ) -> Result<(), RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::UpdateEpisodeCount {
            anime_id,
            episodes,
            reply,
        });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    pub async fn record_watch(&self, anime_id: i64, episode: u32) -> Result<(), RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::RecordWatch {
            anime_id,
            episode,
            reply,
        });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    pub async fn update_library_status(
        &self,
        anime_id: i64,
        status: WatchStatus,
    ) -> Result<(), RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::UpdateLibraryStatus {
            anime_id,
            status,
            reply,
        });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    pub async fn update_library_score(
        &self,
        anime_id: i64,
        score: f32,
    ) -> Result<(), RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::UpdateLibraryScore {
            anime_id,
            score,
            reply,
        });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    pub async fn delete_library_entry(&self, anime_id: i64) -> Result<(), RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self
            .tx
            .send(DbCommand::DeleteLibraryEntry { anime_id, reply });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    pub async fn process_detection(
        &self,
        detected: DetectedMedia,
        config: AppConfig,
    ) -> Result<UpdateOutcome, RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::ProcessDetection {
            detected,
            config: Box::new(config),
            reply,
        });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    pub async fn save_service_token(
        &self,
        service: impl Into<String>,
        token: String,
        refresh: Option<String>,
        expires_at: Option<String>,
    ) -> Result<(), RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::SaveServiceToken {
            service: service.into(),
            token,
            refresh,
            expires_at,
            reply,
        });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    pub async fn get_service_token(
        &self,
        service: impl Into<String>,
    ) -> Result<Option<String>, RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::GetServiceToken {
            service: service.into(),
            reply,
        });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    /// Fetch a single library row (anime + entry) by anime ID.
    pub async fn get_library_row(
        &self,
        anime_id: i64,
    ) -> Result<Option<LibraryRow>, RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::GetLibraryRow { anime_id, reply });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    pub async fn get_watch_history(&self, limit: u32) -> Result<Vec<HistoryRow>, RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::GetWatchHistory { limit, reply });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    // ── Torrent handle methods ────────────────────────────────────

    pub async fn get_torrent_feeds(&self) -> Result<Vec<TorrentFeed>, RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::GetTorrentFeeds { reply });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    pub async fn upsert_torrent_feed(&self, feed: TorrentFeed) -> Result<i64, RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::UpsertTorrentFeed { feed, reply });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    pub async fn delete_torrent_feed(&self, id: i64) -> Result<(), RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::DeleteTorrentFeed { id, reply });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    pub async fn get_torrent_filters(&self) -> Result<Vec<TorrentFilter>, RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::GetTorrentFilters { reply });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    pub async fn upsert_torrent_filter(&self, filter: TorrentFilter) -> Result<i64, RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self
            .tx
            .send(DbCommand::UpsertTorrentFilter { filter, reply });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    pub async fn delete_torrent_filter(&self, id: i64) -> Result<(), RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::DeleteTorrentFilter { id, reply });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    pub async fn is_torrent_archived(&self, guid: String) -> Result<bool, RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::IsTorrentArchived { guid, reply });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    pub async fn archive_torrent(
        &self,
        guid: String,
        title: String,
        action: String,
    ) -> Result<(), RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::ArchiveTorrent {
            guid,
            title,
            action,
            reply,
        });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    pub async fn clear_torrent_archive(&self) -> Result<(), RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::ClearTorrentArchive { reply });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    /// Run title matching on torrent items using the actor's recognition cache.
    pub async fn match_torrent_items(&self, items: Vec<TorrentItem>) -> Vec<TorrentItem> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::MatchTorrentItems { items, reply });
        rx.await.unwrap_or_default()
    }

    pub async fn update_library_dates(
        &self,
        anime_id: i64,
        start_date: Option<String>,
        finish_date: Option<String>,
    ) -> Result<(), RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::UpdateLibraryDates {
            anime_id,
            start_date,
            finish_date,
            reply,
        });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    pub async fn update_library_notes(
        &self,
        anime_id: i64,
        notes: Option<String>,
    ) -> Result<(), RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::UpdateLibraryNotes {
            anime_id,
            notes,
            reply,
        });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    pub async fn update_library_rewatch(
        &self,
        anime_id: i64,
        rewatching: bool,
        rewatch_count: u32,
    ) -> Result<(), RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::UpdateLibraryRewatch {
            anime_id,
            rewatching,
            rewatch_count,
            reply,
        });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }

    /// Import a batch of anime + optional library entries from a service.
    /// Returns the number of anime upserted.
    pub async fn service_import_batch(
        &self,
        service: impl Into<String>,
        entries: Vec<(Anime, Option<LibraryEntry>)>,
    ) -> Result<usize, RyuujiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::ServiceImportBatch {
            service: service.into(),
            entries,
            reply,
        });
        rx.await
            .unwrap_or_else(|_| Err(RyuujiError::Config("DB actor closed".into())))
    }
}

/// Run the actor loop on a dedicated thread.
fn actor_loop(storage: Storage, mut rx: mpsc::UnboundedReceiver<DbCommand>) {
    let mut cache = RecognitionCache::new();
    let relations = RelationDatabase::embedded().unwrap_or_default();

    // Block the thread waiting for commands. We use blocking_recv because
    // this thread has no tokio runtime — it's a plain OS thread.
    while let Some(cmd) = rx.blocking_recv() {
        match cmd {
            DbCommand::GetLibraryByStatus { status, reply } => {
                let _ = reply.send(storage.get_library_by_status(status));
            }
            DbCommand::GetAllLibrary { reply } => {
                let _ = reply.send(storage.get_all_library());
            }
            DbCommand::UpdateEpisodeCount {
                anime_id,
                episodes,
                reply,
            } => {
                let _ = reply.send(storage.update_episode_count(anime_id, episodes));
            }
            DbCommand::RecordWatch {
                anime_id,
                episode,
                reply,
            } => {
                let _ = reply.send(storage.record_watch(anime_id, episode));
            }
            DbCommand::UpdateLibraryStatus {
                anime_id,
                status,
                reply,
            } => {
                let _ = reply.send(storage.update_library_status(anime_id, status));
            }
            DbCommand::UpdateLibraryScore {
                anime_id,
                score,
                reply,
            } => {
                let _ = reply.send(storage.update_library_score(anime_id, score));
            }
            DbCommand::DeleteLibraryEntry { anime_id, reply } => {
                let _ = reply.send(storage.delete_library_entry(anime_id));
            }
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
                // Invalidate cache when new anime is added to the library,
                // so the next detection tick picks up the new entry.
                if let Ok(UpdateOutcome::AddedToLibrary { .. }) = &result {
                    cache.invalidate();
                }
                let _ = reply.send(result);
            }
            DbCommand::SaveServiceToken {
                service,
                token,
                refresh,
                expires_at,
                reply,
            } => {
                let _ = reply.send(storage.save_token(
                    &service,
                    &token,
                    refresh.as_deref(),
                    expires_at.as_deref(),
                ));
            }
            DbCommand::GetServiceToken { service, reply } => {
                let _ = reply.send(storage.get_token(&service));
            }
            DbCommand::GetLibraryRow { anime_id, reply } => {
                let result = match storage.get_anime(anime_id) {
                    Ok(Some(anime)) => match storage.get_library_entry_for_anime(anime_id) {
                        Ok(Some(entry)) => Ok(Some(LibraryRow { anime, entry })),
                        Ok(None) => Ok(None),
                        Err(e) => Err(e),
                    },
                    Ok(None) => Ok(None),
                    Err(e) => Err(e),
                };
                let _ = reply.send(result);
            }
            DbCommand::GetWatchHistory { limit, reply } => {
                let _ = reply.send(storage.get_watch_history(limit));
            }
            // ── Torrent commands ───────────────────────────────────
            DbCommand::GetTorrentFeeds { reply } => {
                let _ = reply.send(storage.get_torrent_feeds());
            }
            DbCommand::UpsertTorrentFeed { feed, reply } => {
                let _ = reply.send(storage.upsert_torrent_feed(&feed));
            }
            DbCommand::DeleteTorrentFeed { id, reply } => {
                let _ = reply.send(storage.delete_torrent_feed(id));
            }
            DbCommand::GetTorrentFilters { reply } => {
                let _ = reply.send(storage.get_torrent_filters());
            }
            DbCommand::UpsertTorrentFilter { filter, reply } => {
                let _ = reply.send(storage.upsert_torrent_filter(&filter));
            }
            DbCommand::DeleteTorrentFilter { id, reply } => {
                let _ = reply.send(storage.delete_torrent_filter(id));
            }
            DbCommand::IsTorrentArchived { guid, reply } => {
                let _ = reply.send(storage.is_torrent_archived(&guid));
            }
            DbCommand::ArchiveTorrent {
                guid,
                title,
                action,
                reply,
            } => {
                let _ = reply.send(storage.archive_torrent(&guid, &title, &action));
            }
            DbCommand::ClearTorrentArchive { reply } => {
                let _ = reply.send(storage.clear_torrent_archive());
            }
            DbCommand::MatchTorrentItems { mut items, reply } => {
                ryuuji_core::torrent::matcher::match_torrent_items(
                    &mut items, &storage, &mut cache,
                );
                let _ = reply.send(items);
            }
            DbCommand::UpdateLibraryDates {
                anime_id,
                start_date,
                finish_date,
                reply,
            } => {
                let _ = reply.send(storage.update_library_dates(
                    anime_id,
                    start_date.as_deref(),
                    finish_date.as_deref(),
                ));
            }
            DbCommand::UpdateLibraryNotes {
                anime_id,
                notes,
                reply,
            } => {
                let _ = reply.send(storage.update_library_notes(anime_id, notes.as_deref()));
            }
            DbCommand::UpdateLibraryRewatch {
                anime_id,
                rewatching,
                rewatch_count,
                reply,
            } => {
                let _ =
                    reply.send(storage.update_library_rewatch(anime_id, rewatching, rewatch_count));
            }
            DbCommand::ServiceImportBatch {
                service,
                entries,
                reply,
            } => {
                let mut count = 0usize;
                let mut err: Option<RyuujiError> = None;

                for (anime, library_entry) in &entries {
                    let upsert_result = match service.as_str() {
                        "anilist" => storage.upsert_anime_by_anilist_id(anime),
                        "kitsu" => storage.upsert_anime_by_kitsu_id(anime),
                        _ => storage.upsert_anime_by_mal_id(anime),
                    };
                    match upsert_result {
                        Ok(anime_id) => {
                            count += 1;
                            if let Some(entry) = library_entry {
                                let mut entry = entry.clone();
                                entry.anime_id = anime_id;
                                if let Err(e) = storage.upsert_library_entry(&entry) {
                                    tracing::warn!("Failed to upsert library entry: {e}");
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to upsert anime: {e}");
                            err = Some(e);
                        }
                    }
                }

                // Invalidate recognition cache after bulk import.
                cache.invalidate();

                let _ = reply.send(match err {
                    Some(e) if count == 0 => Err(e),
                    _ => Ok(count),
                });
            }
        }
    }
}
