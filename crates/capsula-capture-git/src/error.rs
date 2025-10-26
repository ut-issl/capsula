use capsula_core::error::CapsulaError;
use thiserror::Error;

/// Git context specific errors
#[derive(Debug, Error)]
pub enum GitContextError {
    /// Repository not found
    #[error("Not a git repository (or any parent up to mount point)")]
    NotARepository,

    /// Failed to get HEAD
    #[error("Failed to get repository HEAD: {message}")]
    HeadNotFound { message: String },

    /// Git operation failed
    #[error("Git operation failed: {0}")]
    GitOperation(#[from] git2::Error),

    /// Serialization failed
    #[error("Failed to serialize git context: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Run directory not specified and current directory could not be determined: {message}")]
    RunDirNotSpecified { message: String },

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Convert GitContextError to CoreError
impl From<GitContextError> for CapsulaError {
    fn from(err: GitContextError) -> Self {
        CapsulaError::ContextFailed {
            context: "git".to_string(),
            message: err.to_string(),
            source: Box::new(err),
        }
    }
}
