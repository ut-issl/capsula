mod config;
mod error;

use crate::config::{EnvVarHookConfig, EnvVarHookFactory};
use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, HookFactory, RuntimeParams};

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

impl Hook for EnvVarHook {
    type Config = EnvVarHookConfig;
    type Output = EnvVarCaptured;

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn run(&self, _params: &RuntimeParams) -> CapsulaResult<Self::Output> {
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
            "id": KEY,
            "name": self.name,
            "value": self.value,
        })
    }
}

pub fn create_factory() -> Box<dyn HookFactory> {
    Box::new(EnvVarHookFactory)
}
