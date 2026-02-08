//! Async database actor.
//!
//! Owns the `Storage` on a dedicated thread and exposes async methods
//! that communicate via channels. This keeps all SQLite I/O off the
//! render thread.

use std::path::Path;

use tokio::sync::{mpsc, oneshot};

use kurozumi_core::config::AppConfig;
use kurozumi_core::error::KurozumiError;
use kurozumi_core::models::{DetectedMedia, WatchStatus};
use kurozumi_core::orchestrator::{self, UpdateOutcome};
use kurozumi_core::storage::{LibraryRow, Storage};

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
        reply: oneshot::Sender<Result<Vec<LibraryRow>, KurozumiError>>,
    },
    GetAllLibrary {
        reply: oneshot::Sender<Result<Vec<LibraryRow>, KurozumiError>>,
    },
    UpdateEpisodeCount {
        anime_id: i64,
        episodes: u32,
        reply: oneshot::Sender<Result<(), KurozumiError>>,
    },
    RecordWatch {
        anime_id: i64,
        episode: u32,
        reply: oneshot::Sender<Result<(), KurozumiError>>,
    },
    UpdateLibraryStatus {
        anime_id: i64,
        status: WatchStatus,
        reply: oneshot::Sender<Result<(), KurozumiError>>,
    },
    UpdateLibraryScore {
        anime_id: i64,
        score: f32,
        reply: oneshot::Sender<Result<(), KurozumiError>>,
    },
    DeleteLibraryEntry {
        anime_id: i64,
        reply: oneshot::Sender<Result<(), KurozumiError>>,
    },
    ProcessDetection {
        detected: DetectedMedia,
        config: AppConfig,
        reply: oneshot::Sender<Result<UpdateOutcome, KurozumiError>>,
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
    ) -> Result<Vec<LibraryRow>, KurozumiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::GetLibraryByStatus { status, reply });
        rx.await.unwrap_or_else(|_| Err(KurozumiError::Config("DB actor closed".into())))
    }

    pub async fn get_all_library(&self) -> Result<Vec<LibraryRow>, KurozumiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::GetAllLibrary { reply });
        rx.await.unwrap_or_else(|_| Err(KurozumiError::Config("DB actor closed".into())))
    }

    pub async fn update_episode_count(
        &self,
        anime_id: i64,
        episodes: u32,
    ) -> Result<(), KurozumiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::UpdateEpisodeCount {
            anime_id,
            episodes,
            reply,
        });
        rx.await.unwrap_or_else(|_| Err(KurozumiError::Config("DB actor closed".into())))
    }

    pub async fn record_watch(
        &self,
        anime_id: i64,
        episode: u32,
    ) -> Result<(), KurozumiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::RecordWatch {
            anime_id,
            episode,
            reply,
        });
        rx.await.unwrap_or_else(|_| Err(KurozumiError::Config("DB actor closed".into())))
    }

    pub async fn update_library_status(
        &self,
        anime_id: i64,
        status: WatchStatus,
    ) -> Result<(), KurozumiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::UpdateLibraryStatus {
            anime_id,
            status,
            reply,
        });
        rx.await.unwrap_or_else(|_| Err(KurozumiError::Config("DB actor closed".into())))
    }

    pub async fn update_library_score(
        &self,
        anime_id: i64,
        score: f32,
    ) -> Result<(), KurozumiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::UpdateLibraryScore {
            anime_id,
            score,
            reply,
        });
        rx.await.unwrap_or_else(|_| Err(KurozumiError::Config("DB actor closed".into())))
    }

    pub async fn delete_library_entry(&self, anime_id: i64) -> Result<(), KurozumiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::DeleteLibraryEntry { anime_id, reply });
        rx.await.unwrap_or_else(|_| Err(KurozumiError::Config("DB actor closed".into())))
    }

    pub async fn process_detection(
        &self,
        detected: DetectedMedia,
        config: AppConfig,
    ) -> Result<UpdateOutcome, KurozumiError> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(DbCommand::ProcessDetection {
            detected,
            config,
            reply,
        });
        rx.await.unwrap_or_else(|_| Err(KurozumiError::Config("DB actor closed".into())))
    }
}

/// Run the actor loop on a dedicated thread.
fn actor_loop(storage: Storage, mut rx: mpsc::UnboundedReceiver<DbCommand>) {
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
                let _ = reply.send(orchestrator::process_detection(
                    &detected, &storage, &config,
                ));
            }
        }
    }
}
