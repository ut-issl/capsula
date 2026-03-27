mod error;

use crate::error::CommandHookError;
use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::debug;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommandHookConfig {
    command: Vec<String>,
    #[serde(default)]
    abort_on_failure: bool,
    cwd: Option<PathBuf>,
}

#[derive(Debug)]
pub struct CommandHook {
    config: CommandHookConfig,
    working_dir: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct CommandCaptured {
    stdout: String,
    stderr: String,
    status: i32,
    #[serde(skip)]
    abort_requested: bool,
}

impl<P> Hook<P> for CommandHook
where
    P: PhaseMarker,
{
    const ID: &'static str = "capture-command";

    type Config = CommandHookConfig;
    type Output = CommandCaptured;

    fn from_config(
        config: &serde_json::Value,
        project_root: &std::path::Path,
    ) -> CapsulaResult<Self> {
        let config: CommandHookConfig = serde_json::from_value(config.clone())?;

        let working_dir = match &config.cwd {
            Some(cwd) if cwd.is_absolute() => cwd.clone(),
            Some(cwd) => project_root.join(cwd).canonicalize()?,
            None => project_root.to_path_buf(),
        };

        Ok(Self {
            config,
            working_dir,
        })
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

        if self.config.command.is_empty() {
            return Err(CommandHookError::EmptyCommand.into());
        }

        debug!(
            "CommandHook: Executing command: {:?} in {}",
            self.config.command,
            self.working_dir.display()
        );
        let mut cmd = Command::new(&self.config.command[0]);
        cmd.current_dir(&self.working_dir);
        if self.config.command.len() > 1 {
            cmd.args(&self.config.command[1..]);
        }

        let output = cmd
            .output()
            .map_err(|source| CommandHookError::ExecutionFailed {
                command: self.config.command.join(" "),
                source,
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let status = output.status.code().unwrap_or(-1);

        debug!(
            "CommandHook: Command completed with exit code {}, {} bytes stdout, {} bytes stderr",
            status,
            stdout.len(),
            stderr.len()
        );

        let abort_requested = self.config.abort_on_failure && status != 0;
        if abort_requested {
            debug!("CommandHook: Requesting abort due to command failure");
        }

        Ok(CommandCaptured {
            stdout,
            stderr,
            status,
            abort_requested,
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
