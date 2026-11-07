use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Push a single run to the server.
pub fn push_single_run(
    run_dir: &Path,
    vault_name: &str,
    client: &capsula_client::CapsulaClient,
) -> Result<()> {
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
    match client.create_run(&create_run_payload)? {
        capsula_client::CreateRunOutcome::AlreadyExists => {
            info!("Run already registered on server, re-uploading files: {run_name}");
        }
        capsula_client::CreateRunOutcome::Created => {}
    }

    let files = collect_artifact_files(run_dir)?;

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
    Ok(())
}

/// Collect the artifact files of a run as `(local path, path relative to the
/// run directory)` pairs, skipping the run's own `_capsula` metadata
/// directory.
///
/// Only the top-level `_capsula` is skipped: an artifact can itself lie
/// beneath a directory named `_capsula`, such as a captured
/// `results/_capsula/data.csv`.
fn collect_artifact_files(run_dir: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
    let capsula_dir = run_dir.join("_capsula");
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(run_dir)
        .into_iter()
        .filter_entry(|e| e.path() != capsula_dir)
    {
        let entry = entry?;
        if entry.file_type().is_file() {
            let local_path = entry.path();
            let relative_path = local_path.strip_prefix(run_dir)?;
            files.push((local_path.to_path_buf(), relative_path.to_path_buf()));
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn collect_artifact_files_skips_only_the_run_metadata_directory() {
        let run_dir = std::env::temp_dir().join(format!(
            "capsula_push_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(run_dir.join("_capsula")).unwrap();
        fs::create_dir_all(run_dir.join("post-0-capture-file/results/_capsula")).unwrap();
        fs::write(run_dir.join("_capsula/metadata.json"), b"{}").unwrap();
        fs::write(
            run_dir.join("post-0-capture-file/results/_capsula/data.csv"),
            b"a",
        )
        .unwrap();

        let mut relative: Vec<_> = collect_artifact_files(&run_dir)
            .unwrap()
            .into_iter()
            .map(|(_, relative)| relative)
            .collect();
        relative.sort();
        fs::remove_dir_all(&run_dir).ok();

        assert_eq!(
            relative,
            [PathBuf::from(
                "post-0-capture-file/results/_capsula/data.csv"
            )]
        );
    }
}
