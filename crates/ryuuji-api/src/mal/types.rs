use serde::Deserialize;

use crate::traits::{AnimeSearchResult, UserListEntry};

// ── Search / anime detail responses ─────────────────────────────

#[derive(Debug, Deserialize)]
pub struct MalSearchResponse {
    pub data: Vec<MalSearchNode>,
}

#[derive(Debug, Deserialize)]
pub struct MalSearchNode {
    pub node: MalAnimeNode,
}

#[derive(Debug, Deserialize)]
pub struct MalAnimeNode {
    pub id: u64,
    pub title: String,
    pub main_picture: Option<MalPicture>,
    pub alternative_titles: Option<MalAlternativeTitles>,
    pub num_episodes: Option<u32>,
    pub media_type: Option<String>,
    pub status: Option<String>,
    pub synopsis: Option<String>,
    pub genres: Option<Vec<MalGenre>>,
    pub mean: Option<f32>,
    pub studios: Option<Vec<MalStudio>>,
    pub source: Option<String>,
    pub rating: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub start_season: Option<MalSeason>,
}

#[derive(Debug, Deserialize)]
pub struct MalPicture {
    pub medium: Option<String>,
    pub large: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MalAlternativeTitles {
    pub en: Option<String>,
    pub ja: Option<String>,
    pub synonyms: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct MalGenre {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct MalStudio {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct MalSeason {
    pub year: u32,
    pub season: String,
}

// ── Season browse response ──────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct MalSeasonResponse {
    pub data: Vec<MalSearchNode>,
    pub paging: MalPaging,
}

// ── Status mapping ──────────────────────────────────────────────

/// Map internal status strings to MAL API status values.
pub fn map_status_to_mal(status: &str) -> &'static str {
    match status {
        "watching" => "watching",
        "completed" => "completed",
        "on_hold" => "on_hold",
        "dropped" => "dropped",
        "plan_to_watch" => "plan_to_watch",
        _ => "plan_to_watch",
    }
}

// ── User anime list responses ───────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct MalListResponse {
    pub data: Vec<MalAnimeListItem>,
    pub paging: MalPaging,
}

#[derive(Debug, Deserialize)]
pub struct MalAnimeListItem {
    pub node: MalAnimeNode,
    pub list_status: MalListStatus,
}

#[derive(Debug, Deserialize)]
pub struct MalListStatus {
    pub status: Option<String>,
    pub num_episodes_watched: Option<u32>,
    pub score: Option<u32>,
    #[allow(dead_code)]
    pub updated_at: Option<String>,
    pub start_date: Option<String>,
    pub finish_date: Option<String>,
    pub comments: Option<String>,
    pub is_rewatching: Option<bool>,
    pub num_times_rewatched: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct MalPaging {
    pub next: Option<String>,
}

// ── Conversions to shared trait types ───────────────────────────

impl MalAnimeNode {
    pub fn into_search_result(self) -> AnimeSearchResult {
        let season_str = self.start_season.as_ref().map(|s| {
            let mut c = s.season.chars();
            match c.next() {
                Some(first) => first.to_uppercase().to_string() + c.as_str(),
                None => s.season.clone(),
            }
        });
        let year = self.start_season.as_ref().map(|s| s.year);
        AnimeSearchResult {
            service_id: self.id,
            title: self.title,
            title_english: self.alternative_titles.and_then(|alt| alt.en),
            episodes: self.num_episodes,
            cover_url: self.main_picture.and_then(|pic| pic.medium),
            media_type: self.media_type,
            status: self.status,
            synopsis: self.synopsis,
            genres: self
                .genres
                .map(|g| g.into_iter().map(|x| x.name).collect())
                .unwrap_or_default(),
            mean_score: self.mean,
            season: season_str,
            year,
        }
    }
}

impl MalAnimeListItem {
    pub fn into_user_list_entry(self) -> UserListEntry {
        UserListEntry {
            service_id: self.node.id,
            title: self.node.title,
            watched_episodes: self.list_status.num_episodes_watched.unwrap_or(0),
            total_episodes: self.node.num_episodes,
            status: self
                .list_status
                .status
                .unwrap_or_else(|| "watching".to_string()),
            score: self.list_status.score.map(|s| s as f32),
            start_date: self.list_status.start_date,
            finish_date: self.list_status.finish_date,
            notes: self.list_status.comments,
            rewatching: self.list_status.is_rewatching.unwrap_or(false),
            rewatch_count: self.list_status.num_times_rewatched.unwrap_or(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_search_response() {
        let json = r#"{
            "data": [
                {
                    "node": {
                        "id": 52991,
                        "title": "Sousou no Frieren",
                        "main_picture": {
                            "medium": "https://cdn.myanimelist.net/images/anime/1/52991.jpg",
                            "large": "https://cdn.myanimelist.net/images/anime/1/52991l.jpg"
                        },
                        "alternative_titles": {
                            "en": "Frieren: Beyond Journey's End",
                            "ja": "葬送のフリーレン",
                            "synonyms": ["Frieren"]
                        },
                        "num_episodes": 28,
                        "media_type": "tv",
                        "status": "finished_airing",
                        "synopsis": "After the party defeats the Demon King...",
                        "genres": [{"id": 1, "name": "Action"}, {"id": 2, "name": "Adventure"}],
                        "mean": 9.32,
                        "studios": [{"id": 11, "name": "Madhouse"}],
                        "source": "manga",
                        "rating": "pg_13",
                        "start_date": "2023-09-29",
                        "end_date": "2024-03-22",
                        "start_season": {"year": 2023, "season": "fall"}
                    }
                }
            ]
        }"#;

        let resp: MalSearchResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.data.len(), 1);

        let result = resp
            .data
            .into_iter()
            .next()
            .unwrap()
            .node
            .into_search_result();
        assert_eq!(result.service_id, 52991);
        assert_eq!(result.title, "Sousou no Frieren");
        assert_eq!(
            result.title_english.as_deref(),
            Some("Frieren: Beyond Journey's End")
        );
        assert_eq!(result.episodes, Some(28));
        assert!(result.cover_url.is_some());
    }

    #[test]
    fn test_deserialize_list_response() {
        let json = r#"{
            "data": [
                {
                    "node": {
                        "id": 52991,
                        "title": "Sousou no Frieren",
                        "num_episodes": 28
                    },
                    "list_status": {
                        "status": "watching",
                        "num_episodes_watched": 14,
                        "score": 9,
                        "updated_at": "2024-01-15T10:00:00+00:00"
                    }
                }
            ],
            "paging": {
                "next": null
            }
        }"#;

        let resp: MalListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.data.len(), 1);
        assert!(resp.paging.next.is_none());

