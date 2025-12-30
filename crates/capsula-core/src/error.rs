use thiserror::Error;

/// Library-wide result alias
pub type CapsulaResult<T> = Result<T, CapsulaError>;

/// Core error type for the Capsula library
///
/// This enum defines common infrastructure errors. Hook-specific errors
/// should be defined in their respective crates and converted to `CoreError`
/// via the `HookFailed` variant.
#[derive(Debug, Error)]
pub enum CapsulaError {
    /// I/O operation failed
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization failed
    #[error("Serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Configuration-related error
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    /// Hook execution failed
    /// This variant wraps hook-specific errors while preserving the error chain
    #[error("Hook '{hook}' failed")]
    HookFailed {
        hook: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Generic error for unexpected conditions
    #[error("{0}")]
    Other(String),
}
