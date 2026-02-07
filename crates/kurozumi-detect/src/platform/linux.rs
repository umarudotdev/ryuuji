use crate::PlayerInfo;
use mpris::PlayerFinder;
use tracing::{debug, warn};

/// Detect media players via MPRIS D-Bus on Linux.
pub fn detect_mpris() -> Vec<PlayerInfo> {
    let finder = match PlayerFinder::new() {
        Ok(f) => f,
        Err(e) => {
            warn!("Failed to connect to D-Bus: {e}");
            return vec![];
        }
    };

    let players = match finder.find_all() {
        Ok(p) => p,
        Err(e) => {
            debug!("No MPRIS players found: {e}");
            return vec![];
        }
    };

    players
        .into_iter()
        .filter_map(|player| {
            let identity = player.identity().to_string();
            let metadata = player.get_metadata().ok()?;

            let media_title = metadata.title().map(|s| s.to_string());
            let file_path = metadata.url().and_then(|url| {
                if url.starts_with("file://") {
                    urlencoding_decode(url.strip_prefix("file://").unwrap_or(url))
                } else {
                    Some(url.to_string())
                }
            });

            debug!(player = %identity, title = ?media_title, "Detected MPRIS player");

            Some(PlayerInfo {
                player_name: identity,
                media_title,
                file_path,
            })
        })
        .collect()
}

/// Simple percent-decoding for file paths.
fn urlencoding_decode(s: &str) -> Option<String> {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(
                std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?,
                16,
            ) {
                result.push(byte);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(result).ok()
}
