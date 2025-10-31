mod config;
mod error;

use crate::config::CwdHookFactory;
use crate::error::CwdHookError;
use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, HookFactory, RuntimeParams};
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
    type Output = CwdCaptured;

    fn run(&self, _params: &RuntimeParams) -> CapsulaResult<Self::Output> {
        let cwd_abs =
            std::env::current_dir().map_err(|source| CwdHookError::CurrentDirError { source })?;
        Ok(CwdCaptured { cwd_abs })
    }
}

impl Captured for CwdCaptured {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "id": KEY.to_string(),
            "cwd": self.cwd_abs.to_string_lossy(),
        })
    }
}

/// Create a factory for CwdHook
pub fn create_factory() -> Box<dyn HookFactory> {
    Box::new(CwdHookFactory)
}
