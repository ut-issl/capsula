use crate::error::{CapsulaError, CapsulaResult};
use crate::run::PreparedRun;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::path::PathBuf;

/// The semantic outcome of a hook that ran to completion.
///
/// Operational errors are still represented by the outer [`CapsulaResult`]
/// returned from [`Hook::run`]. `Failed` is for hooks that captured useful
/// output but determined that their configured success condition was not met.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookOutcome<T> {
    /// The hook ran and its configured success condition passed.
    Succeeded(T),
    /// The hook ran and captured output, but its configured success condition failed.
    Failed { output: T, reason: String },
}

impl<T> HookOutcome<T> {
    #[must_use]
    pub const fn success(output: T) -> Self {
        Self::Succeeded(output)
    }

    #[must_use]
    pub fn failure(output: T, reason: impl Into<String>) -> Self {
        Self::Failed {
            output,
            reason: reason.into(),
        }
    }

    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Succeeded(_))
    }

    #[must_use]
    pub const fn is_failure(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    #[must_use]
    pub const fn failure_reason(&self) -> Option<&str> {
        match self {
            Self::Succeeded(_) => None,
            Self::Failed { reason, .. } => Some(reason.as_str()),
        }
    }

    #[must_use]
    pub const fn output(&self) -> &T {
        match self {
            Self::Succeeded(output) | Self::Failed { output, .. } => output,
        }
    }

    #[must_use]
    pub fn into_output(self) -> T {
        match self {
            Self::Succeeded(output) | Self::Failed { output, .. } => output,
        }
    }

    #[must_use]
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> HookOutcome<U> {
        match self {
            Self::Succeeded(output) => HookOutcome::Succeeded(f(output)),
            Self::Failed { output, reason } => HookOutcome::Failed {
                output: f(output),
                reason,
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PreRun;
#[derive(Debug, Clone, Default)]
pub struct PostRun;

pub trait PhaseMarker {
    /// Short name used in artifact directory names (e.g., "pre", "post").
    fn phase_name() -> &'static str;
}
impl PhaseMarker for PreRun {
    fn phase_name() -> &'static str {
        "pre"
    }
}
impl PhaseMarker for PostRun {
    fn phase_name() -> &'static str {
        "post"
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeParams<P: PhaseMarker> {
    phase_marker: PhantomData<P>,
    /// Per-hook artifact directory, created by the orchestrator when the hook
    /// requests one via [`Hook::needs_artifact_dir`].
    pub artifact_dir: Option<PathBuf>,
}

impl<P: PhaseMarker> RuntimeParams<P> {
    /// Create `RuntimeParams` with no artifact directory.
    #[must_use]
    const fn new() -> Self {
        Self {
            phase_marker: PhantomData,
            artifact_dir: None,
        }
    }

    /// Create `RuntimeParams` with an artifact directory set.
    #[must_use]
    pub const fn with_artifact_dir(artifact_dir: PathBuf) -> Self {
        Self {
            phase_marker: PhantomData,
            artifact_dir: Some(artifact_dir),
        }
    }
}

impl<P: PhaseMarker> Default for RuntimeParams<P> {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Hook<P: PhaseMarker>: Send + Sync {
    /// The unique identifier for this hook type (e.g., "capture-cwd", "notify-slack")
    const ID: &'static str;

    type Output: super::captured::Captured + 'static;
    type Config: Serialize + for<'de> Deserialize<'de>;

    /// Create a hook instance from JSON configuration
    fn from_config(
        config: &serde_json::Value,
        project_root: &std::path::Path,
    ) -> CapsulaResult<Self>
    where
        Self: Sized;

    fn config(&self) -> &Self::Config;
    fn run(
        &self,
        metadata: &PreparedRun,
        params: &RuntimeParams<P>,
    ) -> CapsulaResult<HookOutcome<Self::Output>>;

    /// Whether this hook needs a dedicated artifact directory.
    ///
    /// When `true`, the orchestrator creates a directory under the run directory
    /// (e.g., `pre-0-capture-file/`) and passes it via [`RuntimeParams::artifact_dir`].
    fn needs_artifact_dir(&self) -> bool {
        false
    }
}

/// Engine-facing trait (object-safe, heterogenous)
pub trait HookErased<P: PhaseMarker>: Send + Sync {
    fn id(&self) -> String;
    fn config_as_json(&self) -> Result<serde_json::Value, serde_json::Error>;
    fn run(
        &self,
        metadata: &PreparedRun,
        params: &RuntimeParams<P>,
    ) -> Result<HookOutcome<Box<dyn super::captured::Captured>>, CapsulaError>;
    fn needs_artifact_dir(&self) -> bool;
}

impl<T, P> HookErased<P> for T
where
    T: Hook<P> + Send + Sync + 'static,
    P: PhaseMarker,
{
    fn id(&self) -> String {
        T::ID.to_string()
    }

    fn config_as_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(<T as Hook<P>>::config(self))
    }

    fn run(
        &self,
        metadata: &PreparedRun,
        params: &RuntimeParams<P>,
    ) -> Result<HookOutcome<Box<dyn super::captured::Captured>>, CapsulaError> {
        let outcome = <T as Hook<P>>::run(self, metadata, params)?;
        Ok(outcome.map(|out| Box::new(out) as Box<dyn super::captured::Captured>))
    }

    fn needs_artifact_dir(&self) -> bool {
        <T as Hook<P>>::needs_artifact_dir(self)
    }
}
