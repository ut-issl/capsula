use capsula_core::error::CapsulaError;
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
    #[error("Invalid glob pattern: {0}")]
    InvalidPattern(#[from] glob::PatternError),

    /// Glob traversal error
    #[error("Glob traversal error: {0}")]
    GlobError(#[from] glob::GlobError),

    #[error("Run directory is not set")]
    RunDirNotSet,

    #[error("Invalid run directory: {path}")]
    InvalidRunDir { path: PathBuf },

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

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Convert `FileHookError` to `CoreError`
impl From<FileHookError> for CapsulaError {
    fn from(err: FileHookError) -> Self {
        Self::HookFailed {
            hook: "capture-file".to_string(),
            source: Box::new(err),
        }
    }
}
