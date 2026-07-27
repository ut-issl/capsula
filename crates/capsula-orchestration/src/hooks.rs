use anyhow::{Context, Result};
use capsula_config::HookPhaseConfig;
use capsula_core::hook::{PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde_json::{Value, json};
use std::path::Path;
use tracing::{debug, error, warn};

fn attach_metadata(mut json: Value, metadata: Value) -> Value {
    if let Value::Object(ref mut map) = json {
        map.insert("__meta".to_string(), metadata);
        json
    } else {
        json!({
            "value": json,
            "__meta": metadata,
        })
    }
}

fn outcome_metadata(hook_id: &str, config: &Value, failure_reason: Option<String>) -> Value {
    failure_reason.map_or_else(
        || {
            json!({
                "id": hook_id,
                "config": config,
                "success": true,
            })
        },
        |reason| {
            json!({
                "id": hook_id,
                "config": config,
                "success": false,
                "failure_reason": reason,
            })
        },
    )
}

fn error_json(hook_id: &str, config: &Value, error: &str) -> Value {
    json!({
        "__meta": json!({
            "id": hook_id,
            "config": config,
            "success": false,
            "error": error,
        })
    })
}

/// Build hooks from configuration and execute them in order.
///
/// Returns a tuple of (JSON results array, `should_abort` flag).
/// Each hook's output includes a `__meta` field with `id`, `config`, and `success` status.
/// Failed hooks are recorded with `success: false` but do not stop remaining hooks
/// from running. The returned `should_abort` flag is true when any hook fails or errors.
///
/// Hooks that request an artifact directory get a
/// dedicated subdirectory under `run_dir` named `{phase}-{index}-{hook_id}/`.
pub fn build_and_run_hooks<P: PhaseMarker>(
    run_metadata: &PreparedRun,
    hook_phase_config: &HookPhaseConfig,
    hook_registry: &capsula_registry::HookRegistry<P>,
    project_root: &Path,
) -> Result<(Vec<serde_json::Value>, bool)> {
    debug!(
        "Building {} hooks from configuration",
        hook_phase_config.hooks.len()
    );
    let hooks = capsula_config::build_hooks(hook_phase_config, project_root, hook_registry)
        .context("Failed to build hooks from configuration")?;
    debug!("Successfully built {} hook instances", hooks.len());

    let phase_name = P::phase_name();

    let results: Vec<_> = hooks
        .iter()
        .enumerate()
        .map(|(idx, hook)| {
            let hook_identifier = hook_phase_config.hooks.get(idx).map_or_else(
                || format!("hook[{idx}]"),
                |config_hook| config_hook.id.clone(),
            );
            let hook_id = hook.id();

            let hook_config_json = hook
                .config_as_json()
                .unwrap_or_else(|_| json!({ "__error": "Failed to serialize hook config" }));

            // Create per-hook artifact directory if requested
            let runtime_params = if hook.needs_artifact_dir() {
                let dir_name = format!("{phase_name}-{idx}-{hook_id}");
                let artifact_dir = run_metadata.run_dir.join(&dir_name);
                debug!(
                    "Creating artifact directory for hook '{}': {}",
                    hook_identifier,
                    artifact_dir.display()
                );
                if let Err(e) = std::fs::create_dir_all(&artifact_dir) {
                    error!(
                        "Failed to create artifact directory {}: {e}",
                        artifact_dir.display()
                    );
                    let json = error_json(
                        &hook_id,
                        &hook_config_json,
                        &format!("Failed to create artifact directory: {e}"),
                    );
                    return (json, true);
                }
                RuntimeParams::<P>::with_artifact_dir(artifact_dir)
            } else {
                RuntimeParams::<P>::default()
            };

            debug!("Running hook: {}", hook_identifier);
            match hook.run(run_metadata, &runtime_params) {
                Ok(outcome) => {
                    let should_abort = outcome.is_failure();
                    let failure_reason = outcome.failure_reason().map(ToOwned::to_owned);
                    if let Some(reason) = &failure_reason {
                        warn!(
                            "Hook '{hook_identifier}' completed with a failure outcome: {reason}"
                        );
                    } else {
                        debug!("Hook '{}' completed successfully", hook_identifier);
                    }

                    let json = outcome.output().serialize_json().unwrap_or_else(
                        |_| json!({ "__error": "Failed to serialize captured data" }),
                    );
                    let metadata = outcome_metadata(&hook_id, &hook_config_json, failure_reason);
                    (attach_metadata(json, metadata), should_abort)
                }
                Err(e) => {
                    let error = anyhow::anyhow!(e);
                    error!("Failed to run {hook_identifier} (config index {idx}): {error:#}");
                    let json = error_json(&hook_id, &hook_config_json, &format!("{error:#}"));
                    (json, true)
                }
            }
        })
        .collect();

    let json_results = results.iter().map(|(json, _)| json.clone()).collect();
    let should_abort = results.iter().any(|(_, abort)| *abort);

    Ok((json_results, should_abort))
}
