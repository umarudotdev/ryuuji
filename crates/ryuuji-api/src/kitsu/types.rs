use serde::Deserialize;

use crate::traits::{AnimeSearchResult, UserListEntry};

// ── JSON:API response types ──────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct JsonApiListResponse {
    pub data: Vec<JsonApiResource>,
    pub included: Option<Vec<JsonApiResource>>,
    pub links: Option<Links>,
}

#[derive(Debug, Deserialize)]
pub struct JsonApiSingleResponse {
    pub data: Vec<JsonApiResource>,
}

#[derive(Debug, Deserialize)]
pub struct JsonApiResource {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub attributes: serde_json::Value,
    pub relationships: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct Links {
    pub next: Option<String>,
}

// ── Single resource response (for get_anime) ───────────────────

#[derive(Debug, Deserialize)]
pub struct JsonApiSingleResourceResponse {
    pub data: JsonApiResource,
}

// ── Status mapping ──────────────────────────────────────────────

/// Map internal status strings to Kitsu API status values.
pub fn map_status_to_kitsu(status: &str) -> &'static str {
    match status {
        "watching" => "current",
        "completed" => "completed",
        "on_hold" => "on_hold",
        "dropped" => "dropped",
        "plan_to_watch" => "planned",
        _ => "planned",
    }
}

