#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "windows")]
pub mod windows_smtc;

use crate::player_db::PlayerDatabase;
use crate::PlayerInfo;

/// Platform-specific detection dispatcher.
pub fn detect(db: &PlayerDatabase) -> Vec<PlayerInfo> {
    #[cfg(target_os = "linux")]
    {
        linux::detect_mpris(db)
    }
    #[cfg(target_os = "windows")]
    {
        detect_windows_combined(db)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = db;
        vec![]
    }
}

/// SMTC-first detection with EnumWindows fallback.
///
/// Runs SMTC detection first (structured metadata), then fills in any players
/// that only EnumWindows can see. Deduplicates by player name so each player
/// appears at most once, with the SMTC result winning ties.
#[cfg(target_os = "windows")]
fn detect_windows_combined(db: &PlayerDatabase) -> Vec<PlayerInfo> {
    use std::collections::HashSet;

    let mut results = windows_smtc::detect_smtc(db);

    let smtc_names: HashSet<String> = results.iter().map(|p| p.player_name.clone()).collect();

    let win32_results = windows::detect_windows(db);
    results.extend(
        win32_results
            .into_iter()
            .filter(|p| !smtc_names.contains(&p.player_name)),
    );

    results
}
