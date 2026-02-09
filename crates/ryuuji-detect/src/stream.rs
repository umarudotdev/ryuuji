use serde::{Deserialize, Serialize};

/// Embedded stream provider database.
const EMBEDDED_DB: &str = include_str!("../data/streams.toml");

/// Definition of a streaming service and how to detect it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDef {
    /// Display name (e.g., "Crunchyroll", "Netflix").
    pub name: String,
    /// Regex patterns matched against the URL (Linux MPRIS metadata URL).
    #[serde(default)]
    pub url_patterns: Vec<String>,
    /// Regex with capture group 1 to extract the anime title from the window/tab title.
    pub title_pattern: String,
    /// Whether this stream provider is enabled for detection.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Wrapper for TOML deserialization.
#[derive(Debug, Deserialize)]
struct StreamDbFile {
    #[serde(rename = "stream")]
    streams: Vec<StreamDef>,
}

/// Result of a successful stream detection.
#[derive(Debug, Clone)]
pub struct StreamMatch {
    /// Name of the streaming service (e.g., "Crunchyroll").
    pub service_name: String,
    /// The anime title extracted from the browser title.
    pub extracted_title: String,
}

/// Database of known streaming services.
#[derive(Debug, Clone)]
pub struct StreamDatabase {
    streams: Vec<StreamDef>,
    compiled_url: Vec<Vec<regex::Regex>>,
    compiled_title: Vec<Option<regex::Regex>>,
}

impl StreamDatabase {
    /// Load the embedded stream database.
    pub fn embedded() -> Self {
        Self::from_toml(EMBEDDED_DB).expect("embedded streams.toml should be valid")
    }

    /// Load a stream database from a TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        let db: StreamDbFile = toml::from_str(toml_str)?;
        let compiled_url = db
            .streams
            .iter()
            .map(|s| {
                s.url_patterns
                    .iter()
                    .filter_map(|p| regex::Regex::new(p).ok())
                    .collect()
            })
            .collect();
        let compiled_title = db
            .streams
            .iter()
            .map(|s| regex::Regex::new(&s.title_pattern).ok())
            .collect();
        Ok(Self {
            streams: db.streams,
            compiled_url,
            compiled_title,
        })
    }

    /// Merge a user database into this one.
    /// Streams with matching names are replaced; new streams are appended.
    pub fn merge_user(&mut self, user_db: &StreamDatabase) {
        for (i, user_stream) in user_db.streams.iter().enumerate() {
            if let Some(pos) = self.streams.iter().position(|s| s.name == user_stream.name) {
                self.streams[pos] = user_stream.clone();
                self.compiled_url[pos] = user_db.compiled_url[i].clone();
                self.compiled_title[pos] = user_db.compiled_title[i].clone();
            } else {
                self.streams.push(user_stream.clone());
                self.compiled_url.push(user_db.compiled_url[i].clone());
                self.compiled_title.push(user_db.compiled_title[i].clone());
            }
        }
    }

    /// Find the first enabled stream whose URL patterns match.
    pub fn match_url(&self, url: &str) -> Option<usize> {
        self.streams.iter().enumerate().find_map(|(i, s)| {
            if s.enabled && self.compiled_url[i].iter().any(|re| re.is_match(url)) {
                Some(i)
            } else {
                None
            }
        })
    }

    /// Find the first enabled stream whose title pattern matches.
    pub fn match_title(&self, title: &str) -> Option<usize> {
        self.streams.iter().enumerate().find_map(|(i, s)| {
            if s.enabled {
                if let Some(re) = &self.compiled_title[i] {
                    if re.is_match(title) {
                        return Some(i);
                    }
                }
            }
            None
        })
    }

    /// Extract the anime title from a browser title using the stream's title_pattern.
    pub fn extract_title(&self, index: usize, title: &str) -> Option<String> {
        let re = self.compiled_title.get(index)?.as_ref()?;
        let caps = re.captures(title)?;
        let extracted = caps.get(1)?.as_str().trim().to_string();
        if extracted.is_empty() {
            None
        } else {
            Some(extracted)
        }
    }

    /// Get the service name for a matched stream index.
    pub fn service_name(&self, index: usize) -> Option<&str> {
        self.streams.get(index).map(|s| s.name.as_str())
    }

    /// Number of stream definitions.
    pub fn len(&self) -> usize {
        self.streams.len()
    }

    /// Whether the database is empty.
    pub fn is_empty(&self) -> bool {
        self.streams.is_empty()
    }
}

