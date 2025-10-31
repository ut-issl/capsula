use crate::{CaptureMode, FileHook, HashAlgorithm, KEY};
use capsula_core::error::CapsulaResult;
use capsula_core::hook::HookErased;
use capsula_core::hook::HookFactory;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileHookConfig {
    pub glob: String,
    #[serde(default)]
    pub mode: CaptureMode,
    #[serde(default)]
    pub hash: HashAlgorithm,
}

pub struct FileHookFactory;

impl HookFactory for FileHookFactory {
    fn key(&self) -> &'static str {
        KEY
    }

    fn create_hook(
        &self,
        config: &Value,
        _project_root: &Path,
    ) -> CapsulaResult<Box<dyn HookErased>> {
        let config: FileHookConfig = serde_json::from_value(config.clone())?;

        let hook = FileHook {
            config: config.clone(),
            glob: config.glob,
            mode: config.mode,
            hash: config.hash,
        };

        Ok(Box::new(hook))
    }
}
