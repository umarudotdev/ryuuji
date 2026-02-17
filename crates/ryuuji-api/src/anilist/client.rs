use reqwest::Client;

use super::error::AniListError;
use super::types::{
    map_status_to_anilist, AniListMedia, GraphQLResponse, MediaListCollectionResponse,
    MediaListEntry, MediaListLookupResponse, MediaResponse, PageResponse, SeasonBrowseResponse,
    ViewerResponse,
};
use crate::traits::{
    AnimeSearchResult, AnimeSeason, AnimeService, LibraryEntryUpdate, SeasonPage, UserListEntry,
};

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
                startedAt { year month day }
                completedAt { year month day }
                notes
                repeat
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

const SEASON_BROWSE_QUERY: &str = r#"
query ($season: MediaSeason, $seasonYear: Int, $page: Int) {
    Page(page: $page, perPage: 50) {
        pageInfo { hasNextPage }
        media(season: $season, seasonYear: $seasonYear, type: ANIME, sort: POPULARITY_DESC) {
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

const UPDATE_LIBRARY_ENTRY_MUTATION: &str = r#"
mutation ($mediaId: Int, $progress: Int, $status: MediaListStatus, $score: Float,
          $startedAt: FuzzyDateInput, $completedAt: FuzzyDateInput,
          $notes: String, $repeat: Int) {
    SaveMediaListEntry(mediaId: $mediaId, progress: $progress, status: $status, scoreRaw: $score,
                       startedAt: $startedAt, completedAt: $completedAt,
                       notes: $notes, repeat: $repeat) {
        id
        progress
    }
}
"#;

const GET_ANIME_QUERY: &str = r#"
query ($id: Int) {
    Media(id: $id, type: ANIME) {
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
"#;

const ADD_LIBRARY_ENTRY_MUTATION: &str = r#"
mutation ($mediaId: Int, $status: MediaListStatus) {
    SaveMediaListEntry(mediaId: $mediaId, status: $status) {
        id
    }
}
"#;

const FIND_MEDIA_LIST_ENTRY_QUERY: &str = r#"
query ($mediaId: Int) {
    MediaList(mediaId: $mediaId, type: ANIME) {
        id
    }
}
"#;

const DELETE_LIBRARY_ENTRY_MUTATION: &str = r#"
mutation ($id: Int) {
    DeleteMediaListEntry(id: $id) {
        deleted
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
        operation: &str,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<T, AniListError> {
        tracing::debug!(operation, "AniList GraphQL request");

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

        let status = resp.status();
        if !status.is_success() {
            let status_code = status.as_u16();
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!(operation, status = status_code, "AniList API error");
            return Err(AniListError::Api {
                status: status_code,
                message: body,
            });
        }

        tracing::debug!(operation, status = %status, "AniList response received");
        resp.json::<T>()
            .await
            .map_err(|e| AniListError::Parse(e.to_string()))
    }

    /// Get the authenticated user's ID.
    pub async fn get_viewer_id(&self) -> Result<u64, AniListError> {
        let resp: GraphQLResponse<ViewerResponse> = self
            .graphql_request("Viewer", VIEWER_QUERY, serde_json::json!({}))
            .await?;
        Ok(resp.data.viewer.id)
    }

    /// Fetch the authenticated user's full anime list (raw types).
    pub async fn get_user_list_full(&self) -> Result<Vec<MediaListEntry>, AniListError> {
        let user_id = self.get_viewer_id().await?;
        let resp: GraphQLResponse<MediaListCollectionResponse> = self
            .graphql_request(
                "UserList",
                USER_LIST_QUERY,
                serde_json::json!({ "userId": user_id }),
            )
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
            .graphql_request(
                "Search",
                SEARCH_QUERY,
                serde_json::json!({ "search": query }),
            )
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

    async fn update_library_entry(
        &self,
        anime_id: u64,
        update: LibraryEntryUpdate,
    ) -> Result<(), AniListError> {
        use super::types::FuzzyDate;

        // Build variables conditionally — AniList ignores null variables.
        let mut vars = serde_json::json!({ "mediaId": anime_id });
        if let Some(ep) = update.episode {
            vars["progress"] = serde_json::json!(ep);
        }
        if let Some(ref status) = update.status {
            vars["status"] = serde_json::json!(map_status_to_anilist(status));
        }
        if let Some(score) = update.score {
            // AniList POINT_100 scale: 0-100; score 0 clears.
            vars["score"] = serde_json::json!((score * 10.0).round() as u32);
        }
        if let Some(ref date) = update.start_date {
            if let Some(fd) = FuzzyDate::from_date_string(date) {
                vars["startedAt"] = fd.to_input_json();
            }
        }
        if let Some(ref date) = update.finish_date {
            if let Some(fd) = FuzzyDate::from_date_string(date) {
                vars["completedAt"] = fd.to_input_json();
            }
        }
        if let Some(ref notes) = update.notes {
            vars["notes"] = serde_json::json!(notes);
        }
        // AniList has no separate rewatching bool — REPEATING status is used instead.
        if let Some(true) = update.rewatching {
            vars["status"] = serde_json::json!("REPEATING");
        }
        if let Some(count) = update.rewatch_count {
            vars["repeat"] = serde_json::json!(count);
        }

        let _: serde_json::Value = self
            .graphql_request("UpdateLibraryEntry", UPDATE_LIBRARY_ENTRY_MUTATION, vars)
            .await?;
        Ok(())
    }

    async fn get_anime(&self, anime_id: u64) -> Result<AnimeSearchResult, AniListError> {
        let resp: GraphQLResponse<MediaResponse> = self
            .graphql_request(
                "GetAnime",
                GET_ANIME_QUERY,
                serde_json::json!({ "id": anime_id }),
            )
            .await?;
        Ok(resp.data.media.into_search_result())
    }

    async fn add_library_entry(&self, anime_id: u64, status: &str) -> Result<(), AniListError> {
        let anilist_status = map_status_to_anilist(status);
        let _: serde_json::Value = self
            .graphql_request(
                "AddLibraryEntry",
                ADD_LIBRARY_ENTRY_MUTATION,
                serde_json::json!({
                    "mediaId": anime_id,
                    "status": anilist_status,
                }),
            )
            .await?;
        Ok(())
    }

    async fn delete_library_entry(&self, anime_id: u64) -> Result<(), AniListError> {
        // Step 1: Look up the library entry ID for this media.
        let lookup: Result<GraphQLResponse<MediaListLookupResponse>, _> = self
            .graphql_request(
                "FindMediaListEntry",
                FIND_MEDIA_LIST_ENTRY_QUERY,
                serde_json::json!({ "mediaId": anime_id }),
            )
            .await;

        let entry_id = match lookup {
            Ok(resp) => match resp.data.media_list {
                Some(entry) => entry.id,
                None => return Ok(()), // Not in list — already deleted.
            },
            Err(e) => {
                tracing::warn!(error = %e, "AniList library entry lookup failed, treating as not in list");
                return Ok(());
            }
        };

        // Step 2: Delete by library entry ID.
        let _: serde_json::Value = self
            .graphql_request(
                "DeleteLibraryEntry",
                DELETE_LIBRARY_ENTRY_MUTATION,
                serde_json::json!({ "id": entry_id }),
            )
            .await?;
        Ok(())
    }

    async fn browse_season(
        &self,
        season: AnimeSeason,
        year: u32,
        page: u32,
    ) -> Result<SeasonPage, AniListError> {
        let resp: GraphQLResponse<SeasonBrowseResponse> = self
            .graphql_request(
                "SeasonBrowse",
                SEASON_BROWSE_QUERY,
                serde_json::json!({
                    "season": season.to_anilist_str(),
                    "seasonYear": year,
                    "page": page,
                }),
            )
            .await?;

        let items = resp
            .data
            .page
            .media
            .into_iter()
            .map(|m| m.into_search_result())
            .collect();

        Ok(SeasonPage {
            items,
            has_next: resp.data.page.page_info.has_next_page,
        })
    }
}
