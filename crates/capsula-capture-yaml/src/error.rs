use capsula_core::error::CapsulaError;
use std::path::PathBuf;
use thiserror::Error;

/// `capture-yaml` hook specific errors
#[derive(Debug, Error)]
pub enum YamlHookError {
    /// Failed to read the configured YAML file
    #[error("Failed to read YAML file '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse the file content as YAML, or to convert it to JSON
    /// for embedding in the run output.
    #[error("Failed to parse YAML: {0}")]
    Yaml(#[from] yaml_serde::Error),

    /// Failed to deserialize the hook config from JSON
    #[error("Failed to (de)serialize JSON: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<YamlHookError> for CapsulaError {
    fn from(err: YamlHookError) -> Self {
        Self::HookFailed {
            hook: "capture-yaml".to_string(),
            source: Box::new(err),
        }
    }
}
