use crate::player_db::PlayerDatabase;
use crate::PlayerInfo;
use tracing::debug;

use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Context passed through the EnumWindows callback.
struct EnumContext<'a> {
    db: &'a PlayerDatabase,
    results: Vec<PlayerInfo>,
}

/// Detect media players via Win32 window enumeration.
pub fn detect_windows(db: &PlayerDatabase) -> Vec<PlayerInfo> {
    let mut ctx = EnumContext {
        db,
        results: Vec::new(),
    };

    unsafe {
        let _ = EnumWindows(
            Some(enum_window_callback),
            LPARAM(&mut ctx as *mut EnumContext as isize),
        );
    }

    ctx.results
}

unsafe extern "system" fn enum_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut EnumContext);

    if !IsWindowVisible(hwnd).as_bool() {
        return TRUE;
    }

    let mut class_name = [0u16; 256];
    let len = GetClassNameW(hwnd, &mut class_name);
    if len == 0 {
        return TRUE;
    }
    let class = String::from_utf16_lossy(&class_name[..len as usize]);

    if let Some(player) = ctx.db.find_by_window_class(&class) {
        let mut title = [0u16; 512];
        let title_len = GetWindowTextW(hwnd, &mut title);
        let window_title = if title_len > 0 {
            Some(String::from_utf16_lossy(&title[..title_len as usize]))
        } else {
            None
        };

        // Try to extract the media title from the window title using player patterns.
        let media_title = window_title
            .as_deref()
            .and_then(|t| ctx.db.extract_title(player, t))
            .or(window_title);

        debug!(player = %player.name, title = ?media_title, "Detected Windows player");

        ctx.results.push(PlayerInfo {
            player_name: player.name.clone(),
            media_title,
            file_path: None,
        });
    }

    TRUE
}
