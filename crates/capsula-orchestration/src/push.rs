use anyhow::{Context, Result};
use std::path::Path;
use tracing::{debug, info};

/// Push a single run to the server.
pub fn push_single_run(
    run_dir: &Path,
    vault_name: &str,
    server_url: &str,
    client: &capsula_client::CapsulaClient,
) -> Result<()> {
    let capsula_dir = run_dir.join("_capsula");

    // Read metadata
    let metadata_path = capsula_dir.join("metadata.json");
    let metadata_content = std::fs::read_to_string(&metadata_path)
        .with_context(|| format!("Failed to read metadata from {}", metadata_path.display()))?;
    let metadata: serde_json::Value = serde_json::from_str(&metadata_content)?;

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

    // Post the run metadata
    let url = format!("{server_url}/api/v1/runs");
    let http_client = reqwest::blocking::Client::new();
    let response = http_client.post(&url).json(&create_run_payload).send()?;

    if !response.status().is_success() {
        if response.status().as_u16() == 409 {
            info!("Run already registered on server, re-uploading files: {run_name}");
        } else {
            anyhow::bail!("Failed to create run on server: {}", response.status());
        }
    }

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
    Ok(())
}
