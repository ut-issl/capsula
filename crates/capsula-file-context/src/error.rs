use capsula_core::error::CoreError;
use std::path::PathBuf;
use thiserror::Error;

/// File context specific errors
#[derive(Debug, Error)]
pub enum FileContextError {
    /// File not found
    #[error("File not found: {path}")]
    FileNotFound {
        path: PathBuf,
    },

    /// No files matched the pattern
    #[error("No files matched the pattern: {pattern}")]
    NoFilesMatched {
        pattern: String,
    },

    /// Pattern is invalid
    #[error("Invalid file pattern: {pattern}")]
    InvalidPattern {
        pattern: String,
        #[source]
        source: globwalk::GlobError,
    },

    /// Failed to read file
    #[error("Failed to read file {path}: {source}")]
    ReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to compute hash
    #[error("Failed to compute hash for {path}: {message}")]
    HashError {
        path: PathBuf,
        message: String,
    },

    /// Serialization failed
    #[error("Failed to serialize file context: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Convert FileContextError to CoreError
impl From<FileContextError> for CoreError {
    fn from(err: FileContextError) -> Self {
        match &err {
            FileContextError::FileNotFound { path } |
            FileContextError::ReadError { path, .. } => {
                CoreError::io_with_path(
                    path.clone(),
                    std::io::Error::new(std::io::ErrorKind::NotFound, err.to_string()),
                )
            }
            _ => CoreError::ContextFailed {
                context: "file".to_string(),
                message: err.to_string(),
                source: Box::new(err),
            }
        }
    }
}
