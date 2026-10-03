mod error;

use crate::error::CommandHookError;
use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, HookOutcome, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::debug;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommandHookConfig {
    command: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    success_codes: Option<Vec<i32>>,
    /// Deprecated compatibility knob. When omitted, only exit status 0 is
    /// successful. When explicitly set to false, any exit status is accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    abort_on_failure: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
        if config.success_codes.as_ref().is_some_and(Vec::is_empty) {
            return Err(CommandHookError::EmptySuccessCodes.into());
        }
        if config.success_codes.is_some() && config.abort_on_failure.is_some() {
            return Err(CommandHookError::ConflictingStatusPolicy.into());
        }

        let working_dir = match &config.cwd {
            Some(cwd) => capsula_core::util::resolve_relative(cwd, project_root)?,
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
    ) -> CapsulaResult<HookOutcome<Self::Output>> {
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

        let captured = CommandCaptured {
            stdout,
            stderr,
            status,
        };

        if self.is_success_status(status) {
            Ok(HookOutcome::success(captured))
        } else {
            let reason = self.failure_reason(status);
            debug!("CommandHook: {reason}");
            Ok(HookOutcome::failure(captured, reason))
        }
    }
}

impl CommandHook {
    fn is_success_status(&self, status: i32) -> bool {
        self.config.success_codes.as_ref().map_or_else(
            || match self.config.abort_on_failure {
                // Backward compatibility: the previous default accepted non-zero
                // statuses unless this flag was true. New configs should use
                // `success_codes` when a non-zero status is expected.
                Some(false) => true,
                Some(true) | None => status == 0,
            },
            |success_codes| success_codes.contains(&status),
        )
    }

    fn failure_reason(&self, status: i32) -> String {
        self.config.success_codes.as_ref().map_or_else(
            || format!("command exited with status {status}; expected 0"),
            |success_codes| {
                format!("command exited with status {status}; expected one of {success_codes:?}")
            },
        )
    }
}

impl Captured for CommandCaptured {
    fn serialize_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}
