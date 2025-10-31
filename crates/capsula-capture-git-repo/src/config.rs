use crate::{GitHook, KEY};
use capsula_core::error::CapsulaResult;
use capsula_core::hook::HookErased;
use capsula_core::hook::HookFactory;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Configuration for GitHook
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitHookConfig {
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub allow_dirty: bool,
}

/// Factory for creating GitHook instances
pub struct GitHookFactory;

impl HookFactory for GitHookFactory {
    fn key(&self) -> &'static str {
        KEY
    }

    fn create_hook(
        &self,
        config: &Value,
        project_root: &Path,
    ) -> CapsulaResult<Box<dyn HookErased>> {
        let config: GitHookConfig = serde_json::from_value(config.clone()).map_err(|e| {
            capsula_core::error::CapsulaError::Configuration {
                message: format!("Invalid git hook configuration: {}", e),
            }
        })?;

        let working_dir = if config.path.is_absolute() {
            config.path.clone()
        } else {
            project_root.join(&config.path).canonicalize()?
        };

        let hook = GitHook {
            config: config.clone(),
            name: config.name,
            working_dir,
            allow_dirty: config.allow_dirty,
        };

        Ok(Box::new(hook))
    }
}
