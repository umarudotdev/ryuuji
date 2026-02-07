use thiserror::Error;

/// Errors from the MyAnimeList API client.
#[derive(Debug, Error)]
pub enum MalError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("auth error: {0}")]
    Auth(String),

    #[error("API error (status {status}): {message}")]
    Api { status: u16, message: String },

    #[error("parse error: {0}")]
    Parse(String),
}
