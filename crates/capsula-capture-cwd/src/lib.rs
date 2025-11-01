mod config;
mod error;

use crate::config::CwdHookConfig;
use crate::error::CwdHookError;
use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde::Serialize;
use std::path::PathBuf;

pub const KEY: &str = "capture-cwd";

#[derive(Debug, Default)]
pub struct CwdHook {
    config: CwdHookConfig,
}

#[derive(Debug, Serialize)]
pub struct CwdCaptured {
    #[serde(rename = "cwd")]
    pub cwd_abs: PathBuf,
}

impl<P> Hook<P> for CwdHook
where
    P: PhaseMarker,
{
    const KEY: &'static str = KEY;

    type Config = CwdHookConfig;
    type Output = CwdCaptured;

    fn from_config(
        _config: &serde_json::Value,
        _project_root: &std::path::Path,
    ) -> CapsulaResult<Self> {
        Ok(Self {
            config: CwdHookConfig {},
        })
    }

    fn id(&self) -> String {
        KEY.to_string()
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn run(
        &self,
        _metadata: &PreparedRun,
        _params: &RuntimeParams<P>,
    ) -> CapsulaResult<Self::Output> {
        let cwd_abs =
            std::env::current_dir().map_err(|source| CwdHookError::CurrentDirError { source })?;
        Ok(CwdCaptured { cwd_abs })
    }
}

impl Captured for CwdCaptured {
    fn serialize_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}
