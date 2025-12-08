use capsula_core::error::CapsulaError;
use thiserror::Error;

/// Current working directory hook specific errors
#[derive(Debug, Error)]
pub enum CwdHookError {
    /// Failed to get current working directory
    #[error("Failed to get current working directory: {source}")]
    CurrentDirError {
        #[source]
        source: std::io::Error,
    },

    /// Serialization failed
    #[error("Failed to serialize cwd hook: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Convert `CwdHookError` to `CoreError`
impl From<CwdHookError> for CapsulaError {
    fn from(err: CwdHookError) -> Self {
        Self::HookFailed {
            hook: "capture-cwd".to_string(),
            message: err.to_string(),
            source: Box::new(err),
        }
    }
}
