use crate::error::{CapsulaError, CapsulaResult};
use crate::run::PreparedRun;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

pub struct PreRun;
pub struct PostRun;

pub trait PhaseMarker {}
impl PhaseMarker for PreRun {}
impl PhaseMarker for PostRun {}

#[derive(Debug, Clone)]
pub struct RuntimeParams<P: PhaseMarker> {
    phase_marker: PhantomData<P>,
}

impl Default for RuntimeParams<PreRun> {
    fn default() -> Self {
        Self {
            phase_marker: PhantomData,
        }
    }
}

impl Default for RuntimeParams<PostRun> {
    fn default() -> Self {
        Self {
            phase_marker: PhantomData,
        }
    }
}

pub trait Hook<P: PhaseMarker>: Send + Sync {
    /// The unique identifier key for this hook type (e.g., "capture-cwd", "notify-slack")
    const KEY: &'static str;

    type Output: super::captured::Captured + 'static;
    type Config: Serialize + for<'de> Deserialize<'de>;

    /// Create a hook instance from JSON configuration
    fn from_config(
        config: &serde_json::Value,
        project_root: &std::path::Path,
    ) -> CapsulaResult<Self>
    where
        Self: Sized;

    fn id(&self) -> String;
    fn config(&self) -> &Self::Config;
    fn run(&self, metadata: &PreparedRun, params: &RuntimeParams<P>)
    -> CapsulaResult<Self::Output>;
}

/// Engine-facing trait (object-safe, heterogenous)
pub trait HookErased<P: PhaseMarker>: Send + Sync {
    fn id(&self) -> String;
    fn config_as_json(&self) -> Result<serde_json::Value, serde_json::Error>;
    fn run(
        &self,
        metadata: &PreparedRun,
        parmas: &RuntimeParams<P>,
    ) -> Result<Box<dyn super::captured::Captured>, CapsulaError>;
}

impl<T, P> HookErased<P> for T
where
    T: Hook<P> + Send + Sync + 'static,
    P: PhaseMarker,
{
    fn id(&self) -> String {
        <T as Hook<P>>::id(self)
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
}
