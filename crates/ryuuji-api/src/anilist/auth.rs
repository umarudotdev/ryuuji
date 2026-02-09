use std::io::{Read, Write};
use std::net::TcpListener;

use serde::Deserialize;
use url::Url;

use super::error::AniListError;

const AUTH_URL: &str = "https://anilist.co/api/v2/oauth/authorize";
const TOKEN_URL: &str = "https://anilist.co/api/v2/oauth/token";
const REDIRECT_URI: &str = "http://localhost:19742";

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: Option<String>,
}

/// Run the OAuth2 Authorization Code Grant flow for AniList.
///
/// 1. Open the browser to the AniList consent page.
/// 2. Listen on localhost:19742 for the redirect with `?code=...`.
/// 3. Exchange the code for an access token.
pub async fn authorize(
    client_id: &str,
    client_secret: &str,
) -> Result<TokenResponse, AniListError> {
    let auth_url = format!(
        "{AUTH_URL}?client_id={client_id}\
         &redirect_uri={REDIRECT_URI}\
         &response_type=code"
    );

    tracing::info!("Opening AniList authorization URL in browser");
    open::that(&auth_url)
        .map_err(|e| AniListError::Auth(format!("failed to open browser: {e}")))?;

    let code = listen_for_redirect()?;
    exchange_code(client_id, client_secret, &code).await
}

/// Spawn a one-shot TCP listener on port 19742, wait for the OAuth redirect,
/// extract the `code` query parameter, and return it.
fn listen_for_redirect() -> Result<String, AniListError> {
    let listener = TcpListener::bind("127.0.0.1:19742")
        .map_err(|e| AniListError::Auth(format!("failed to bind localhost:19742: {e}")))?;

    tracing::info!("Waiting for AniList OAuth redirect on localhost:19742...");

    let (mut stream, _) = listener
        .accept()
        .map_err(|e| AniListError::Auth(format!("failed to accept connection: {e}")))?;

    let mut buf = [0u8; 4096];
    let n = stream
        .read(&mut buf)
        .map_err(|e| AniListError::Auth(format!("failed to read from stream: {e}")))?;
    let request = String::from_utf8_lossy(&buf[..n]);

    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| AniListError::Auth("malformed HTTP request from redirect".into()))?;

    let full_url = format!("http://localhost{path}");
    let parsed = Url::parse(&full_url)
        .map_err(|e| AniListError::Auth(format!("failed to parse redirect URL: {e}")))?;

    let code = parsed
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.to_string())
        .ok_or_else(|| AniListError::Auth("no 'code' parameter in redirect".into()))?;

    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
                    <html><body><h2>Authorization successful!</h2>\
                    <p>You can close this tab and return to ryuuji.</p></body></html>";
    let _ = stream.write_all(response.as_bytes());

    Ok(code)
}

/// Exchange the authorization code for an access token.
async fn exchange_code(
    client_id: &str,
    client_secret: &str,
    code: &str,
) -> Result<TokenResponse, AniListError> {
    let http = reqwest::Client::new();
    let resp = http
        .post(TOKEN_URL)
        .json(&serde_json::json!({
            "grant_type": "authorization_code",
            "client_id": client_id,
            "client_secret": client_secret,
            "redirect_uri": REDIRECT_URI,
            "code": code,
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

    resp.json::<TokenResponse>()
        .await
        .map_err(|e| AniListError::Parse(e.to_string()))
}
