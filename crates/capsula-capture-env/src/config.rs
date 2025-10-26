use crate::{EnvVarHook, KEY};
use capsula_core::error::CoreResult;
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

    fn create_hook(&self, config: &Value, _project_root: &Path) -> CoreResult<Box<dyn HookErased>> {
        let config: EnvVarHookConfig = serde_json::from_value(config.clone())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let hook = EnvVarHook { name: config.name };

        Ok(Box::new(hook))
    }
}
