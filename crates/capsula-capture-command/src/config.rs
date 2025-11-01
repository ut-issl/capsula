use crate::{CommandHook, KEY};
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{HookErased, HookFactory, PhaseMarker};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommandHookConfig {
    pub command: Vec<String>,
    #[serde(default)]
    pub abort_on_failure: bool,
}

pub struct CommandHookFactory;

impl<P> HookFactory<P> for CommandHookFactory
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
        let config: CommandHookConfig = serde_json::from_value(config.clone())?;

        let hook = CommandHook {
            config: config.clone(),
            command: config.command,
            abort_on_failure: config.abort_on_failure,
        };

        Ok(Box::new(hook))
    }
}
