use capsula_core::error::CoreError;
use thiserror::Error;

/// Current working directory context specific errors
#[derive(Debug, Error)]
pub enum CwdContextError {
    /// Failed to get current working directory
    #[error("Failed to get current working directory: {source}")]
    CurrentDirError {
        #[source]
        source: std::io::Error,
    },

    /// Serialization failed
    #[error("Failed to serialize cwd context: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Convert CwdContextError to CoreError
impl From<CwdContextError> for CoreError {
    fn from(err: CwdContextError) -> Self {
        CoreError::ContextFailed {
            context: "cwd".to_string(),
            message: err.to_string(),
            source: Box::new(err),
        }
    }
}