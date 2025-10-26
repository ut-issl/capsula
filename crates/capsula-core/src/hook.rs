use crate::error::{CapsulaError, CoreResult};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookPhase {
    Pre,
    Post,
}

#[derive(Debug, Clone)]
pub struct RuntimeParams {
    pub phase: HookPhase,
    // TODO: Make it non-optional by making struct for each command
    pub run_dir: Option<std::path::PathBuf>,
    pub project_root: std::path::PathBuf,
}

pub trait Hook {
    type Output: super::captured::Captured;
    fn run(&self, params: &RuntimeParams) -> CoreResult<Self::Output>;
}

/// Engine-facing trait (object-safe, heterogenous)
pub trait HookErased: Send + Sync {
    fn run_erased(
        &self,
        parmas: &RuntimeParams,
    ) -> Result<Box<dyn super::captured::Captured>, CapsulaError>;
}

impl<T> HookErased for T
where
    T: Hook + Send + Sync + 'static,
{
    fn run_erased(
        &self,
        params: &RuntimeParams,
    ) -> Result<Box<dyn super::captured::Captured>, CapsulaError> {
        let out = <T as Hook>::run(self, params)?;
        Ok(Box::new(out))
    }
}

/// Factory trait for creating hooks from configuration
pub trait HookFactory: Send + Sync {
    /// The type key this factory handles (e.g., "cwd", "git", "file")
    fn key(&self) -> &'static str;

    /// Create a hook instance from JSON configuration
    fn create_hook(
        &self,
        config: &Value,
        project_root: &std::path::Path,
    ) -> CoreResult<Box<dyn HookErased>>;
}
