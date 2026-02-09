use serde::Deserialize;

use crate::traits::{AnimeSearchResult, UserListEntry};

// ── GraphQL response wrappers ────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct GraphQLResponse<T> {
    pub data: T,
}

// ── Search / media queries ───────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PageResponse {
    #[serde(rename = "Page")]
    pub page: PageData,
}

#[derive(Debug, Deserialize)]
pub struct PageData {
    pub media: Vec<AniListMedia>,
}

#[derive(Debug, Deserialize)]
pub struct AniListMedia {
    pub id: u64,
    pub title: Option<AniListTitle>,
    pub episodes: Option<u32>,
    #[serde(rename = "coverImage")]
    pub cover_image: Option<CoverImage>,
    #[serde(rename = "meanScore")]
    pub mean_score: Option<u32>,
    pub season: Option<String>,
    #[serde(rename = "seasonYear")]
    pub season_year: Option<u32>,
    pub genres: Option<Vec<String>>,
    pub studios: Option<StudioConnection>,
    pub format: Option<String>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub source: Option<String>,
    pub synonyms: Option<Vec<String>>,
    #[serde(rename = "startDate")]
    pub start_date: Option<FuzzyDate>,
    #[serde(rename = "endDate")]
    pub end_date: Option<FuzzyDate>,
}

#[derive(Debug, Deserialize)]
pub struct AniListTitle {
    pub romaji: Option<String>,
    pub english: Option<String>,
    pub native: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CoverImage {
    pub large: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StudioConnection {
    pub nodes: Option<Vec<StudioNode>>,
}

#[derive(Debug, Deserialize)]
pub struct StudioNode {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct FuzzyDate {
    pub year: Option<u32>,
    pub month: Option<u32>,
    pub day: Option<u32>,
}

// ── User list queries ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct MediaListCollectionResponse {
    #[serde(rename = "MediaListCollection")]
    pub media_list_collection: MediaListCollection,
}

#[derive(Debug, Deserialize)]
pub struct MediaListCollection {
    pub lists: Vec<MediaListGroup>,
}

#[derive(Debug, Deserialize)]
pub struct MediaListGroup {
    pub entries: Vec<MediaListEntry>,
}

#[derive(Debug, Deserialize)]
pub struct MediaListEntry {
    #[serde(rename = "mediaId")]
    pub media_id: u64,
    pub progress: u32,
    pub score: Option<f32>,
    pub status: Option<String>,
    pub media: AniListMedia,
}

// ── Viewer query ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ViewerResponse {
    #[serde(rename = "Viewer")]
    pub viewer: Viewer,
}

#[derive(Debug, Deserialize)]
pub struct Viewer {
    pub id: u64,
    pub name: String,
}

// ── Conversions ──────────────────────────────────────────────────

impl FuzzyDate {
    pub fn to_string_opt(&self) -> Option<String> {
        let y = self.year?;
        let m = self.month.unwrap_or(1);
        let d = self.day.unwrap_or(1);
        Some(format!("{y:04}-{m:02}-{d:02}"))
    }
}

fn capitalize_season(s: &str) -> String {
    match s {
        "WINTER" => "Winter".into(),
        "SPRING" => "Spring".into(),
        "SUMMER" => "Summer".into(),
        "FALL" => "Fall".into(),
        other => {
            let mut c = other.chars();
            match c.next() {
                Some(first) => first.to_uppercase().to_string() + &c.as_str().to_lowercase(),
                None => String::new(),
            }
        }
    }
}

fn map_anilist_status(s: &str) -> &'static str {
    match s {
        "CURRENT" => "watching",
        "COMPLETED" => "completed",
        "PAUSED" => "on_hold",
        "DROPPED" => "dropped",
        "PLANNING" => "plan_to_watch",
        "REPEATING" => "watching",
        _ => "watching",
    }
}

fn map_anilist_format(s: &str) -> String {
    s.to_lowercase()
}

impl AniListMedia {
    pub fn into_search_result(self) -> AnimeSearchResult {
        let title = self
            .title
            .as_ref()
            .and_then(|t| t.romaji.clone())
            .unwrap_or_default();
        let title_english = self.title.as_ref().and_then(|t| t.english.clone());

        AnimeSearchResult {
            service_id: self.id,
            title,
            title_english,
            episodes: self.episodes,
            cover_url: self.cover_image.and_then(|c| c.large),
            media_type: self.format.map(|f| map_anilist_format(&f)),
            status: self.status.as_deref().map(|s| s.to_lowercase()),
            synopsis: self.description,
            genres: self.genres.unwrap_or_default(),
            mean_score: self.mean_score.map(|s| s as f32 / 10.0),
            season: self.season.as_deref().map(capitalize_season),
            year: self.season_year,
        }
    }
}

