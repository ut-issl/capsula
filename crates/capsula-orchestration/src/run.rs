use anyhow::{Context, Result, bail};
use capsula_core::hook::{PhaseMarker, PostRun, PreRun};
use capsula_core::run::{PreparedRun, UnpreparedRun};
use names::Generator;
use serde_json::Value;
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

/// Check that a run can be finalized by `run-end`, reading its
/// `pre-run.json` and `post-run.json` from `capsula_dir`.
pub fn ensure_finalizable(run_name: &str, capsula_dir: &Path) -> Result<()> {
    let pre_run_path = capsula_dir.join("pre-run.json");
    let pre_run = if pre_run_path.exists() {
        let content = std::fs::read_to_string(&pre_run_path)
            .with_context(|| format!("Failed to read {}", pre_run_path.display()))?;
        Some(
            serde_json::from_str::<Value>(&content)
                .with_context(|| format!("Failed to parse {}", pre_run_path.display()))?,
        )
    } else {
        None
    };
    let post_run_exists = capsula_dir.join("post-run.json").exists();
    check_finalizable(run_name, pre_run.as_ref(), post_run_exists)
}

/// Decide whether a run can be finalized, given the contents of its
/// `pre-run.json` (if any) and whether `post-run.json` exists.
///
/// A run whose pre-run hooks failed was never started, so finalizing it
/// would record post-run results for a command that did not run.
fn check_finalizable(run_name: &str, pre_run: Option<&Value>, post_run_exists: bool) -> Result<()> {
    if post_run_exists {
        bail!("Run '{run_name}' has already been finalized (post-run.json already exists)");
    }
    let Some(pre_run) = pre_run else {
        bail!("Run '{run_name}' cannot be finalized because its pre-run hooks were not recorded");
    };
    let report = PhaseReport::<PreRun>::from_json(pre_run)
        .with_context(|| format!("Failed to read the pre-run hook results of run '{run_name}'"))?;
    if report.is_failed() {
        bail!(
            "Run '{run_name}' cannot be finalized because a pre-run hook failed and the run was \
             not started:\n  {}",
            report.failure_lines().join("\n  ")
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "Tests use unwrap for brevity")]

    use super::*;
    use serde_json::json;

    fn pre_run(success: bool) -> Value {
        let meta = if success {
            json!({ "id": "capture-git-repo", "config": {}, "success": true })
        } else {
            json!({
                "id": "capture-git-repo",
                "config": {},
                "success": false,
                "failure_reason": "repository has uncommitted changes"
            })
        };
        json!([{ "__meta": meta }])
    }

    #[test]
    fn check_finalizable_accepts_started_run() {
        assert!(check_finalizable("r", Some(&pre_run(true)), false).is_ok());
    }

    #[test]
    fn check_finalizable_rejects_finalized_run() {
        let err = check_finalizable("r", Some(&pre_run(true)), true).unwrap_err();
        assert!(err.to_string().contains("already been finalized"), "{err}");
    }

    #[test]
    fn check_finalizable_rejects_run_whose_pre_run_failed() {
        let err = check_finalizable("r", Some(&pre_run(false)), false).unwrap_err();
        assert!(
            err.to_string()
                .contains("pre[0] capture-git-repo: repository has uncommitted changes"),
            "{err}"
        );
    }

    #[test]
    fn check_finalizable_rejects_run_without_pre_run() {
        let err = check_finalizable("r", None, false).unwrap_err();
        assert!(err.to_string().contains("not recorded"), "{err}");
    }
}
