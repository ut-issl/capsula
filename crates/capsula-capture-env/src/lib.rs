mod error;

use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde::{Deserialize, Serialize};

pub const KEY: &str = "capture-env";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnvVarHookConfig {
    pub name: String,
}

#[derive(Debug)]
pub struct EnvVarHook {
    pub config: EnvVarHookConfig,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct EnvVarCaptured {
    #[serde(skip)]
    pub name: String,
    pub value: Option<String>,
}

impl<P> Hook<P> for EnvVarHook
where
    P: PhaseMarker,
{
    const KEY: &'static str = KEY;

    type Config = EnvVarHookConfig;
    type Output = EnvVarCaptured;

    fn from_config(
        config: &serde_json::Value,
        _project_root: &std::path::Path,
    ) -> CapsulaResult<Self> {
        let config: EnvVarHookConfig = serde_json::from_value(config.clone())?;
        Ok(EnvVarHook {
            name: config.name.clone(),
            config,
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
        let value = std::env::var(&self.name).ok();
        Ok(EnvVarCaptured {
            name: self.name.clone(),
            value,
        })
    }
}

impl Captured for EnvVarCaptured {
    fn serialize_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}
