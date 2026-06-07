use capsula_core::error::CapsulaError;
use thiserror::Error;

/// Parameter hook specific errors
#[derive(Debug, Error)]
pub enum ParameterHookError {
    /// Failed to deserialize json parameter
    #[error("Failed to deserialize json parameter: {0}")]
    JsonDeserializationFailed(#[from] serde_json::Error),

    /// Failed to deserialize toml parameter
    #[error("Failed to deserialize toml parameter: {0}")]
    TomlDeserializationFailed(#[from] toml::de::Error),

    /// Failed to deserialize yaml parameter
    #[error("Failed to deserialize yaml parameter: {0}")]
    YamlDeserializationFailed(#[from] serde_yaml::Error),

    /// Unsupported file type for parameter capture
    #[error("Unsupported file type for parameter capture: {0}")]
    UnsupportedFileType(String),

    /// Two files contribute conflicting values at the same nested key path
    #[error("Parameter conflict at key path '{0}'")]
    ParameterConflict(String),

    /// strip_prefix is not a literal prefix of a matched file's relative path
    #[error("strip_prefix '{prefix}' is not a prefix of matched path '{path}'")]
    StripPrefixMismatch { prefix: String, path: String },

    /// Failed to capture file for parameter
    #[error(transparent)]
    FileCapture(#[from] capsula_capture_file::error::FileHookError),
}

impl From<ParameterHookError> for CapsulaError {
    fn from(err: ParameterHookError) -> Self {
        Self::HookFailed {
            hook: "capture-parameter".to_string(),
            source: Box::new(err),
        }
    }
}
