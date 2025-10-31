use capsula_core::error::CapsulaResult;
use capsula_core::hook::{HookErased, HookFactory};
// use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

use crate::{CwdHook, KEY};

/// Factory for creating CwdHook instances
pub struct CwdHookFactory;

impl HookFactory for CwdHookFactory {
    fn key(&self) -> &'static str {
        KEY
    }

    fn create_hook(
        &self,
        _config: &Value,
        _project_root: &Path,
    ) -> CapsulaResult<Box<dyn HookErased>> {
        Ok(Box::new(CwdHook))
    }
}