// ── Kitsu-specific types ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KitsuAnimeAttributes {
    pub canonical_title: Option<String>,
    pub titles: Option<KitsuTitles>,
    pub episode_count: Option<u32>,
    pub poster_image: Option<KitsuImage>,
    pub average_rating: Option<String>,
    pub synopsis: Option<String>,
    pub subtype: Option<String>,
    pub status: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct KitsuTitles {
    pub en: Option<String>,
    pub en_jp: Option<String>,
    pub ja_jp: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct KitsuImage {
    pub small: Option<String>,
    pub medium: Option<String>,
    pub large: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KitsuLibraryAttributes {
    pub progress: Option<u32>,
    pub rating_twenty: Option<u32>,
    pub status: Option<String>,
}

// ── Conversions ──────────────────────────────────────────────────

fn map_kitsu_status(s: &str) -> &'static str {
    match s {
        "current" => "watching",
        "completed" => "completed",
        "on_hold" => "on_hold",
        "dropped" => "dropped",
        "planned" => "plan_to_watch",
        _ => "watching",
    }
}

impl KitsuAnimeAttributes {
    pub fn into_search_result(self, id: u64) -> AnimeSearchResult {
        let title = self
            .canonical_title
            .or_else(|| self.titles.as_ref().and_then(|t| t.en_jp.clone()))
            .unwrap_or_default();
        let title_english = self.titles.as_ref().and_then(|t| t.en.clone());

        AnimeSearchResult {
            service_id: id,
            title,
            title_english,
            episodes: self.episode_count,
            cover_url: self.poster_image.and_then(|p| p.medium.or(p.large)),
            media_type: self.subtype.map(|s| s.to_lowercase()),
            status: self.status.map(|s| s.to_lowercase()),
            synopsis: self.synopsis,
            genres: Vec::new(), // Kitsu genres require a separate include
            mean_score: self
                .average_rating
                .as_deref()
                .and_then(|s| s.parse::<f32>().ok())
                .map(|r| r / 10.0),
            season: None, // Kitsu doesn't have a direct season field
            year: self
                .start_date
                .as_deref()
                .and_then(|d| d.split('-').next())
                .and_then(|y| y.parse().ok()),
        }
    }
}

pub struct KitsuListItem {
    pub anime_id: u64,
    pub anime: KitsuAnimeAttributes,
    pub entry: KitsuLibraryAttributes,
}

impl KitsuListItem {
    pub fn into_user_list_entry(self) -> UserListEntry {
        let title = self
            .anime
            .canonical_title
            .or_else(|| self.anime.titles.as_ref().and_then(|t| t.en_jp.clone()))
            .unwrap_or_default();

        UserListEntry {
            service_id: self.anime_id,
            title,
            watched_episodes: self.entry.progress.unwrap_or(0),
            total_episodes: self.anime.episode_count,
            status: self
                .entry
                .status
                .as_deref()
                .map(map_kitsu_status)
                .unwrap_or("watching")
                .to_string(),
            // Kitsu ratingTwenty is 2-20 scale; divide by 2 to get 1-10
            score: self.entry.rating_twenty.map(|r| r as f32 / 2.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_anime_search() {
        let json = r#"{
            "data": [
                {
                    "id": "12",
                    "type": "anime",
                    "attributes": {
                        "canonicalTitle": "One Piece",
                        "titles": { "en": "One Piece", "en_jp": "One Piece", "ja_jp": "ワンピース" },
                        "episodeCount": 1000,
                        "posterImage": { "small": "https://media.kitsu.app/anime/poster_images/12/small.jpg", "medium": "https://media.kitsu.app/anime/poster_images/12/medium.jpg" },
                        "averageRating": "83.45",
                        "synopsis": "Gol D. Roger...",
                        "subtype": "TV",
                        "status": "current",
                        "startDate": "1999-10-20",
                        "endDate": null
                    }
                }
            ],
            "links": { "next": null }
        }"#;

        let resp: JsonApiListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.data.len(), 1);

        let resource = &resp.data[0];
        let attrs: KitsuAnimeAttributes =
            serde_json::from_value(resource.attributes.clone()).unwrap();
        let id: u64 = resource.id.parse().unwrap();

        let result = attrs.into_search_result(id);
        assert_eq!(result.service_id, 12);
        assert_eq!(result.title, "One Piece");
        assert_eq!(result.title_english.as_deref(), Some("One Piece"));
        assert_eq!(result.episodes, Some(1000));
        assert!(result.cover_url.is_some());
        // 83.45 / 10 = 8.345
        assert!((result.mean_score.unwrap() - 8.345).abs() < 0.01);
        assert_eq!(result.year, Some(1999));
    }

    #[test]
    fn test_deserialize_library_entry() {
        let json = r#"{
            "data": [
                {
                    "id": "999",
                    "type": "libraryEntries",
                    "attributes": {
                        "progress": 14,
                        "ratingTwenty": 18,
                        "status": "current"
                    },
                    "relationships": null
                }
            ],
            "included": [
                {
                    "id": "12",
                    "type": "anime",
                    "attributes": {
                        "canonicalTitle": "One Piece",
                        "titles": { "en": "One Piece" },
                        "episodeCount": 1000
                    }
                }
            ],
            "links": { "next": null }
        }"#;

        let resp: JsonApiListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.data.len(), 1);

        let entry_attrs: KitsuLibraryAttributes =
            serde_json::from_value(resp.data[0].attributes.clone()).unwrap();
        assert_eq!(entry_attrs.progress, Some(14));
        assert_eq!(entry_attrs.rating_twenty, Some(18));
        assert_eq!(entry_attrs.status.as_deref(), Some("current"));

        let included = resp.included.unwrap();
        let anime_attrs: KitsuAnimeAttributes =
            serde_json::from_value(included[0].attributes.clone()).unwrap();
        let anime_id: u64 = included[0].id.parse().unwrap();

        let item = KitsuListItem {
            anime_id,
            anime: anime_attrs,
            entry: entry_attrs,
        };
        let user_entry = item.into_user_list_entry();
        assert_eq!(user_entry.service_id, 12);
        assert_eq!(user_entry.title, "One Piece");
        assert_eq!(user_entry.watched_episodes, 14);
        assert_eq!(user_entry.status, "watching");
        // 18 / 2 = 9.0
        assert_eq!(user_entry.score, Some(9.0));
    }

    #[test]
    fn test_status_mapping() {
        assert_eq!(map_kitsu_status("current"), "watching");
        assert_eq!(map_kitsu_status("completed"), "completed");
        assert_eq!(map_kitsu_status("on_hold"), "on_hold");
        assert_eq!(map_kitsu_status("dropped"), "dropped");
        assert_eq!(map_kitsu_status("planned"), "plan_to_watch");
    }

    #[test]
    fn test_deserialize_single_resource_response() {
        let json = r#"{
            "data": {
                "id": "12",
                "type": "anime",
                "attributes": {
                    "canonicalTitle": "One Piece",
                    "titles": { "en": "One Piece", "en_jp": "One Piece" },
                    "episodeCount": 1000,
                    "posterImage": { "medium": "https://example.com/img.jpg" },
                    "averageRating": "83.45",
                    "synopsis": "Pirates!",
                    "subtype": "TV",
                    "status": "current",
                    "startDate": "1999-10-20",
                    "endDate": null
                }
            }
        }"#;

        let resp: JsonApiSingleResourceResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.data.id, "12");
        assert_eq!(resp.data.type_, "anime");

        let attrs: KitsuAnimeAttributes =
            serde_json::from_value(resp.data.attributes).unwrap();
        assert_eq!(attrs.canonical_title.as_deref(), Some("One Piece"));
        assert_eq!(attrs.episode_count, Some(1000));
    }

    #[test]
    fn test_status_to_kitsu_mapping() {
        assert_eq!(map_status_to_kitsu("watching"), "current");
        assert_eq!(map_status_to_kitsu("completed"), "completed");
        assert_eq!(map_status_to_kitsu("on_hold"), "on_hold");
        assert_eq!(map_status_to_kitsu("dropped"), "dropped");
        assert_eq!(map_status_to_kitsu("plan_to_watch"), "planned");
        assert_eq!(map_status_to_kitsu("unknown"), "planned");
    }
}
