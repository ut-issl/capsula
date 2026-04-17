use anyhow::{Context, Result};
use chrono::DateTime;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tracing::warn;
use ulid::Ulid;

/// Metadata for a single run, as stored in `_capsula/metadata.json`.
#[derive(Debug, Deserialize)]
pub struct RunMetadata {
    pub id: Ulid,
    pub name: String,
    pub command: Vec<String>,
    pub timestamp: String,
}

/// Read and parse `{run_dir}/_capsula/metadata.json`.
pub fn read_run_metadata(run_dir: &Path) -> Result<RunMetadata> {
    let metadata_path = run_dir.join("_capsula").join("metadata.json");
    let content = std::fs::read_to_string(&metadata_path)
        .with_context(|| format!("Failed to read metadata from {}", metadata_path.display()))?;
    serde_json::from_str::<RunMetadata>(&content)
        .with_context(|| format!("Failed to parse metadata from {}", metadata_path.display()))
}

/// Read and parse `{run_dir}/_capsula/metadata.json` as raw JSON.
///
/// Used by callers (e.g. the CLI `show` command) that consume metadata
/// fields beyond the structured [`RunMetadata`] struct.
pub fn read_run_metadata_json(run_dir: &Path) -> Result<serde_json::Value> {
    let metadata_path = run_dir.join("_capsula").join("metadata.json");
    let content = std::fs::read_to_string(&metadata_path)
        .with_context(|| format!("Failed to read metadata from {}", metadata_path.display()))?;
    serde_json::from_str::<serde_json::Value>(&content)
        .with_context(|| format!("Failed to parse metadata from {}", metadata_path.display()))
}

/// List all runs in a vault directory, sorted by timestamp (newest first).
pub fn list_runs(vault_dir: &Path) -> Result<Vec<RunMetadata>> {
    let mut runs = Vec::new();

    if !vault_dir.exists() {
        return Ok(runs);
    }

    for date_entry in std::fs::read_dir(vault_dir)
        .with_context(|| format!("Failed to read vault directory: {}", vault_dir.display()))?
    {
        let date_entry = date_entry?;
        let date_path = date_entry.path();

        if !date_path.is_dir() {
            continue;
        }

        for run_entry in std::fs::read_dir(&date_path)
            .with_context(|| format!("Failed to read date directory: {}", date_path.display()))?
        {
            let run_entry = run_entry?;
            let run_path = run_entry.path();

            if !run_path.is_dir() {
                continue;
            }

            let metadata_path = run_path.join("_capsula").join("metadata.json");
            if metadata_path.exists() {
                match std::fs::read_to_string(&metadata_path) {
                    Ok(content) => match serde_json::from_str::<RunMetadata>(&content) {
                        Ok(metadata) => runs.push(metadata),
                        Err(e) => {
                            warn!(
                                "Failed to parse metadata from {}: {}",
                                metadata_path.display(),
                                e
                            );
                        }
                    },
                    Err(e) => {
                        warn!(
                            "Failed to read metadata from {}: {}",
                            metadata_path.display(),
                            e
                        );
                    }
                }
            }
        }
    }

    runs.sort_by(|a, b| {
        let time_a = DateTime::parse_from_rfc3339(&a.timestamp).ok();
        let time_b = DateTime::parse_from_rfc3339(&b.timestamp).ok();

        match (time_a, time_b) {
            (Some(a), Some(b)) => b.cmp(&a),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });

    Ok(runs)
}

/// Assert that `run_dir` is a descendant of `vault_dir` once both have been
/// resolved through symlinks. Defends against a metadata file that names a
/// run outside of its parent vault.
fn ensure_within_vault(run_dir: &Path, vault_dir: &Path) -> Result<PathBuf> {
    let canonical_vault = vault_dir
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize vault {}", vault_dir.display()))?;
    let canonical_run = run_dir
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize run dir {}", run_dir.display()))?;
    if !canonical_run.starts_with(&canonical_vault) {
        anyhow::bail!(
            "Run directory {} is outside vault {}",
            canonical_run.display(),
            canonical_vault.display()
        );
    }
    Ok(run_dir.to_path_buf())
}

/// Find a run directory by ID or name.
pub fn find_run_dir(vault_dir: &Path, run_id: &str) -> Result<PathBuf> {
    for entry in walkdir::WalkDir::new(vault_dir)
        .min_depth(2)
        .max_depth(2)
        .into_iter()
        .filter_entry(|e| e.file_type().is_dir())
    {
        let entry = entry?;
        let capsula_dir = entry.path().join("_capsula");
        if !capsula_dir.exists() {
            continue;
        }

        let metadata_path = capsula_dir.join("metadata.json");
        if !metadata_path.exists() {
            continue;
        }

        let metadata_content = std::fs::read_to_string(&metadata_path)?;
        let metadata: serde_json::Value = serde_json::from_str(&metadata_content)?;

        if let Some(id) = metadata.get("id").and_then(|v| v.as_str())
            && id == run_id
        {
            return ensure_within_vault(entry.path(), vault_dir);
        }

        if let Some(name) = metadata.get("name").and_then(|v| v.as_str())
            && name == run_id
        {
            return ensure_within_vault(entry.path(), vault_dir);
        }
    }

    anyhow::bail!(
        "Run with ID or name '{}' not found in vault {}",
        run_id,
        vault_dir.display()
    )
}

/// Find a run directory by name, returning the newest if multiple matches exist.
pub fn find_run_dir_by_name(vault_dir: &Path, run_name: &str) -> Result<PathBuf> {
    let mut best_match: Option<(PathBuf, Option<DateTime<chrono::FixedOffset>>)> = None;
    let mut match_count = 0usize;

    for entry in walkdir::WalkDir::new(vault_dir)
        .min_depth(2)
        .max_depth(2)
        .into_iter()
        .filter_entry(|e| e.file_type().is_dir())
    {
        let entry = entry?;
        let capsula_dir = entry.path().join("_capsula");
        if !capsula_dir.exists() {
            continue;
        }

        let metadata_path = capsula_dir.join("metadata.json");
        if !metadata_path.exists() {
            continue;
        }

        let metadata_content = std::fs::read_to_string(&metadata_path)?;
        let metadata: RunMetadata = serde_json::from_str(&metadata_content)?;

        if metadata.name != run_name {
            continue;
        }

        match_count += 1;
        let timestamp = DateTime::parse_from_rfc3339(&metadata.timestamp).ok();
        match best_match {
            None => {
                best_match = Some((entry.path().to_path_buf(), timestamp));
            }
            Some((_, ref best_timestamp)) => {
                let is_newer = match (timestamp, best_timestamp) {
                    (Some(current), Some(best)) => current > *best,
                    (Some(_), None) => true,
                    _ => false,
                };
                if is_newer {
                    best_match = Some((entry.path().to_path_buf(), timestamp));
                }
            }
        }
    }

    if let Some((path, _)) = best_match {
        if match_count > 1 {
            warn!(
                "Found {} runs named '{}'; returning the newest by timestamp.",
                match_count, run_name
            );
        }
        return ensure_within_vault(&path, vault_dir);
    }

    anyhow::bail!(
        "Run with name '{}' not found in vault {}",
        run_name,
        vault_dir.display()
    )
}