impl MediaListEntry {
    pub fn into_user_list_entry(self) -> UserListEntry {
        let title = self
            .media
            .title
            .as_ref()
            .and_then(|t| t.romaji.clone())
            .unwrap_or_default();

        UserListEntry {
            service_id: self.media_id,
            title,
            watched_episodes: self.progress,
            total_episodes: self.media.episodes,
            status: self
                .status
                .as_deref()
                .map(map_anilist_status)
                .unwrap_or("watching")
                .to_string(),
            score: self.score.map(|s| s / 10.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_search_response() {
        let json = r#"{
            "data": {
                "Page": {
                    "media": [
                        {
                            "id": 154587,
                            "title": {
                                "romaji": "Sousou no Frieren",
                                "english": "Frieren: Beyond Journey's End",
                                "native": "葬送のフリーレン"
                            },
                            "episodes": 28,
                            "coverImage": { "large": "https://s4.anilist.co/file/anilistcdn/media/anime/cover/large/154587.jpg" },
                            "meanScore": 90,
                            "season": "FALL",
                            "seasonYear": 2023,
                            "genres": ["Adventure", "Drama", "Fantasy"],
                            "format": "TV",
                            "status": "FINISHED",
                            "description": "After the party defeats the Demon King...",
                            "source": "MANGA",
                            "synonyms": ["Frieren at the Funeral"]
                        }
                    ]
                }
            }
        }"#;

        let resp: GraphQLResponse<PageResponse> = serde_json::from_str(json).unwrap();
        let media = resp.data.page.media;
        assert_eq!(media.len(), 1);

        let result = media.into_iter().next().unwrap().into_search_result();
        assert_eq!(result.service_id, 154587);
        assert_eq!(result.title, "Sousou no Frieren");
        assert_eq!(
            result.title_english.as_deref(),
            Some("Frieren: Beyond Journey's End")
        );
        assert_eq!(result.episodes, Some(28));
        assert_eq!(result.mean_score, Some(9.0));
        assert_eq!(result.season.as_deref(), Some("Fall"));
        assert_eq!(result.year, Some(2023));
    }

    #[test]
    fn test_deserialize_user_list_response() {
        let json = r#"{
            "data": {
                "MediaListCollection": {
                    "lists": [
                        {
                            "entries": [
                                {
                                    "mediaId": 154587,
                                    "progress": 14,
                                    "score": 9.0,
                                    "status": "CURRENT",
                                    "media": {
                                        "id": 154587,
                                        "title": { "romaji": "Sousou no Frieren" },
                                        "episodes": 28
                                    }
                                }
                            ]
                        }
                    ]
                }
            }
        }"#;

        let resp: GraphQLResponse<MediaListCollectionResponse> =
            serde_json::from_str(json).unwrap();
        let lists = resp.data.media_list_collection.lists;
        assert_eq!(lists.len(), 1);

        let entry = lists
            .into_iter()
            .next()
            .unwrap()
            .entries
            .into_iter()
            .next()
            .unwrap()
            .into_user_list_entry();
        assert_eq!(entry.service_id, 154587);
        assert_eq!(entry.title, "Sousou no Frieren");
        assert_eq!(entry.watched_episodes, 14);
        assert_eq!(entry.total_episodes, Some(28));
        assert_eq!(entry.status, "watching");
        // AniList score is on 0-100 scale, we divide by 10
        assert_eq!(entry.score, Some(0.9));
    }

    #[test]
    fn test_deserialize_minimal_media() {
        let json = r#"{ "id": 1, "title": { "romaji": "Test" } }"#;
        let media: AniListMedia = serde_json::from_str(json).unwrap();
        let result = media.into_search_result();
        assert_eq!(result.service_id, 1);
        assert_eq!(result.title, "Test");
        assert!(result.cover_url.is_none());
        assert!(result.title_english.is_none());
    }

    #[test]
    fn test_status_mapping() {
        assert_eq!(map_anilist_status("CURRENT"), "watching");
        assert_eq!(map_anilist_status("COMPLETED"), "completed");
        assert_eq!(map_anilist_status("PAUSED"), "on_hold");
        assert_eq!(map_anilist_status("DROPPED"), "dropped");
        assert_eq!(map_anilist_status("PLANNING"), "plan_to_watch");
    }
}
