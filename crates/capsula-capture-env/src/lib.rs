mod error;

use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
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
        Ok(Self { config })
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn run(
        &self,
        _metadata: &PreparedRun,
        _params: &RuntimeParams<P>,
    ) -> CapsulaResult<Self::Output> {
        debug!(
            "EnvVarHook: Capturing environment variable: {}",
            self.config.name
        );
        let value = std::env::var(&self.config.name).ok();
        if let Some(ref val) = value {
            debug!(
                "EnvVarHook: Variable '{}' found ({} bytes)",
                self.config.name,
                val.len()
            );
        } else {
            debug!("EnvVarHook: Variable '{}' not found", self.config.name);
        }
        Ok(EnvVarCaptured { value })
    }
}

impl Captured for EnvVarCaptured {
    fn serialize_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}
