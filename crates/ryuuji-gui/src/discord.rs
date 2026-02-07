//! Discord Rich Presence integration.
//!
//! Runs a `DiscordIpcClient` on a dedicated OS thread (IPC is blocking)
//! and exposes an async-friendly `DiscordHandle` via MPSC channels.
//! Connects lazily on first presence update and auto-reconnects on failure.

use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};

/// Default Discord Application ID for Ryuuji.
///
/// This identifies the app in Discord's developer portal. It's not a secret —
/// it controls the activity name shown to users ("Playing Ryuuji").
const APP_ID: &str = "1339263648872411156";

/// Commands sent to the Discord actor thread.
#[allow(dead_code)]
enum DiscordCommand {
    Update {
        title: String,
        episode: Option<u32>,
        service: Option<String>,
    },
    Clear,
    Shutdown,
}

/// Cloneable handle to the Discord actor thread.
#[derive(Clone)]
#[allow(dead_code)]
pub struct DiscordHandle {
    tx: mpsc::Sender<DiscordCommand>,
}

impl DiscordHandle {
    /// Spawn the Discord actor thread and return a handle.
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel();

        std::thread::Builder::new()
            .name("discord-rpc".into())
            .spawn(move || actor_loop(rx))
            .expect("failed to spawn discord-rpc thread");

        Self { tx }
    }

    /// Update the rich presence with the currently playing anime.
    pub fn update_presence(&self, title: String, episode: Option<u32>, service: Option<String>) {
        let _ = self.tx.send(DiscordCommand::Update {
            title,
            episode,
            service,
        });
    }

    /// Clear the rich presence (nothing playing).
    pub fn clear_presence(&self) {
        let _ = self.tx.send(DiscordCommand::Clear);
    }

    /// Shut down the actor thread.
    #[allow(dead_code)]
    pub fn shutdown(&self) {
        let _ = self.tx.send(DiscordCommand::Shutdown);
    }
}

/// The actor loop: owns the IPC client and processes commands.
fn actor_loop(rx: mpsc::Receiver<DiscordCommand>) {
    let mut client: Option<DiscordIpcClient> = None;
    let mut connected = false;

    for cmd in rx {
        match cmd {
            DiscordCommand::Update {
                title,
                episode,
                service,
            } => {
                // Lazy-connect on first update.
                if client.is_none() {
                    client = Some(DiscordIpcClient::new(APP_ID));
                }

                let ipc = client.as_mut().unwrap();

                // Connect if not connected.
                if !connected {
                    match ipc.connect() {
                        Ok(()) => {
                            connected = true;
                            tracing::info!("Connected to Discord IPC");
                        }
                        Err(e) => {
                            tracing::debug!(error = %e, "Discord not available");
                            continue;
                        }
                    }
                }

                // Build the activity.
                let details = match episode {
                    Some(ep) => format!("Episode {ep}"),
                    None => "Watching".into(),
                };

                let state_text = match &service {
                    Some(svc) => format!("via {svc}"),
                    None => "Local file".into(),
                };

                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;

                let payload = activity::Activity::new()
                    .details(&title)
                    .state(&state_text)
                    .timestamps(activity::Timestamps::new().start(now))
                    .assets(
                        activity::Assets::new()
                            .large_image("ryuuji_logo")
                            .large_text(&details),
                    );

                if let Err(e) = ipc.set_activity(payload) {
                    tracing::debug!(error = %e, "Failed to set Discord activity");
                    // Connection probably died — reset state for reconnect.
                    connected = false;
                    client = None;
                }
            }
            DiscordCommand::Clear => {
                if let Some(ipc) = client.as_mut() {
                    if connected {
                        let _ = ipc.clear_activity();
                    }
                }
            }
            DiscordCommand::Shutdown => {
                if let Some(ref mut ipc) = client {
                    if connected {
                        let _ = ipc.clear_activity();
                        let _ = ipc.close();
                    }
                }
                break;
            }
        }
    }
}
