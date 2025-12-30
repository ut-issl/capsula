use capsula_core::error::CapsulaError;
use thiserror::Error;

/// Environment variable hook specific errors
#[derive(Debug, Error)]
pub enum EnvHookError {
    /// Environment variable required but not found
    #[error("Required environment variable '{name}' not found")]
    VariableNotFound { name: String },

    /// Environment variable contains invalid UTF-8
    #[error("Environment variable '{name}' contains invalid UTF-8")]
    InvalidUtf8 {
        name: String,
        #[source]
        source: std::env::VarError,
    },

    /// Serialization failed
    #[error("Failed to serialize environment hook: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Convert `EnvHookError` to `CoreError`
impl From<EnvHookError> for CapsulaError {
    fn from(err: EnvHookError) -> Self {
        Self::HookFailed {
            hook: "capture-env".to_string(),
            source: Box::new(err),
        }
    }
}
