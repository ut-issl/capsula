use crate::{EnvVarHook, KEY};
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{HookErased, HookFactory};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct EnvVarHookConfig {
    pub name: String,
}

pub struct EnvVarHookFactory;

impl HookFactory for EnvVarHookFactory {
    fn key(&self) -> &'static str {
        KEY
    }

    fn create_hook(
        &self,
        config: &Value,
        _project_root: &Path,
    ) -> CapsulaResult<Box<dyn HookErased>> {
        let config: EnvVarHookConfig = serde_json::from_value(config.clone())?;

        let hook = EnvVarHook { name: config.name };

        Ok(Box::new(hook))
    }
}
