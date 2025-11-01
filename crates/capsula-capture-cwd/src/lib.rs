mod config;
mod error;

use crate::config::{CwdHookConfig, CwdHookFactory};
use crate::error::CwdHookError;
use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, HookFactory, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde_json::json;
use std::path::PathBuf;

pub const KEY: &str = "capture-cwd";

#[derive(Debug, Default)]
pub struct CwdHook;

#[derive(Debug)]
pub struct CwdCaptured {
    pub cwd_abs: PathBuf,
}

impl Hook for CwdHook {
    type Config = CwdHookConfig;
    type Output = CwdCaptured;

    fn id(&self) -> String {
        KEY.to_string()
    }

    fn config(&self) -> &Self::Config {
        &CwdHookConfig {}
    }

    fn run(&self, _metadata: &PreparedRun, _params: &RuntimeParams) -> CapsulaResult<Self::Output> {
        let cwd_abs =
            std::env::current_dir().map_err(|source| CwdHookError::CurrentDirError { source })?;
        Ok(CwdCaptured { cwd_abs })
    }
}

impl Captured for CwdCaptured {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "cwd": self.cwd_abs.to_string_lossy(),
        })
    }
}

/// Create a factory for CwdHook
pub fn create_factory() -> Box<dyn HookFactory> {
    Box::new(CwdHookFactory)
}
