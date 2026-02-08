use std::collections::HashMap;
use std::path::PathBuf;

/// State of a cover image for a given anime.
#[derive(Debug, Clone)]
pub enum CoverState {
    #[allow(dead_code)]
    NotRequested,
    Loading,
    Loaded(PathBuf),
    Failed,
}

/// In-memory cache mapping anime IDs to their cover image state.
#[derive(Debug, Default)]
pub struct CoverCache {
    pub states: HashMap<i64, CoverState>,
}

impl CoverCache {
    pub fn get(&self, anime_id: i64) -> Option<&CoverState> {
        self.states.get(&anime_id)
    }
}

/// Directory for cached cover images.
pub fn covers_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", "kurozumi")
        .map(|dirs| dirs.data_dir().join("covers"))
        .unwrap_or_else(|| PathBuf::from("covers"))
}

/// Expected file path for a cover image.
pub fn cover_path(anime_id: i64) -> PathBuf {
    covers_dir().join(format!("{anime_id}.jpg"))
}

/// Download a cover image and save it to disk. Returns the saved path.
pub async fn fetch_cover(anime_id: i64, url: String) -> Result<PathBuf, String> {
    let dir = covers_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let path = cover_path(anime_id);

    let bytes = reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(path)
}
