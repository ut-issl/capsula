use capsula_core::error::CapsulaError;
use thiserror::Error;

/// Machine hook specific errors
#[derive(Debug, Error)]
pub enum MachineHookError {
    /// Failed to collect system information
    #[error("Failed to collect system information: {message}")]
    SystemInfoError { message: String },

    /// Failed to get OS information
    #[error("Failed to get OS information")]
    OsInfoError,

    /// Failed to get CPU information
    #[error("Failed to get CPU information")]
    CpuInfoError,

    /// Failed to get memory information
    #[error("Failed to get memory information")]
    MemoryInfoError,

    /// Failed to get hostname
    #[error("Failed to get hostname")]
    HostnameError,

    /// Serialization failed
    #[error("Failed to serialize machine hook: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Convert MachineHookError to CoreError
impl From<MachineHookError> for CapsulaError {
    fn from(err: MachineHookError) -> Self {
        CapsulaError::HookFailed {
            hook: "machine".to_string(),
            message: err.to_string(),
            source: Box::new(err),
        }
    }
}
