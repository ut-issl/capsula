use capsula_core::error::CapsulaError;
use thiserror::Error;

/// Environment variable context specific errors
#[derive(Debug, Error)]
pub enum EnvContextError {
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
    #[error("Failed to serialize environment context: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Convert EnvContextError to CoreError
impl From<EnvContextError> for CapsulaError {
    fn from(err: EnvContextError) -> Self {
        CapsulaError::ContextFailed {
            context: "env".to_string(),
            message: err.to_string(),
            source: Box::new(err),
        }
    }
}
