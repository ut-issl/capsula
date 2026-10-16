use capsula_core::error::CapsulaError;
use thiserror::Error;

/// Command hook specific errors
#[derive(Debug, Error)]
pub enum CommandHookError {
    /// Command list is empty
    #[error("Command cannot be empty")]
    EmptyCommand,

    /// Command execution failed
    #[error("Failed to execute command '{command}'")]
    ExecutionFailed {
        command: String,
        #[source]
        source: std::io::Error,
    },

    /// The configured working directory cannot be resolved
    #[error("Failed to resolve working directory '{}'", path.display())]
    WorkingDirectory {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// No exit status can be accepted by the configured success code set.
    #[error("success_codes cannot be empty")]
    EmptySuccessCodes,

    /// The removed `abort_on_failure` option was set.
    #[error(
        "abort_on_failure has been removed: replace `abort_on_failure = false` with \
         `success_codes = \"any\"`, and delete `abort_on_failure = true`"
    )]
    RemovedAbortOnFailure,

    /// Command output contains invalid UTF-8
    #[error("Command output contains invalid UTF-8: {message}")]
    InvalidUtf8 { message: String },

    /// Serialization failed
    #[error("Failed to serialize command hook")]
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
