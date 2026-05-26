use capsula_core::error::CapsulaError;
use thiserror::Error;

/// Git hook specific errors
#[derive(Debug, Error)]
pub enum GitHookError {
    /// Repository not found
    #[error("Not a git repository (or any parent up to mount point)")]
    NotARepository,

    /// Failed to get HEAD
    #[error("Failed to get repository HEAD: {message}")]
    HeadNotFound { message: String },

    #[error("Failed to discover git repository: {0}")]
    Discover(#[from] gix::discover::Error),

    #[error("Failed to create git tag: {0}")]
    TagReference(#[from] gix::reference::edit::Error),

    #[error("Failed to create git status platform: {0}")]
    Status(#[from] gix::status::Error),

    #[error("Failed to create git status iterator: {0}")]
    StatusIterator(#[from] gix::status::into_iter::Error),

    #[error("Failed to read git status item: {0}")]
    StatusItem(#[from] gix::status::iter::Error),

    #[error("Failed to read git references: {0}")]
    References(#[from] gix::reference::iter::Error),

    #[error("Failed to initialize git reference iterator: {0}")]
    ReferenceIterator(#[from] gix::reference::iter::init::Error),

    #[error("Failed to read git reference: {message}")]
    ReferenceItem { message: String },

    #[error("Failed to calculate git merge base: {0}")]
    MergeBase(#[from] gix::repository::merge_base::Error),

    #[error("Failed to read git blob: {0}")]
    FindBlob(#[from] gix::object::find::existing::with_conversion::Error),

    /// Serialization failed
    #[error("Failed to serialize git hook: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Run directory not specified and current directory could not be determined: {message}")]
    RunDirNotSpecified { message: String },

    #[error("capture-git-repo hook requires an artifact directory but none was provided")]
    ArtifactDirMissing,

    #[error("Git repository has no worktree")]
    WorktreeMissing,

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Convert `GitHookError` to `CoreError`
impl From<GitHookError> for CapsulaError {
    fn from(err: GitHookError) -> Self {
        Self::HookFailed {
            hook: "capture-git-repo".to_string(),
            source: Box::new(err),
        }
    }
}
