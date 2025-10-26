use capsula_core::error::CoreError;
use std::path::PathBuf;
use thiserror::Error;

/// File hook specific errors
#[derive(Debug, Error)]
pub enum FileHookError {
    /// File not found
    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    /// No files matched the pattern
    #[error("No files matched the pattern: {pattern}")]
    NoFilesMatched { pattern: String },

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
    HashError { path: PathBuf, message: String },

    /// Serialization failed
    #[error("Failed to serialize file hook: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Convert FileHookError to CoreError
impl From<FileHookError> for CoreError {
    fn from(err: FileHookError) -> Self {
        match &err {
            FileHookError::FileNotFound { path } | FileHookError::ReadError { path, .. } => {
                CoreError::io_with_path(
                    path.clone(),
                    std::io::Error::new(std::io::ErrorKind::NotFound, err.to_string()),
                )
            }
            _ => CoreError::HookFailed {
                hook: "file".to_string(),
                message: err.to_string(),
                source: Box::new(err),
            },
        }
    }
}
