use crate::players::KNOWN_PLAYERS;
use crate::PlayerInfo;
use tracing::debug;

use windows::Win32::Foundation::*;
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::UI::WindowsAndMessaging::*;

/// Detect media players via Win32 window enumeration.
pub fn detect_windows() -> Vec<PlayerInfo> {
    let mut results = Vec::new();

    unsafe {
        let _ = EnumWindows(
            Some(enum_window_callback),
            LPARAM(&mut results as *mut Vec<PlayerInfo> as isize),
        );
    }

    results
}

unsafe extern "system" fn enum_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let results = &mut *(lparam.0 as *mut Vec<PlayerInfo>);

    if !IsWindowVisible(hwnd).as_bool() {
        return TRUE;
    }

    let mut class_name = [0u16; 256];
    let len = GetClassNameW(hwnd, &mut class_name);
    if len == 0 {
        return TRUE;
    }
    let class = String::from_utf16_lossy(&class_name[..len as usize]);

    for player in KNOWN_PLAYERS {
        if !player.window_class.is_empty() && class.contains(player.window_class) {
            let mut title = [0u16; 512];
            let title_len = GetWindowTextW(hwnd, &mut title);
            let window_title = if title_len > 0 {
                Some(String::from_utf16_lossy(&title[..title_len as usize]))
            } else {
                None
            };

            debug!(player = player.name, title = ?window_title, "Detected Windows player");

            results.push(PlayerInfo {
                player_name: player.name.to_string(),
                media_title: window_title,
                file_path: None,
            });
        }
    }

    TRUE
}
