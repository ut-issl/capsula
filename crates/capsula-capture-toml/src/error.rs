use capsula_core::error::CapsulaError;
use std::path::PathBuf;
use thiserror::Error;

/// `capture-toml` hook specific errors
#[derive(Debug, Error)]
pub enum TomlHookError {
    /// Failed to read the configured TOML file
    #[error("Failed to read TOML file '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse the file content as TOML
    #[error("Failed to parse TOML: {0}")]
    Toml(#[from] toml::de::Error),

    /// Failed to deserialize the hook config from JSON, or to convert the
    /// parsed TOML value to JSON for embedding in the run output.
    #[error("Failed to (de)serialize JSON: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<TomlHookError> for CapsulaError {
    fn from(err: TomlHookError) -> Self {
        Self::HookFailed {
            hook: "capture-toml".to_string(),
            source: Box::new(err),
        }
    }
}
