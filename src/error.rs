use std::{io, path::PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("I/O error for {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("invalid base URL: {0}")]
    InvalidBaseUrl(String),
    #[error("serialization error: {0}")]
    Serialization(String),
}

impl AppError {
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
