use crate::{KEY, MachineContext};
use capsula_core::context::{ContextErased, ContextFactory};
use capsula_core::error::CoreResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct MachineContextConfig {}

pub struct MachineContextFactory;

impl ContextFactory for MachineContextFactory {
    fn key(&self) -> &'static str {
        KEY
    }

    fn create_context(
        &self,
        _config: &Value,
        _project_root: &Path,
    ) -> CoreResult<Box<dyn ContextErased>> {
        // Config could be deserialized if needed:
        // let _config: MachineContextConfig = serde_json::from_value(config.clone())?;
        Ok(Box::new(MachineContext::default()))
    }
}
