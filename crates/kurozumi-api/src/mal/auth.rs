use std::io::{Read, Write};
use std::net::TcpListener;

use serde::Deserialize;
use url::Url;

use super::error::MalError;

const AUTH_URL: &str = "https://myanimelist.net/v1/oauth2/authorize";
const TOKEN_URL: &str = "https://myanimelist.net/v1/oauth2/token";
const REDIRECT_URI: &str = "http://localhost:19742";

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    #[allow(dead_code)]
    pub token_type: Option<String>,
}

/// Run the full OAuth2 PKCE authorization flow for MAL.
///
/// 1. Generate a PKCE verifier (MAL requires `plain` method).
/// 2. Open the browser to the MAL consent page.
/// 3. Listen on localhost:19742 for the redirect with `?code=...`.
/// 4. Exchange the code for tokens.
pub async fn authorize(client_id: &str) -> Result<TokenResponse, MalError> {
    let verifier = generate_verifier();

    // MAL uses plain PKCE: challenge == verifier.
    let auth_url = format!(
        "{AUTH_URL}?response_type=code\
         &client_id={client_id}\
         &code_challenge={verifier}\
         &code_challenge_method=plain\
         &redirect_uri={REDIRECT_URI}"
    );

    tracing::info!("Opening MAL authorization URL in browser");
    open::that(&auth_url).map_err(|e| MalError::Auth(format!("failed to open browser: {e}")))?;

    let code = listen_for_redirect()?;
    exchange_code(client_id, &code, &verifier).await
}

/// Refresh an expired access token.
pub async fn refresh(client_id: &str, refresh_token: &str) -> Result<TokenResponse, MalError> {
    let http = reqwest::Client::new();
    let resp = http
        .post(TOKEN_URL)
        .form(&[
            ("client_id", client_id),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(MalError::Api {
            status,
            message: body,
        });
    }

    resp.json::<TokenResponse>()
        .await
        .map_err(|e| MalError::Parse(e.to_string()))
}

// ── Internals ───────────────────────────────────────────────────

/// Generate a random 128-character URL-safe PKCE verifier.
fn generate_verifier() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    // Generate enough randomness from multiple hashers.
    let mut out = String::with_capacity(128);
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    while out.len() < 128 {
        let s = RandomState::new();
        let mut h = s.build_hasher();
        h.write_usize(out.len());
        let val = h.finish();
        for byte in val.to_le_bytes() {
            if out.len() < 128 {
                out.push(CHARS[(byte as usize) % CHARS.len()] as char);
            }
        }
    }
    out
}

/// Spawn a one-shot TCP listener on port 19742, wait for the OAuth redirect,
/// extract the `code` query parameter, and return it.
fn listen_for_redirect() -> Result<String, MalError> {
    let listener = TcpListener::bind("127.0.0.1:19742")
        .map_err(|e| MalError::Auth(format!("failed to bind localhost:19742: {e}")))?;

    tracing::info!("Waiting for MAL OAuth redirect on localhost:19742...");

    let (mut stream, _) = listener
        .accept()
        .map_err(|e| MalError::Auth(format!("failed to accept connection: {e}")))?;

    let mut buf = [0u8; 4096];
    let n = stream
        .read(&mut buf)
        .map_err(|e| MalError::Auth(format!("failed to read from stream: {e}")))?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // Extract the path from the HTTP request line: "GET /?code=...&state=... HTTP/1.1"
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| MalError::Auth("malformed HTTP request from redirect".into()))?;

    let full_url = format!("http://localhost{path}");
    let parsed = Url::parse(&full_url)
        .map_err(|e| MalError::Auth(format!("failed to parse redirect URL: {e}")))?;

    let code = parsed
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.to_string())
        .ok_or_else(|| MalError::Auth("no 'code' parameter in redirect".into()))?;

    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
                    <html><body><h2>Authorization successful!</h2>\
                    <p>You can close this tab and return to kurozumi.</p></body></html>";
    let _ = stream.write_all(response.as_bytes());

    Ok(code)
}

/// Exchange the authorization code for tokens.
async fn exchange_code(
    client_id: &str,
    code: &str,
    verifier: &str,
) -> Result<TokenResponse, MalError> {
    let http = reqwest::Client::new();
    let resp = http
        .post(TOKEN_URL)
        .form(&[
            ("client_id", client_id),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("code_verifier", verifier),
            ("redirect_uri", REDIRECT_URI),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(MalError::Api {
            status,
            message: body,
        });
    }

    resp.json::<TokenResponse>()
        .await
        .map_err(|e| MalError::Parse(e.to_string()))
}
