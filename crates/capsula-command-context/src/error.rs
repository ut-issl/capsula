use capsula_core::error::CoreError;
use thiserror::Error;

/// Command context specific errors
#[derive(Debug, Error)]
pub enum CommandContextError {
    /// Command list is empty
    #[error("Command cannot be empty")]
    EmptyCommand,

    /// Command execution failed
    #[error("Failed to execute command '{command}': {source}")]
    ExecutionFailed {
        command: String,
        #[source]
        source: std::io::Error,
    },

    /// Command output contains invalid UTF-8
    #[error("Command output contains invalid UTF-8: {message}")]
    InvalidUtf8 { message: String },

    /// Command exited with non-zero status
    #[error("Command '{command}' exited with status {status}")]
    NonZeroExit { command: String, status: i32 },

    /// Serialization failed
    #[error("Failed to serialize command context: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Convert CommandContextError to CoreError
impl From<CommandContextError> for CoreError {
    fn from(err: CommandContextError) -> Self {
        CoreError::ContextFailed {
            context: "command".to_string(),
            message: err.to_string(),
            source: Box::new(err),
        }
    }
}
