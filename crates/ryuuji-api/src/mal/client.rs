use reqwest::Client;

use super::auth;
use super::error::MalError;
use super::types::{
    map_status_to_mal, MalAnimeListItem, MalAnimeNode, MalListResponse, MalSearchResponse,
    MalSeasonResponse,
};
use crate::traits::{
    AnimeSearchResult, AnimeSeason, AnimeService, LibraryEntryUpdate, SeasonPage, UserListEntry,
};

const BASE_URL: &str = "https://api.myanimelist.net";

/// Shared fields parameter for MAL anime queries.
const ANIME_FIELDS: &str = "id,title,alternative_titles,num_episodes,main_picture,media_type,\
                             status,synopsis,genres,mean,studios,source,rating,start_date,\
                             end_date,start_season";

/// MyAnimeList API v2 client.
pub struct MalClient {
    client_id: String,
    access_token: String,
    http: Client,
}

impl MalClient {
    pub fn new(client_id: String, access_token: String) -> Self {
        Self {
            client_id,
            access_token,
            http: Client::new(),
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.access_token)
    }

    /// Check the HTTP response for errors and return the body text on failure.
    async fn check_response(resp: reqwest::Response) -> Result<reqwest::Response, MalError> {
        if resp.status().is_success() {
            Ok(resp)
        } else {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!(status, "MAL API error");
            Err(MalError::Api {
                status,
                message: body,
            })
        }
    }

    /// Fetch the authenticated user's full anime list with all MAL-specific fields.
    ///
    /// Unlike the trait method `get_user_list()`, this preserves the raw MAL types
    /// including alternative titles, synonyms, and cover art.
    pub async fn get_user_list_full(&self) -> Result<Vec<MalAnimeListItem>, MalError> {
        let mut items = Vec::new();
        let mut url = format!(
            "{BASE_URL}/v2/users/@me/animelist\
             ?fields=list_status,{ANIME_FIELDS}\
             &limit=100&nsfw=true"
        );

        loop {
            let resp = self
                .http
                .get(&url)
                .header("Authorization", self.auth_header())
                .send()
                .await?;

            let resp = Self::check_response(resp).await?;
            let page: MalListResponse = resp
                .json()
                .await
                .map_err(|e| MalError::Parse(e.to_string()))?;

            items.extend(page.data);

            match page.paging.next {
                Some(next_url) => url = next_url,
                None => break,
            }
        }

        Ok(items)
    }
}

impl AnimeService for MalClient {
    type Error = MalError;

    async fn authenticate(&self) -> Result<String, MalError> {
        let token_resp = auth::authorize(&self.client_id).await?;
        Ok(token_resp.access_token)
    }

    async fn search_anime(&self, query: &str) -> Result<Vec<AnimeSearchResult>, MalError> {
        let resp = self
            .http
            .get(format!("{BASE_URL}/v2/anime"))
            .header("Authorization", self.auth_header())
            .query(&[("q", query), ("limit", "10"), ("fields", ANIME_FIELDS)])
            .send()
            .await?;

        let resp = Self::check_response(resp).await?;
        let search: MalSearchResponse = resp
            .json()
            .await
            .map_err(|e| MalError::Parse(e.to_string()))?;

        Ok(search
            .data
            .into_iter()
            .map(|n| n.node.into_search_result())
            .collect())
    }

    async fn get_user_list(&self) -> Result<Vec<UserListEntry>, MalError> {
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
    ) -> Result<(), MalError> {
        let url = format!("{BASE_URL}/v2/anime/{anime_id}/my_list_status");

        // MAL requires form-encoded body for PATCH, not JSON.
        // Build params conditionally — only send fields that are Some.
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(ep) = update.episode {
            params.push(("num_watched_episodes", ep.to_string()));
        }
        if let Some(ref status) = update.status {
            params.push(("status", map_status_to_mal(status).to_string()));
        }
        if let Some(score) = update.score {
            // MAL uses integer scores 0-10; 0 clears the score.
            params.push(("score", (score.round() as u32).min(10).to_string()));
        }
        if let Some(ref date) = update.start_date {
            params.push(("start_date", date.clone()));
        }
        if let Some(ref date) = update.finish_date {
            params.push(("finish_date", date.clone()));
        }
        if let Some(ref notes) = update.notes {
            params.push(("comments", notes.clone()));
        }
        if let Some(rewatching) = update.rewatching {
            params.push((
                "is_rewatching",
                if rewatching { "true" } else { "false" }.into(),
            ));
        }
        if let Some(count) = update.rewatch_count {
            params.push(("num_times_rewatched", count.to_string()));
        }

        let resp = self
            .http
            .patch(&url)
            .header("Authorization", self.auth_header())
            .form(&params)
            .send()
            .await?;

        Self::check_response(resp).await?;
        Ok(())
    }

    async fn get_anime(&self, anime_id: u64) -> Result<AnimeSearchResult, MalError> {
        let resp = self
            .http
            .get(format!("{BASE_URL}/v2/anime/{anime_id}"))
            .header("Authorization", self.auth_header())
            .query(&[("fields", ANIME_FIELDS)])
            .send()
            .await?;

        let resp = Self::check_response(resp).await?;
        let node: MalAnimeNode = resp
            .json()
            .await
            .map_err(|e| MalError::Parse(e.to_string()))?;

        Ok(node.into_search_result())
    }

    async fn add_library_entry(&self, anime_id: u64, status: &str) -> Result<(), MalError> {
        // MAL's PATCH my_list_status is an upsert — creates if absent.
        let url = format!("{BASE_URL}/v2/anime/{anime_id}/my_list_status");
        let mal_status = map_status_to_mal(status);

        let resp = self
            .http
            .patch(&url)
            .header("Authorization", self.auth_header())
            .form(&[("status", mal_status)])
            .send()
            .await?;

        Self::check_response(resp).await?;
        Ok(())
    }

    async fn delete_library_entry(&self, anime_id: u64) -> Result<(), MalError> {
        let url = format!("{BASE_URL}/v2/anime/{anime_id}/my_list_status");

        let resp = self
            .http
            .delete(&url)
            .header("Authorization", self.auth_header())
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
    ) -> Result<SeasonPage, MalError> {
        let offset = (page.saturating_sub(1)) * 100;
        let url = format!("{BASE_URL}/v2/anime/season/{year}/{}", season.to_mal_str());

        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .query(&[
                ("fields", ANIME_FIELDS),
                ("limit", "100"),
                ("offset", &offset.to_string()),
                ("sort", "anime_num_list_users"),
                ("nsfw", "true"),
            ])
            .send()
            .await?;

        let resp = Self::check_response(resp).await?;
        let season_resp: MalSeasonResponse = resp
            .json()
            .await
            .map_err(|e| MalError::Parse(e.to_string()))?;

        let items = season_resp
            .data
            .into_iter()
            .map(|n| n.node.into_search_result())
            .collect();

        Ok(SeasonPage {
            items,
            has_next: season_resp.paging.next.is_some(),
        })
    }
}
