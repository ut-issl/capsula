use crate::{KEY, SlackNotifyHook};
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{HookErased, HookFactory, PostRun, PreRun};

use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SlackNotifyHookConfig {
    pub channel: String,
    pub token: String,
}

pub struct SlackNotifyHookFactory;

impl HookFactory<PreRun> for SlackNotifyHookFactory {
    fn key(&self) -> &'static str {
        KEY
    }

    fn create_hook(
        &self,
        config: &serde_json::Value,
        _project_root: &std::path::Path,
    ) -> CapsulaResult<Box<dyn HookErased<PreRun>>> {
        let config = serde_json::from_value::<SlackNotifyHookConfig>(config.clone())?;

        let hook = SlackNotifyHook { config };

        Ok(Box::new(hook))
    }
}

impl HookFactory<PostRun> for SlackNotifyHookFactory {
    fn key(&self) -> &'static str {
        KEY
    }

    fn create_hook(
        &self,
        config: &serde_json::Value,
        _project_root: &std::path::Path,
    ) -> CapsulaResult<Box<dyn HookErased<PostRun>>> {
        let config = serde_json::from_value::<SlackNotifyHookConfig>(config.clone())?;

        let hook = SlackNotifyHook { config };

        Ok(Box::new(hook))
    }
}
