use reqwest::Client;

use super::error::KitsuError;
use super::types::{
    map_status_to_kitsu, JsonApiListResponse, JsonApiSingleResourceResponse,
    JsonApiSingleResponse, KitsuAnimeAttributes, KitsuLibraryAttributes, KitsuListItem,
};
use crate::traits::{
    AnimeSearchResult, AnimeSeason, AnimeService, LibraryEntryUpdate, SeasonPage, UserListEntry,
};

const BASE_URL: &str = "https://kitsu.app/api/edge";

/// Kitsu JSON:API client.
pub struct KitsuClient {
    access_token: String,
    http: Client,
}

impl KitsuClient {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http: Client::new(),
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.access_token)
    }

    async fn check_response(resp: reqwest::Response) -> Result<reqwest::Response, KitsuError> {
        if resp.status().is_success() {
            Ok(resp)
        } else {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            Err(KitsuError::Api {
                status,
                message: body,
            })
        }
    }

    /// Get the authenticated user's Kitsu ID.
    pub async fn get_user_id(&self) -> Result<String, KitsuError> {
        let resp = self
            .http
            .get(format!("{BASE_URL}/users"))
            .header("Authorization", self.auth_header())
            .header("Accept", "application/vnd.api+json")
            .query(&[("filter[self]", "true")])
            .send()
            .await?;

        let resp = Self::check_response(resp).await?;
        let body: JsonApiSingleResponse = resp
            .json()
            .await
            .map_err(|e| KitsuError::Parse(e.to_string()))?;

        body.data
            .first()
            .map(|r| r.id.clone())
            .ok_or_else(|| KitsuError::Auth("could not find authenticated user".into()))
    }

    /// Find the library entry ID for a given anime, or `None` if not in the user's list.
    async fn find_library_entry_id(
        &self,
        user_id: &str,
        anime_id: u64,
    ) -> Result<Option<String>, KitsuError> {
        let resp = self
            .http
            .get(format!("{BASE_URL}/library-entries"))
            .header("Authorization", self.auth_header())
            .header("Accept", "application/vnd.api+json")
            .query(&[
                ("filter[userId]", user_id),
                ("filter[animeId]", &anime_id.to_string()),
                ("page[limit]", "1"),
            ])
            .send()
            .await?;

        let resp = Self::check_response(resp).await?;
        let body: JsonApiListResponse = resp
            .json()
            .await
            .map_err(|e| KitsuError::Parse(e.to_string()))?;

        Ok(body.data.first().map(|r| r.id.clone()))
    }

    /// Fetch the user's full anime library with included anime data.
    pub async fn get_user_list_full(&self) -> Result<Vec<KitsuListItem>, KitsuError> {
        let user_id = self.get_user_id().await?;
        let mut items = Vec::new();
        let mut url = format!(
            "{BASE_URL}/users/{user_id}/library-entries\
             ?filter[kind]=anime\
             &include=anime\
             &fields[libraryEntries]=progress,ratingTwenty,status,startedAt,finishedAt,notes,reconsuming,reconsumeCount,anime\
             &fields[anime]=canonicalTitle,titles,episodeCount,posterImage,averageRating,synopsis,subtype,status,startDate,endDate\
             &page[limit]=50"
        );

        loop {
            let resp = self
                .http
                .get(&url)
                .header("Authorization", self.auth_header())
                .header("Accept", "application/vnd.api+json")
                .send()
                .await?;

            let resp = Self::check_response(resp).await?;
            let page: JsonApiListResponse = resp
                .json()
                .await
                .map_err(|e| KitsuError::Parse(e.to_string()))?;

            // Build a map of included anime by ID.
            let included = page.included.unwrap_or_default();
            let anime_map: std::collections::HashMap<String, &super::types::JsonApiResource> =
                included.iter().map(|r| (r.id.clone(), r)).collect();

            for entry_resource in &page.data {
                let entry_attrs: KitsuLibraryAttributes =
                    serde_json::from_value(entry_resource.attributes.clone())
                        .map_err(|e| KitsuError::Parse(e.to_string()))?;

                // Extract anime ID from relationships.
                let anime_id_str = entry_resource
                    .relationships
                    .as_ref()
                    .and_then(|r| r.get("anime"))
                    .and_then(|a| a.get("data"))
                    .and_then(|d| d.get("id"))
                    .and_then(|id| id.as_str())
                    .unwrap_or("");

                if let Some(anime_resource) = anime_map.get(anime_id_str) {
                    let anime_attrs: KitsuAnimeAttributes =
                        serde_json::from_value(anime_resource.attributes.clone())
                            .map_err(|e| KitsuError::Parse(e.to_string()))?;

                    let anime_id: u64 = anime_id_str.parse().unwrap_or(0);

                    items.push(KitsuListItem {
                        anime_id,
                        anime: anime_attrs,
                        entry: entry_attrs,
                    });
                }
            }

            match page.links.and_then(|l| l.next) {
                Some(next_url) => url = next_url,
                None => break,
            }
        }

        Ok(items)
    }
}

