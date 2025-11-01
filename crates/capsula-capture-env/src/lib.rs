mod config;
mod error;

use crate::config::{EnvVarHookConfig, EnvVarHookFactory};
use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, HookFactory, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
pub const KEY: &str = "capture-env";

#[derive(Debug)]
pub struct EnvVarHook {
    pub config: EnvVarHookConfig,
    pub name: String,
}

#[derive(Debug)]
pub struct EnvVarCaptured {
    pub name: String,
    pub value: Option<String>,
}

impl<P> Hook<P> for EnvVarHook
where
    P: PhaseMarker,
{
    type Config = EnvVarHookConfig;
    type Output = EnvVarCaptured;

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
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "value": self.value,
        })
    }
}

pub fn create_factory<P: PhaseMarker>() -> Box<dyn HookFactory<P>> {
    Box::new(EnvVarHookFactory)
}
