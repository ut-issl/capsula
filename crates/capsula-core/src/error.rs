use std::{io, path::PathBuf};
use thiserror::Error;

/// Library-wide result alias
pub type CoreResult<T> = Result<T, CoreError>;

/// Core error type for the Capsula library
///
/// This enum defines common infrastructure errors. Context-specific errors
/// should be defined in their respective crates and converted to CoreError
/// via the ContextFailed variant.
#[derive(Debug, Error)]
pub enum CoreError {
    /// I/O operation failed
    #[error("I/O error at {path:?}: {source}")]
    Io {
        path: Option<PathBuf>,
        #[source]
        source: io::Error,
    },

    /// Serialization/deserialization failed
    #[error("Serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Configuration-related error
    #[error("Configuration error: {message}")]
    Configuration {
        message: String,
    },

    /// Context execution failed
    /// This variant wraps context-specific errors while preserving the error chain
    #[error("Context '{context}' failed: {message}")]
    ContextFailed {
        context: String,
        message: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Generic error for unexpected conditions
    #[error("{0}")]
    Other(String),
}

impl From<std::io::Error> for CoreError {
    fn from(e: std::io::Error) -> Self {
        CoreError::Io {
            path: None,
            source: e,
        }
    }
}

/// Helper to create I/O errors with path context
impl CoreError {
    pub fn io_with_path(path: impl Into<PathBuf>, source: io::Error) -> Self {
        CoreError::Io {
            path: Some(path.into()),
            source,
        }
    }
}