impl AnimeService for KitsuClient {
    type Error = KitsuError;

    async fn authenticate(&self) -> Result<String, KitsuError> {
        // Kitsu auth is handled externally via the auth module.
        // Validate the token by fetching the user.
        let _ = self.get_user_id().await?;
        Ok(self.access_token.clone())
    }

    async fn search_anime(&self, query: &str) -> Result<Vec<AnimeSearchResult>, KitsuError> {
        let resp = self
            .http
            .get(format!("{BASE_URL}/anime"))
            .header("Authorization", self.auth_header())
            .header("Accept", "application/vnd.api+json")
            .query(&[
                ("filter[text]", query),
                ("page[limit]", "10"),
                ("fields[anime]", "canonicalTitle,titles,episodeCount,posterImage,averageRating,synopsis,subtype,status,startDate,endDate"),
            ])
            .send()
            .await?;

        let resp = Self::check_response(resp).await?;
        let body: JsonApiListResponse = resp
            .json()
            .await
            .map_err(|e| KitsuError::Parse(e.to_string()))?;

        Ok(body
            .data
            .into_iter()
            .filter_map(|r| {
                let id: u64 = r.id.parse().ok()?;
                let attrs: KitsuAnimeAttributes = serde_json::from_value(r.attributes).ok()?;
                Some(attrs.into_search_result(id))
            })
            .collect())
    }

    async fn get_user_list(&self) -> Result<Vec<UserListEntry>, KitsuError> {
        let items = self.get_user_list_full().await?;
        Ok(items
            .into_iter()
            .map(|item| item.into_user_list_entry())
            .collect())
    }

    async fn update_library_entry(
        &self,
        anime_id: u64,
        update: LibraryEntryUpdate,
    ) -> Result<(), KitsuError> {
        // First, find the library entry ID for this anime.
        let user_id = self.get_user_id().await?;
        let entry_id = self.find_library_entry_id(&user_id, anime_id).await?;

        let entry_id = entry_id.ok_or_else(|| KitsuError::Api {
            status: 404,
            message: "library entry not found".into(),
        })?;

        // Build attributes conditionally — only send fields that are Some.
        let mut attrs = serde_json::Map::new();
        if let Some(ep) = update.episode {
            attrs.insert("progress".into(), serde_json::json!(ep));
        }
        if let Some(ref status) = update.status {
            attrs.insert("status".into(), serde_json::json!(map_status_to_kitsu(status)));
        }
        if let Some(score) = update.score {
            // Kitsu ratingTwenty: 2-20 scale; score <= 0 sends null to clear.
            if score <= 0.0 {
                attrs.insert("ratingTwenty".into(), serde_json::Value::Null);
            } else {
                let rating = ((score * 2.0).round() as u32).clamp(2, 20);
                attrs.insert("ratingTwenty".into(), serde_json::json!(rating));
            }
        }
        if let Some(ref date) = update.start_date {
            // Kitsu expects ISO-8601 datetime; append time component.
            attrs.insert(
                "startedAt".into(),
                serde_json::json!(format!("{date}T00:00:00.000Z")),
            );
        }
        if let Some(ref date) = update.finish_date {
            attrs.insert(
                "finishedAt".into(),
                serde_json::json!(format!("{date}T00:00:00.000Z")),
            );
        }
        if let Some(ref notes) = update.notes {
            attrs.insert("notes".into(), serde_json::json!(notes));
        }
        if let Some(reconsuming) = update.rewatching {
            attrs.insert("reconsuming".into(), serde_json::json!(reconsuming));
        }
        if let Some(count) = update.rewatch_count {
            attrs.insert("reconsumeCount".into(), serde_json::json!(count));
        }

        let patch_body = serde_json::json!({
            "data": {
                "id": entry_id,
                "type": "libraryEntries",
                "attributes": attrs
            }
        });

        let resp = self
            .http
            .patch(format!("{BASE_URL}/library-entries/{entry_id}"))
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/vnd.api+json")
            .header("Accept", "application/vnd.api+json")
            .json(&patch_body)
            .send()
            .await?;

        Self::check_response(resp).await?;
        Ok(())
    }

