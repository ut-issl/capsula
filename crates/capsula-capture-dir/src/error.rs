use capsula_core::error::CapsulaError;
use std::path::PathBuf;
use thiserror::Error;

/// Directory hook specific errors.
#[derive(Debug, Error)]
pub enum DirHookError {
    #[error("Directory not found: {path}")]
    DirectoryNotFound { path: PathBuf },

    #[error("Path is not a directory: {path}")]
    NotADirectory { path: PathBuf },

    #[error("Failed to resolve directory {path}: {source}")]
    ResolveDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Invalid directory name: {path}")]
    InvalidDirectoryName { path: PathBuf },

    #[error("capture-dir hook requires an artifact directory but none was provided")]
    ArtifactDirMissing,

    #[error("Cannot move directory {source_dir} into destination {destination} inside itself")]
    DestinationInsideSource {
        source_dir: PathBuf,
        destination: PathBuf,
    },

    #[error("Destination already exists: {path}")]
    DestinationAlreadyExists { path: PathBuf },

    #[error("Failed to compute path of {path} relative to {base}")]
    StripPrefix { path: PathBuf, base: PathBuf },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to serialize directory hook: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl From<DirHookError> for CapsulaError {
    fn from(err: DirHookError) -> Self {
        Self::HookFailed {
            hook: "capture-dir".to_string(),
            source: Box::new(err),
        }
    }
}
