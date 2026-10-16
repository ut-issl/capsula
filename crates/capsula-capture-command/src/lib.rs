mod error;

use crate::error::CommandHookError;
use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, HookOutcome, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;
use tracing::debug;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommandHookConfig {
    command: Vec<String>,
    /// Always serialized, so that `__meta.config` records the effective policy.
    #[serde(default)]
    success_codes: SuccessCodes,
    cwd: Option<PathBuf>,
}

/// Exit statuses of the command that count as a successful hook outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuccessCodes {
    /// Any exit status: `success_codes = "any"`.
    Any,
    /// Only the listed exit statuses: `success_codes = [0, 1]`.
    Only(Vec<i32>),
}

impl Default for SuccessCodes {
    fn default() -> Self {
        Self::Only(vec![0])
    }
}

impl SuccessCodes {
    /// Check an exit status, returning the failure reason when it is not one
    /// of the success codes.
    fn check(&self, status: i32) -> Result<(), String> {
        match self {
            Self::Any => Ok(()),
            Self::Only(codes) if codes.contains(&status) => Ok(()),
            Self::Only(codes) if codes.as_slice() == [0] => {
                Err(format!("command exited with status {status}; expected 0"))
            }
            Self::Only(codes) => Err(format!(
                "command exited with status {status}; expected one of {codes:?}"
            )),
        }
    }
}

impl Serialize for SuccessCodes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Any => serializer.serialize_str("any"),
            Self::Only(codes) => codes.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for SuccessCodes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Keyword(String),
            Codes(Vec<i32>),
        }

        const EXPECTED: &str = r#"expected "any" or an array of exit statuses"#;
        match Raw::deserialize(deserializer).map_err(|_| D::Error::custom(EXPECTED))? {
            Raw::Keyword(keyword) if keyword == "any" => Ok(Self::Any),
            Raw::Keyword(keyword) => Err(D::Error::custom(format!(
                "invalid success_codes {keyword:?}, {EXPECTED}"
            ))),
            Raw::Codes(codes) => Ok(Self::Only(codes)),
        }
    }
}

#[derive(Debug)]
pub struct CommandHook {
    config: CommandHookConfig,
    project_root: PathBuf,
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
        if config.get("abort_on_failure").is_some() {
            return Err(CommandHookError::RemovedAbortOnFailure.into());
        }
        let config: CommandHookConfig = serde_json::from_value(config.clone())?;
        if config.command.is_empty() {
            return Err(CommandHookError::EmptyCommand.into());
        }
        if config.success_codes == SuccessCodes::Only(Vec::new()) {
            return Err(CommandHookError::EmptySuccessCodes.into());
        }

        // `cwd` is resolved when the hook runs, because a post-run hook may
        // refer to a directory that the command creates.
        Ok(Self {
            config,
            project_root: project_root.to_path_buf(),
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

        let Some((program, args)) = self.config.command.split_first() else {
            return Err(CommandHookError::EmptyCommand.into());
        };
        let working_dir = self.working_dir()?;

        debug!(
            "CommandHook: Executing command: {:?} in {}",
            self.config.command,
            working_dir.display()
        );
        let mut cmd = Command::new(program);
        cmd.current_dir(&working_dir).args(args);

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

        match self.config.success_codes.check(status) {
            Ok(()) => Ok(HookOutcome::success(captured)),
            Err(reason) => {
                debug!("CommandHook: {reason}");
                Ok(HookOutcome::failure(captured, reason))
            }
        }
    }
}

impl CommandHook {
    /// The directory the command runs in: `cwd` resolved against the project
    /// root, or the project root itself.
    fn working_dir(&self) -> Result<PathBuf, CommandHookError> {
        self.config.cwd.as_ref().map_or_else(
            || Ok(self.project_root.clone()),
            |cwd| {
                capsula_core::util::resolve_relative(cwd, &self.project_root).map_err(|source| {
                    CommandHookError::WorkingDirectory {
                        path: cwd.clone(),
                        source,
                    }
                })
            },
        )
    }
}

impl Captured for CommandCaptured {
    fn serialize_json(
        &self,
    ) -> Result<serde_json::Map<String, serde_json::Value>, serde_json::Error> {
        capsula_core::captured::to_object(self)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "Tests use unwrap for brevity")]

    use super::*;
    use serde_json::json;

    #[test]
    fn success_codes_deserializes_any_and_lists() {
        let parse = |value| serde_json::from_value::<SuccessCodes>(value);
        assert_eq!(parse(json!("any")).unwrap(), SuccessCodes::Any);
        assert_eq!(
            parse(json!([0, 1])).unwrap(),
            SuccessCodes::Only(vec![0, 1])
        );
    }

    #[test]
    fn success_codes_rejects_other_values() {
        for value in [json!("all"), json!(0), json!(["0"])] {
            let err = serde_json::from_value::<SuccessCodes>(value.clone()).unwrap_err();
            assert!(err.to_string().contains(r#""any""#), "{value}: {err}");
        }
    }

    #[test]
    fn success_codes_serializes_as_written() {
        assert_eq!(
            serde_json::to_value(SuccessCodes::Any).unwrap(),
            json!("any")
        );
        assert_eq!(
            serde_json::to_value(SuccessCodes::default()).unwrap(),
            json!([0])
        );
    }

    #[test]
    fn success_codes_checks_status() {
        assert_eq!(SuccessCodes::Any.check(3), Ok(()));
        assert_eq!(SuccessCodes::default().check(0), Ok(()));
        assert_eq!(
            SuccessCodes::default().check(1),
            Err("command exited with status 1; expected 0".to_string())
        );
        assert_eq!(SuccessCodes::Only(vec![0, 1]).check(1), Ok(()));
        assert_eq!(
            SuccessCodes::Only(vec![0, 1]).check(2),
            Err("command exited with status 2; expected one of [0, 1]".to_string())
        );
    }
}
