mod config;
mod error;

use crate::config::CommandContextFactory;
use crate::error::CommandContextError;
use capsula_core::captured::Captured;
use capsula_core::context::{Context, ContextFactory, RuntimeParams};
use capsula_core::error::CoreResult;

pub const KEY: &str = "command";

#[derive(Debug)]
pub struct CommandContext {
    pub command: Vec<String>,
    pub abort_on_failure: bool,
}

#[derive(Debug)]
pub struct CommandCaptured {
    pub command: Vec<String>,
    pub stdout: String,
    pub stderr: String,
    pub status: i32,
    pub abort_requested: bool,
}

impl Context for CommandContext {
    type Output = CommandCaptured;

    fn run(&self, _params: &RuntimeParams) -> CoreResult<Self::Output> {
        use std::process::Command;

        if self.command.is_empty() {
            return Err(CommandContextError::EmptyCommand.into());
        }

        let mut cmd = Command::new(&self.command[0]);
        if self.command.len() > 1 {
            cmd.args(&self.command[1..]);
        }

        let output = cmd
            .output()
            .map_err(|source| CommandContextError::ExecutionFailed {
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
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "type": KEY,
            "command": self.command,
            "stdout": self.stdout,
            "stderr": self.stderr,
            "status": self.status,
            "abort_requested": self.abort_requested,
        })
    }

    fn abort_requested(&self) -> bool {
        self.abort_requested
    }
}

pub fn create_factory() -> Box<dyn ContextFactory> {
    Box::new(CommandContextFactory)
}
