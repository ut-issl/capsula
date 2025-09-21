use crate::error::{CapsulaError, CoreResult};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContextPhase {
    Pre,
    Post,
}

#[derive(Debug, Clone)]
pub struct RuntimeParams {
    pub phase: ContextPhase,
    pub run_dir: Option<std::path::PathBuf>,
    pub project_root: std::path::PathBuf,
}

pub trait Context {
    type Output: super::captured::Captured;
    fn run(&self, params: &RuntimeParams) -> CoreResult<Self::Output>;
}

/// Engine-facing trait (object-safe, heterogenous)
pub trait ContextErased: Send + Sync {
    fn run_erased(
        &self,
        parmas: &RuntimeParams,
    ) -> Result<Box<dyn super::captured::Captured>, CapsulaError>;
}

impl<T> ContextErased for T
where
    T: Context + Send + Sync + 'static,
{
    fn run_erased(
        &self,
        params: &RuntimeParams,
    ) -> Result<Box<dyn super::captured::Captured>, CapsulaError> {
        let out = <T as Context>::run(self, params)?;
        Ok(Box::new(out))
    }
}

/// Factory trait for creating contexts from configuration
pub trait ContextFactory: Send + Sync {
    /// The type key this factory handles (e.g., "cwd", "git", "file")
    fn key(&self) -> &'static str;

    /// Create a context instance from JSON configuration
    fn create_context(
        &self,
        config: &Value,
        project_root: &std::path::Path,
    ) -> CoreResult<Box<dyn ContextErased>>;
}
