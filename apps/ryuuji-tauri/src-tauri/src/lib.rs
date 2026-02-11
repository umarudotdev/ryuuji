use std::sync::Arc;
use std::time::Duration;

use ryuuji_api::traits::AnimeSearchResult;
use ryuuji_core::config::AppConfig;
use ryuuji_core::models::WatchStatus;
use ryuuji_core::storage::LibraryRow;
use ryuuji_runtime::{DetectionStateDto, LibraryPatchDto, Runtime, ServiceLoginDto};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

struct AppState {
    runtime: Arc<Runtime>,
    detection_task: Mutex<Option<JoinHandle<()>>>,
}

#[tauri::command]
async fn get_app_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    Ok(state.runtime.get_config().await)
}

#[tauri::command]
async fn update_app_config(state: State<'_, AppState>, config: AppConfig) -> Result<(), String> {
    state
        .runtime
        .update_config(config)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_now_playing_state(state: State<'_, AppState>) -> Result<DetectionStateDto, String> {
    Ok(state.runtime.get_detection_state().await)
}

#[tauri::command]
async fn run_detection_tick(state: State<'_, AppState>) -> Result<DetectionStateDto, String> {
    state
        .runtime
        .run_detection_tick()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_library(
    state: State<'_, AppState>,
    status: Option<String>,
    query: Option<String>,
) -> Result<Vec<LibraryRow>, String> {
    let status = match status {
        Some(raw) => Some(
            parse_watch_status(&raw).ok_or_else(|| format!("invalid watch status: {raw}"))?,
        ),
        None => None,
    };

    state
        .runtime
        .get_library(status, query)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn patch_library_entry(
    state: State<'_, AppState>,
    anime_id: i64,
    patch: LibraryPatchDto,
) -> Result<(), String> {
    state
        .runtime
        .patch_library_entry(anime_id, patch)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_library_entry(state: State<'_, AppState>, anime_id: i64) -> Result<(), String> {
    state
        .runtime
        .delete_library_entry(anime_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn search_remote(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<AnimeSearchResult>, String> {
    state
        .runtime
        .search_remote(query)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_to_library_from_search(
    state: State<'_, AppState>,
    result: AnimeSearchResult,
) -> Result<i64, String> {
    state
        .runtime
        .add_remote_search_result_to_library(result)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn service_login(
    state: State<'_, AppState>,
    service: String,
    input: ServiceLoginDto,
) -> Result<(), String> {
    state
        .runtime
        .service_login(&service, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn service_import(state: State<'_, AppState>, service: String) -> Result<usize, String> {
    state
        .runtime
        .service_import(&service)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn service_auth_state(state: State<'_, AppState>, service: String) -> Result<bool, String> {
    state
        .runtime
        .service_auth_state(&service)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_detection_loop(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    let mut guard = state.detection_task.lock().await;
    if guard.is_some() {
        return Ok(());
    }

    let runtime = state.runtime.clone();
    *guard = Some(tokio::spawn(async move {
        loop {
            match runtime.run_detection_tick().await {
                Ok(snapshot) => {
                    let _ = app.emit("ryuuji://detection-updated", &snapshot);
                    let _ = app.emit("ryuuji://status-message", &snapshot.status_message);
                }
                Err(err) => {
                    let _ = app.emit("ryuuji://status-message", format!("Detection error: {err}"));
                }
            }

            let interval_seconds = runtime.get_config().await.general.detection_interval;
            tokio::time::sleep(Duration::from_secs(interval_seconds.max(1))).await;
        }
    }));

    Ok(())
}

#[tauri::command]
async fn stop_detection_loop(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.detection_task.lock().await;
    if let Some(task) = guard.take() {
        task.abort();
    }
    Ok(())
}

fn parse_watch_status(s: &str) -> Option<WatchStatus> {
    match s {
        "watching" => Some(WatchStatus::Watching),
        "completed" => Some(WatchStatus::Completed),
        "on_hold" => Some(WatchStatus::OnHold),
        "dropped" => Some(WatchStatus::Dropped),
        "plan_to_watch" => Some(WatchStatus::PlanToWatch),
        _ => None,
    }
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter("ryuuji=debug,ryuuji_runtime=debug,ryuuji_tauri=debug")
        .init();

    let runtime = Arc::new(Runtime::new().expect("failed to initialize runtime"));

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }))
        .manage(AppState {
            runtime,
            detection_task: Mutex::new(None),
        })
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Some(state) = handle.try_state::<AppState>() {
                    let mut guard = state.detection_task.lock().await;
                    if guard.is_none() {
                        let runtime = state.runtime.clone();
                        let app_handle = handle.clone();
                        *guard = Some(tokio::spawn(async move {
                            loop {
                                match runtime.run_detection_tick().await {
                                    Ok(snapshot) => {
                                        let _ = app_handle
                                            .emit("ryuuji://detection-updated", &snapshot);
                                        let _ = app_handle.emit(
                                            "ryuuji://status-message",
                                            &snapshot.status_message,
                                        );
                                    }
                                    Err(err) => {
                                        let _ = app_handle.emit(
                                            "ryuuji://status-message",
                                            format!("Detection error: {err}"),
                                        );
                                    }
                                }

                                let interval_seconds =
                                    runtime.get_config().await.general.detection_interval;
                                tokio::time::sleep(Duration::from_secs(interval_seconds.max(1)))
                                    .await;
                            }
                        }));
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_config,
            update_app_config,
            get_now_playing_state,
            run_detection_tick,
            get_library,
            patch_library_entry,
            delete_library_entry,
            search_remote,
            add_to_library_from_search,
            service_login,
            service_import,
            service_auth_state,
            start_detection_loop,
            stop_detection_loop
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
