use crate::{CommandContext, KEY};
use capsula_core::error::CoreResult;
use capsula_core::hook::{ContextErased, ContextFactory};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct CommandContextConfig {
    pub command: Vec<String>,
    #[serde(default)]
    pub abort_on_failure: bool,
}

pub struct CommandContextFactory;

impl ContextFactory for CommandContextFactory {
    fn key(&self) -> &'static str {
        KEY
    }

    fn create_context(
        &self,
        config: &Value,
        _project_root: &Path,
    ) -> CoreResult<Box<dyn ContextErased>> {
        let config: CommandContextConfig = serde_json::from_value(config.clone())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let context = CommandContext {
            command: config.command,
            abort_on_failure: config.abort_on_failure,
        };

        Ok(Box::new(context))
    }
}
