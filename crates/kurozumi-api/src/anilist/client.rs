use reqwest::Client;

use super::error::AniListError;
use super::types::{
    AniListMedia, GraphQLResponse, MediaListCollectionResponse, MediaListEntry, PageResponse,
    ViewerResponse,
};
use crate::traits::{AnimeSearchResult, AnimeService, UserListEntry};

const API_URL: &str = "https://graphql.anilist.co";

const SEARCH_QUERY: &str = r#"
query ($search: String) {
    Page(perPage: 10) {
        media(search: $search, type: ANIME) {
            id
            title { romaji english native }
            episodes
            coverImage { large }
            meanScore
            season
            seasonYear
            genres
            studios { nodes { name } }
            format
            status
            description
            source
            synonyms
            startDate { year month day }
            endDate { year month day }
        }
    }
}
"#;

const USER_LIST_QUERY: &str = r#"
query ($userId: Int) {
    MediaListCollection(userId: $userId, type: ANIME) {
        lists {
            entries {
                mediaId
                progress
                score(format: POINT_100)
                status
                media {
                    id
                    title { romaji english native }
                    episodes
                    coverImage { large }
                    meanScore
                    season
                    seasonYear
                    genres
                    studios { nodes { name } }
                    format
                    status
                    description
                    source
                    synonyms
                    startDate { year month day }
                    endDate { year month day }
                }
            }
        }
    }
}
"#;

const VIEWER_QUERY: &str = r#"
query {
    Viewer {
        id
        name
    }
}
"#;

const UPDATE_PROGRESS_MUTATION: &str = r#"
mutation ($mediaId: Int, $progress: Int) {
    SaveMediaListEntry(mediaId: $mediaId, progress: $progress) {
        id
        progress
    }
}
"#;

/// AniList GraphQL API client.
pub struct AniListClient {
    access_token: String,
    http: Client,
}

impl AniListClient {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            http: Client::new(),
        }
    }

    async fn graphql_request<T: serde::de::DeserializeOwned>(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<T, AniListError> {
        let resp = self
            .http
            .post(API_URL)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&serde_json::json!({
                "query": query,
                "variables": variables,
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(AniListError::Api {
                status,
                message: body,
            });
        }

        resp.json::<T>()
            .await
            .map_err(|e| AniListError::Parse(e.to_string()))
    }

    /// Get the authenticated user's ID.
    pub async fn get_viewer_id(&self) -> Result<u64, AniListError> {
        let resp: GraphQLResponse<ViewerResponse> = self
            .graphql_request(VIEWER_QUERY, serde_json::json!({}))
            .await?;
        Ok(resp.data.viewer.id)
    }

    /// Fetch the authenticated user's full anime list (raw types).
    pub async fn get_user_list_full(&self) -> Result<Vec<MediaListEntry>, AniListError> {
        let user_id = self.get_viewer_id().await?;
        let resp: GraphQLResponse<MediaListCollectionResponse> = self
            .graphql_request(USER_LIST_QUERY, serde_json::json!({ "userId": user_id }))
            .await?;

        let entries: Vec<MediaListEntry> = resp
            .data
            .media_list_collection
            .lists
            .into_iter()
            .flat_map(|group| group.entries)
            .collect();

        Ok(entries)
    }

    /// Search for anime (raw types).
    async fn search_raw(&self, query: &str) -> Result<Vec<AniListMedia>, AniListError> {
        let resp: GraphQLResponse<PageResponse> = self
            .graphql_request(SEARCH_QUERY, serde_json::json!({ "search": query }))
            .await?;

        Ok(resp.data.page.media)
    }
}

impl AnimeService for AniListClient {
    type Error = AniListError;

    async fn authenticate(&self) -> Result<String, AniListError> {
        // AniList auth is handled externally via the auth module.
        // This just validates the token by fetching the viewer.
        let _ = self.get_viewer_id().await?;
        Ok(self.access_token.clone())
    }

    async fn search_anime(&self, query: &str) -> Result<Vec<AnimeSearchResult>, AniListError> {
        let media = self.search_raw(query).await?;
        Ok(media.into_iter().map(|m| m.into_search_result()).collect())
    }

    async fn get_user_list(&self) -> Result<Vec<UserListEntry>, AniListError> {
        let entries = self.get_user_list_full().await?;
        Ok(entries
            .into_iter()
            .map(|e| e.into_user_list_entry())
            .collect())
    }

    async fn update_progress(&self, anime_id: u64, episode: u32) -> Result<(), AniListError> {
        let _: serde_json::Value = self
            .graphql_request(
                UPDATE_PROGRESS_MUTATION,
                serde_json::json!({
                    "mediaId": anime_id,
                    "progress": episode,
                }),
            )
            .await?;
        Ok(())
    }
}
