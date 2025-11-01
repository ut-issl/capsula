use crate::error::{CapsulaError, CapsulaResult};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

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
    type Config: Serialize + for<'de> Deserialize<'de>;
    fn id(&self) -> String;
    fn config(&self) -> &Self::Config;
    fn config_as_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self.config())
    }
    fn run(&self, params: &RuntimeParams) -> CapsulaResult<Self::Output>;
}

/// Engine-facing trait (object-safe, heterogenous)
pub trait HookErased: Send + Sync {
    fn id(&self) -> String;
    fn config_as_json(&self) -> Result<serde_json::Value, serde_json::Error>;
    fn run(
        &self,
        parmas: &RuntimeParams,
    ) -> Result<Box<dyn super::captured::Captured>, CapsulaError>;
}

impl<T> HookErased for T
where
    T: Hook + Send + Sync + 'static,
{
    fn id(&self) -> String {
        <T as Hook>::id(self)
    }
    fn config_as_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        <T as Hook>::config_as_json(self)
    }

    fn run(
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
        config: &serde_json::Value,
        project_root: &std::path::Path,
    ) -> CapsulaResult<Box<dyn HookErased>>;
}
