use tracing::warn;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};

use crate::player_db::PlayerDatabase;
use crate::PlayerInfo;

/// Detect playing media via Windows SMTC (System Media Transport Controls).
///
/// Returns an empty vec on any error — the caller should fall back to
/// EnumWindows-based detection.
pub fn detect_smtc(db: &PlayerDatabase) -> Vec<PlayerInfo> {
    match try_detect_smtc(db) {
        Ok(results) => results,
        Err(e) => {
            warn!("SMTC detection failed: {e}");
            vec![]
        }
    }
}

fn try_detect_smtc(db: &PlayerDatabase) -> windows::core::Result<Vec<PlayerInfo>> {
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.get()?;
    let sessions = manager.GetSessions()?;
    let mut results = Vec::new();

    for i in 0..sessions.Size()? {
        let session = sessions.GetAt(i)?;
        if let Some(info) = extract_session(db, &session) {
            results.push(info);
        }
    }

    Ok(results)
}

fn extract_session(
    db: &PlayerDatabase,
    session: &GlobalSystemMediaTransportControlsSession,
) -> Option<PlayerInfo> {
    let status = session.GetPlaybackInfo().ok()?.PlaybackStatus().ok()?;
    if status != GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing {
        return None;
    }

    let app_id = session.SourceAppUserModelId().ok()?.to_string();

    let (player_name, is_browser) = match db.find_by_smtc(&app_id) {
        Some(def) => (def.name.clone(), def.is_browser),
        None => (app_id_to_display_name(&app_id), false),
    };

    let title = session
        .TryGetMediaPropertiesAsync()
        .ok()?
        .get()
        .ok()?
        .Title()
        .ok()?
        .to_string();

    if title.is_empty() {
        return None;
    }

    Some(PlayerInfo {
        player_name,
        media_title: Some(title),
        file_path: None,
        is_browser,
    })
}

/// Convert an SMTC app user model ID into a human-readable display name.
///
/// Strips `.exe` suffixes, package family name suffixes (after `_`),
/// and path components.
fn app_id_to_display_name(app_id: &str) -> String {
    let mut name = app_id;

    if let Some(pos) = name.rfind('\\') {
        name = &name[pos + 1..];
    }

    if let Some(stripped) = name.strip_suffix(".exe") {
        name = stripped;
    }

    if let Some(pos) = name.rfind('_') {
        let suffix = &name[pos + 1..];
        if suffix.len() >= 8 && suffix.chars().all(|c| c.is_ascii_alphanumeric()) {
            name = &name[..pos];
        }
    }

    name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_name_strips_exe() {
        assert_eq!(app_id_to_display_name("mpv.exe"), "mpv");
    }

    #[test]
    fn test_display_name_strips_path() {
        assert_eq!(
            app_id_to_display_name("C:\\Program Files\\mpv\\mpv.exe"),
            "mpv"
        );
    }

    #[test]
    fn test_display_name_strips_uwp_hash() {
        assert_eq!(
            app_id_to_display_name("Microsoft.ZuneMusic_8wekyb3d8bbwe"),
            "Microsoft.ZuneMusic"
        );
    }

    #[test]
    fn test_display_name_preserves_short_suffix() {
        assert_eq!(app_id_to_display_name("My_App"), "My_App");
    }

    #[test]
    fn test_display_name_plain_name() {
        assert_eq!(app_id_to_display_name("SomePlayer"), "SomePlayer");
    }
}
