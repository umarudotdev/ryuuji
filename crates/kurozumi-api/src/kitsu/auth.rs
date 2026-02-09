use serde::Deserialize;

use super::error::KitsuError;

const TOKEN_URL: &str = "https://kitsu.app/api/oauth/token";

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    #[allow(dead_code)]
    pub token_type: Option<String>,
    #[allow(dead_code)]
    pub created_at: Option<u64>,
}

/// Authenticate with Kitsu using Resource Owner Password Grant.
pub async fn authenticate(username: &str, password: &str) -> Result<TokenResponse, KitsuError> {
    let http = reqwest::Client::new();
    let resp = http
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "password"),
            ("username", username),
            ("password", password),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(KitsuError::Api {
            status,
            message: body,
        });
    }

    resp.json::<TokenResponse>()
        .await
        .map_err(|e| KitsuError::Parse(e.to_string()))
}

/// Refresh an expired access token.
pub async fn refresh(refresh_token: &str) -> Result<TokenResponse, KitsuError> {
    let http = reqwest::Client::new();
    let resp = http
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(KitsuError::Api {
            status,
            message: body,
        });
    }

    resp.json::<TokenResponse>()
        .await
        .map_err(|e| KitsuError::Parse(e.to_string()))
}