        let entry = resp.data.into_iter().next().unwrap().into_user_list_entry();
        assert_eq!(entry.service_id, 52991);
        assert_eq!(entry.title, "Sousou no Frieren");
        assert_eq!(entry.watched_episodes, 14);
        assert_eq!(entry.total_episodes, Some(28));
        assert_eq!(entry.status, "watching");
        assert_eq!(entry.score, Some(9.0));
    }

    #[test]
    fn test_deserialize_season_response() {
        let json = r#"{
            "data": [
                {
                    "node": {
                        "id": 52991,
                        "title": "Sousou no Frieren",
                        "num_episodes": 28,
                        "start_season": {"year": 2023, "season": "fall"}
                    }
                },
                {
                    "node": {
                        "id": 54898,
                        "title": "Jujutsu Kaisen 2nd Season",
                        "num_episodes": 23
                    }
                }
            ],
            "paging": {
                "next": "https://api.myanimelist.net/v2/anime/season/2023/fall?offset=100"
            }
        }"#;

        let resp: MalSeasonResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.data.len(), 2);
        assert_eq!(
            resp.paging.next.as_deref(),
            Some("https://api.myanimelist.net/v2/anime/season/2023/fall?offset=100")
        );

        let mut items = resp.data.into_iter();
        let first = items.next().unwrap().node.into_search_result();
        assert_eq!(first.service_id, 52991);
        assert_eq!(first.title, "Sousou no Frieren");
        let second = items.next().unwrap().node.into_search_result();
        assert_eq!(second.service_id, 54898);
    }

    #[test]
    fn test_status_mapping() {
        assert_eq!(map_status_to_mal("watching"), "watching");
        assert_eq!(map_status_to_mal("completed"), "completed");
        assert_eq!(map_status_to_mal("on_hold"), "on_hold");
        assert_eq!(map_status_to_mal("dropped"), "dropped");
        assert_eq!(map_status_to_mal("plan_to_watch"), "plan_to_watch");
        assert_eq!(map_status_to_mal("unknown"), "plan_to_watch");
    }

    #[test]
    fn test_deserialize_minimal_node() {
        let json = r#"{ "id": 1, "title": "Test" }"#;
        let node: MalAnimeNode = serde_json::from_str(json).unwrap();
        let result = node.into_search_result();
        assert_eq!(result.service_id, 1);
        assert!(result.cover_url.is_none());
        assert!(result.title_english.is_none());
    }
}
