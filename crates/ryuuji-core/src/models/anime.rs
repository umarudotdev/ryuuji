use serde::{Deserialize, Serialize};

/// Cross-service anime identifiers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnimeIds {
    pub anilist: Option<u64>,
    pub kitsu: Option<u64>,
    pub mal: Option<u64>,
}

/// A single title with language variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimeTitle {
    pub romaji: Option<String>,
    pub english: Option<String>,
    pub native: Option<String>,
}

impl AnimeTitle {
    /// Returns the best available display title.
    pub fn preferred(&self) -> &str {
        self.romaji
            .as_deref()
            .or(self.english.as_deref())
            .or(self.native.as_deref())
            .unwrap_or("Unknown")
    }
}

/// Core anime entity, stored locally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anime {
    pub id: i64,
    pub ids: AnimeIds,
    pub title: AnimeTitle,
    pub synonyms: Vec<String>,
    pub episodes: Option<u32>,
    pub cover_url: Option<String>,
    pub season: Option<String>,
    pub year: Option<u32>,
    pub synopsis: Option<String>,
    pub genres: Vec<String>,
    pub media_type: Option<String>,
    pub airing_status: Option<String>,
    pub mean_score: Option<f32>,
    pub studios: Vec<String>,
    pub source: Option<String>,
    pub rating: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}
