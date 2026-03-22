use anyhow::{Context, Result};
use capsula_config::HookPhaseConfig;
use capsula_core::hook::{PostRun, PreRun, RuntimeParams};
use capsula_core::run::{PreparedRun, Run};
use names::Generator;
use std::path::{Path, PathBuf};
use tracing::{debug, info};
use ulid::Ulid;

use crate::hooks::build_and_run_hooks;

/// Create a new run, set up its directory structure, and write metadata.
///
/// Returns the prepared run and the path to the `_capsula` metadata directory.
pub fn create_and_setup_run(
    command: Vec<String>,
    project_root: &Path,
    vault_dir: &Path,
) -> Result<(PreparedRun, PathBuf)> {
    debug!("Creating run metadata");
    let run = Run::<()> {
        id: Ulid::new(),
        name: Generator::default()
            .next()
            .with_context(|| "Failed to generate a random name for the run")?,
        command,
        run_dir: (),
        project_root: project_root.to_path_buf(),
    };

    info!("Run ID: {}, Name: {}", run.id, run.name);
    debug!("Setting up run directory in vault: {}", vault_dir.display());
    let run = run.setup_run_dir(vault_dir, 5)?;
    info!("Run directory: {}", run.run_dir.to_string_lossy());

    let capsula_dir = run.run_dir.join("_capsula");
    std::fs::create_dir(&capsula_dir).with_context(|| {
        format!(
            "Failed to create _capsula directory in run directory {}",
            run.run_dir.display()
        )
    })?;

    let run_metadata_path = capsula_dir.join("metadata.json");
    std::fs::write(&run_metadata_path, serde_json::to_string_pretty(&run)?).with_context(|| {
        format!(
            "Failed to write metadata to {}",
            run_metadata_path.display()
        )
    })?;

    Ok((run, capsula_dir))
}

/// Execute pre-run hooks and write results to `pre-run.json`.
///
/// Returns whether any hook requested an abort.
pub fn run_pre_hooks(
    run: &PreparedRun,
    capsula_dir: &Path,
    config: &HookPhaseConfig,
    registry: &capsula_registry::HookRegistry<PreRun>,
    project_root: &Path,
) -> Result<bool> {
    debug!("Executing pre-run hooks");
    let pre_params = RuntimeParams::<PreRun>::default();
    let (pre_json, should_abort) =
        build_and_run_hooks(run, &pre_params, config, registry, project_root)
            .context("Failed to execute pre-run hooks")?;
    debug!("Pre-run hooks completed");

    let pre_json_path = capsula_dir.join("pre-run.json");
    std::fs::write(&pre_json_path, serde_json::to_string_pretty(&pre_json)?).with_context(
        || {
            format!(
                "Failed to write pre-run hook results to {}",
                pre_json_path.display()
            )
        },
    )?;

    Ok(should_abort)
}

/// Execute post-run hooks and write results to `post-run.json`.
pub fn run_post_hooks(
    run: &PreparedRun,
    capsula_dir: &Path,
    config: &HookPhaseConfig,
    registry: &capsula_registry::HookRegistry<PostRun>,
    project_root: &Path,
) -> Result<()> {
    debug!("Executing post-run hooks");
    let post_params = RuntimeParams::<PostRun>::default();
    let (post_json, _should_abort) =
        build_and_run_hooks::<PostRun>(run, &post_params, config, registry, project_root)
            .context("Failed to execute post-run hooks")?;
    debug!("Post-run hooks completed");

    let post_json_path = capsula_dir.join("post-run.json");
    std::fs::write(&post_json_path, serde_json::to_string_pretty(&post_json)?).with_context(
        || {
            format!(
                "Failed to write post-run hook results to {}",
                post_json_path.display()
            )
        },
    )?;

    Ok(())
}
