use capsula_core::error::CapsulaError;
use thiserror::Error;

/// Command hook specific errors
#[derive(Debug, Error)]
pub enum CommandHookError {
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

    /// No exit status can be accepted by the configured success code set.
    #[error("success_codes cannot be empty")]
    EmptySuccessCodes,

    /// Legacy and preferred status-policy options were both provided.
    #[error("success_codes and abort_on_failure cannot both be set")]
    ConflictingStatusPolicy,

    /// Command output contains invalid UTF-8
    #[error("Command output contains invalid UTF-8: {message}")]
    InvalidUtf8 { message: String },

    /// Command exited with non-zero status
    #[error("Command '{command}' exited with status {status}")]
    NonZeroExit { command: String, status: i32 },

    /// Serialization failed
    #[error("Failed to serialize command hook: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Convert `CommandHookError` to `CoreError`
impl From<CommandHookError> for CapsulaError {
    fn from(err: CommandHookError) -> Self {
        Self::HookFailed {
            hook: "capture-command".to_string(),
            source: Box::new(err),
        }
    }
}
