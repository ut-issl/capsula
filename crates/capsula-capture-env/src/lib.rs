mod config;
mod error;

use crate::config::EnvVarContextFactory;
#[allow(unused_imports)]
use crate::error::EnvContextError;
use capsula_core::captured::Captured;
use capsula_core::context::{Context, ContextFactory, RuntimeParams};
use capsula_core::error::CoreResult;

pub const KEY: &str = "capture-env";

#[derive(Debug)]
pub struct EnvVarContext {
    pub name: String,
}

#[derive(Debug)]
pub struct EnvVarCaptured {
    pub name: String,
    pub value: Option<String>,
}

impl Context for EnvVarContext {
    type Output = EnvVarCaptured;

    fn run(&self, _params: &RuntimeParams) -> CoreResult<Self::Output> {
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

pub fn create_factory() -> Box<dyn ContextFactory> {
    Box::new(EnvVarContextFactory)
}
