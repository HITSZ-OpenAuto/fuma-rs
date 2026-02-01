use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FumaError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parsing error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Missing required directory: {0}")]
    MissingDirectory(PathBuf),
}

pub type Result<T> = std::result::Result<T, FumaError>;
