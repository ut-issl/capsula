use crate::{KEY, MachineHook};
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{HookErased, HookFactory};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

pub struct MachineHookFactory;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MachineHookConfig {}

impl HookFactory for MachineHookFactory {
    fn key(&self) -> &'static str {
        KEY
    }

    fn create_hook(
        &self,
        _config: &Value,
        _project_root: &Path,
    ) -> CapsulaResult<Box<dyn HookErased>> {
        // Config could be deserialized if needed:
        // let _config: MachineHookConfig = serde_json::from_value(config.clone())?;
        Ok(Box::new(MachineHook))
    }
}
