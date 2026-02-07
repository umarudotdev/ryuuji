#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

use crate::PlayerInfo;

/// Platform-specific detection dispatcher.
pub fn detect() -> Vec<PlayerInfo> {
    #[cfg(target_os = "linux")]
    {
        linux::detect_mpris()
    }
    #[cfg(target_os = "windows")]
    {
        windows::detect_windows()
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        vec![]
    }
}