/// Detect if a browser player is watching a streaming service.
///
/// Returns `None` if the player is not a browser, or if no streaming service matches.
pub fn detect_stream(
    player: &crate::PlayerInfo,
    stream_db: &StreamDatabase,
) -> Option<StreamMatch> {
    if !player.is_browser {
        return None;
    }

    // Strategy 1: URL matching (primary — available on Linux via MPRIS metadata URL).
    if let Some(url) = player.file_path.as_deref() {
        if url.starts_with("http") {
            if let Some(idx) = stream_db.match_url(url) {
                // URL matched a service — extract the title from the browser tab title.
                let title = player.media_title.as_deref()?;
                let extracted = stream_db.extract_title(idx, title)?;
                return Some(StreamMatch {
                    service_name: stream_db.service_name(idx)?.to_string(),
                    extracted_title: extracted,
                });
            }
        }
    }

    // Strategy 2: Title matching (fallback — Windows or when no URL is available).
    if let Some(title) = player.media_title.as_deref() {
        if let Some(idx) = stream_db.match_title(title) {
            let extracted = stream_db.extract_title(idx, title)?;
            return Some(StreamMatch {
                service_name: stream_db.service_name(idx)?.to_string(),
                extracted_title: extracted,
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PlayerInfo;

    #[test]
    fn test_embedded_loads() {
        let db = StreamDatabase::embedded();
        assert_eq!(db.len(), 6, "Expected 6 stream providers, got {}", db.len());
    }

    #[test]
    fn test_match_url_crunchyroll() {
        let db = StreamDatabase::embedded();
        let idx = db
            .match_url("https://www.crunchyroll.com/watch/G1XHJV2W1/episode-5")
            .unwrap();
        assert_eq!(db.service_name(idx).unwrap(), "Crunchyroll");
    }

    #[test]
    fn test_match_url_netflix() {
        let db = StreamDatabase::embedded();
        let idx = db
            .match_url("https://www.netflix.com/watch/81564905")
            .unwrap();
        assert_eq!(db.service_name(idx).unwrap(), "Netflix");
    }

    #[test]
    fn test_match_url_non_streaming() {
        let db = StreamDatabase::embedded();
        assert!(db
            .match_url("https://www.youtube.com/watch?v=abc")
            .is_none());
    }

    #[test]
    fn test_match_title_crunchyroll() {
        let db = StreamDatabase::embedded();
        let idx = db
            .match_title("Attack on Titan - Watch on Crunchyroll")
            .unwrap();
        assert_eq!(db.service_name(idx).unwrap(), "Crunchyroll");
    }

    #[test]
    fn test_match_title_netflix() {
        let db = StreamDatabase::embedded();
        let idx = db.match_title("One Piece | Netflix").unwrap();
        assert_eq!(db.service_name(idx).unwrap(), "Netflix");
    }

    #[test]
    fn test_extract_title_crunchyroll() {
        let db = StreamDatabase::embedded();
        let idx = db
            .match_title("Attack on Titan - Watch on Crunchyroll")
            .unwrap();
        let title = db
            .extract_title(idx, "Attack on Titan - Watch on Crunchyroll")
            .unwrap();
        assert_eq!(title, "Attack on Titan");
    }

    #[test]
    fn test_extract_title_netflix() {
        let db = StreamDatabase::embedded();
        let idx = db.match_title("One Piece | Netflix").unwrap();
        let title = db.extract_title(idx, "One Piece | Netflix").unwrap();
        assert_eq!(title, "One Piece");
    }

    #[test]
    fn test_extract_title_jellyfin() {
        let db = StreamDatabase::embedded();
        let idx = db
            .match_title("Frieren: Beyond Journey\u{2019}s End S01E05 \u{2013} Jellyfin")
            .unwrap();
        let title = db
            .extract_title(
                idx,
                "Frieren: Beyond Journey\u{2019}s End S01E05 \u{2013} Jellyfin",
            )
            .unwrap();
        assert_eq!(title, "Frieren: Beyond Journey\u{2019}s End S01E05");
    }

    #[test]
    fn test_detect_stream_non_browser() {
        let db = StreamDatabase::embedded();
        let player = PlayerInfo {
            player_name: "mpv".into(),
            media_title: Some("Attack on Titan - Watch on Crunchyroll".into()),
            file_path: None,
            is_browser: false,
        };
        assert!(detect_stream(&player, &db).is_none());
    }

    #[test]
    fn test_detect_stream_url_path() {
        let db = StreamDatabase::embedded();
        let player = PlayerInfo {
            player_name: "Firefox".into(),
            media_title: Some("Attack on Titan - Watch on Crunchyroll".into()),
            file_path: Some("https://www.crunchyroll.com/watch/G1XHJV2W1/episode-5".into()),
            is_browser: true,
        };
        let m = detect_stream(&player, &db).unwrap();
        assert_eq!(m.service_name, "Crunchyroll");
        assert_eq!(m.extracted_title, "Attack on Titan");
    }

    #[test]
    fn test_detect_stream_title_path() {
        let db = StreamDatabase::embedded();
        let player = PlayerInfo {
            player_name: "Chrome".into(),
            media_title: Some("One Piece | Netflix".into()),
            file_path: None,
            is_browser: true,
        };
        let m = detect_stream(&player, &db).unwrap();
        assert_eq!(m.service_name, "Netflix");
        assert_eq!(m.extracted_title, "One Piece");
    }

    #[test]
    fn test_detect_stream_browser_non_streaming() {
        let db = StreamDatabase::embedded();
        let player = PlayerInfo {
            player_name: "Firefox".into(),
            media_title: Some("GitHub - rust-lang/rust".into()),
            file_path: Some("https://github.com/rust-lang/rust".into()),
            is_browser: true,
        };
        assert!(detect_stream(&player, &db).is_none());
    }

    #[test]
    fn test_merge_user() {
        let mut db = StreamDatabase::embedded();
        assert_eq!(db.len(), 6);

        let user_toml = r#"
            [[stream]]
            name = "Crunchyroll"
            url_patterns = ["crunchyroll\\.com/watch/"]
            title_pattern = "^(.+?)\\s*-\\s*Crunchyroll$"
            enabled = false

            [[stream]]
            name = "Funimation"
            url_patterns = ["funimation\\.com/v/"]
            title_pattern = "^(.+?)\\s*-\\s*Funimation$"
            enabled = true
        "#;
        let user_db = StreamDatabase::from_toml(user_toml).unwrap();
        db.merge_user(&user_db);

        // Crunchyroll should be disabled now — no URL match.
        assert!(db
            .match_url("https://www.crunchyroll.com/watch/G1XHJV2W1/episode-5")
            .is_none());

        // Funimation should be added.
        assert_eq!(db.len(), 7);
        let idx = db
            .match_url("https://www.funimation.com/v/attack-on-titan")
            .unwrap();
        assert_eq!(db.service_name(idx).unwrap(), "Funimation");
    }
}
