use thiserror::Error;

#[derive(Debug, Error)]
pub enum KurozumiError {
    #[error("detection failed: {0}")]
    Detection(String),

    #[error("parse failed: {0}")]
    Parse(String),

    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),

    #[error("config error: {0}")]
    Config(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
