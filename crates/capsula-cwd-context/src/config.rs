use capsula_core::context::{ContextErased, ContextFactory};
use capsula_core::error::CoreResult;
// use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

use crate::{CwdContext, KEY};

// Configuration for CwdContext
// #[derive(Debug, Clone, Default, Deserialize, Serialize)]
// struct CwdContextConfig {}

/// Factory for creating CwdContext instances
pub struct CwdContextFactory;

impl ContextFactory for CwdContextFactory {
    fn key(&self) -> &'static str {
        KEY
    }

    fn create_context(
        &self,
        _config: &Value,
        _project_root: &Path,
    ) -> CoreResult<Box<dyn ContextErased>> {
        // Config could be deserialized if needed:
        // let _config: CwdContextConfig = serde_json::from_value(config.clone())?;
        Ok(Box::new(CwdContext))
    }
}
