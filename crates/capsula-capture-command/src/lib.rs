mod config;
mod error;

use crate::config::{CommandHookConfig, CommandHookFactory};
use crate::error::CommandHookError;
use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, HookFactory, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde::Serialize;

pub const KEY: &str = "capture-command";

#[derive(Debug)]
pub struct CommandHook {
    pub config: CommandHookConfig,
    pub command: Vec<String>,
    pub abort_on_failure: bool,
}

#[derive(Debug, Serialize)]
pub struct CommandCaptured {
    #[serde(skip)]
    pub command: Vec<String>,
    pub stdout: String,
    pub stderr: String,
    pub status: i32,
    pub abort_requested: bool,
}

impl<P> Hook<P> for CommandHook
where
    P: PhaseMarker,
{
    type Config = CommandHookConfig;
    type Output = CommandCaptured;

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
        use std::process::Command;

        if self.command.is_empty() {
            return Err(CommandHookError::EmptyCommand.into());
        }

        let mut cmd = Command::new(&self.command[0]);
        if self.command.len() > 1 {
            cmd.args(&self.command[1..]);
        }

        let output = cmd
            .output()
            .map_err(|source| CommandHookError::ExecutionFailed {
                command: self.command.join(" "),
                source,
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let status = output.status.code().unwrap_or(-1);

        Ok(CommandCaptured {
            command: self.command.clone(),
            stdout,
            stderr,
            status,
            abort_requested: self.abort_on_failure && status != 0,
        })
    }
}

impl Captured for CommandCaptured {
    fn serialize_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    fn abort_requested(&self) -> bool {
        self.abort_requested
    }
}

pub fn create_factory<P: PhaseMarker>() -> Box<dyn HookFactory<P>> {
    Box::new(CommandHookFactory)
}
