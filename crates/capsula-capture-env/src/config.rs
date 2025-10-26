use crate::{EnvVarContext, KEY};
use capsula_core::error::CoreResult;
use capsula_core::hook::{ContextErased, ContextFactory};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct EnvVarContextConfig {
    pub name: String,
}

pub struct EnvVarContextFactory;

impl ContextFactory for EnvVarContextFactory {
    fn key(&self) -> &'static str {
        KEY
    }

    fn create_context(
        &self,
        config: &Value,
        _project_root: &Path,
    ) -> CoreResult<Box<dyn ContextErased>> {
        let config: EnvVarContextConfig = serde_json::from_value(config.clone())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let context = EnvVarContext { name: config.name };

        Ok(Box::new(context))
    }
}
