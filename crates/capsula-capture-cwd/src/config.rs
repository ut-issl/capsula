use capsula_core::error::CapsulaResult;
use capsula_core::hook::{HookErased, HookFactory, PhaseMarker};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::{CwdHook, KEY};

/// Factory for creating CwdHook instances
pub struct CwdHookFactory;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CwdHookConfig {}

impl<P> HookFactory<P> for CwdHookFactory
where
    P: PhaseMarker,
{
    fn key(&self) -> &'static str {
        KEY
    }

    fn create_hook(
        &self,
        _config: &serde_json::Value,
        _project_root: &Path,
    ) -> CapsulaResult<Box<dyn HookErased<P>>> {
        Ok(Box::new(CwdHook))
    }
}
