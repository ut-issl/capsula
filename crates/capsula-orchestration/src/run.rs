use anyhow::{Context, Result};
use capsula_core::hook::{PhaseMarker, PostRun, PreRun};
use capsula_core::run::{PreparedRun, UnpreparedRun};
use names::Generator;
use std::path::{Path, PathBuf};
use tracing::{debug, info};
use ulid::Ulid;

use crate::hooks::run_hooks;
pub use crate::hooks::{Hooks, build_hooks};
use crate::record::PhaseReport;

/// Create a new run, set up its directory structure, and write metadata.
///
/// Returns the prepared run and the path to the `_capsula` metadata directory.
pub fn create_and_setup_run(
    command: Vec<String>,
    project_root: &Path,
    vault_dir: &Path,
) -> Result<(PreparedRun, PathBuf)> {
    debug!("Creating run metadata");
    let run = UnpreparedRun {
        id: Ulid::generate(),
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
/// Returns the report of every hook; the run should abort when
/// [`PhaseReport::is_failed`] is true.
pub fn run_pre_hooks(
    run: &PreparedRun,
    capsula_dir: &Path,
    hooks: &Hooks<PreRun>,
) -> Result<PhaseReport<PreRun>> {
    debug!("Executing pre-run hooks");
    let report = run_hooks(run, hooks);
    debug!("Pre-run hooks completed");
    write_report(capsula_dir, &report)?;
    Ok(report)
}

/// Execute post-run hooks and write results to `post-run.json`.
///
/// Returns the report of every hook.
pub fn run_post_hooks(
    run: &PreparedRun,
    capsula_dir: &Path,
    hooks: &Hooks<PostRun>,
) -> Result<PhaseReport<PostRun>> {
    debug!("Executing post-run hooks");
    let report = run_hooks(run, hooks);
    debug!("Post-run hooks completed");
    write_report(capsula_dir, &report)?;
    Ok(report)
}

/// Write a phase report to `pre-run.json` / `post-run.json`.
fn write_report<P: PhaseMarker>(capsula_dir: &Path, report: &PhaseReport<P>) -> Result<()> {
    let path = capsula_dir.join(format!("{}-run.json", P::phase_name()));
    std::fs::write(&path, serde_json::to_string_pretty(&report.to_json())?).with_context(|| {
        format!(
            "Failed to write {}-run hook results to {}",
            P::phase_name(),
            path.display()
        )
    })
}
