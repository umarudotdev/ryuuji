use serde::{Deserialize, Serialize};

/// Embedded player database.
const EMBEDDED_DB: &str = include_str!("../data/players.toml");

/// Definition of a media player and how to detect/identify it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerDef {
    /// Display name (e.g., "mpv", "VLC").
    pub name: String,
    /// Executable names to match against process names.
    #[serde(default)]
    pub executables: Vec<String>,
    /// MPRIS bus name substrings (Linux D-Bus detection).
    #[serde(default)]
    pub mpris_identities: Vec<String>,
    /// Window class names (Windows detection).
    #[serde(default)]
    pub window_classes: Vec<String>,
    /// Regex patterns to extract the media title from the window title.
    /// The first capture group is used as the title.
    #[serde(default)]
    pub title_patterns: Vec<String>,
    /// Whether this player is enabled for detection.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Wrapper for TOML deserialization.
#[derive(Debug, Deserialize)]
struct PlayerDbFile {
    #[serde(rename = "player")]
    players: Vec<PlayerDef>,
}

/// Database of known media players.
#[derive(Debug, Clone)]
pub struct PlayerDatabase {
    pub players: Vec<PlayerDef>,
}

impl PlayerDatabase {
    /// Create an empty database.
    pub fn new() -> Self {
        Self {
            players: Vec::new(),
        }
    }

    /// Load the embedded player database.
    pub fn embedded() -> Self {
        let db: PlayerDbFile =
            toml::from_str(EMBEDDED_DB).expect("embedded players.toml should be valid");
        Self {
            players: db.players,
        }
    }

    /// Merge a user database into this one.
    /// Players with matching names are replaced; new players are appended.
    pub fn merge_user(&mut self, user_db: &PlayerDatabase) {
        for user_player in &user_db.players {
            if let Some(existing) = self.players.iter_mut().find(|p| p.name == user_player.name) {
                *existing = user_player.clone();
            } else {
                self.players.push(user_player.clone());
            }
        }
    }

    /// Load a player database from TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        let db: PlayerDbFile = toml::from_str(toml_str)?;
        Ok(Self {
            players: db.players,
        })
    }

    /// Find a player by MPRIS identity (case-insensitive substring match).
    pub fn find_by_mpris(&self, identity: &str) -> Option<&PlayerDef> {
        let identity_lower = identity.to_lowercase();
        self.players.iter().find(|p| {
            p.enabled
                && p.mpris_identities
                    .iter()
                    .any(|id| identity_lower.contains(&id.to_lowercase()))
        })
    }

    /// Find a player by window class name (exact match).
    pub fn find_by_window_class(&self, class: &str) -> Option<&PlayerDef> {
        self.players
            .iter()
            .find(|p| p.enabled && p.window_classes.iter().any(|wc| wc == class))
    }

    /// Find a player by executable name (exact match, case-insensitive).
    pub fn find_by_executable(&self, exe_name: &str) -> Option<&PlayerDef> {
        let exe_lower = exe_name.to_lowercase();
        self.players
            .iter()
            .find(|p| p.enabled && p.executables.iter().any(|e| e.to_lowercase() == exe_lower))
    }

    /// Extract the media title from a window title using the player's title patterns.
    /// Returns `None` if no pattern matches or the player has no patterns.
    pub fn extract_title(&self, player: &PlayerDef, window_title: &str) -> Option<String> {
        for pattern_str in &player.title_patterns {
            if let Ok(re) = regex::Regex::new(pattern_str) {
                if let Some(caps) = re.captures(window_title) {
                    if let Some(m) = caps.get(1) {
                        let title = m.as_str().trim().to_string();
                        if !title.is_empty() {
                            return Some(title);
                        }
                    }
                }
            }
        }
        None
    }

    /// Get all enabled players.
    pub fn enabled_players(&self) -> impl Iterator<Item = &PlayerDef> {
        self.players.iter().filter(|p| p.enabled)
    }
}

impl Default for PlayerDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_loads() {
        let db = PlayerDatabase::embedded();
        assert!(
            db.players.len() >= 20,
            "Expected 20+ players, got {}",
            db.players.len()
        );
    }

    #[test]
    fn test_find_by_mpris_vlc() {
        let db = PlayerDatabase::embedded();
        let player = db.find_by_mpris("vlc").unwrap();
        assert_eq!(player.name, "VLC");
    }

    #[test]
    fn test_find_by_mpris_case_insensitive() {
        let db = PlayerDatabase::embedded();
        let player = db.find_by_mpris("VLC media player").unwrap();
        assert_eq!(player.name, "VLC");
    }

    #[test]
    fn test_find_by_window_class() {
        let db = PlayerDatabase::embedded();
        let player = db.find_by_window_class("MediaPlayerClassicW").unwrap();
        assert!(player.name == "MPC-HC" || player.name == "MPC-BE");
    }

    #[test]
    fn test_find_by_executable() {
        let db = PlayerDatabase::embedded();
        let player = db.find_by_executable("mpv").unwrap();
        assert_eq!(player.name, "mpv");
    }

    #[test]
    fn test_extract_title_vlc() {
        let db = PlayerDatabase::embedded();
        let player = db.find_by_mpris("vlc").unwrap();
        let title = db
            .extract_title(player, "Sousou no Frieren - 05.mkv - VLC media player")
            .unwrap();
        assert_eq!(title, "Sousou no Frieren - 05.mkv");
    }

    #[test]
    fn test_extract_title_no_pattern() {
        let db = PlayerDatabase::embedded();
        let player = db.find_by_mpris("mpv").unwrap();
        // mpv has no title patterns, so extraction should return None.
        assert!(db.extract_title(player, "anything").is_none());
    }

    #[test]
    fn test_merge_user() {
        let mut db = PlayerDatabase::embedded();
        let original_count = db.players.len();

        let user_toml = r#"
            [[player]]
            name = "VLC"
            executables = ["vlc"]
            mpris_identities = ["vlc"]
            window_classes = ["vlc"]
            title_patterns = []
            enabled = false

            [[player]]
            name = "Custom Player"
            executables = ["custom"]
            mpris_identities = ["custom"]
            window_classes = ["custom"]
            title_patterns = []
            enabled = true
        "#;
        let user_db = PlayerDatabase::from_toml(user_toml).unwrap();
        db.merge_user(&user_db);

        // VLC should be disabled now.
        assert!(db.find_by_mpris("vlc").is_none());

        // Custom Player should be added.
        assert_eq!(db.players.len(), original_count + 1);
        let custom = db.find_by_mpris("custom").unwrap();
        assert_eq!(custom.name, "Custom Player");
    }

    #[test]
    fn test_disabled_player_not_found() {
        let toml = r#"
            [[player]]
            name = "Disabled"
            executables = ["disabled"]
            mpris_identities = ["disabled"]
            window_classes = ["disabled"]
            enabled = false
        "#;
        let db = PlayerDatabase::from_toml(toml).unwrap();
        assert!(db.find_by_mpris("disabled").is_none());
    }
}
