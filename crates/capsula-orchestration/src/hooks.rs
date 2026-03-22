use anyhow::{Context, Result};
use capsula_config::HookPhaseConfig;
use capsula_core::hook::{PhaseMarker, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde_json::json;
use std::path::Path;
use tracing::{debug, error};

/// Build hooks from configuration and execute them in order.
///
/// Returns a tuple of (JSON results array, `should_abort` flag).
/// Each hook's output includes a `__meta` field with `id`, `config`, and `success` status.
/// Failed hooks are recorded with `success: false` and an `error` field but do not stop
/// other hooks from running.
pub fn build_and_run_hooks<P: PhaseMarker>(
    run_metadata: &PreparedRun,
    runtime_params: &RuntimeParams<P>,
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

    let results: Vec<_> = hooks
        .iter()
        .enumerate()
        .map(|(idx, hook)| {
            let hook_identifier = hook_phase_config.hooks.get(idx).map_or_else(
                || format!("hook[{idx}]"),
                |config_hook| config_hook.id.clone(),
            );

            let hook_config_json = hook
                .config_as_json()
                .unwrap_or_else(|_| json!({ "__error": "Failed to serialize hook config" }));

            debug!("Running hook: {}", hook_identifier);
            match hook.run(run_metadata, runtime_params) {
                Ok(captured) => {
                    debug!("Hook '{}' completed successfully", hook_identifier);
                    let should_abort = captured.abort_requested();

                    let mut json = captured.serialize_json().unwrap_or_else(
                        |_| json!({ "__error": "Failed to serialize captured data" }),
                    );
                    if let serde_json::Value::Object(ref mut map) = json {
                        let metadata = json!({
                            "id": hook.id(),
                            "config": hook_config_json,
                            "success": true,
                        });
                        map.insert("__meta".to_string(), metadata);
                    }
                    (json, should_abort)
                }
                Err(e) => {
                    let error = anyhow::anyhow!(e);
                    error!("Failed to run {hook_identifier} (config index {idx}): {error:#}");
                    let json = json!({
                        "__meta": json!({
                            "config": hook_config_json,
                            "success": false,
                            "error": format!("{}", error)
                        })}
                    );
                    (json, false)
                }
            }
        })
        .collect();

    let json_results = results.iter().map(|(json, _)| json.clone()).collect();
    let should_abort = results.iter().any(|(_, abort)| *abort);

    Ok((json_results, should_abort))
}