    async fn get_anime(&self, anime_id: u64) -> Result<AnimeSearchResult, KitsuError> {
        let resp = self
            .http
            .get(format!("{BASE_URL}/anime/{anime_id}"))
            .header("Authorization", self.auth_header())
            .header("Accept", "application/vnd.api+json")
            .query(&[(
                "fields[anime]",
                "canonicalTitle,titles,episodeCount,posterImage,averageRating,synopsis,subtype,\
                 status,startDate,endDate",
            )])
            .send()
            .await?;

        let resp = Self::check_response(resp).await?;
        let body: JsonApiSingleResourceResponse = resp
            .json()
            .await
            .map_err(|e| KitsuError::Parse(e.to_string()))?;

        let id: u64 = body.data.id.parse().map_err(|_| KitsuError::Parse(
            "invalid anime id in response".into(),
        ))?;
        let attrs: KitsuAnimeAttributes = serde_json::from_value(body.data.attributes)
            .map_err(|e| KitsuError::Parse(e.to_string()))?;

        Ok(attrs.into_search_result(id))
    }

    async fn add_library_entry(&self, anime_id: u64, status: &str) -> Result<(), KitsuError> {
        let user_id = self.get_user_id().await?;
        let kitsu_status = map_status_to_kitsu(status);

        let body = serde_json::json!({
            "data": {
                "type": "libraryEntries",
                "attributes": {
                    "status": kitsu_status
                },
                "relationships": {
                    "user": {
                        "data": {
                            "id": user_id,
                            "type": "users"
                        }
                    },
                    "anime": {
                        "data": {
                            "id": anime_id.to_string(),
                            "type": "anime"
                        }
                    }
                }
            }
        });

        let resp = self
            .http
            .post(format!("{BASE_URL}/library-entries"))
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/vnd.api+json")
            .header("Accept", "application/vnd.api+json")
            .json(&body)
            .send()
            .await?;

        // Treat 422 as success — entry already exists (Kitsu doesn't upsert).
        if resp.status().as_u16() == 422 {
            return Ok(());
        }
        Self::check_response(resp).await?;
        Ok(())
    }

    async fn delete_library_entry(&self, anime_id: u64) -> Result<(), KitsuError> {
        let user_id = self.get_user_id().await?;
        let entry_id = self.find_library_entry_id(&user_id, anime_id).await?;

        let entry_id = match entry_id {
            Some(id) => id,
            None => return Ok(()), // Not in list — already deleted.
        };

        let resp = self
            .http
            .delete(format!("{BASE_URL}/library-entries/{entry_id}"))
            .header("Authorization", self.auth_header())
            .header("Accept", "application/vnd.api+json")
            .send()
            .await?;

        // Treat 404 as success — entry already gone.
        if resp.status().as_u16() == 404 {
            return Ok(());
        }
        Self::check_response(resp).await?;
        Ok(())
    }

    async fn browse_season(
        &self,
        season: AnimeSeason,
        year: u32,
        page: u32,
    ) -> Result<SeasonPage, KitsuError> {
        let offset = (page.saturating_sub(1)) * 20;

        let resp = self
            .http
            .get(format!("{BASE_URL}/anime"))
            .header("Authorization", self.auth_header())
            .header("Accept", "application/vnd.api+json")
            .query(&[
                ("filter[season]", season.to_kitsu_str()),
                ("filter[seasonYear]", &year.to_string()),
                ("sort", "-user_count"),
                ("page[limit]", "20"),
                ("page[offset]", &offset.to_string()),
                (
                    "fields[anime]",
                    "canonicalTitle,titles,episodeCount,posterImage,averageRating,synopsis,subtype,\
                     status,startDate,endDate",
                ),
            ])
            .send()
            .await?;

        let resp = Self::check_response(resp).await?;
        let body: JsonApiListResponse = resp
            .json()
            .await
            .map_err(|e| KitsuError::Parse(e.to_string()))?;

        let has_next = body.links.as_ref().and_then(|l| l.next.as_ref()).is_some();
        let items = body
            .data
            .into_iter()
            .filter_map(|r| {
                let id: u64 = r.id.parse().ok()?;
                let attrs: KitsuAnimeAttributes = serde_json::from_value(r.attributes).ok()?;
                Some(attrs.into_search_result(id))
            })
            .collect();

        Ok(SeasonPage { items, has_next })
    }
}
