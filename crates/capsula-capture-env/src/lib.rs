mod error;

use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnvVarHookConfig {
    name: String,
}

#[derive(Debug)]
pub struct EnvVarHook {
    config: EnvVarHookConfig,
}

#[derive(Debug, Serialize)]
pub struct EnvVarCaptured {
    value: Option<String>,
}

impl<P> Hook<P> for EnvVarHook
where
    P: PhaseMarker,
{
    const ID: &'static str = "capture-env";

    type Config = EnvVarHookConfig;
    type Output = EnvVarCaptured;

    fn from_config(
        config: &serde_json::Value,
        _project_root: &std::path::Path,
    ) -> CapsulaResult<Self> {
        let config: EnvVarHookConfig = serde_json::from_value(config.clone())?;
        Ok(EnvVarHook { config })
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn run(
        &self,
        _metadata: &PreparedRun,
        _params: &RuntimeParams<P>,
    ) -> CapsulaResult<Self::Output> {
        let value = std::env::var(&self.config.name).ok();
        Ok(EnvVarCaptured { value })
    }
}

impl Captured for EnvVarCaptured {
    fn serialize_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}
