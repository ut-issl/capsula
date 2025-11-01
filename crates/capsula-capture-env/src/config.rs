use crate::{EnvVarHook, KEY};
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{HookErased, HookFactory, PhaseMarker};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnvVarHookConfig {
    pub name: String,
}

pub struct EnvVarHookFactory;

impl<P> HookFactory<P> for EnvVarHookFactory
where
    P: PhaseMarker,
{
    fn key(&self) -> &'static str {
        KEY
    }

    fn create_hook(
        &self,
        config: &Value,
        _project_root: &Path,
    ) -> CapsulaResult<Box<dyn HookErased<P>>> {
        let config: EnvVarHookConfig = serde_json::from_value(config.clone())?;

        let hook = EnvVarHook {
            config: config.clone(),
            name: config.name,
        };

        Ok(Box::new(hook))
    }
}
