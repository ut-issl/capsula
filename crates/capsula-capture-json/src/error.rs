use capsula_core::error::CapsulaError;
use std::path::PathBuf;
use thiserror::Error;

/// `capture-json` hook specific errors
#[derive(Debug, Error)]
pub enum JsonHookError {
    /// Failed to read the configured JSON file
    #[error("Failed to read JSON file '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to deserialize the file content as JSON, or failed to parse the
    /// hook config from TOML.
    #[error("Failed to deserialize JSON: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<JsonHookError> for CapsulaError {
    fn from(err: JsonHookError) -> Self {
        Self::HookFailed {
            hook: "capture-json".to_string(),
            source: Box::new(err),
        }
    }
}
