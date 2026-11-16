use crate::project_glob::{FindFilesError, ProjectGlobError};
use capsula_core::error::CapsulaError;
use std::path::PathBuf;
use thiserror::Error;

/// File hook specific errors
#[derive(Debug, Error)]
pub enum FileHookError {
    /// File not found
    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    /// No files matched the pattern
    #[error("No files matched the pattern: {pattern}")]
    NoFilesMatched { pattern: String },

    /// The configured glob pattern is not allowed or invalid.
    #[error(transparent)]
    Pattern(#[from] ProjectGlobError),

    /// Matching files cannot be found under the project root.
    #[error(transparent)]
    FindFiles(#[from] FindFilesError),

    #[error("Run directory is not set")]
    RunDirNotSet,

    #[error("Invalid run directory: {path}")]
    InvalidRunDir { path: PathBuf },

    /// An artifact cannot be checked or created in the artifact directory.
    #[error("Failed to create artifact '{}'", path.display())]
    ArtifactPath {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The artifact destination already exists.
    #[error("Refusing to overwrite existing artifact '{}'", path.display())]
    ArtifactExists { path: PathBuf },

    /// A matched source cannot be opened.
    #[error("Failed to open '{}'", path.display())]
    OpenSource {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// A matched source is not a regular file when it is opened, for example
    /// because it was replaced after the search.
    #[error("Matched path '{}' is no longer a regular file", path.display())]
    SourceNotRegularFile { path: PathBuf },

    /// A moved source changed after it was opened, so it is not removed.
    #[error(
        "'{}' changed while it was moved; the archived copy is kept, and the original is left in place",
        path.display()
    )]
    SourceChanged { path: PathBuf },

    /// A moved source was replaced while it was copied, so it is not removed.
    #[error(
        "'{}' was replaced while it was moved; the archived copy is of the original, and the replacement is left in place",
        path.display()
    )]
    SourceReplaced { path: PathBuf },

    /// The source of a moved file cannot be removed after it was copied.
    #[error("Copied '{}' to the artifact directory but failed to remove it", path.display())]
    RemoveMovedSource {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("capture-file hook requires an artifact directory but none was provided")]
    ArtifactDirMissing,

    /// Failed to read file
    #[error("Failed to read file {path}")]
    ReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to compute hash
    #[error("Failed to compute hash for {path}: {message}")]
    HashError { path: PathBuf, message: String },

    /// Serialization failed
    #[error("Failed to serialize file hook")]
    Serialization(#[from] serde_json::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Convert `FileHookError` to `CoreError`
impl From<FileHookError> for CapsulaError {
    fn from(err: FileHookError) -> Self {
        Self::HookFailed {
            hook: "capture-file".to_string(),
            source: Box::new(err),
        }
    }
}
