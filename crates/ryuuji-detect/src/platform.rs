#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

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
        windows::detect_windows(db)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = db;
        vec![]
    }
}
