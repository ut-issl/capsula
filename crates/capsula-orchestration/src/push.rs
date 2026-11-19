use anyhow::Result;
use std::fmt;
use std::path::Path;
use tracing::{debug, info};

/// How a successful push registered the run on the server.
///
/// Both variants mean that every required upload completed successfully.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushOutcome {
    /// The server created a new run.
    Created,
    /// The server already had a run with this ID (HTTP 409); its files and
    /// hook outputs, if any, were uploaded again.
    Existing,
}

/// Tally of the per-run results of `capsula push --all`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PushAllSummary {
    pub created: usize,
    pub existing: usize,
    pub skipped_pulled: usize,
    pub failed: usize,
}

impl PushAllSummary {
    /// Count a successful push.
    pub const fn record(&mut self, outcome: PushOutcome) {
        match outcome {
            PushOutcome::Created => self.created += 1,
            PushOutcome::Existing => self.existing += 1,
        }
    }

    /// Count a run skipped because it was pulled from a server.
    pub const fn record_skipped_pulled(&mut self) {
        self.skipped_pulled += 1;
    }

    /// Count a run (or vault entry) that failed to push.
    pub const fn record_failure(&mut self) {
        self.failed += 1;
    }
}

impl fmt::Display for PushAllSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Push all completed: {} created, {} re-uploaded, {} skipped (pulled runs), {} failed",
            self.created, self.existing, self.skipped_pulled, self.failed
        )
    }
}

/// Push a single run to the server.
///
/// Returns whether the run was newly created or already existed on the
/// server, once all uploads have succeeded.
pub fn push_single_run(
    run_dir: &Path,
    vault_name: &str,
    client: &capsula_client::CapsulaClient,
) -> Result<PushOutcome> {
    let capsula_dir = run_dir.join("_capsula");

    // Runs restored by `pull` are lossy reconstructions (see pull.rs);
    // re-uploading one would degrade the server's copy of the run.
    if crate::pull::is_pulled_run(run_dir) {
        anyhow::bail!(
            "Run at {} was pulled from a server (found _capsula/pulled.json); \
             refusing to push a lossy reconstruction",
            run_dir.display()
        );
    }

    // Read metadata
    let metadata = crate::vault::read_run_metadata_json(run_dir)?;

    let run_name = metadata["name"].as_str().unwrap_or("unknown");
    info!("Pushing run: {}", run_name);

    // Read pre-run and post-run hooks
    let pre_run_path = capsula_dir.join("pre-run.json");
    let pre_run_hooks = if pre_run_path.exists() {
        let content = std::fs::read_to_string(&pre_run_path)?;
        Some(serde_json::from_str::<Vec<serde_json::Value>>(&content)?)
    } else {
        None
    };

    let post_run_path = capsula_dir.join("post-run.json");
    let post_run_hooks = if post_run_path.exists() {
        let content = std::fs::read_to_string(&post_run_path)?;
        Some(serde_json::from_str::<Vec<serde_json::Value>>(&content)?)
    } else {
        None
    };

    // Read command output
    let command_json_path = capsula_dir.join("command.json");
    let command_output = if command_json_path.exists() {
        let content = std::fs::read_to_string(&command_json_path)?;
        serde_json::from_str::<serde_json::Value>(&content)?
    } else {
        serde_json::json!({})
    };

    // Convert duration object to milliseconds
    let duration_ms = command_output.get("duration").and_then(|d| {
        let secs = d.get("secs")?.as_u64()?;
        let nanos = d.get("nanos")?.as_u64()?;
        Some((secs * 1000) + (nanos / 1_000_000))
    });

    let create_run_payload = serde_json::json!({
        "id": metadata["id"],
        "name": metadata["name"],
        "timestamp": metadata["timestamp"],
        "command": serde_json::to_string(&metadata["command"])?,
        "vault": vault_name,
        "project_root": metadata["project_root"],
        "exit_code": command_output.get("exit_code"),
        "duration_ms": duration_ms,
        "stdout": command_output.get("stdout"),
        "stderr": command_output.get("stderr"),
    });

    // Post the run metadata through the authenticated client
    let outcome = match client.create_run(&create_run_payload)? {
        capsula_client::CreateRunOutcome::AlreadyExists => {
            info!("Run already registered on server, re-uploading files: {run_name}");
            PushOutcome::Existing
        }
        capsula_client::CreateRunOutcome::Created => PushOutcome::Created,
    };

    // Collect files to upload
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(run_dir)
        .into_iter()
        .filter_entry(|e| e.file_name() != "_capsula")
    {
        let entry = entry?;
        if entry.file_type().is_file() {
            let local_path = entry.path();
            let relative_path = local_path.strip_prefix(run_dir)?;
            files.push((local_path.to_path_buf(), relative_path.to_path_buf()));
        }
    }

    // Upload files and hooks
    if !files.is_empty() || pre_run_hooks.is_some() || post_run_hooks.is_some() {
        let actual_run_id = metadata["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Run ID not found in metadata"))?;
        let response = client.upload_run(actual_run_id, &files, pre_run_hooks, post_run_hooks)?;
        debug!(
            "Upload complete: {} files, {} bytes",
            response.files_processed, response.total_bytes
        );
    }

    info!("Pushed: {}", run_name);
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_counts_each_outcome() {
        let mut summary = PushAllSummary::default();
        summary.record(PushOutcome::Created);
        summary.record(PushOutcome::Created);
        summary.record(PushOutcome::Existing);
        summary.record_skipped_pulled();
        summary.record_failure();

        assert_eq!(
            summary,
            PushAllSummary {
                created: 2,
                existing: 1,
                skipped_pulled: 1,
                failed: 1,
            }
        );
    }

    #[test]
    fn summary_formats_counts() {
        let summary = PushAllSummary {
            created: 2,
            existing: 1,
            skipped_pulled: 0,
            failed: 0,
        };
        assert_eq!(
            summary.to_string(),
            "Push all completed: 2 created, 1 re-uploaded, 0 skipped (pulled runs), 0 failed"
        );
    }
}
