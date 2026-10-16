use anyhow::{Context, Result, anyhow};
use capsula_config::HookPhaseConfig;
use capsula_core::error::CapsulaError;
use capsula_core::hook::{HookErased, PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde_json::Value;
use std::path::Path;
use tracing::{debug, error, warn};

use crate::record::{HookRecord, HookStatus, PhaseReport};

/// The hooks of one phase, in configuration order.
pub type Hooks<P> = Vec<Box<dyn HookErased<P>>>;

/// Build the hooks of one phase from configuration.
///
/// Building only validates the configuration and does not touch the file
/// system, so the hooks of both phases can be built before a run starts and a
/// configuration error is reported before any hook or the command runs.
pub fn build_hooks<P: PhaseMarker>(
    hook_phase_config: &HookPhaseConfig,
    hook_registry: &capsula_registry::HookRegistry<P>,
    project_root: &Path,
) -> Result<Hooks<P>> {
    debug!(
        "Building {} {}-run hooks from configuration",
        hook_phase_config.hooks.len(),
        P::phase_name()
    );
    hook_phase_config
        .hooks
        .iter()
        .enumerate()
        .map(|(index, envelope)| {
            hook_registry
                .create_hook(&envelope.id, envelope.config(), project_root)
                .map_err(|e| {
                    anyhow!(
                        "{}[{index}] {}: {}",
                        P::phase_name(),
                        envelope.id,
                        describe_error(&e)
                    )
                })
        })
        .collect::<Result<_>>()
        .context("Invalid hook configuration")
}

/// Execute hooks in order.
///
/// Every hook is attempted; a failed or errored hook does not stop the
/// remaining hooks. The returned report records each hook's result.
///
/// Hooks that request an artifact directory get a
/// dedicated subdirectory under `run_dir` named `{phase}-{index}-{hook_id}/`.
pub fn run_hooks<P: PhaseMarker>(run_metadata: &PreparedRun, hooks: &Hooks<P>) -> PhaseReport<P> {
    let records = hooks
        .iter()
        .enumerate()
        .map(|(idx, hook)| {
            debug!("Running hook: {}", hook.id());
            let record = run_hook(idx, hook.as_ref(), run_metadata);
            log_record::<P>(&record);
            record
        })
        .collect();

    PhaseReport::new(records)
}

/// Run one hook and record its result. Never fails: every problem is
/// recorded as [`HookStatus::Errored`].
fn run_hook<P: PhaseMarker>(
    index: usize,
    hook: &dyn HookErased<P>,
    run_metadata: &PreparedRun,
) -> HookRecord {
    let hook_id = hook.id();

    let config = match hook.config_as_json() {
        Ok(config) => config,
        Err(e) => {
            return HookRecord::errored(
                index,
                hook_id,
                Value::Null,
                format!("Failed to serialize hook config: {e}"),
            );
        }
    };

    let runtime_params = if hook.needs_artifact_dir() {
        let dir_name = format!("{}-{index}-{hook_id}", P::phase_name());
        let artifact_dir = run_metadata.run_dir.join(&dir_name);
        debug!(
            "Creating artifact directory for hook '{}': {}",
            hook_id,
            artifact_dir.display()
        );
        if let Err(e) = std::fs::create_dir_all(&artifact_dir) {
            return HookRecord::errored(
                index,
                hook_id,
                config,
                format!("Failed to create artifact directory: {e}"),
            );
        }
        RuntimeParams::<P>::with_artifact_dir(artifact_dir)
    } else {
        RuntimeParams::<P>::default()
    };

    let result = hook
        .run(run_metadata, &runtime_params)
        .map_err(|e| describe_error(&e));
    HookRecord::from_run(index, hook_id, config, result)
}

/// Describe a hook error as its chain of messages joined by `": "`.
///
/// The [`CapsulaError::HookFailed`] wrapper only names the hook, which every
/// caller already reports next to the message, so the description starts at
/// the hook's own error.
fn describe_error(error: &CapsulaError) -> String {
    let error: &(dyn std::error::Error + 'static) = match error {
        CapsulaError::HookFailed { source, .. } => source.as_ref(),
        other => other,
    };
    anyhow::Chain::new(error)
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(": ")
}

fn log_record<P: PhaseMarker>(record: &HookRecord) {
    let phase = P::phase_name();
    let HookRecord { index, hook_id, .. } = record;
    match &record.status {
        HookStatus::Succeeded => debug!("Hook {phase}[{index}] {hook_id} completed successfully"),
        HookStatus::Failed { reason } => {
            warn!("Hook {phase}[{index}] {hook_id} completed with a failure outcome: {reason}");
        }
        HookStatus::Errored { error } => error!("Hook {phase}[{index}] {hook_id} errored: {error}"),
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        reason = "Tests use unwrap/expect for brevity"
    )]

    use super::*;
    use capsula_core::captured::Captured;
    use capsula_core::hook::{HookOutcome, PreRun};
    use serde::ser::Error as _;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// A hook whose configuration cannot be serialized.
    struct UnserializableConfigHook {
        ran: AtomicBool,
    }

    impl HookErased<PreRun> for UnserializableConfigHook {
        fn id(&self) -> String {
            "broken-config".to_string()
        }

        fn config_as_json(&self) -> Result<Value, serde_json::Error> {
            Err(serde_json::Error::custom("boom"))
        }

        fn run(
            &self,
            _metadata: &PreparedRun,
            _params: &RuntimeParams<PreRun>,
        ) -> Result<HookOutcome<Box<dyn Captured>>, CapsulaError> {
            self.ran.store(true, Ordering::SeqCst);
            Err(CapsulaError::Other("should not run".to_string()))
        }

        fn needs_artifact_dir(&self) -> bool {
            false
        }
    }

    fn prepared_run(dir: &Path) -> PreparedRun {
        PreparedRun {
            id: ulid::Ulid::generate(),
            name: "test-run".to_string(),
            command: vec![],
            run_dir: dir.to_path_buf(),
            project_root: dir.to_path_buf(),
        }
    }

    #[test]
    fn describe_error_skips_hook_wrapper_and_joins_sources() {
        let error = CapsulaError::HookFailed {
            hook: "capture-cwd".to_string(),
            source: Box::new(CapsulaError::Serialization(serde_json::Error::custom(
                "boom",
            ))),
        };
        assert_eq!(describe_error(&error), "Serialization failed: boom");
    }

    #[test]
    fn describe_error_keeps_errors_without_hook_wrapper() {
        let error = CapsulaError::Other("unexpected".to_string());
        assert_eq!(describe_error(&error), "unexpected");
    }

    #[test]
    fn unserializable_config_errors_without_running_hook() {
        let dir = tempfile::tempdir().unwrap();
        let hook = UnserializableConfigHook {
            ran: AtomicBool::new(false),
        };

        let record = run_hook(0, &hook, &prepared_run(dir.path()));

        assert!(!hook.ran.load(Ordering::SeqCst), "hook should not run");
        assert_eq!(record.config, Value::Null);
        assert!(
            matches!(&record.status, HookStatus::Errored { error } if error.contains("Failed to serialize hook config")),
            "unexpected status: {:?}",
            record.status
        );
    }
}
