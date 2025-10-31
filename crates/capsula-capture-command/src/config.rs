use crate::{CommandHook, KEY};
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{HookErased, HookFactory};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct CommandHookConfig {
    pub command: Vec<String>,
    #[serde(default)]
    pub abort_on_failure: bool,
}

pub struct CommandHookFactory;

impl HookFactory for CommandHookFactory {
    fn key(&self) -> &'static str {
        KEY
    }

    fn create_hook(
        &self,
        config: &Value,
        _project_root: &Path,
    ) -> CapsulaResult<Box<dyn HookErased>> {
        let config: CommandHookConfig = serde_json::from_value(config.clone())?;

        let hook = CommandHook {
            command: config.command,
            abort_on_failure: config.abort_on_failure,
        };

        Ok(Box::new(hook))
    }
}
