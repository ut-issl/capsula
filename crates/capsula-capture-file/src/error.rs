use capsula_core::error::CapsulaError;
use capsula_core::project_path::ProjectPathError;
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

    /// Pattern is invalid
    #[error("Invalid glob pattern: {0}")]
    InvalidPattern(#[from] glob::PatternError),

    /// The glob is absolute or has a platform path prefix.
    #[error("Glob pattern must be relative to the project root: {pattern}")]
    NonRelativePattern { pattern: String },

    /// The glob contains an explicit parent-directory traversal.
    #[error("Glob pattern cannot contain parent traversal ('..'): {pattern}")]
    ParentTraversalPattern { pattern: String },

    /// Walking the project unexpectedly produced an out-of-project path.
    #[error("Walked path '{path}' is outside project root '{project_root}'")]
    WalkedPathOutsideProject {
        path: PathBuf,
        project_root: PathBuf,
    },

    /// A path matched the glob but is a symbolic link.
    #[error("Refusing to capture symbolic link: {path}")]
    SymlinkNotAllowed { path: PathBuf },

    /// A matched source stopped being a regular file before capture.
    #[error("Capture source is no longer a regular file: {path}")]
    SourceNotRegularFile { path: PathBuf },

    /// A matched source resolved differently when it was revalidated.
    #[error("Capture source '{path}' changed to resolve as '{resolved}'")]
    SourcePathChanged { path: PathBuf, resolved: PathBuf },

    /// Project-root-aware path validation failed.
    #[error(transparent)]
    ProjectPath(#[from] ProjectPathError),

    /// Project traversal failed.
    #[error("Failed to walk project files: {0}")]
    Walk(#[from] walkdir::Error),

    #[error("Run directory is not set")]
    RunDirNotSet,

    #[error("Invalid run directory: {path}")]
    InvalidRunDir { path: PathBuf },

    #[error("Artifact destination has no parent directory: {path}")]
    InvalidArtifactDestination { path: PathBuf },

    #[error("Refusing to overwrite existing artifact destination: {path}")]
    ArtifactDestinationExists { path: PathBuf },

    #[error("Failed to remove moved source {path}: {source}")]
    RemoveMovedSource {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("capture-file hook requires an artifact directory but none was provided")]
    ArtifactDirMissing,

    /// Failed to read file
    #[error("Failed to read file {path}: {source}")]
    ReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to compute hash
    #[error("Failed to compute hash for {path}: {message}")]
    HashError { path: PathBuf, message: String },

    /// Serialization failed
    #[error("Failed to serialize file hook: {0}")]
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
