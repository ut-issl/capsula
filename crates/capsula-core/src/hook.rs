use crate::error::{CapsulaError, CapsulaResult};
use crate::run::PreparedRun;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::path::PathBuf;

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
    pub const fn new() -> Self {
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
    fn run(&self, metadata: &PreparedRun, params: &RuntimeParams<P>)
    -> CapsulaResult<Self::Output>;

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
    ) -> Result<Box<dyn super::captured::Captured>, CapsulaError>;
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
    ) -> Result<Box<dyn super::captured::Captured>, CapsulaError> {
        let out = <T as Hook<P>>::run(self, metadata, params)?;
        Ok(Box::new(out))
    }

    fn needs_artifact_dir(&self) -> bool {
        <T as Hook<P>>::needs_artifact_dir(self)
    }
}
