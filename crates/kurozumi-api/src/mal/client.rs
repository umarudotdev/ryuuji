use reqwest::Client;

use super::auth;
use super::error::MalError;
use super::types::{MalListResponse, MalSearchResponse};
use crate::traits::{AnimeSearchResult, AnimeService, UserListEntry};

const BASE_URL: &str = "https://api.myanimelist.net";

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
            Err(MalError::Api {
                status,
                message: body,
            })
        }
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
            .query(&[
                ("q", query),
                ("limit", "10"),
                (
                    "fields",
                    "id,title,alternative_titles,num_episodes,main_picture,media_type,status",
                ),
            ])
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
        let mut entries = Vec::new();
        let mut url =
            format!("{BASE_URL}/v2/users/@me/animelist?fields=list_status&limit=100&nsfw=true");

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

            entries.extend(
                page.data
                    .into_iter()
                    .map(|item| item.into_user_list_entry()),
            );

            match page.paging.next {
                Some(next_url) => url = next_url,
                None => break,
            }
        }

        Ok(entries)
    }

    async fn update_progress(&self, anime_id: u64, episode: u32) -> Result<(), MalError> {
        let url = format!("{BASE_URL}/v2/anime/{anime_id}/my_list_status");

        // MAL requires form-encoded body for PATCH, not JSON.
        let resp = self
            .http
            .patch(&url)
            .header("Authorization", self.auth_header())
            .form(&[("num_watched_episodes", episode.to_string())])
            .send()
            .await?;

        Self::check_response(resp).await?;
        Ok(())
    }
}
