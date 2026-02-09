//! Run with: cargo run -p ryuuji-detect --example detect
//!
//! Detects all active media players and prints their info.

fn main() {
    let players = ryuuji_detect::detect_players();

    if players.is_empty() {
        println!("No media players detected.");
    } else {
        for player in &players {
            println!("Player: {}", player.player_name);
            if let Some(title) = &player.media_title {
                println!("  Title: {title}");
            }
            if let Some(path) = &player.file_path {
                println!("  File:  {path}");
            }
            println!();
        }
    }
}
