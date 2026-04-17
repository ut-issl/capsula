use capsula_core::error::CapsulaError;
use thiserror::Error;

/// Machine hook specific errors
#[derive(Debug, Error)]
pub enum MachineHookError {
    /// Failed to get OS information
    #[error("Failed to get OS information")]
    OsInfoError,

    /// Failed to get hostname
    #[error("Failed to get hostname")]
    HostnameError,

    /// Serialization failed
    #[error("Failed to serialize machine hook: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Convert `MachineHookError` to `CoreError`
impl From<MachineHookError> for CapsulaError {
    fn from(err: MachineHookError) -> Self {
        Self::HookFailed {
            hook: "capture-machine".to_string(),
            source: Box::new(err),
        }
    }
}
